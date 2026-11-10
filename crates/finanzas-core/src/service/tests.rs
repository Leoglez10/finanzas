use super::*;
use crate::{Account, Category, CategoryGroup, Entry, Schedule, Recurrence, Frequency, WeekendMode, store::Store};
use chrono::NaiveDate;

fn day(y: i32, m: u32, d: u32) -> NaiveDate { NaiveDate::from_ymd_opt(y, m, d).unwrap() }

#[test]
fn overview_first_run_requires_an_open_account() {
    let store = Store::open_in_memory().unwrap();
    store.insert_account(&Account { id: 0, name: "Cerrada".into(), off_budget: false, closed: true, sort_order: 0.0 }).unwrap();
    let view = overview(&store, 202610, 0).unwrap();
    assert!(!view.first_run.has_account);
}

#[test]
fn overview_budget_progress_selects_largest_positive_assignments_and_net_activity() {
    let store = Store::open_in_memory().unwrap();
    let account = store.insert_account(&Account { id: 0, name: "Banco".into(), off_budget: false, closed: false, sort_order: 0.0 }).unwrap();
    let group = store.insert_category_group(&CategoryGroup { id: 0, name: "Gastos".into(), is_income: false, sort_order: 0.0 }).unwrap();
    let mut categories = Vec::new();
    for i in 0..7 {
        let category = store.insert_category(&Category { id: 0, name: format!("Categoría {i}"), hidden: false, sort_order: i as f64, group_id: Some(group) }).unwrap();
        store.set_budget(202610, category, i * 100).unwrap();
        categories.push(category);
    }
    for (category, amount) in [(categories[0], -900), (categories[6], -80), (categories[6], 100)] {
        store.insert_entry(&Entry { id: 0, date: day(2026, 10, 5), amount, notes: String::new(), cleared: false, account_id: Some(account), category_id: Some(category), payee_id: None, schedule_id: None }).unwrap();
    }
    let view = overview(&store, 202610, 0).unwrap();
    assert_eq!(view.budget_progress.iter().map(|r| (r.category_id, r.spent, r.budgeted)).collect::<Vec<_>>(), vec![(categories[6], -20, 600), (categories[5], 0, 500), (categories[4], 0, 400), (categories[3], 0, 300), (categories[2], 0, 200)]);
    assert_eq!((view.budget_spent, view.budget_assigned), (900, 2100));
}

#[test]
fn overview_checks_and_cards_follow_open_accounts_and_all_entries() {
    let store = Store::open_in_memory().unwrap();
    let open = store.insert_account(&Account { id: 0, name: "Abierta".into(), off_budget: true, closed: false, sort_order: 0.0 }).unwrap();
    let closed = store.insert_account(&Account { id: 0, name: "Cerrada".into(), off_budget: false, closed: true, sort_order: 1.0 }).unwrap();
    let income_group = store.insert_category_group(&CategoryGroup { id: 0, name: "Ingresos".into(), is_income: true, sort_order: 0.0 }).unwrap();
    let expense_group = store.insert_category_group(&CategoryGroup { id: 0, name: "Gastos".into(), is_income: false, sort_order: 1.0 }).unwrap();
    let income = store.insert_category(&Category { id: 0, name: "Sueldo".into(), hidden: false, sort_order: 0.0, group_id: Some(income_group) }).unwrap();
    let expense = store.insert_category(&Category { id: 0, name: "Comida".into(), hidden: false, sort_order: 1.0, group_id: Some(expense_group) }).unwrap();
    for (date, account, category, amount) in [(1, open, income, -10), (2, closed, expense, 30)] {
        store.insert_entry(&Entry { id: 0, date: day(2026, 10, date), amount, notes: String::new(), cleared: false, account_id: Some(account), category_id: Some(category), payee_id: None, schedule_id: None }).unwrap();
    }
    store.set_budget(202610, expense, -1).unwrap();
    let view = overview(&store, 202610, 6).unwrap();
    assert_eq!((view.open_account_count, view.total_balance), (1, -10));
    assert!(view.first_run.has_account && !view.first_run.has_categorized_income && !view.first_run.has_budget);
    store.insert_entry(&Entry { id: 0, date: day(2026, 10, 3), amount: 20, notes: String::new(), cleared: false, account_id: Some(open), category_id: Some(income), payee_id: None, schedule_id: None }).unwrap();
    store.set_budget(202609, expense, 1).unwrap();
    let view = overview(&store, 202610, 6).unwrap();
    assert!(view.first_run.has_categorized_income && view.first_run.has_budget);
    assert_eq!(view.total_balance, 10);
}

