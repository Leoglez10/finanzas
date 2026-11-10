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
    assert_eq!(call(&mut s.store,"get_upcoming",&json!({"days":366}),day(10,15)).unwrap_err().to_string(),"days: must be between 0 and 365");
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
