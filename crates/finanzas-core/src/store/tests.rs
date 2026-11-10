use super::{EntryFilter, Store};
use crate::models::{Account, BudgetAmount, Category, CategoryGroup, Entry, Frequency, Payee, Recurrence, Schedule, WeekendMode};
use chrono::NaiveDate;

fn unique_db_path(name: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("finanzas-{name}-{}-{nanos}-{}.db", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)))
}

fn assert_foreign_database_untouched(sql: &str) {
    let path = unique_db_path("foreign");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(sql).unwrap();
    drop(conn);
    let before = std::fs::read(&path).unwrap();
    let result = Store::open(&path);
    assert!(result.is_err(), "foreign database must be refused");
    let error = result.err().unwrap();
    assert!(error.to_string().contains("not a Finanzas database"), "unexpected error: {error}");
    match error {
        rusqlite::Error::ToSqlConversionFailure(inner) => {
            assert_eq!(inner.downcast_ref::<super::ForeignDatabase>().unwrap().0, path);
        }
        other => panic!("unexpected error type: {other}"),
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = path.as_os_str().to_owned();
        sidecar.push(suffix);
        assert!(!std::path::Path::new(&sidecar).exists());
    }
}

#[test]
fn open_refuses_core_data_without_mutation() {
    assert_foreign_database_untouched("CREATE TABLE Z_METADATA (id INTEGER); CREATE TABLE ZACCOUNT (id INTEGER); PRAGMA user_version = 3;");
}

#[test]
fn open_refuses_core_data_primarykey_marker() {
    assert_foreign_database_untouched("CREATE TABLE Z_PRIMARYKEY (id INTEGER); PRAGMA user_version = 1;");
}

#[test]
fn open_refuses_unversioned_foreign_database_without_mutation() {
    assert_foreign_database_untouched("CREATE TABLE unrelated (id INTEGER);");
}

#[test]
fn open_reopens_existing_finanzas_database() {
    let path = unique_db_path("reopen");
    let store = Store::open(&path).unwrap();
    let id = store.insert_account(&Account { id: 0, name: "Persisted".into(), off_budget: false, closed: false, sort_order: 0.0 }).unwrap();
    drop(store);
    assert_eq!(Store::open(&path).unwrap().get_account(id).unwrap().unwrap().name, "Persisted");
}

#[test]
fn open_initializes_empty_file() {
    let path = unique_db_path("empty");
    std::fs::File::create(&path).unwrap();
    assert!(Store::open(&path).unwrap().list_accounts().unwrap().is_empty());
}

#[test]
fn open_creates_fresh_database() {
    let path = unique_db_path("fresh");
    assert!(!path.exists());
    let store = Store::open(&path).unwrap();
    assert!(path.exists());
    assert!(store.list_accounts().unwrap().is_empty());
}

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
fn uncategorized_income_counts_only_this_months_on_budget_deposits_without_category() {
    let store = Store::open_in_memory().unwrap();
    let account = Account { id: 0, name: "Bank".into(), off_budget: false, closed: false, sort_order: 0.0 };
    let on = store.insert_account(&account).unwrap();
    let off = store.insert_account(&Account { off_budget: true, ..account }).unwrap();
    let category = store.insert_category(&Category { id: 0, name: "Income".into(), hidden: false, sort_order: 0.0, group_id: None }).unwrap();
    let entry = Entry { id: 0, date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(), amount: 500, notes: String::new(), cleared: false, account_id: Some(on), category_id: None, payee_id: None, schedule_id: None };
    store.insert_entry(&entry).unwrap();
    store.insert_entry(&Entry { account_id: Some(off), ..entry.clone() }).unwrap();
    store.insert_entry(&Entry { account_id: None, ..entry.clone() }).unwrap();
    store.insert_entry(&Entry { category_id: Some(category), ..entry.clone() }).unwrap();
    store.insert_entry(&Entry { amount: -100, ..entry.clone() }).unwrap();
    store.insert_entry(&Entry { date: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(), ..entry }).unwrap();
    assert_eq!(store.uncategorized_income(202609).unwrap(), 500);
    assert_eq!(store.uncategorized_income(202608).unwrap(), 0);
}