#[test]
fn overview_recent_eight_and_spending_top_six_are_sorted() {
    let store = Store::open_in_memory().unwrap();
    let account = store.insert_account(&Account { id: 0, name: "Banco".into(), off_budget: false, closed: false, sort_order: 0.0 }).unwrap();
    let group = store.insert_category_group(&CategoryGroup { id: 0, name: "Gastos".into(), is_income: false, sort_order: 0.0 }).unwrap();
    for i in 1..=9 {
        let category = store.insert_category(&Category { id: 0, name: format!("Categoría {i}"), hidden: false, sort_order: i as f64, group_id: Some(group) }).unwrap();
        store.insert_entry(&Entry { id: 0, date: day(2026, 10, i), amount: -(i as i64) * 100, notes: String::new(), cleared: false, account_id: Some(account), category_id: Some(category), payee_id: None, schedule_id: None }).unwrap();
    }
    let view = overview(&store, 202610, 6).unwrap();
    assert_eq!(view.spending_total, 4500);
    assert_eq!(view.spending.iter().map(|r| r.spent).collect::<Vec<_>>(), [900, 800, 700, 600, 500, 400]);
    assert_eq!(view.recent_entries.iter().map(|e| e.date).collect::<Vec<_>>(), (2..=9).rev().map(|d| day(2026, 10, d)).collect::<Vec<_>>());
}

#[test]
fn posting_creates_linked_entry_and_advances_and_auto_posts_due() {
    let mut store = Store::open_in_memory().unwrap();
    let account = store.insert_account(&Account { id: 0, name: "Banco".into(), off_budget: false, closed: false, sort_order: 1.0 }).unwrap();
    let schedule = Schedule { id: 0, name: "Gym".into(), amount: -55000, auto_post: true, completed: false, next_date: None, recurrence: Some(Recurrence { start: day(2026, 8, 11), frequency: Frequency::Monthly, interval: 1, skip_weekend: false, weekend_mode: WeekendMode::After, end_date: None }), account_id: Some(account), category_id: None, payee_id: None };
    let id = store.insert_schedule(&schedule).unwrap();
    let views = schedule_views(&store).unwrap();
    assert_eq!(views[0].next_date, Some(day(2026, 8, 11)));
    let items = upcoming(&store, day(2026, 10, 15), 30).unwrap();
    assert_eq!(items[0].projected_balance, Some(-55_000));
    assert!(items[0].overdue);
    assert!(!reminders(&store, day(2026, 10, 15).and_hms_opt(12, 0, 0).unwrap(), 2, 9).unwrap().is_empty());
    let entry = post_schedule(&mut store, id, day(2026, 10, 15)).unwrap();
    assert_eq!((entry.date, entry.schedule_id), (day(2026, 8, 11), Some(id)));
    assert_eq!(auto_post_due(&mut store, day(2026, 10, 15)).unwrap(), 2);
    assert_eq!(store.get_schedule(id).unwrap().unwrap().next_date, Some(day(2026, 11, 11)));
    let skipped = skip_schedule(&mut store, id, Some(day(2027, 1, 31))).unwrap();
    assert_eq!((skipped.skipped, skipped.next_date), (3, Some(day(2027, 2, 11))));
    let mut last = schedule.clone();
    last.auto_post = false;
    last.recurrence.as_mut().unwrap().end_date = Some(day(2026, 8, 11));
    let last_id = store.insert_schedule(&last).unwrap();
    post_schedule(&mut store, last_id, day(2026, 10, 15)).unwrap();
    assert!(store.get_schedule(last_id).unwrap().unwrap().completed);
    assert!(matches!(skip_schedule(&mut store, last_id, None), Err(ServiceError::CompletedSchedule)));
    assert!(matches!(post_schedule(&mut store, last_id, day(2026, 10, 15)), Err(ServiceError::CompletedSchedule)));
}

