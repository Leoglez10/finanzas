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
        spec("get_upcoming", "Upcoming scheduled payments from overdue ones through the next `days` days (default 30), oldest first, with each account's projected balance after the payment.", json!({"days":{"type":"integer","minimum":0,"maximum":365,"description":"Days ahead (default 30)"}}), &[], true, false),
    ]
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
    json!({"id":id("LedgerEntry",r.id),"date":r.date.to_string(),"amount":dec(r.amount),"accountId":oid("Account",r.account_id),"account":r.account_name,"payee":r.payee_name,"categoryId":oid("Category",r.category_id),"category":r.category_name,"notes":r.notes,"cleared":r.cleared,"scheduleId":oid("Schedule",r.schedule_id)})
}
fn category(a: &Args<'_>, store: &Store) -> Result<Option<i64>, ToolError> {
    let category = a.id("categoryId", "Category")?;
    if let Some(n) = category { if store.get_category(n).map_err(err)?.is_none() { return Err(ToolError("categoryId: not found".into())); } }
    Ok(category)
}
pub fn call(store: &mut Store, name: &str, args: &Value, today: NaiveDate) -> Result<Value, ToolError> {
    let a = Args(args);
    Ok(match name {
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
            json!({"total":total,"truncated":total>limit,"transactions":rows.into_iter().map(|r| json!({"id":id("LedgerEntry",r.id),"date":r.date.to_string(),"amount":dec(r.amount),"accountId":oid("Account",r.account_id),"account":r.account_name,"payee":r.payee_name,"categoryId":oid("Category",r.category_id),"category":r.category_name,"notes":r.notes,"cleared":r.cleared,"scheduleId":oid("Schedule",r.schedule_id)})).collect::<Vec<_>>()})
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
            json!(rows.into_iter().map(|v| json!({"id":id("Schedule",v.schedule.id),"name":v.schedule.name,"amount":dec(v.schedule.amount),"payee":v.payee,"accountId":oid("Account",v.schedule.account_id),"account":v.account,"frequency":v.schedule.recurrence.as_ref().map(|r| r.frequency),"interval":v.schedule.recurrence.as_ref().map(|r| r.interval).unwrap_or(1),"nextDate":v.next_date.map(|d| d.to_string()),"completed":v.schedule.completed,"autoPost":v.schedule.auto_post})).collect::<Vec<_>>())
        }
        "get_upcoming" => {
            let days = a.int("days",30,0,365)?;
            json!(service::upcoming(store,today,days).map_err(err)?.into_iter().map(|v| json!({"scheduleId":id("Schedule",v.schedule_id),"name":v.name,"date":v.date.to_string(),"amount":dec(v.amount),"accountId":oid("Account",v.account_id),"account":v.account,"overdue":v.overdue,"projectedBalance":v.projected_balance.map(dec)})).collect::<Vec<_>>())
        }
        "add_transaction" => {
            let account = required_id(&a,"accountId","Account")?;
            if store.get_account(account).map_err(err)?.is_none() { return Err(ToolError("accountId: not found".into())); }
            let category_id = category(&a,store)?;
            let payee = a.optional_string("payee")?.map(str::to_owned);
            let date = a.date("date")?;
            let amount = a.cents("amount")?;
            let notes = a.optional_string("notes")?.unwrap_or("").to_owned();
            let cleared = a.bool("cleared")?.unwrap_or(true);
            entry_json(service::add_entry(store,service::NewEntry { date,amount,account_id:account,payee,category_id,notes,cleared }).map_err(err)?)
        }
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
