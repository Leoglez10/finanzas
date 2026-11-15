use chrono::NaiveDate;
use finanzas_core::{service::*, store::Store, *};

fn today() -> NaiveDate { NaiveDate::from_ymd_opt(2026, 11, 10).unwrap() }
fn seed() -> (Store, Id, Id) {
    let store = Store::open_in_memory().unwrap();
    let account = store.insert_account(&Account { id: 0, name: "Banco".into(), off_budget: false, closed: false, sort_order: 0.0 }).unwrap();
    let group = store.insert_category_group(&CategoryGroup { id: 0, name: "Gastos".into(), is_income: false, sort_order: 0.0 }).unwrap();
    let category = store.insert_category(&Category { id: 0, name: "Ocio".into(), hidden: false, sort_order: 0.0, group_id: Some(group) }).unwrap();
    store.set_budget(202611, category, 50_000).unwrap();
    let incomes = store.insert_category_group(&CategoryGroup { id: 0, name: "Ingresos".into(), is_income: true, sort_order: 1.0 }).unwrap();
    let salary = store.insert_category(&Category { id: 0, name: "Sueldo".into(), hidden: false, sort_order: 0.0, group_id: Some(incomes) }).unwrap();
    store.insert_entry(&Entry { id: 0, date: today(), amount: 100_000, notes: String::new(), cleared: true, account_id: Some(account), category_id: Some(salary), payee_id: None, schedule_id: None }).unwrap();
    (store, account, category)
}
fn draft(category: Option<Id>) -> WishDraft {
    WishDraft { name: "PlayStation".into(), price: 50_000, url: "https://example.com/ps5".into(), notes: "Con mando".into(), category_id: category }
}
#[test]
fn wish_details_and_partial_reservations_do_not_double_allocate() {
    let (mut store, _, category) = seed();
    let first = save_wish(&mut store, None, draft(Some(category))).unwrap();
    let second = save_wish(&mut store, None, draft(Some(category))).unwrap();
    reserve_wish(&mut store, first.id, 35_000, today()).unwrap();
    assert!(reserve_wish(&mut store, second.id, 20_000, today()).is_err());
    let views = list_wishes(&mut store, today(), false).unwrap();
    assert_eq!(views.len(), 2);
    assert_eq!(views[0].wish.notes, "Con mando");
    assert_eq!(views[0].wish.url, "https://example.com/ps5");
    assert_eq!(views[0].evaluation.budget_shortfall, Some(0));
    assert_eq!(views[0].evaluation.status, "affordable");
    assert_eq!(views[1].evaluation.budget_shortfall, Some(35_000));
    assert_eq!(views[1].wish.reserved, 0);
    assert_eq!(budget_month(&store, 202611).unwrap().groups[0].categories[0].available, 50_000);
    reserve_wish(&mut store, first.id, 0, today()).unwrap();
    reserve_wish(&mut store, second.id, 50_000, today()).unwrap();
    archive_wish(&mut store, second.id).unwrap();
    assert_eq!(list_wishes(&mut store, today(), false).unwrap().len(), 1);
    let archived = list_wishes(&mut store, today(), true).unwrap();
    assert_eq!(archived[1].wish.reserved, 0);
    assert!(archived[1].wish.archived);
}
#[test]
fn invalid_edits_leave_wishes_unchanged_and_category_changes_release_claims() {
    let (mut store, _, category) = seed();
    let wish = save_wish(&mut store, None, draft(Some(category))).unwrap();
    reserve_wish(&mut store, wish.id, 10_000, today()).unwrap();
    for invalid in [WishDraft { price: 0, ..draft(Some(category)) }, WishDraft { name: " ".into(), ..draft(Some(category)) }, WishDraft { url: "javascript:alert(1)".into(), ..draft(Some(category)) }, draft(Some(999))] {
        assert!(save_wish(&mut store, Some(wish.id), invalid).is_err());
    }
    assert!(reserve_wish(&mut store, wish.id, -1, today()).is_err());
    assert!(reserve_wish(&mut store, wish.id, 50_001, today()).is_err());
    assert_eq!(list_wishes(&mut store, today(), false).unwrap()[0].wish.reserved, 10_000);
    assert!(save_wish(&mut store, Some(wish.id), WishDraft { price: 9_999, ..draft(Some(category)) }).is_err());
    let changed = save_wish(&mut store, Some(wish.id), draft(None)).unwrap();
    assert_eq!(changed.reserved, 0);
    let evaluation = evaluate_wish(&mut store, wish.id, today()).unwrap();
    assert_eq!(evaluation.budget_shortfall, None);
    assert_eq!(evaluation.status, "unknown");
    assert!(reserve_wish(&mut store, wish.id, 1, today()).is_err());
}
#[test]
fn spending_after_reservation_is_reported_and_release_remains_possible() {
    let (mut store, account, category) = seed();
    let wish = save_wish(&mut store, None, draft(Some(category))).unwrap();
    reserve_wish(&mut store, wish.id, 40_000, today()).unwrap();
    store.insert_entry(&Entry { id: 0, date: today(), amount: -20_000, notes: String::new(), cleared: true, account_id: Some(account), category_id: Some(category), payee_id: None, schedule_id: None }).unwrap();
    let view = evaluate_wish(&mut store, wish.id, today()).unwrap();
    assert_eq!(view.reservation_deficit, 10_000);
    assert_eq!(view.budget_shortfall, Some(20_000));
    assert_ne!(view.status, "affordable");
    reserve_wish(&mut store, wish.id, 20_000, today()).unwrap();
}
#[test]
fn commitments_are_a_separate_cash_constraint_not_a_second_budget_deduction() {
    let (mut store, account, category) = seed();
    let wish = save_wish(&mut store, None, draft(Some(category))).unwrap();
    store.insert_schedule(&Schedule { id: 0, name: "Alquiler".into(), amount: -60_000, auto_post: false, completed: false, next_date: Some(today()), recurrence: Some(Recurrence { start: today(), frequency: Frequency::Yearly, interval: 1, skip_weekend: false, weekend_mode: WeekendMode::After, end_date: Some(today()) }), account_id: Some(account), category_id: Some(category), payee_id: None }).unwrap();
    let view = evaluate_wish(&mut store, wish.id, today()).unwrap();
    assert_eq!(view.budget_shortfall, Some(0));
    assert_eq!(view.upcoming_outflows, 60_000);
    assert_eq!(view.cash_shortfall, Some(10_000));
    assert_eq!(view.status, "cash_shortfall");
}
#[test]
fn unknown_accounts_and_off_budget_cash_never_produce_affordable() {
    let mut store = Store::open_in_memory().unwrap();
    let wish = save_wish(&mut store, None, draft(None)).unwrap();
    let view = evaluate_wish(&mut store, wish.id, today()).unwrap();
    assert_eq!((view.status.as_str(), view.cash_shortfall), ("unknown", None));
}
#[test]
fn reservations_serialize_across_two_database_connections() {
    let path = std::env::temp_dir().join(format!("finanzas-wishes-{}.db", std::process::id()));
    let mut first = Store::open(&path).unwrap();
    let group = first.insert_category_group(&CategoryGroup { id: 0, name: "Gastos".into(), is_income: false, sort_order: 0.0 }).unwrap();
    let category = first.insert_category(&Category { id: 0, name: "Ocio".into(), hidden: false, sort_order: 0.0, group_id: Some(group) }).unwrap();
    first.set_budget(202611, category, 50_000).unwrap();
    let a = save_wish(&mut first, None, draft(Some(category))).unwrap().id;
    let b = save_wish(&mut first, None, draft(Some(category))).unwrap().id;
    let mut second = Store::open(&path).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let other = barrier.clone();
    let handle = std::thread::spawn(move || { other.wait(); reserve_wish(&mut second, b, 40_000, today()).is_ok() });
    barrier.wait();
    let success = reserve_wish(&mut first, a, 40_000, today()).is_ok();
    assert_ne!(success, handle.join().unwrap());
    assert_eq!(list_wishes(&mut first, today(), false).unwrap().iter().map(|w| w.wish.reserved).sum::<i64>(), 40_000);
    drop(first);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn v1_migration_preserves_ledger_and_new_wishes_survive_reopen() {
    let path = std::env::temp_dir().join(format!("finanzas-wish-migration-{}.db", std::process::id()));
    let store = Store::open(&path).unwrap();
    let account = store.insert_account(&Account { id: 0, name: "Existing".into(), off_budget: false, closed: false, sort_order: 0.0 }).unwrap();
    store.insert_entry(&Entry { id: 0, date: today(), amount: 12345, notes: "Preserve".into(), cleared: true, account_id: Some(account), category_id: None, payee_id: None, schedule_id: None }).unwrap();
    drop(store);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("DROP TABLE wish; PRAGMA user_version=1;").unwrap();
    drop(conn);
    let mut migrated = Store::open(&path).unwrap();
    assert_eq!(migrated.account_balances().unwrap()[&account], 12345);
    let wish = save_wish(&mut migrated, None, draft(None)).unwrap();
    drop(migrated);
    let mut reopened = Store::open(&path).unwrap();
    assert_eq!(list_wishes(&mut reopened, today(), false).unwrap()[0].wish.id, wish.id);
    assert_eq!(reopened.account_balances().unwrap()[&account], 12345);
    drop(reopened);
    let conn = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0)).unwrap(), 2);
    drop(conn);
    std::fs::remove_file(path).unwrap();
}
