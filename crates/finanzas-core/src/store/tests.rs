use super::Store;
use crate::models::{Account, Category, CategoryGroup, Entry};
use chrono::NaiveDate;

#[test]
fn default_path_uses_environment_or_platform_data_dir() {
    let expected = std::env::var_os("FINANZAS_DB").map(std::path::PathBuf::from)
        .or_else(|| dirs::data_dir().map(|p| p.join("Finanzas").join("finanzas.db")));
    assert!(expected.is_some());
    assert_eq!(Store::default_path(), expected);
}

#[test]
fn account_balance_sums_its_entries() {
    let store = Store::open_in_memory().unwrap();
    let account = Account { id: 0, name: "Checking".into(), off_budget: false, closed: false, sort_order: 0.0 };
    let id = store.insert_account(&account).unwrap();
    let entry = Entry { id: 0, date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(), amount: 100_050, notes: String::new(), cleared: false, account_id: Some(id), category_id: None, payee_id: None, schedule_id: None };
    store.insert_entry(&entry).unwrap();
    store.insert_entry(&Entry { amount: -2550, ..entry }).unwrap();
    let empty_id = store.insert_account(&Account { name: "Empty".into(), ..account }).unwrap();
    let balances = store.account_balances().unwrap();
    assert_eq!(balances[&id], 97_500);
    assert_eq!(balances.get(&empty_id), Some(&0));
}

#[test]
fn loads_budget_from_store_ignoring_off_budget_and_uncategorized() {
    let store = Store::open_in_memory().unwrap();
    let account = Account { id: 0, name: "Bank".into(), off_budget: false, closed: false, sort_order: 0.0 };
    let on = store.insert_account(&account).unwrap();
    let off = store.insert_account(&Account { name: "Savings".into(), off_budget: true, ..account }).unwrap();
    let group = store.insert_category_group(&CategoryGroup { id: 0, name: "Food".into(), is_income: false, sort_order: 0.0 }).unwrap();
    let category = store.insert_category(&Category { id: 0, name: "Groceries".into(), hidden: false, sort_order: 0.0, group_id: Some(group) }).unwrap();
    let entry = Entry { id: 0, date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(), amount: -100, notes: String::new(), cleared: false, account_id: Some(on), category_id: Some(category), payee_id: None, schedule_id: None };
    store.insert_entry(&entry).unwrap();
    store.insert_entry(&Entry { account_id: Some(off), amount: -900, ..entry.clone() }).unwrap();
    store.insert_entry(&Entry { category_id: None, amount: 500, ..entry }).unwrap();
    assert_eq!(store.activity_by_month_category().unwrap().get(&(202609, category)), Some(&-100));
    store.set_budget(202609, category, 300).unwrap();
    let inputs = store.envelope_inputs().unwrap();
    assert_eq!(inputs.categories, vec![(category, false)]);
    assert_eq!(inputs.activity.get(&(202609, category)), Some(&-100));
    assert_eq!(inputs.budgeted.get(&(202609, category)), Some(&300));
}

#[test]
fn setting_a_budget_updates_instead_of_duplicating() {
    let store = Store::open_in_memory().unwrap();
    let category = store.insert_category(&Category { id: 0, name: "Food".into(), hidden: false, sort_order: 0.0, group_id: None }).unwrap();
    store.set_budget(202609, category, 100).unwrap();
    store.set_budget(202609, category, 200).unwrap();
    let (amounts, _) = store.budget_rows().unwrap();
    assert_eq!(amounts.len(), 1);
    assert_eq!(amounts[&(202609, category)], 200);
}