#[test]
fn budget_month_matches_envelope_rules() {
    let store = Store::open_in_memory().unwrap();
    let bank = store.insert_account(&Account { id: 0, name: "Banco".into(), off_budget: false, closed: false, sort_order: 1.0 }).unwrap();
    let invest = store.insert_account(&Account { id: 0, name: "Inversión".into(), off_budget: true, closed: false, sort_order: 2.0 }).unwrap();
    let expenses = store.insert_category_group(&CategoryGroup { id: 0, name: "Gastos".into(), is_income: false, sort_order: 1.0 }).unwrap();
    let income = store.insert_category_group(&CategoryGroup { id: 0, name: "Ingresos".into(), is_income: true, sort_order: 0.0 }).unwrap();
    let food = store.insert_category(&Category { id: 0, name: "Comida".into(), hidden: false, sort_order: 1.0, group_id: Some(expenses) }).unwrap();
    let salary = store.insert_category(&Category { id: 0, name: "Sueldo".into(), hidden: false, sort_order: 1.0, group_id: Some(income) }).unwrap();
    for (account, category, amount) in [(bank, salary, 1_000_000), (bank, food, -12_550), (invest, food, -50_000)] {
        store.insert_entry(&Entry { id: 0, date: day(2026, 9, 5), amount, notes: String::new(), cleared: false, account_id: Some(account), category_id: Some(category), payee_id: None, schedule_id: None }).unwrap();
    }
    store.set_budget(202609, food, 30_000).unwrap();
    let dashboard = overview(&store, 202610, 2).unwrap();
    assert_eq!(dashboard.chart.iter().map(|p| (p.month, p.income, p.expenses)).collect::<Vec<_>>(), [(202609, 1_000_000, 12_550), (202610, 0, 0)]);
    assert!(dashboard.first_run.has_account && dashboard.first_run.has_categorized_income && dashboard.first_run.has_budget);
    let spending = spending_by_category(&store, day(2026, 9, 1), day(2026, 9, 30), false).unwrap();
    assert_eq!(spending.total, 12_550);
    assert_eq!(spending.categories[0].category, "Comida");
    assert_eq!(spending_by_category(&store, day(2026, 9, 1), day(2026, 9, 30), true).unwrap().total, 62_550);
    let view = budget_month(&store, 202609).unwrap();
    assert_eq!((view.to_budget, view.income, view.assigned), (970_000, 1_000_000, 30_000));
    assert_eq!(view.groups.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["Gastos", "Ingresos"]);
    let row = &view.groups[0].categories[0];
    assert_eq!((row.budgeted, row.activity, row.available), (30_000, -12_550, 17_450));
    assert_eq!(set_budget_from_text(&store, 202609, food, "+100").unwrap(), Some(40_000));
    assert!(matches!(set_budget_from_text(&store, 202609, salary, "100"), Err(ServiceError::IncomeCategory)));
    assert_eq!(set_budget_from_text(&store, 202609, food, "invalid").unwrap(), None);
}

#[test]
fn add_and_update_transaction_reuses_payee_and_learns_category() {
    let store = Store::open_in_memory().unwrap();
    let account = store.insert_account(&Account { id: 0, name: "Banco".into(), off_budget: false, closed: false, sort_order: 1.0 }).unwrap();
    let category = store.insert_category(&Category { id: 0, name: "Comida".into(), hidden: false, sort_order: 1.0, group_id: None }).unwrap();
    let empty = overview(&store, 202610, 0).unwrap();
    assert!(empty.first_run.has_account && !empty.first_run.has_categorized_income && !empty.first_run.has_budget);
    assert!(empty.chart.is_empty());
    let first = add_entry(&store, NewEntry { date: day(2026, 9, 5), amount: -12_550, account_id: account, payee: Some("Oxxo".into()), category_id: Some(category), notes: "súper".into(), cleared: true }).unwrap();
    let second = add_entry(&store, NewEntry { date: day(2026, 10, 14), amount: -2_000, account_id: account, payee: Some("OXXO".into()), category_id: None, notes: String::new(), cleared: true }).unwrap();
    assert_eq!(second.category_id, Some(category));
    assert_eq!(first.payee_id, second.payee_id);
    assert_eq!(store.list_payees().unwrap().len(), 1);
    assert!(matches!(add_entry(&store, NewEntry { date: day(2026, 10, 14), amount: -1, account_id: 999, payee: None, category_id: None, notes: String::new(), cleared: true }), Err(ServiceError::NotFound)));
    let updated = update_entry(&store, second.id, EntryChanges { notes: Some("corregido".into()), payee: Some("Uber".into()), ..Default::default() }).unwrap();
    assert_eq!(updated.amount, -2_000);
    assert_eq!(updated.payee_name.as_deref(), Some("Uber"));
    assert_eq!(updated.notes, "corregido");
    assert_eq!(delete_entry(&store, second.id).unwrap().id, second.id);
    assert!(matches!(delete_entry(&store, second.id), Err(ServiceError::NotFound)));
}
