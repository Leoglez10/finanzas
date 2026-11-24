use chrono::NaiveDate;
use finanzas_core::{service, store::{EntryFilter, Store}};
use serde_json::{json, Value};
use crate::args::Args;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{0}")]
pub struct ToolError(pub String);
#[derive(Debug)]
pub struct ToolSpec { pub name: &'static str, pub description: &'static str, pub input_schema: Value, pub read_only: bool, pub destructive: bool }
fn schema(properties: Value, required: &[&str]) -> Value { json!({"type":"object","properties":properties,"required":required}) }
fn spec(name: &'static str, description: &'static str, properties: Value, required: &[&str], read_only: bool, destructive: bool) -> ToolSpec {
    let mut properties = properties;
    properties.as_object_mut().expect("tool properties are an object").insert("perfil".into(), json!({"type":"string","description":"Profile whose data to use: personal (default) or a profile created in the app, e.g. negocio"}));
    ToolSpec { name, description, input_schema: schema(properties, required), read_only, destructive }
}
pub fn specs() -> Vec<ToolSpec> {
    let date = json!({"type":"string","description":"YYYY-MM-DD"});
    let month = json!({"type":"string","description":"YYYY-MM"});
    let amount = json!({"type":"number","description":"Decimal currency units, max 2 decimals; negative = outflow"});
    vec![
        spec("list_accounts", "List all accounts with id, name, off-budget/closed flags and current balance.", json!({}), &[], true, false),
        spec("get_transactions", "Get transactions in a date range (newest first), optionally for one account and/or category. Amounts: negative = outflow.", json!({"accountId":{"type":"string","description":"Account id; all accounts if omitted"},"startDate":date,"endDate":date,"categoryId":{"type":"string","description":"Only transactions in this category"},"limit":{"type":"integer","minimum":1,"maximum":1000,"description":"Max rows (default 100, max 1000)"}}), &["startDate","endDate"], true, false),
        spec("list_categories", "List category groups with their categories (ids, hidden and income flags).", json!({}), &[], true, false),
        spec("get_budget_month", "Envelope budget for a month: money left to budget, income, and per category the assigned, spent and available amounts.", json!({"month":month}), &["month"], true, false),
        spec("spending_by_category", "Total outflows per category in a date range, largest first. Inflows are excluded, and so are off-budget accounts unless includeOffBudget is true.", json!({"startDate":date,"endDate":date,"includeOffBudget":{"type":"boolean","description":"Also count off-budget accounts (default false)"}}), &["startDate","endDate"], true, false),
        spec("add_transaction", "Add a transaction to an account. The payee is given by name: an existing payee is reused (case-insensitive), otherwise it is created. Without categoryId, the category last used with that payee is applied. Cleared by default. Returns the transaction.", json!({"accountId":{"type":"string","description":"Account id (see list_accounts)"},"date":date,"amount":amount,"payee":{"type":"string","description":"Payee name"},"categoryId":{"type":"string","description":"Category id (see list_categories)"},"notes":{"type":"string"},"cleared":{"type":"boolean","description":"Mark as cleared (default true)"}}), &["accountId","date","amount"], false, false),
        spec("update_transaction", "Change fields of an existing transaction; only the fields given are changed, at least one is required. The payee is given by name (reused case-insensitively, created if new).", json!({"id":{"type":"string","description":"Transaction id (see get_transactions)"},"date":date,"amount":amount,"payee":{"type":"string","description":"Payee name (non-empty)"},"categoryId":{"type":"string","description":"Category id"},"notes":{"type":"string","description":"Empty string clears the notes"},"cleared":{"type":"boolean"}}), &["id"], false, false),
        spec("delete_transaction", "Permanently delete a transaction by id. Returns the deleted transaction.", json!({"id":{"type":"string","description":"Transaction id (see get_transactions)"}}), &["id"], false, true),
        spec("set_budget_amount", "Set the amount assigned to an expense category for a month. Income categories cannot be budgeted. Returns the resulting assigned amount.", json!({"month":month,"categoryId":{"type":"string","description":"Expense category id (see list_categories)"},"amount":{"type":"number","minimum":0,"description":"Decimal currency units, 0 or more, max 2 decimals"}}), &["month","categoryId","amount"], false, false),
        spec("list_schedules", "List scheduled payments with id, name, payee, account, amount, frequency, interval, next date, completed and auto-post flags.", json!({}), &[], true, false),
        spec("skip_schedule", "Skip the next occurrence of a scheduled payment once, or with `through` keep skipping every occurrence dated on or before that day (max 500 skips per call). Completed schedules cannot be skipped.", json!({"id":{"type":"string","description":"Schedule id (see list_schedules)"},"through":{"type":"string","description":"YYYY-MM-DD; skip all occurrences up to and including this date"}}), &["id"], false, false),
        spec("list_wishes", "List wishes and current-month affordability evaluations. Amounts are decimal currency units.", json!({"includeArchived":{"type":"boolean","description":"Include archived wishes (default false)"}}), &[], true, false),
        spec("save_wish", "Create a wish or replace details when id is supplied. Changing category releases funds. Does not record a purchase.", json!({"id":{"type":"string"},"name":{"type":"string"},"price":{"type":"number","exclusiveMinimum":0},"url":{"type":"string","description":"HTTP(S) link; default empty"},"notes":{"type":"string"},"categoryId":{"type":["string","null"],"description":"Expense category; omitted/null clears it"},"kind":{"type":"string","enum":["compra","viaje","experiencia","fondo"],"description":"Goal type; default compra"},"targetDate":{"type":["string","null"],"description":"YYYY-MM-DD the goal is for; omitted/null clears it"},"items":{"type":"array","description":"Line items (flight, hotel…); their sum becomes the price. Omitted keeps the current items; [] removes them. Send an item's id to keep it.","items":{"type":"object","properties":{"id":{"type":"string"},"name":{"type":"string"},"amount":{"type":"number","exclusiveMinimum":0},"paid":{"type":"boolean"}},"required":["name","amount"]}}}), &["name","price"], false, false),
        spec("reserve_wish", "Set absolute reserved amount, not a delta. Zero releases funds. Partial reservations allowed; increases require unreserved current-month category funds. Does not move money or create expenses.", json!({"id":{"type":"string"},"amount":{"type":"number","minimum":0}}), &["id","amount"], false, false),
        spec("archive_wish", "Archive a wish and release its reservation. Keeps history and does not create or delete transactions.", json!({"id":{"type":"string"}}), &["id"], false, false),
        spec("evaluate_wish", "Evaluate current-month budget and outgoing payments for 30 days including overdue ones. Reports budget/cash shortfalls and unknown data. No future income or off-budget/closed accounts; not a guarantee.", json!({"id":{"type":"string"}}), &["id"], true, false),
        spec("get_upcoming", "Upcoming scheduled payments from overdue ones through the next `days` days (default 30), oldest first, with each account's projected balance after the payment.", json!({"days":{"type":"integer","minimum":0,"maximum":365,"description":"Days ahead (default 30)"}}), &[], true, false),
        spec("get_debts", "Debts: credit cards (with optional terms) and other accounts with a negative balance, highest rate first. Per debt: amount owed, credit limit and available, last/next cut, due date and overdue flag, statement balance, amount left to pay to avoid interest, minimum payment, estimated interest if only the minimum is paid (16% VAT included), installment purchases, and `missing` card terms. Null fields need terms that were not entered.", json!({}), &[], true, false),
        spec("set_card_terms", "Mark an account as a credit card and set its terms, replacing the previous ones: omitted or null terms are cleared (every term is optional). isCard false stops treating it as a card; its installments are kept. Returns the account as get_debts reports it.", json!({"accountId":{"type":"string","description":"Account id (see list_accounts)"},"isCard":{"type":"boolean","description":"Default true; false removes the card terms"},"creditLimit":{"type":["number","null"],"exclusiveMinimum":0,"description":"Credit limit in decimal currency units"},"cutDay":{"type":["integer","null"],"minimum":1,"maximum":31,"description":"Statement cut day of the month"},"dueDay":{"type":["integer","null"],"minimum":1,"maximum":31,"description":"Payment due day of the month"},"annualRatePercent":{"type":["number","null"],"minimum":0,"maximum":1000,"description":"Annual interest rate in percent, e.g. 45.5"},"minPaymentPercent":{"type":["number","null"],"minimum":0,"maximum":100,"description":"Minimum payment as a percent of the statement balance"}}), &["accountId"], false, false),
        spec("save_installment", "Create a purchase paid in monthly installments on a card, or replace one when id is supplied. Record the full purchase as a transaction on the card separately; this only splits it so each cut bills one monthly charge. annualRatePercent 0 (default) means without interest (MSI). Returns the plan with its monthly charge and months left.", json!({"id":{"type":"string","description":"Installment id (see get_debts) to replace"},"accountId":{"type":"string","description":"Card account id"},"name":{"type":"string"},"total":{"type":"number","exclusiveMinimum":0,"description":"Purchase total in decimal currency units"},"months":{"type":"integer","minimum":1,"maximum":120},"startDate":{"type":"string","description":"YYYY-MM-DD purchase date"},"annualRatePercent":{"type":"number","minimum":0,"maximum":1000,"description":"Default 0 (without interest)"}}), &["accountId","name","total","months","startDate"], false, false),
        spec("delete_installment", "Permanently delete an installment plan by id. Does not touch transactions. Returns the deleted plan.", json!({"id":{"type":"string","description":"Installment id (see get_debts)"}}), &["id"], false, true),
        spec("create_account", "Create an account at the end of the list. A non-zero openingBalance is recorded as a \"Saldo inicial\" transaction on date (default today; negative for a debt such as a card). Use set_card_terms afterwards to make it a credit card. Returns the account with its balance.", json!({"name":{"type":"string"},"offBudget":{"type":"boolean","description":"For savings or investments kept out of the budget (default false)"},"openingBalance":{"type":"number","description":"Current balance in decimal currency units; negative = owed"},"date":date.clone()}), &["name"], false, false),
        spec("create_category", "Create a category at the end of a group (see list_categories for group ids).", json!({"groupId":{"type":"string"},"name":{"type":"string"}}), &["groupId","name"], false, false),
        spec("update_category", "Rename a category and/or hide or show it; only the fields given change, at least one is required.", json!({"id":{"type":"string","description":"Category id"},"name":{"type":"string"},"hidden":{"type":"boolean"}}), &["id"], false, false),
        spec("save_category_group", "Rename a category group, or create an expense group at the end when id is omitted.", json!({"id":{"type":"string","description":"CategoryGroup id to rename"},"name":{"type":"string"}}), &["name"], false, false),
        spec("save_schedule", "Create a scheduled payment or income, or replace one when id is supplied. The payee is given by name (reused or created). An autoPost schedule already due is recorded right away. Returns the schedule as list_schedules shows it.", json!({"id":{"type":"string","description":"Schedule id to replace"},"name":{"type":"string"},"amount":amount.clone(),"accountId":{"type":"string"},"categoryId":{"type":"string"},"payee":{"type":"string"},"frequency":{"type":"string","enum":["daily","weekly","monthly","yearly"],"description":"Default monthly"},"interval":{"type":"integer","minimum":1,"maximum":52,"description":"Every N periods (default 1)"},"start":{"type":"string","description":"YYYY-MM-DD first occurrence"},"skipWeekend":{"type":"boolean","description":"Move weekend dates to Monday (default false)"},"autoPost":{"type":"boolean","description":"Record it automatically when due (default false)"}}), &["name","amount","accountId","start"], false, false),
        spec("post_schedule", "Record the next occurrence of a scheduled payment as a transaction now and move the schedule forward. Returns the transaction.", json!({"id":{"type":"string","description":"Schedule id"}}), &["id"], false, false),
        spec("delete_schedule", "Permanently delete a scheduled payment. Transactions it already recorded stay. Returns the deleted schedule.", json!({"id":{"type":"string","description":"Schedule id"}}), &["id"], false, true),
        spec("set_wish_item_paid", "Mark a goal's line item as paid or unpaid. Money set aside beyond what is still unpaid is released. Returns the goal.", json!({"id":{"type":"string","description":"WishItem id (see list_wishes)"},"paid":{"type":"boolean"}}), &["id","paid"], false, false),
        spec("complete_wish", "Complete a goal: records its expense on the account (one per unpaid line item, or one for the price) and archives it as an achievement. All or nothing. Returns the goal and the new transactions.", json!({"id":{"type":"string","description":"Wish id"},"accountId":{"type":"string","description":"Account that pays"}}), &["id","accountId"], false, false),
        spec("tag_transaction", "Tag a transaction with a goal and optionally one of its line items, to compare spent with planned; wishId null removes the tag. Returns the transaction.", json!({"id":{"type":"string","description":"Transaction id"},"wishId":{"type":["string","null"]},"itemId":{"type":["string","null"],"description":"WishItem id of that goal"}}), &["id"], false, false),
        spec("get_overview", "Summary like the app's Resumen for the `months` months ending at `month` (default the current month): balances, income, expenses, savings rate compared with the same number of months before, spending per category and budget progress.", json!({"month":month.clone(),"months":{"type":"integer","minimum":1,"maximum":24,"description":"Months in the period (default 1)"}}), &[], true, false),
        spec("add_transactions", "Add up to 500 transactions at once, all or none (like the app's CSV import). Each item takes the add_transaction fields. Returns how many were added and the transactions.", json!({"transactions":{"type":"array","minItems":1,"maxItems":500,"items":{"type":"object","properties":{"accountId":{"type":"string"},"date":date.clone(),"amount":amount.clone(),"payee":{"type":"string"},"categoryId":{"type":"string"},"notes":{"type":"string"},"cleared":{"type":"boolean"}},"required":["accountId","date","amount"]}}}), &["transactions"], false, false),
        spec("list_profiles", "List the profiles created in the app; pass one as `perfil` to any tool to use its data.", json!({}), &[], true, false),
    ]
}
/// The `perfil` argument, `personal` when omitted.
pub fn profile(args: &Value) -> Result<String, ToolError> {
    match args.get("perfil") {
        None => Ok("personal".into()),
        Some(v) => v.as_str().map(str::to_owned).ok_or_else(|| ToolError("perfil: expected a string".into())),
    }
}
fn err(e: impl std::fmt::Display) -> ToolError { ToolError(e.to_string()) }
fn id(entity: &str, n: i64) -> String { format!("{entity}/p{n}") }
fn oid(entity: &str, n: Option<i64>) -> Option<String> { n.map(|n| id(entity, n)) }
fn dec(n: i64) -> f64 { n as f64 / 100.0 }
fn required_id(a: &Args<'_>, key: &str, entity: &str) -> Result<i64, ToolError> {
    a.required_string(key)?;
    a.id(key, entity)?.ok_or_else(|| ToolError(format!("{key}: required")))
}
fn entry_json(r: finanzas_core::store::EntryRow) -> Value {
    json!({"id":id("LedgerEntry",r.id),"date":r.date.to_string(),"amount":dec(r.amount),"accountId":oid("Account",r.account_id),"account":r.account_name,"payee":r.payee_name,"categoryId":oid("Category",r.category_id),"category":r.category_name,"notes":r.notes,"cleared":r.cleared,"scheduleId":oid("Schedule",r.schedule_id),"wishId":oid("Wish",r.wish_id),"wishItemId":oid("WishItem",r.wish_item_id)})
}
fn category(a: &Args<'_>, store: &Store) -> Result<Option<i64>, ToolError> {
    let category = a.id("categoryId", "Category")?;
    if let Some(n) = category { if store.get_category(n).map_err(err)?.is_none() { return Err(ToolError("categoryId: not found".into())); } }
    Ok(category)
}
fn wish_json(w: service::Wish) -> Value {
    json!({"id":id("Wish",w.id),"name":w.name,"price":dec(w.price),"url":w.url,"notes":w.notes,
        "categoryId":oid("Category",w.category_id),"reserved":dec(w.reserved),"archived":w.archived,
        "kind":w.kind.as_str(),"targetDate":w.target_date.map(|d| d.to_string()),"paid":dec(w.paid),"achievedDate":w.achieved_date.map(|d| d.to_string())})
}
fn plan_json(p: service::WishPlan) -> Value {
    json!({"targetDate":p.target_date.to_string(),"remaining":dec(p.remaining),"monthsLeft":p.months_left,"monthly":dec(p.monthly),"overdue":p.overdue})
}
fn items(a: &Args) -> Result<Option<Vec<service::WishItemDraft>>, ToolError> {
    let Some(list) = a.0.get("items").filter(|v| !v.is_null()) else { return Ok(None) };
    let list = list.as_array().ok_or_else(|| ToolError("items: expected an array".into()))?;
    list.iter().map(|v| {
        let i = Args(v);
        Ok(service::WishItemDraft { id: i.id("id", "WishItem")?, name: i.optional_string("name")?.unwrap_or("").into(), amount: i.cents("amount")?, paid: i.bool("paid")?.unwrap_or(false) })
    }).collect::<Result<Vec<_>, _>>().map(Some)
}
fn kind(a: &Args) -> Result<service::WishKind, ToolError> {
    match a.optional_string("kind")? {
        None => Ok(service::WishKind::Compra),
        Some(text) => ["compra","viaje","experiencia","fondo"].contains(&text).then(|| service::WishKind::parse(text))
            .ok_or_else(|| ToolError("kind: expected compra, viaje, experiencia or fondo".into())),
    }
}
fn new_entry(a: &Args, store: &Store) -> Result<service::NewEntry, ToolError> {
    let account = required_id(a,"accountId","Account")?;
    if store.get_account(account).map_err(err)?.is_none() { return Err(ToolError("accountId: not found".into())); }
    let category_id = category(a,store)?;
    let payee = a.optional_string("payee")?.map(str::to_owned);
    let date = a.date("date")?;
    let amount = a.cents("amount")?;
    let notes = a.optional_string("notes")?.unwrap_or("").to_owned();
    let cleared = a.bool("cleared")?.unwrap_or(true);
    Ok(service::NewEntry { date,amount,account_id:account,payee,category_id,notes,cleared })
}
fn schedule_json(v: service::ScheduleView) -> Value {
    json!({"id":id("Schedule",v.schedule.id),"name":v.schedule.name,"amount":dec(v.schedule.amount),"payee":v.payee,"accountId":oid("Account",v.schedule.account_id),"account":v.account,"categoryId":oid("Category",v.schedule.category_id),"frequency":v.schedule.recurrence.as_ref().map(|r| r.frequency),"interval":v.schedule.recurrence.as_ref().map(|r| r.interval).unwrap_or(1),"nextDate":v.next_date.map(|d| d.to_string()),"completed":v.schedule.completed,"autoPost":v.schedule.auto_post})
}
fn schedule_by_id(store: &Store, n: i64) -> Result<Value, ToolError> {
    service::schedule_views(store).map_err(err)?.into_iter().find(|v| v.schedule.id == n).map(schedule_json).ok_or_else(|| ToolError("id: not found".into()))
}
fn account_json(store: &Store, n: i64) -> Result<Value, ToolError> {
    let a = store.get_account(n).map_err(err)?.ok_or_else(|| ToolError("accountId: not found".into()))?;
    let balance = store.account_balances().map_err(err)?.get(&n).copied().unwrap_or(0);
    Ok(json!({"id":id("Account",a.id),"name":a.name,"offBudget":a.off_budget,"closed":a.closed,"balance":dec(balance)}))
}
fn category_json(c: finanzas_core::Category) -> Value { json!({"id":id("Category",c.id),"name":c.name,"hidden":c.hidden,"groupId":oid("CategoryGroup",c.group_id)}) }
/// Maps the service's NotFound to the argument that names the missing thing.
fn missing(key: &'static str) -> impl Fn(service::ServiceError) -> ToolError { move |e| match e { service::ServiceError::NotFound => ToolError(format!("{key}: not found")), e => err(e) } }
fn summary_json(m: &service::MonthSummaryView) -> Value { json!({"income":dec(m.income),"expenses":dec(m.expenses),"budgeted":dec(m.budgeted),"savingsRate":m.savings_rate}) }
fn account(a: &Args, store: &Store) -> Result<i64, ToolError> {
    let n = required_id(a, "accountId", "Account")?;
    if store.get_account(n).map_err(err)?.is_none() { return Err(ToolError("accountId: not found".into())); }
    Ok(n)
}
/// Money greater than zero, or None when omitted/null.
fn positive(a: &Args, key: &str) -> Result<Option<i64>, ToolError> {
    if a.0.get(key).is_none_or(Value::is_null) { return Ok(None); }
    let n = a.cents(key)?;
    if n <= 0 { return Err(ToolError(format!("{key}: must be greater than 0"))); }
    Ok(Some(n))
}
fn day_of_month(a: &Args, key: &str) -> Result<Option<u32>, ToolError> {
    if a.0.get(key).is_none_or(Value::is_null) { return Ok(None); }
    a.int(key, 0, 1, 31).map(|d| Some(d as u32))
}
/// A percentage with up to 2 decimals, as basis points (45.5 → 4550).
fn percent(a: &Args, key: &str, max: i64) -> Result<Option<i64>, ToolError> {
    if a.0.get(key).is_none_or(Value::is_null) { return Ok(None); }
    let bp = a.cents(key).map_err(|_| ToolError(format!("{key}: expected 0 to {max}")))?;
    if !(0..=max * 100).contains(&bp) { return Err(ToolError(format!("{key}: expected 0 to {max}"))); }
    Ok(Some(bp))
}
fn installment_json(p: &service::Installment, view: Option<&service::InstallmentView>) -> Value {
    json!({"id":id("Installment",p.id),"accountId":id("Account",p.account_id),"name":p.name,"total":dec(p.total),"months":p.months,
        "startDate":p.start_date.to_string(),"annualRatePercent":p.annual_rate as f64 / 100.0,
        "monthly":view.map(|v| dec(v.monthly)),"monthsLeft":view.map(|v| v.left),"remaining":view.map(|v| dec(v.remaining))})
}
fn installment_view(store: &Store, today: NaiveDate, n: i64) -> Result<Option<Value>, ToolError> {
    Ok(service::debts(store, today).map_err(err)?.debts.iter().flat_map(|d| &d.installments).find(|p| p.installment.id == n).map(|p| installment_json(&p.installment, Some(p))))
}
fn debt_json(d: service::DebtView) -> Value {
    let t = d.terms.clone().unwrap_or_default();
    let pct = |bp: Option<i64>| bp.map(|b| b as f64 / 100.0);
    json!({"accountId":id("Account",d.account_id),"name":d.name,"isCard":d.is_card,"debt":dec(d.debt),
        "creditLimit":t.credit_limit.map(dec),"available":d.available.map(dec),"annualRatePercent":pct(t.annual_rate),"minPaymentPercent":pct(t.min_payment),
        "cutDay":t.cut_day,"dueDay":t.due_day,"lastCut":d.last_cut.map(|x| x.to_string()),"nextCut":d.next_cut.map(|x| x.to_string()),
        "dueDate":d.due_date.map(|x| x.to_string()),"overdue":d.overdue,"statement":d.statement.map(dec),"toAvoidInterest":d.to_avoid_interest.map(dec),
        "minimum":d.minimum.map(dec),"interestIfMinimum":d.interest_if_minimum.map(dec),"missing":d.missing,
        "installments":d.installments.iter().map(|p| installment_json(&p.installment, Some(p))).collect::<Vec<_>>()})
}
fn evaluation_json(e: service::WishEvaluation) -> Value {
    json!({"status":e.status,"asOf":e.as_of.to_string(),"horizonDays":e.horizon_days,
        "categoryAvailable":e.category_available.map(dec),"availableForWish":e.available_for_wish.map(dec),
        "budgetShortfall":e.budget_shortfall.map(dec),"reservationDeficit":dec(e.reservation_deficit),
        "cashBalance":e.cash_balance.map(dec),"upcomingOutflows":dec(e.upcoming_outflows),
        "cashShortfall":e.cash_shortfall.map(dec),"reasons":e.reasons})
}
pub fn call(store: &mut Store, name: &str, args: &Value, today: NaiveDate) -> Result<Value, ToolError> {
    let a = Args(args);
    Ok(match name {
        "list_wishes" => json!(service::list_wishes(store, today, a.bool("includeArchived")?.unwrap_or(false)).map_err(err)?.into_iter()
            .map(|v| json!({"wish":wish_json(v.wish),"evaluation":evaluation_json(v.evaluation),"plan":v.plan.map(plan_json),
                "spending":json!({"total":dec(v.spending.total),"other":dec(v.spending.other),"items":v.spending.items.into_iter().map(|i| json!({"itemId":id("WishItem",i.item_id),"spent":dec(i.spent)})).collect::<Vec<_>>()}),
                "contributions":v.contributions.into_iter().map(|c| json!({"date":c.date.to_string(),"amount":dec(c.amount)})).collect::<Vec<_>>(),
                "items":v.items.into_iter().map(|i| json!({"id":id("WishItem",i.id),"name":i.name,"amount":dec(i.amount),"paid":i.paid})).collect::<Vec<_>>()})).collect::<Vec<_>>()),
        "save_wish" => {
            let draft = service::WishDraft { name:a.required_string("name")?.into(), price:a.cents("price")?,
                url:a.optional_string("url")?.unwrap_or("").into(), notes:a.optional_string("notes")?.unwrap_or("").into(),
                category_id:category(&a,store)?, kind:kind(&a)?, target_date:a.optional_date("targetDate")?, items:items(&a)? };
            wish_json(service::save_wish(store,a.id("id","Wish")?,draft).map_err(err)?)
        }
        "reserve_wish" => {
            let n = required_id(&a,"id","Wish")?;
            wish_json(service::reserve_wish(store,n,a.cents("amount")?,today).map_err(err)?)
        }
        "archive_wish" => {
            let n = required_id(&a,"id","Wish")?;
            wish_json(service::archive_wish(store,n).map_err(err)?)
        }
        "evaluate_wish" => {
            let n = required_id(&a,"id","Wish")?;
            evaluation_json(service::evaluate_wish(store,n,today).map_err(err)?)
        }
        "list_accounts" => {
            let balances = store.account_balances().map_err(err)?;
            json!(store.list_accounts().map_err(err)?.into_iter().map(|v| json!({"id":id("Account",v.id),"name":v.name,"offBudget":v.off_budget,"closed":v.closed,"balance":dec(*balances.get(&v.id).unwrap_or(&0))})).collect::<Vec<_>>())
        }
        "get_transactions" => {
            let (start,end) = a.range()?;
            let account = a.id("accountId", "Account")?;
            if let Some(n) = account { if store.get_account(n).map_err(err)?.is_none() { return Err(ToolError("accountId: not found".into())); } }
            let category = a.id("categoryId", "Category")?;
            if let Some(n) = category { if store.get_category(n).map_err(err)?.is_none() { return Err(ToolError("categoryId: not found".into())); } }
            let limit = a.int("limit",100,1,1000)? as usize;
            let (rows,total) = store.entries(&EntryFilter { account_id:account,category_id:category,start:Some(start),end:Some(end),limit:Some(limit),..Default::default() }).map_err(err)?;
            json!({"total":total,"truncated":total>limit,"transactions":rows.into_iter().map(|r| json!({"id":id("LedgerEntry",r.id),"date":r.date.to_string(),"amount":dec(r.amount),"accountId":oid("Account",r.account_id),"account":r.account_name,"payee":r.payee_name,"categoryId":oid("Category",r.category_id),"category":r.category_name,"notes":r.notes,"cleared":r.cleared,"scheduleId":oid("Schedule",r.schedule_id),"wishId":oid("Wish",r.wish_id),"wishItemId":oid("WishItem",r.wish_item_id)})).collect::<Vec<_>>()})
        }
        "list_categories" => {
            let categories = store.list_categories().map_err(err)?;
            json!(store.list_category_groups().map_err(err)?.into_iter().map(|g| json!({"id":id("CategoryGroup",g.id),"name":g.name,"isIncome":g.is_income,"categories":categories.iter().filter(|c| c.group_id==Some(g.id)).map(|c| json!({"id":id("Category",c.id),"name":c.name,"hidden":c.hidden})).collect::<Vec<_>>()})).collect::<Vec<_>>())
        }
        "get_budget_month" => {
            let month = a.month("month")?;
            let mut view = service::budget_month(store,month).map_err(err)?;
            let groups = store.list_category_groups().map_err(err)?;
            view.groups.sort_by_key(|group| groups.iter().position(|g| g.id == group.id).unwrap_or(usize::MAX));
            json!({"month":format!("{:04}-{:02}",month/100,month%100),"toBudget":dec(view.to_budget),"income":dec(view.income),"budgeted":dec(view.assigned),"groups":view.groups.into_iter().map(|g| json!({"id":id("CategoryGroup",g.id),"name":g.name,"isIncome":g.is_income,"categories":g.categories.into_iter().map(|c| json!({"id":id("Category",c.id),"name":c.name,"budgeted":dec(c.budgeted),"spent":dec(c.activity),"available":dec(c.available)})).collect::<Vec<_>>()})).collect::<Vec<_>>()})
        }
        "spending_by_category" => {
            let (start,end) = a.range()?;
            let view = service::spending_by_category(store,start,end,a.bool("includeOffBudget")?.unwrap_or(false)).map_err(err)?;
            json!({"startDate":start.to_string(),"endDate":end.to_string(),"total":dec(view.total),"categories":view.categories.into_iter().map(|c| json!({"categoryId":oid("Category",c.category_id),"category":c.category,"spent":dec(c.spent)})).collect::<Vec<_>>()})
        }
        "list_schedules" => {
            let mut rows = service::schedule_views(store).map_err(err)?;
            rows.sort_by(|a,b| a.schedule.name.cmp(&b.schedule.name));
            json!(rows.into_iter().map(schedule_json).collect::<Vec<_>>())
        }
        "get_debts" => {
            let v = service::debts(store, today).map_err(err)?;
            json!({"total":dec(v.total),"monthlyInstallments":dec(v.monthly_installments),"debts":v.debts.into_iter().map(debt_json).collect::<Vec<_>>()})
        }
        "set_card_terms" => {
            let account = account(&a, store)?;
            let terms = if a.bool("isCard")?.unwrap_or(true) { Some(service::CardTerms {
                credit_limit: positive(&a, "creditLimit")?, cut_day: day_of_month(&a, "cutDay")?, due_day: day_of_month(&a, "dueDay")?,
                annual_rate: percent(&a, "annualRatePercent", 1000)?, min_payment: percent(&a, "minPaymentPercent", 100)?,
            }) } else { None };
            service::set_card_terms(store, account, terms).map_err(err)?;
            // A card that stopped being one and owes nothing is no longer listed.
            service::debts(store, today).map_err(err)?.debts.into_iter().find(|d| d.account_id == account).map(debt_json)
                .unwrap_or_else(|| json!({"accountId":id("Account",account),"isCard":false,"debt":0.0}))
        }
        "save_installment" => {
            let plan_id = a.id("id", "Installment")?;
            let account_id = account(&a, store)?;
            let name = a.required_string("name")?.trim();
            if name.is_empty() { return Err(ToolError("name: required".into())); }
            let total = positive(&a, "total")?.ok_or_else(|| ToolError("total: required".into()))?;
            if a.0.get("months").is_none_or(Value::is_null) { return Err(ToolError("months: expected an integer from 1 to 120".into())); }
            let draft = service::InstallmentDraft { account_id, name: name.into(), total, months: a.int("months", 0, 1, 120)? as u32,
                start_date: a.date("startDate")?, annual_rate: percent(&a, "annualRatePercent", 1000)?.unwrap_or(0) };
            // The account was checked above, so NotFound here means the plan id.
            let plan = service::save_installment(store, plan_id, draft).map_err(|e| match e { service::ServiceError::NotFound => ToolError("id: not found".into()), e => err(e) })?;
            installment_view(store, today, plan.id)?.unwrap_or_else(|| installment_json(&plan, None))
        }
        "delete_installment" => {
            let n = required_id(&a, "id", "Installment")?;
            let plan = service::delete_installment(store, n).map_err(|_| ToolError("id: not found".into()))?;
            json!({"deleted": installment_json(&plan, None)})
        }
        "create_account" => {
            let name = a.required_string("name")?.trim();
            if name.is_empty() { return Err(ToolError("name: required".into())); }
            let opening = if a.0.get("openingBalance").is_none_or(Value::is_null) { None } else { Some(a.cents("openingBalance")?) };
            let created = service::create_account(store, name, a.bool("offBudget")?.unwrap_or(false), opening, a.optional_date("date")?.unwrap_or(today), None).map_err(err)?;
            account_json(store, created.id)?
        }
        "create_category" => {
            let group = required_id(&a, "groupId", "CategoryGroup")?;
            let name = a.required_string("name")?;
            if name.trim().is_empty() { return Err(ToolError("name: required".into())); }
            category_json(service::create_category(store, group, name).map_err(missing("groupId"))?)
        }
        "update_category" => {
            let n = required_id(&a, "id", "Category")?;
            let (name, hidden) = (a.optional_string("name")?, a.bool("hidden")?);
            if name.is_none() && hidden.is_none() { return Err(ToolError("Provide name or hidden".into())); }
            if name.is_some_and(|n| n.trim().is_empty()) { return Err(ToolError("name: must not be empty".into())); }
            category_json(service::update_category(store, n, name, hidden).map_err(missing("id"))?)
        }
        "save_category_group" => {
            let name = a.required_string("name")?;
            if name.trim().is_empty() { return Err(ToolError("name: required".into())); }
            let g = service::save_category_group(store, a.id("id", "CategoryGroup")?, name).map_err(missing("id"))?;
            json!({"id":id("CategoryGroup",g.id),"name":g.name,"isIncome":g.is_income})
        }
        "save_schedule" => {
            let n = a.id("id", "Schedule")?;
            let name = a.required_string("name")?.trim().to_owned();
            if name.is_empty() { return Err(ToolError("name: required".into())); }
            let amount = a.cents("amount")?;
            if amount == 0 { return Err(ToolError("amount: must not be 0".into())); }
            let account = required_id(&a, "accountId", "Account")?;
            if store.get_account(account).map_err(err)?.is_none() { return Err(ToolError("accountId: not found".into())); }
            let frequency = match a.optional_string("frequency")?.unwrap_or("monthly") {
                "daily" => finanzas_core::Frequency::Daily, "weekly" => finanzas_core::Frequency::Weekly,
                "monthly" => finanzas_core::Frequency::Monthly, "yearly" => finanzas_core::Frequency::Yearly,
                _ => return Err(ToolError("frequency: expected daily, weekly, monthly or yearly".into())),
            };
            let draft = service::ScheduleDraft { name, amount, account_id: account, category_id: category(&a, store)?, payee: a.optional_string("payee")?.unwrap_or("").into(),
                frequency, interval: a.int("interval", 1, 1, 52)? as u32, start: a.date("start")?, skip_weekend: a.bool("skipWeekend")?.unwrap_or(false), auto_post: a.bool("autoPost")?.unwrap_or(false) };
            let saved = service::save_schedule(store, n, &draft).map_err(missing("id"))?;
            // Same as the app: an automatic payment already due is recorded right away.
            service::auto_post_due(store, today).map_err(err)?;
            schedule_by_id(store, saved)?
        }
        "post_schedule" => {
            let n = required_id(&a, "id", "Schedule")?;
            entry_json(service::post_schedule(store, n, today).map_err(missing("id"))?)
        }
        "delete_schedule" => {
            let n = required_id(&a, "id", "Schedule")?;
            let before = schedule_by_id(store, n)?;
            service::delete_schedule(store, n).map_err(missing("id"))?;
            json!({"deleted": before})
        }
        "set_wish_item_paid" => {
            let n = required_id(&a, "id", "WishItem")?;
            let paid = a.bool("paid")?.ok_or_else(|| ToolError("paid: required".into()))?;
            wish_json(service::set_wish_item_paid(store, n, paid, today).map_err(missing("id"))?)
        }
        "complete_wish" => {
            let n = required_id(&a, "id", "Wish")?;
            let account = required_id(&a, "accountId", "Account")?;
            if store.get_account(account).map_err(err)?.is_none() { return Err(ToolError("accountId: not found".into())); }
            let done = service::complete_wish(store, n, account, today).map_err(missing("id"))?;
            json!({"wish": wish_json(done.wish), "transactions": done.entries.into_iter().map(entry_json).collect::<Vec<_>>()})
        }
        "tag_transaction" => {
            let n = required_id(&a, "id", "LedgerEntry")?;
            let wish = a.id("wishId", "Wish")?;
            let item = a.id("itemId", "WishItem")?;
            if wish.is_none() && item.is_some() { return Err(ToolError("itemId: requires wishId".into())); }
            entry_json(service::tag_entry(store, n, wish, item).map_err(missing("id"))?)
        }
        "get_overview" => {
            let month = match a.0.get("month").filter(|v| !v.is_null()) { Some(_) => a.month("month")?, None => { use chrono::Datelike; today.year() * 100 + today.month() as i32 } };
            let span = a.int("months", 1, 1, 24)? as u32;
            let o = service::overview_span(store, month, span, 0).map_err(err)?;
            let progress = |list: Vec<service::CategoryProgress>| list.into_iter().map(|p| json!({"categoryId":id("Category",p.category_id),"category":p.name,"spent":dec(p.spent),"budgeted":dec(p.budgeted)})).collect::<Vec<_>>();
            json!({"month":format!("{}-{:02}", month / 100, month % 100),"months":span,"totalBalance":dec(o.total_balance),"openAccounts":o.open_account_count,
                "current":summary_json(&o.current),"previous":summary_json(&o.previous),"incomeChange":o.income_change,"expensesChange":o.expenses_change,"savingsRateChange":o.savings_rate_change,
                "spendingTotal":dec(o.spending_total),"spending":progress(o.spending),"budgetSpent":dec(o.budget_spent),"budgetAssigned":dec(o.budget_assigned),"budgetProgress":progress(o.budget_progress)})
        }
        "add_transactions" => {
            let list = a.0.get("transactions").and_then(Value::as_array).ok_or_else(|| ToolError("transactions: expected an array".into()))?;
            if list.is_empty() || list.len() > 500 { return Err(ToolError("transactions: expected 1 to 500 items".into())); }
            let drafts = list.iter().enumerate().map(|(i, v)| new_entry(&Args(v), store).map_err(|e| ToolError(format!("transactions[{i}].{}", e.0)))).collect::<Result<Vec<_>, _>>()?;
            // All or none, like the app's CSV import.
            let rows = store.transaction(|s| drafts.into_iter().map(|d| service::add_entry(s, d)).collect::<Result<Vec<_>, service::ServiceError>>()).map_err(err)?;
            json!({"added": rows.len(), "transactions": rows.into_iter().map(entry_json).collect::<Vec<_>>()})
        }
        "list_profiles" => {
            let home = Store::default_path().ok_or_else(|| ToolError("Unable to determine database path".into()))?;
            json!(Store::profiles_beside(&home).into_iter().map(|slug| json!({"perfil":slug,"name":Store::profile_label(&slug)})).collect::<Vec<_>>())
        }
        "get_upcoming" => {
            let days = a.int("days",30,0,365)?;
            json!(service::upcoming(store,today,days).map_err(err)?.into_iter().map(|v| json!({"scheduleId":id("Schedule",v.schedule_id),"name":v.name,"date":v.date.to_string(),"amount":dec(v.amount),"accountId":oid("Account",v.account_id),"account":v.account,"overdue":v.overdue,"projectedBalance":v.projected_balance.map(dec)})).collect::<Vec<_>>())
        }
        "add_transaction" => entry_json(service::add_entry(store, new_entry(&a, store)?).map_err(err)?),
        "update_transaction" => {
            let n = required_id(&a,"id","LedgerEntry")?;
            if store.get_entry(n).map_err(err)?.is_none() { return Err(ToolError("id: not found".into())); }
            let fields = ["date","amount","payee","categoryId","notes","cleared"];
            if !fields.iter().any(|k| args.get(k).is_some()) { return Err(ToolError("Provide at least one field to change (date, amount, payee, categoryId, notes, cleared)".into())); }
            let payee = a.optional_string("payee")?.map(str::to_owned);
            if payee.as_ref().is_some_and(|s| s.trim().is_empty()) { return Err(ToolError("Payee must not be empty".into())); }
            let date = if args.get("date").is_some() { Some(a.date("date")?) } else { None };
            let amount = if args.get("amount").is_some() { Some(a.cents("amount")?) } else { None };
            let category_id = category(&a,store)?.map(Some);
            let notes = a.optional_string("notes")?.map(str::to_owned);
            let cleared = a.bool("cleared")?;
            entry_json(service::update_entry(store,n,service::EntryChanges {date,amount,payee,category_id,notes,cleared,..Default::default()}).map_err(err)?)
        }
        "delete_transaction" => {
            let n = required_id(&a,"id","LedgerEntry")?;
            if store.get_entry(n).map_err(err)?.is_none() { return Err(ToolError("id: not found".into())); }
            json!({"deleted":entry_json(service::delete_entry(store,n).map_err(err)?)})
        }
        "set_budget_amount" => {
            let month = a.month("month")?;
            let n = required_id(&a,"categoryId","Category")?;
            let c = store.get_category(n).map_err(err)?.ok_or_else(|| ToolError("categoryId: not found".into()))?;
            let amount = a.cents("amount")?;
            if amount < 0 { return Err(ToolError("amount must be 0 or more".into())); }
            let text = format!("{}.{:02}",amount / 100,amount % 100);
            service::set_budget_from_text(store,month,n,&text).map_err(err)?;
            json!({"month":format!("{:04}-{:02}",month/100,month%100),"categoryId":id("Category",n),"category":c.name,"budgeted":dec(amount)})
        }
        "skip_schedule" => {
            let n = required_id(&a,"id","Schedule")?;
            let s = store.get_schedule(n).map_err(err)?.ok_or_else(|| ToolError("id: not found".into()))?;
            if finanzas_core::domain::schedule::effective_next_date(&s).is_none() { return Err(ToolError("Schedule is completed".into())); }
            let through = if args.get("through").is_some() { Some(a.date("through")?) } else { None };
            let r = service::skip_schedule(store,n,through).map_err(err)?;
            json!({"id":id("Schedule",n),"name":s.name,"previousNextDate":r.previous_next_date.map(|d| d.to_string()),"nextDate":r.next_date.map(|d| d.to_string()),"completed":r.completed,"skipped":r.skipped})
        }
        _ => return Err(ToolError(format!("Unknown tool: {name}"))),
    })
}