#[test]
fn payee_lookup_is_case_insensitive_and_creates_when_missing() {
    let store = Store::open_in_memory().unwrap();
    let id = store.find_or_create_payee("  Café Ñandú  ").unwrap().unwrap();
    assert_eq!(store.find_or_create_payee("CAFE nandu").unwrap(), Some(id));
    assert_eq!(store.find_or_create_payee("  ").unwrap(), None);
    assert_eq!(store.payee_suggestions(" CAFÉ n", 10).unwrap(), vec![Payee { id, name: "Café Ñandú".into() }]);
    assert!(store.payee_suggestions("cafe", 0).unwrap().is_empty());
}

macro_rules! crud_round_trip {
    ($name:ident, $value:expr, $insert:ident, $get:ident, $list:ident, $update:ident, $delete:ident) => {
        #[test]
        fn $name() {
            let store = Store::open_in_memory().unwrap();
            let mut value = $value;
            value.id = store.$insert(&value).unwrap();
            assert_eq!(store.$get(value.id).unwrap(), Some(value.clone()));
            assert_eq!(store.$list().unwrap(), vec![value.clone()]);
            value.name = "Updated".into();
            store.$update(&value).unwrap();
            assert_eq!(store.$get(value.id).unwrap(), Some(value.clone()));
            store.$delete(value.id).unwrap();
            assert_eq!(store.$get(value.id).unwrap(), None);
        }
    };
}
crud_round_trip!(account_crud_round_trip, Account { id: 0, name: "A".into(), off_budget: true, closed: true, sort_order: 1.0 }, insert_account, get_account, list_accounts, update_account, delete_account);
crud_round_trip!(category_group_crud_round_trip, CategoryGroup { id: 0, name: "G".into(), is_income: true, sort_order: 1.0 }, insert_category_group, get_category_group, list_category_groups, update_category_group, delete_category_group);
crud_round_trip!(category_crud_round_trip, Category { id: 0, name: "C".into(), hidden: true, sort_order: 1.0, group_id: None }, insert_category, get_category, list_categories, update_category, delete_category);

#[test]
fn entries_filter_search_sort_and_truncate() {
    let store = Store::open_in_memory().unwrap();
    let account = store.insert_account(&Account { id: 0, name: "Bank".into(), off_budget: false, closed: false, sort_order: 0.0 }).unwrap();
    let category = store.insert_category(&Category { id: 0, name: "Café".into(), hidden: false, sort_order: 0.0, group_id: None }).unwrap();
    let payee = store.insert_payee(&Payee { id: 0, name: "Peña".into() }).unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 9, 10).unwrap();
    let entry = Entry { id: 0, date, amount: -10, notes: "Té".into(), cleared: true, account_id: Some(account), category_id: Some(category), payee_id: Some(payee), schedule_id: None };
    let first = store.insert_entry(&entry).unwrap();
    let second = store.insert_entry(&entry).unwrap();
    store.insert_entry(&Entry { date: NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(), ..entry }).unwrap();
    let mut filter = EntryFilter { account_id: Some(account), category_id: Some(category), start: Some(date), end: Some(date), search: Some("CAFE".into()), limit: Some(1) };
    let (rows, total) = store.entries(&filter).unwrap();
    assert_eq!(total, 2);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, second);
    assert_eq!(rows[0].account_name.as_deref(), Some("Bank"));
    assert_eq!(rows[0].payee_name.as_deref(), Some("Peña"));
    assert_eq!(rows[0].category_name.as_deref(), Some("Café"));
    filter.search = Some("pena".into()); filter.limit = None;
    assert_eq!(store.entries(&filter).unwrap().0.iter().map(|r| r.id).collect::<Vec<_>>(), vec![second, first]);
    filter.search = Some("TE".into());
    assert_eq!(store.entries(&filter).unwrap().1, 2);
    filter.search = Some("absent".into());
    assert_eq!(store.entries(&filter).unwrap().1, 0);
    assert_eq!(store.recent_entries(1).unwrap()[0].id, second);
}

#[test]
fn entry_crud_round_trip() {
    let store = Store::open_in_memory().unwrap();
    let mut value = Entry { id: 0, date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(), amount: -20, notes: "note".into(), cleared: true, account_id: None, category_id: None, payee_id: None, schedule_id: None };
    value.id = store.insert_entry(&value).unwrap();
    assert_eq!(store.get_entry(value.id).unwrap(), Some(value.clone()));
    assert_eq!(store.list_entries().unwrap(), vec![value.clone()]);
    value.amount = 30;
    store.update_entry(&value).unwrap();
    assert_eq!(store.get_entry(value.id).unwrap(), Some(value.clone()));
    store.delete_entry(value.id).unwrap();
    assert_eq!(store.get_entry(value.id).unwrap(), None);
}

