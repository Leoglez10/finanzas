use chrono::NaiveDate;
use finanzas_core::{Account, Category, CategoryGroup, Entry, Payee, Schedule, Recurrence, Frequency, WeekendMode, store::Store};
use finanzas_mcp::{args::Args, tools::{call, specs}};
use serde_json::{json, Value};

fn day(m: u32, d: u32) -> NaiveDate { NaiveDate::from_ymd_opt(2026, m, d).unwrap() }
struct Seed { store: Store, bank: i64, food: i64 }
impl Seed {
    fn new() -> Self {
        let store = Store::open_in_memory().unwrap();
        let bank = store.insert_account(&Account { id:0, name:"Banco".into(), off_budget:false, closed:false, sort_order:1.0 }).unwrap();
        let invest = store.insert_account(&Account { id:0, name:"Inversión".into(), off_budget:true, closed:false, sort_order:2.0 }).unwrap();
        let expenses = store.insert_category_group(&CategoryGroup { id:0, name:"Gastos".into(), is_income:false, sort_order:1.0 }).unwrap();
        let income = store.insert_category_group(&CategoryGroup { id:0, name:"Ingresos".into(), is_income:true, sort_order:2.0 }).unwrap();
        let food = store.insert_category(&Category { id:0, name:"Comida".into(), hidden:false, sort_order:1.0, group_id:Some(expenses) }).unwrap();
        let salary = store.insert_category(&Category { id:0, name:"Sueldo".into(), hidden:false, sort_order:1.0, group_id:Some(income) }).unwrap();
        let payee = store.insert_payee(&Payee { id:0, name:"Oxxo".into() }).unwrap();
        for (date,amount,account,category,payee,notes,cleared) in [(day(9,1),1000000,bank,Some(salary),None,"",true),(day(9,5),-12550,bank,Some(food),Some(payee),"súper",false),(day(9,6),-50000,invest,Some(food),None,"",false),(day(10,2),-3000,bank,None,None,"",false)] {
            store.insert_entry(&Entry { id:0,date,amount,notes:notes.into(),cleared,account_id:Some(account),category_id:category,payee_id:payee,schedule_id:None }).unwrap();
        }
        store.set_budget(202609,food,30000).unwrap();
        store.insert_schedule(&Schedule { id:0,name:"Gym".into(),amount:-55000,auto_post:false,completed:false,next_date:Some(day(10,11)),recurrence:Some(Recurrence {start:day(5,11),frequency:Frequency::Monthly,interval:1,skip_weekend:false,weekend_mode:WeekendMode::After,end_date:None}),account_id:Some(bank),category_id:None,payee_id:None }).unwrap();
        Self {store,bank,food}
    }
    fn call(&mut self, name: &str, args: Value) -> Value { call(&mut self.store,name,&args,day(10,15)).unwrap() }
}
#[test]
fn write_tools_behavior_red() {
    let mut s = Seed::new();
    let bank = format!("Account/p{}", s.bank);
    let food = format!("Category/p{}", s.food);
    let cases = [
        ("add_transaction", json!({"accountId":bank,"date":"2026-10-14","amount":1})),
        ("update_transaction", json!({"id":"LedgerEntry/p1","notes":"changed"})),
        ("delete_transaction", json!({"id":"LedgerEntry/p2"})),
        ("set_budget_amount", json!({"month":"2026-09","categoryId":food,"amount":42})),
        ("skip_schedule", json!({"id":"Schedule/p1"})),
    ];
    let failures: Vec<_> = cases.iter().filter_map(|(name,args)| call(&mut s.store,name,args,day(10,15)).err().map(|e| format!("{name}: {e}"))).collect();
    assert!(failures.is_empty(), "{}", failures.join("; "));
}
#[test]
fn set_budget_amount_validates_and_upserts() {
    let mut s = Seed::new(); let food = format!("Category/p{}",s.food);
    for amount in [50.0, 27.25, 0.0] {
        let r = s.call("set_budget_amount",json!({"month":"2026-09","categoryId":food,"amount":amount}));
        assert_eq!(r["budgeted"],amount); assert_eq!(r["category"],"Comida");
    }
    let salary = s.store.list_categories().unwrap().into_iter().find(|c| c.name == "Sueldo").unwrap().id;
    for (category,amount,expected) in [(format!("Category/p{salary}"),1,"Income categories cannot be budgeted"),(food,-1,"amount must be 0 or more")] {
        assert_eq!(call(&mut s.store,"set_budget_amount",&json!({"month":"2026-09","categoryId":category,"amount":amount}),day(10,15)).unwrap_err().to_string(),expected);
    }
}
#[test]
fn add_transaction_reuses_payee_defaults_cleared_and_learns_category() {
    let mut s = Seed::new(); let bank = format!("Account/p{}",s.bank);
    let r = s.call("add_transaction",json!({"accountId":bank,"date":"2026-10-14","amount":-20,"payee":"oxxo"}));
    assert_eq!(r["payee"],"Oxxo"); assert_eq!(r["category"],"Comida"); assert_eq!(r["cleared"],true);
    assert_eq!(s.store.list_payees().unwrap().len(),1);
    let salary = s.store.list_categories().unwrap().into_iter().find(|c| c.name == "Sueldo").unwrap().id;
    let r = s.call("add_transaction",json!({"accountId":bank,"date":"2026-10-14","amount":-20,"payee":"Oxxo","categoryId":format!("Category/p{salary}")}));
    assert_eq!(r["category"],"Sueldo");
}
#[test]
fn update_transaction_changes_only_given_fields() {
    let mut s = Seed::new(); let bank = format!("Account/p{}",s.bank);
    let t = s.call("add_transaction",json!({"accountId":bank,"date":"2026-10-14","amount":-20,"payee":"oxxo","notes":"note"}));
    let r = s.call("update_transaction",json!({"id":t["id"],"notes":"","cleared":false}));
    assert_eq!(r["notes"],""); assert_eq!(r["cleared"],false); assert_eq!(r["amount"],-20.0); assert_eq!(r["payee"],"Oxxo");
    for (args,expected) in [(json!({"id":t["id"]}),"Provide at least one field to change (date, amount, payee, categoryId, notes, cleared)"),(json!({"id":t["id"],"payee":"  "}),"Payee must not be empty")] {
        assert_eq!(call(&mut s.store,"update_transaction",&args,day(10,15)).unwrap_err().to_string(),expected);
    }
}
#[test]
fn delete_transaction_returns_the_deleted_row() {
    let mut s=Seed::new(); let bank=format!("Account/p{}",s.bank);
    let t=s.call("add_transaction",json!({"accountId":bank,"date":"2026-10-14","amount":42}));
    let r=s.call("delete_transaction",json!({"id":t["id"]})); assert_eq!(r["deleted"],t);
    assert_eq!(call(&mut s.store,"delete_transaction",&json!({"id":t["id"]}),day(10,15)).unwrap_err().to_string(),"id: not found");
}
#[test]
fn schedules_skip_once_and_through() {
    let mut s=Seed::new();
    let r=s.call("skip_schedule",json!({"id":"Schedule/p1"})); assert_eq!(r["skipped"],1); assert_eq!(r["nextDate"],"2026-11-11");
    let r=s.call("skip_schedule",json!({"id":"Schedule/p1","through":"2027-01-31"})); assert_eq!(r["skipped"],3); assert_eq!(r["nextDate"],"2027-02-11");
    let mut schedule=s.store.get_schedule(1).unwrap().unwrap(); schedule.completed=true; s.store.update_schedule(&schedule).unwrap();
    assert_eq!(call(&mut s.store,"skip_schedule",&json!({"id":"Schedule/p1"}),day(10,15)).unwrap_err().to_string(),"Schedule is completed");
}
#[test]
fn write_bad_ids_and_wrong_types_are_errors() {
    let mut s=Seed::new(); let bank=format!("Account/p{}",s.bank);
    for (name,args,expected) in [
        ("skip_schedule",json!({"id":bank}),"id: not found"),
        ("add_transaction",json!({"date":"2026-10-14","amount":1}),"accountId: required"),
        ("add_transaction",json!({"accountId":bank,"date":"2026-10-14","amount":5,"cleared":1}),"cleared: expected true or false"),
        ("add_transaction",json!({"accountId":bank,"date":"2026-10-14","amount":"12.5"}),"amount: expected a number"),
        ("skip_schedule",json!({"id":"Schedule/p1","through":12}),"through: expected YYYY-MM-DD"),
        ("add_transaction",json!({"accountId":bank,"date":"2026-10-14","amount":1e17}),"amount: must be between -1000000000000 and 1000000000000"),
    ] { assert_eq!(call(&mut s.store,name,&args,day(10,15)).unwrap_err().to_string(),expected); }
}
#[test]
fn json_zero_and_one_are_amounts_not_booleans() {
    let mut s=Seed::new(); let bank=format!("Account/p{}",s.bank); let food=format!("Category/p{}",s.food);
    let r=s.call("set_budget_amount",json!({"month":"2026-09","categoryId":food,"amount":0})); assert_eq!(r["budgeted"],0.0);
    let r=s.call("add_transaction",json!({"accountId":bank,"date":"2026-10-14","amount":1})); assert_eq!(r["amount"],1.0);
    let r=s.call("update_transaction",json!({"id":r["id"],"amount":0})); assert_eq!(r["amount"],0.0);
}
#[test]
fn another_store_connection_bumps_data_version() {
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target").join(format!("data-version-{}-{:?}.db",std::process::id(),std::thread::current().id()));
    let app=Store::open(&path).unwrap(); let before=app.data_version().unwrap(); let other=Store::open(&path).unwrap();
    other.insert_account(&Account{id:0,name:"Desde IA".into(),off_budget:false,closed:false,sort_order:0.0}).unwrap();
    assert_ne!(app.data_version().unwrap(),before);
}
#[test]
fn remaining_swift_write_parity_cases() {
    let mut s = Seed::new();
    let bank = format!("Account/p{}", s.bank);
    let food = format!("Category/p{}", s.food);
    let first = s.call("set_budget_amount", json!({"month":"2026-10","categoryId":food,"amount":450.25}));
    assert_eq!(first["budgeted"], 450.25);
    s.call("set_budget_amount", json!({"month":"2026-10","categoryId":food,"amount":100}));
    assert_eq!(s.store.budget_rows().unwrap().0.len(), 2);
    let t = s.call("add_transaction", json!({"accountId":bank,"date":"2026-10-14","amount":-89.9,"payee":"OXXO","categoryId":food,"notes":"café"}));
    assert_eq!(t["amount"], -89.9); assert_eq!(t["notes"], "café");
    assert_eq!(s.store.account_balances().unwrap()[&s.bank], 984450 - 8990);
    let u = s.call("update_transaction", json!({"id":t["id"],"notes":"corregido","payee":"Uber"}));
    assert_eq!(u["notes"], "corregido"); assert_eq!(u["payee"], "Uber"); assert_eq!(u["amount"], -89.9);
    let float = s.call("add_transaction", json!({"accountId":bank,"date":"2026-10-14","amount":1.0,"cleared":false}));
    assert_eq!(float["amount"], 1.0); assert_eq!(float["cleared"], false);
    let big = s.call("add_transaction", serde_json::from_str(&format!(r#"{{"accountId":"{bank}","date":"2026-10-14","amount":-999999999999.99}}"#)).unwrap());
    assert_eq!(big["amount"].as_f64().unwrap(), -999_999_999_999.99);
    for (name,args,expected) in [
        ("add_transaction",json!({"accountId":bank,"date":"2026-10-14","amount":1.234}),"amount: at most 2 decimals"),
        ("add_transaction",json!({"accountId":"nope","date":"2026-10-14","amount":1}),"accountId: not found"),
        ("add_transaction",json!({"accountId":bank,"date":"2026-10-14","amount":5,"cleared":"false"}),"cleared: expected true or false"),
        ("get_transactions",json!({"startDate":"2026-09-01","endDate":"2026-09-30","limit":1.5}),"limit: expected an integer from 1 to 1000"),
        ("get_transactions",json!({"startDate":"2026-09-01","endDate":"2026-09-30","limit":1e300}),"limit: expected an integer from 1 to 1000"),
        ("get_upcoming",json!({"days":-1e300}),"days: expected an integer from 0 to 365"),
        ("spending_by_category",json!({"startDate":"2026-09-01","endDate":"2026-09-30","includeOffBudget":"yes"}),"includeOffBudget: expected true or false"),
    ] { assert_eq!(call(&mut s.store,name,&args,day(10,15)).unwrap_err().to_string(),expected); }
}
#[test]
fn skip_schedule_caps_at_500_and_can_skip_none() {
    let mut s=Seed::new();
    let r=s.call("skip_schedule",json!({"id":"Schedule/p1","through":"2026-01-01"}));
    assert_eq!(r["skipped"],0); assert_eq!(r["nextDate"],"2026-10-11");
    let r=s.call("skip_schedule",json!({"id":"Schedule/p1","through":"2100-12-31"}));
    assert_eq!(r["skipped"],500); assert_eq!(r["nextDate"],"2068-06-11");
}
#[test]
fn amounts_accept_decimal_numbers_without_float_error() {
    for (text, expected) in [("0",0), ("1",100), ("89.9",8990), ("-999999999999.99",-99999999999999), ("1e-2",1)] {
        let value: Value = serde_json::from_str(&format!("{{\"amount\":{text}}}")).unwrap();
        assert_eq!(Args(&value).cents("amount").unwrap(), expected);
    }
    for (text, expected) in [("true","amount: expected a number"), ("1.234","amount: at most 2 decimals"), ("1e300","amount: must be between -1000000000000 and 1000000000000")] {
        let value: Value = serde_json::from_str(&format!("{{\"amount\":{text}}}")).unwrap();
        assert_eq!(Args(&value).cents("amount").unwrap_err().to_string(), expected);
    }
}
#[test]
fn huge_and_tiny_non_integer_values_are_rejected() {
    let mut s=Seed::new();
    for text in ["1e300", "1e-400"] {
        let args: Value = serde_json::from_str(&format!("{{\"days\":{text}}}")).unwrap();
        assert_eq!(call(&mut s.store,"get_upcoming",&args,day(10,15)).unwrap_err().to_string(),"days: expected an integer from 0 to 365");
    }
}
#[test]
fn bounded_integer_reports_its_range() {
    let mut s=Seed::new();
    assert_eq!(call(&mut s.store,"get_upcoming",&json!({"days":366}),day(10,15)).unwrap_err().to_string(),"days: expected an integer from 0 to 365");
}
#[test]
fn every_spec_has_an_object_schema() {
    let specs = specs();
    assert_eq!(specs.iter().map(|s| s.name).collect::<Vec<_>>(), ["list_accounts","get_transactions","list_categories","get_budget_month","spending_by_category","add_transaction","update_transaction","delete_transaction","set_budget_amount","list_schedules","skip_schedule","get_upcoming"]);
    assert!(specs.iter().all(|s| s.input_schema["type"] == "object"));
}
#[test]
fn list_accounts_returns_decimal_balances() {
    let mut s=Seed::new(); let bank=s.bank; let rows=s.call("list_accounts",json!({}));
    assert_eq!(rows[0]["name"],"Banco"); assert_eq!(rows[0]["balance"],9844.5); assert_eq!(rows[0]["id"],format!("Account/p{bank}")); assert_eq!(rows[1]["offBudget"],true);
}
#[test]
fn get_transactions_filters_sorts_and_truncates() {
    let mut s=Seed::new();
    let rows=s.call("get_transactions",json!({"startDate":"2026-09-01","endDate":"2026-09-30"}));
    assert_eq!(rows["total"],3); assert_eq!(rows["transactions"][0]["date"],"2026-09-06"); assert_eq!(rows["transactions"][1]["amount"],-125.5); assert_eq!(rows["transactions"][1]["payee"],"Oxxo");
    let r=s.call("get_transactions",json!({"startDate":"2026-09-01","endDate":"2026-10-31","accountId":format!("Account/p{}",s.bank),"limit":2})); assert_eq!(r["total"],3); assert_eq!(r["truncated"],true);
    let r=s.call("get_transactions",json!({"startDate":"2026-09-01","endDate":"2026-09-30","categoryId":format!("Category/p{}",s.food)})); assert_eq!(r["total"],2);
}
#[test]
fn list_categories_groups_with_flags() {
    let mut s=Seed::new(); let r=s.call("list_categories",json!({})); assert_eq!(r[0]["name"],"Gastos"); assert_eq!(r[1]["isIncome"],true); assert_eq!(r[0]["categories"][0]["hidden"],false);
}
#[test]
fn budget_month_matches_envelope_rules() {
    let mut s=Seed::new(); let r=s.call("get_budget_month",json!({"month":"2026-09"})); assert_eq!(r["toBudget"],9700.0); assert_eq!(r["income"],10000.0); assert_eq!(r["budgeted"],300.0); assert_eq!(r["groups"][0]["categories"][0]["spent"],-125.5); assert_eq!(r["groups"][0]["categories"][0]["available"],174.5);
}
#[test]
fn spending_by_category_excludes_inflows_and_off_budget() {
    let mut s=Seed::new(); let r=s.call("spending_by_category",json!({"startDate":"2026-09-01","endDate":"2026-10-31"})); assert_eq!(r["total"],155.5); assert_eq!(r["categories"][1]["category"],"Sin categoría");
    let r=s.call("spending_by_category",json!({"startDate":"2026-09-01","endDate":"2026-10-31","includeOffBudget":true})); assert_eq!(r["categories"][0]["spent"],625.5);
}
#[test]
fn spending_counts_outflows_without_an_account() {
    let mut s=Seed::new();
    s.store.insert_entry(&Entry {id:0,date:day(9,9),amount:-100,notes:String::new(),cleared:false,account_id:None,category_id:None,payee_id:None,schedule_id:None}).unwrap();
    let r=s.call("spending_by_category",json!({"startDate":"2026-09-01","endDate":"2026-10-31"}));
    assert_eq!(r["total"],156.5);
}
#[test]
fn budget_groups_follow_sort_order_not_income_status() {
    let mut s=Seed::new();
    let mut income=s.store.list_category_groups().unwrap().into_iter().find(|g| g.is_income).unwrap(); income.sort_order=0.0; s.store.update_category_group(&income).unwrap();
    let r=s.call("get_budget_month",json!({"month":"2026-09"})); assert_eq!(r["groups"][0]["name"],"Ingresos");
}
#[test]
fn schedules_list_and_upcoming() {
    let mut s=Seed::new(); let r=s.call("list_schedules",json!({})); assert_eq!(r[0]["nextDate"],"2026-10-11"); assert_eq!(r[0]["frequency"],"monthly"); assert_eq!(r[0]["amount"],-550.0);
    let r=s.call("get_upcoming",json!({"days":30})); assert_eq!(r[0]["date"],"2026-10-11"); assert_eq!(r[1]["date"],"2026-11-11"); assert_eq!(r[0]["projectedBalance"],9294.5); assert_eq!(r[1]["projectedBalance"],8744.5);
}
#[test]
fn unknown_tool_bad_ids_and_wrong_types_are_errors() {
    let mut s=Seed::new(); let today=day(10,15);
    for (name,args,expected) in [("nope",json!({}),"Unknown tool: nope"),("get_transactions",json!({"startDate":"2026-09-01","endDate":"2026-09-30","accountId":"Schedule/p1"}),"accountId: not found"),("get_transactions",json!({"startDate":"2026-09-01","endDate":"2026-09-30","accountId":123}),"accountId: expected a string"),("get_upcoming",json!({"days":true}),"days: expected an integer from 0 to 365"),("spending_by_category",json!({"startDate":"2026-09-01","endDate":"2026-09-30","includeOffBudget":1}),"includeOffBudget: expected true or false"),("get_budget_month",json!({"month":"2026-13"}),"month: expected YYYY-MM")] { assert_eq!(call(&mut s.store,name,&args,today).unwrap_err().to_string(),expected); }
}