#[test]
fn schedule_round_trips_its_recurrence() {
    let store = Store::open_in_memory().unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
    let mut value = Schedule { id: 0, name: "Rent".into(), amount: -100, auto_post: true, completed: false, next_date: Some(date), recurrence: Some(Recurrence { start: date, frequency: Frequency::Monthly, interval: 2, skip_weekend: true, weekend_mode: WeekendMode::Before, end_date: Some(date) }), account_id: None, category_id: None, payee_id: None };
    value.id = store.insert_schedule(&value).unwrap();
    assert_eq!(store.get_schedule(value.id).unwrap(), Some(value.clone()));
    assert_eq!(store.list_schedules().unwrap(), vec![value.clone()]);
    value.recurrence = None;
    store.update_schedule(&value).unwrap();
    assert_eq!(store.get_schedule(value.id).unwrap(), Some(value.clone()));
    store.delete_schedule(value.id).unwrap();
    assert_eq!(store.get_schedule(value.id).unwrap(), None);
}

#[test]
fn budget_amount_crud_round_trip() {
    let store = Store::open_in_memory().unwrap();
    let mut value = BudgetAmount { id: 0, month: 202609, amount: 50, carryover: true, category_id: None };
    value.id = store.insert_budget_amount(&value).unwrap();
    assert_eq!(store.get_budget_amount(value.id).unwrap(), Some(value.clone()));
    assert_eq!(store.list_budget_amounts().unwrap(), vec![value.clone()]);
    value.amount = 70;
    store.update_budget_amount(&value).unwrap();
    assert_eq!(store.get_budget_amount(value.id).unwrap(), Some(value.clone()));
    store.delete_budget_amount(value.id).unwrap();
    assert_eq!(store.get_budget_amount(value.id).unwrap(), None);
}

#[test]
fn payee_crud_round_trip() {
    let store = Store::open_in_memory().unwrap();
    let mut value = Payee { id: 0, name: "One".into() };
    value.id = store.insert_payee(&value).unwrap();
    assert_eq!(store.get_payee(value.id).unwrap(), Some(value.clone()));
    assert_eq!(store.list_payees().unwrap(), vec![value.clone()]);
    value.name = "Two".into();
    store.update_payee(&value).unwrap();
    assert_eq!(store.get_payee(value.id).unwrap(), Some(value.clone()));
    store.delete_payee(value.id).unwrap();
    assert_eq!(store.get_payee(value.id).unwrap(), None);
}

#[test]
fn payee_named_finds_existing_without_creating() {
    let store = Store::open_in_memory().unwrap();
    let payee = Payee { id: 0, name: "José".into() };
    let id = store.insert_payee(&payee).unwrap();
    assert_eq!(store.payee_named(" JOSE ").unwrap(), Some(Payee { id, ..payee }));
    assert_eq!(store.payee_named("missing").unwrap(), None);
    assert_eq!(store.payee_named("").unwrap(), None);
}

#[test]
fn payee_suggests_its_most_recent_category() {
    let store = Store::open_in_memory().unwrap();
    let payee = store.insert_payee(&Payee { id: 0, name: "Shop".into() }).unwrap();
    let first = store.insert_category(&Category { id: 0, name: "First".into(), hidden: false, sort_order: 0.0, group_id: None }).unwrap();
    let second = store.insert_category(&Category { id: 0, name: "Second".into(), hidden: false, sort_order: 0.0, group_id: None }).unwrap();
    let entry = Entry { id: 0, date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(), amount: -1, notes: String::new(), cleared: false, account_id: None, category_id: Some(first), payee_id: Some(payee), schedule_id: None };
    store.insert_entry(&entry).unwrap();
    store.insert_entry(&Entry { category_id: Some(second), ..entry.clone() }).unwrap();
    store.insert_entry(&Entry { category_id: None, date: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(), ..entry }).unwrap();
    assert_eq!(store.suggested_category(payee).unwrap(), Some(second));
    assert_eq!(store.suggested_category(payee + 100).unwrap(), None);
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
