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
    WishDraft { name: "PlayStation".into(), price: 50_000, url: "https://example.com/ps5".into(), notes: "Con mando".into(), category_id: category, kind: WishKind::Compra, target_date: None, items: None }
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
    let mut payment=store.get_schedule(1).unwrap().unwrap();
    payment.account_id=None;
    store.update_schedule(&payment).unwrap();
    assert_eq!(evaluate_wish(&mut store,wish.id,today()).unwrap().status,"unknown");
}
#[test]
fn unknown_accounts_and_off_budget_cash_never_produce_affordable() {
    let mut store = Store::open_in_memory().unwrap();
    let wish = save_wish(&mut store, None, draft(None)).unwrap();
    let view = evaluate_wish(&mut store, wish.id, today()).unwrap();
    assert_eq!((view.status.as_str(), view.cash_shortfall), ("unknown", None));
    for (closed, off_budget) in [(true,false),(false,true)] {
        let account=store.insert_account(&Account {id:0,name:"Excluded".into(),off_budget,closed,sort_order:0.0}).unwrap();
        store.insert_entry(&Entry {id:0,date:today(),amount:1_000_000,notes:String::new(),cleared:true,account_id:Some(account),category_id:None,payee_id:None,schedule_id:None}).unwrap();
    }
    assert_eq!(evaluate_wish(&mut store,wish.id,today()).unwrap().cash_balance,None);
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
    assert_eq!(conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0)).unwrap(), 7);
    drop(conn);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn restore_wish_undoes_an_archive_including_its_reservation() {
    let (mut store, _, category) = seed();
    let wish = save_wish(&mut store, None, draft(Some(category))).unwrap();
    let before = reserve_wish(&mut store, wish.id, 20_000, today()).unwrap();
    archive_wish(&mut store, wish.id).unwrap();
    assert!(list_wishes(&mut store, today(), false).unwrap().is_empty());
    restore_wish(&mut store, &before).unwrap();
    let views = list_wishes(&mut store, today(), false).unwrap();
    assert_eq!((views.len(), views[0].wish.reserved, views[0].wish.archived), (1, 20_000, false));
    let missing = Wish { id: wish.id + 99, ..before };
    assert!(matches!(restore_wish(&mut store, &missing), Err(ServiceError::NotFound)));
}

fn day(y: i32, m: u32, d: u32) -> NaiveDate { NaiveDate::from_ymd_opt(y, m, d).unwrap() }

#[test]
fn goal_kind_and_target_date_are_saved_and_listed() {
    let (mut store, _, category) = seed();
    let trip = save_wish(&mut store, None, WishDraft { name: "Cancún".into(), kind: WishKind::Viaje, target_date: Some(day(2027, 6, 15)), ..draft(Some(category)) }).unwrap();
    let plain = save_wish(&mut store, None, draft(Some(category))).unwrap();
    let listed = list_wishes(&mut store, today(), false).unwrap();
    let found = |id| listed.iter().find(|v| v.wish.id == id).unwrap();
    assert_eq!((found(trip.id).wish.kind, found(trip.id).wish.target_date), (WishKind::Viaje, Some(day(2027, 6, 15))));
    assert_eq!((found(plain.id).wish.kind, found(plain.id).wish.target_date), (WishKind::Compra, None));
    assert_eq!(serde_json::to_value(&found(trip.id).wish).unwrap()["kind"], "viaje");
}

#[test]
fn a_target_date_gives_the_monthly_amount_to_set_aside() {
    let (mut store, _, category) = seed();
    // From Nov 10 2026 to Jun 15 2027 there are 8 monthly set-asides: Nov 10, Dec 10, … Jun 10.
    let trip = save_wish(&mut store, None, WishDraft { kind: WishKind::Viaje, target_date: Some(day(2027, 6, 15)), ..draft(Some(category)) }).unwrap();
    let plan = |store: &mut Store| list_wishes(store, today(), false).unwrap().into_iter().find(|v| v.wish.id == trip.id).unwrap().plan.unwrap();
    let p = plan(&mut store);
    assert_eq!((p.months_left, p.monthly, p.remaining, p.overdue), (8, 6_250, 50_000, false));
    reserve_wish(&mut store, trip.id, 10_000, today()).unwrap();
    assert_eq!((plan(&mut store).monthly, plan(&mut store).remaining), (5_000, 40_000));
    // The amount rounds up so the goal is reached on time.
    save_wish(&mut store, Some(trip.id), WishDraft { price: 50_001, target_date: Some(day(2027, 6, 15)), ..draft(Some(category)) }).unwrap();
    assert_eq!(plan(&mut store).monthly, 5_001);
}

#[test]
fn a_past_target_date_is_overdue_and_no_date_has_no_plan() {
    let (mut store, _, category) = seed();
    let late = save_wish(&mut store, None, WishDraft { target_date: Some(day(2026, 10, 1)), ..draft(Some(category)) }).unwrap();
    let open = save_wish(&mut store, None, draft(Some(category))).unwrap();
    let views = list_wishes(&mut store, today(), false).unwrap();
    let late = views.iter().find(|v| v.wish.id == late.id).unwrap().plan.as_ref().unwrap();
    assert_eq!((late.overdue, late.months_left, late.monthly), (true, 0, 50_000));
    assert!(views.iter().find(|v| v.wish.id == open.id).unwrap().plan.is_none());
    // Same month as the target: one last set-aside covers the rest.
    let soon = save_wish(&mut store, None, WishDraft { target_date: Some(day(2026, 11, 30)), ..draft(Some(category)) }).unwrap();
    let soon = list_wishes(&mut store, today(), false).unwrap().into_iter().find(|v| v.wish.id == soon.id).unwrap().plan.unwrap();
    assert_eq!((soon.months_left, soon.monthly, soon.overdue), (1, 50_000, false));
}

#[test]
fn archived_goals_have_no_plan() {
    let (mut store, _, category) = seed();
    let late = save_wish(&mut store, None, WishDraft { target_date: Some(day(2026, 10, 1)), ..draft(Some(category)) }).unwrap();
    archive_wish(&mut store, late.id).unwrap();
    assert!(list_wishes(&mut store, today(), true).unwrap()[0].plan.is_none());
}

fn item(name: &str, amount: Cents) -> WishItemDraft { WishItemDraft { id: None, name: name.into(), amount, paid: false } }
fn view(store: &mut Store, id: Id) -> WishView { list_wishes(store, today(), true).unwrap().into_iter().find(|v| v.wish.id == id).unwrap() }

#[test]
fn line_items_set_the_price_and_keep_their_ids_across_edits() {
    let (mut store, _, category) = seed();
    let trip = save_wish(&mut store, None, WishDraft { kind: WishKind::Viaje, items: Some(vec![item("Vuelo", 30_000), item("Hotel", 15_000)]), ..draft(Some(category)) }).unwrap();
    assert_eq!(trip.price, 45_000, "the price is the sum of the items");
    let v = view(&mut store, trip.id);
    assert_eq!(v.items.iter().map(|i| (i.name.as_str(), i.amount, i.paid)).collect::<Vec<_>>(), [("Vuelo", 30_000, false), ("Hotel", 15_000, false)]);
    let flight = v.items[0].id;
    // Edit: keep the flight (new amount), drop the hotel, add food.
    let edited = save_wish(&mut store, Some(trip.id), WishDraft { kind: WishKind::Viaje, items: Some(vec![WishItemDraft { id: Some(flight), name: "Vuelo".into(), amount: 32_000, paid: false }, item("Comida", 8_000)]), ..draft(Some(category)) }).unwrap();
    assert_eq!(edited.price, 40_000);
    let v = view(&mut store, trip.id);
    assert_eq!(v.items.iter().map(|i| (i.name.as_str(), i.amount)).collect::<Vec<_>>(), [("Vuelo", 32_000), ("Comida", 8_000)]);
    assert_eq!(v.items[0].id, flight, "an edited item keeps its id");
    // No items in the draft (None) keeps them; an empty list removes them and uses the draft price.
    save_wish(&mut store, Some(trip.id), WishDraft { price: 1, ..draft(Some(category)) }).unwrap();
    assert_eq!((view(&mut store, trip.id).items.len(), view(&mut store, trip.id).wish.price), (2, 40_000));
    save_wish(&mut store, Some(trip.id), WishDraft { items: Some(vec![]), ..draft(Some(category)) }).unwrap();
    assert_eq!((view(&mut store, trip.id).items.len(), view(&mut store, trip.id).wish.price), (0, 50_000));
}

#[test]
fn line_items_are_validated() {
    let (mut store, _, category) = seed();
    for (items, message) in [(vec![item(" ", 100)], "Line item name must not be empty"), (vec![item("Hotel", 0)], "Line item amount must be greater than zero")] {
        let err = save_wish(&mut store, None, WishDraft { items: Some(items), ..draft(Some(category)) }).unwrap_err();
        assert_eq!(err.to_string(), message);
    }
    let other = save_wish(&mut store, None, WishDraft { items: Some(vec![item("A", 100)]), ..draft(Some(category)) }).unwrap();
    let foreign = view(&mut store, other.id).items[0].id;
    let mine = save_wish(&mut store, None, draft(Some(category))).unwrap();
    let err = save_wish(&mut store, Some(mine.id), WishDraft { items: Some(vec![WishItemDraft { id: Some(foreign), name: "A".into(), amount: 100, paid: false }]), ..draft(Some(category)) }).unwrap_err();
    assert!(matches!(err, ServiceError::NotFound), "an item id from another goal is refused");
    // Items cannot bring the price under what is already set aside.
    let reserved = save_wish(&mut store, None, WishDraft { items: Some(vec![item("A", 30_000)]), ..draft(Some(category)) }).unwrap();
    reserve_wish(&mut store, reserved.id, 20_000, today()).unwrap();
    let err = save_wish(&mut store, Some(reserved.id), WishDraft { items: Some(vec![item("A", 10_000)]), ..draft(Some(category)) }).unwrap_err();
    assert_eq!(err.to_string(), "Release reserved funds before reducing the price");
}

#[test]
fn paid_items_count_as_progress_and_release_what_they_cover() {
    let (mut store, _, category) = seed();
    let trip = save_wish(&mut store, None, WishDraft { kind: WishKind::Viaje, target_date: Some(day(2027, 6, 15)), items: Some(vec![item("Vuelo", 30_000), item("Hotel", 20_000)]), ..draft(Some(category)) }).unwrap();
    reserve_wish(&mut store, trip.id, 25_000, today()).unwrap();
    let flight = view(&mut store, trip.id).items[0].id;
    // Paying the flight: 50,000 - 30,000 paid leaves 20,000 to cover, so 5,000 of the 25,000 set aside is released.
    let after = set_wish_item_paid(&mut store, flight, true, today()).unwrap();
    assert_eq!((after.paid, after.reserved), (30_000, 20_000));
    let v = view(&mut store, trip.id);
    assert!(v.items[0].paid);
    assert_eq!(v.plan.unwrap().remaining, 0);
    assert!(reserve_wish(&mut store, trip.id, 20_001, today()).is_err(), "the reservation limit is what is still unpaid");
    // Unmarking restores the room to reserve again but does not re-reserve.
    let back = set_wish_item_paid(&mut store, flight, false, today()).unwrap();
    assert_eq!((back.paid, back.reserved), (0, 20_000));
    assert!(matches!(set_wish_item_paid(&mut store, 9_999, true, today()), Err(ServiceError::NotFound)));
}

#[test]
fn every_change_to_the_set_aside_amount_is_logged_with_its_date() {
    let (mut store, _, category) = seed();
    let trip = save_wish(&mut store, None, WishDraft { items: Some(vec![item("Vuelo", 30_000), item("Hotel", 20_000)]), ..draft(Some(category)) }).unwrap();
    reserve_wish(&mut store, trip.id, 10_000, today()).unwrap();
    reserve_wish(&mut store, trip.id, 25_000, day(2026, 11, 20)).unwrap();
    reserve_wish(&mut store, trip.id, 25_000, day(2026, 11, 21)).unwrap(); // no change: nothing logged
    reserve_wish(&mut store, trip.id, 22_000, day(2026, 11, 22)).unwrap();
    // Paying the flight releases 2,000 (50,000 - 30,000 paid leaves 20,000 to cover).
    let flight = view(&mut store, trip.id).items[0].id;
    set_wish_item_paid(&mut store, flight, true, day(2026, 11, 23)).unwrap();
    let log: Vec<_> = view(&mut store, trip.id).contributions.iter().map(|c| (c.date, c.amount)).collect();
    assert_eq!(log, [(today(), 10_000), (day(2026, 11, 20), 15_000), (day(2026, 11, 22), -3_000), (day(2026, 11, 23), -2_000)]);
    let other = save_wish(&mut store, None, draft(Some(category))).unwrap();
    assert!(view(&mut store, other.id).contributions.is_empty(), "each goal has its own log");
}

#[test]
fn completing_a_goal_records_its_expense_and_keeps_it_as_an_achievement() {
    let (mut store, account, category) = seed();
    let bike = save_wish(&mut store, None, draft(Some(category))).unwrap();
    reserve_wish(&mut store, bike.id, 20_000, today()).unwrap();
    let done = complete_wish(&mut store, bike.id, account, today()).unwrap();
    assert_eq!((done.wish.archived, done.wish.reserved, done.wish.achieved_date), (true, 0, Some(today())));
    assert_eq!(done.entries.iter().map(|e| (e.amount, e.category_id, e.payee_name.as_deref(), e.notes.as_str())).collect::<Vec<_>>(),
        [(-50_000, Some(category), Some("PlayStation"), "Compra de un deseo")]);
    assert!(matches!(complete_wish(&mut store, bike.id, account, today()), Err(ServiceError::InvalidWish("Archived wishes cannot be edited"))));
}

#[test]
fn completing_a_goal_with_items_records_one_expense_per_unpaid_item() {
    let (mut store, account, category) = seed();
    let trip = save_wish(&mut store, None, WishDraft { name: "Cancún".into(), kind: WishKind::Viaje, items: Some(vec![item("Vuelo", 30_000), item("Hotel", 15_000), item("Comida", 5_000)]), ..draft(Some(category)) }).unwrap();
    let flight = view(&mut store, trip.id).items[0].id;
    set_wish_item_paid(&mut store, flight, true, today()).unwrap();
    let done = complete_wish(&mut store, trip.id, account, today()).unwrap();
    assert_eq!(done.entries.iter().map(|e| (e.amount, e.notes.as_str())).collect::<Vec<_>>(), [(-15_000, "Cancún · Hotel"), (-5_000, "Cancún · Comida")]);
    assert!(done.entries.iter().all(|e| e.payee_name.as_deref() == Some("Cancún") && e.category_id == Some(category)));
}

#[test]
fn completing_fails_without_writing_anything() {
    let (mut store, _, category) = seed();
    let bike = save_wish(&mut store, None, draft(Some(category))).unwrap();
    let before = store.list_entries().unwrap().len();
    assert!(matches!(complete_wish(&mut store, bike.id, 9_999, today()), Err(ServiceError::NotFound)));
    assert_eq!(store.list_entries().unwrap().len(), before);
    assert!(!view(&mut store, bike.id).wish.archived);
    // Undo: restoring the snapshot brings the goal back as it was.
    let (mut store, account, category) = seed();
    let bike = save_wish(&mut store, None, draft(Some(category))).unwrap();
    complete_wish(&mut store, bike.id, account, today()).unwrap();
    restore_wish(&mut store, &bike).unwrap();
    let back = view(&mut store, bike.id).wish;
    assert_eq!((back.archived, back.achieved_date), (false, None));
}

fn spend(store: &Store, account: Id, amount: Cents) -> Id {
    add_entry(store, NewEntry { date: today(), amount, account_id: account, payee: Some("Gasto".into()), category_id: None, notes: String::new(), cleared: true }).unwrap().id
}

#[test]
fn movements_tagged_to_a_goal_compare_spent_with_planned_per_item() {
    let (mut store, account, category) = seed();
    let trip = save_wish(&mut store, None, WishDraft { kind: WishKind::Viaje, items: Some(vec![item("Hotel", 15_000), item("Comida", 5_000)]), ..draft(Some(category)) }).unwrap();
    let (hotel, food) = { let v = view(&mut store, trip.id); (v.items[0].id, v.items[1].id) };
    let night1 = spend(&store, account, -8_000);
    let tacos = spend(&store, account, -1_200);
    let taxi = spend(&store, account, -600);
    tag_entry(&mut store, night1, Some(trip.id), Some(hotel)).unwrap();
    tag_entry(&mut store, tacos, Some(trip.id), Some(food)).unwrap();
    let row = tag_entry(&mut store, taxi, Some(trip.id), None).unwrap();
    assert_eq!((row.wish_id, row.wish_item_id), (Some(trip.id), None));
    let s = view(&mut store, trip.id).spending;
    assert_eq!((s.total, s.other), (9_800, 600));
    assert_eq!(s.items, [ItemSpend { item_id: hotel, spent: 8_000 }, ItemSpend { item_id: food, spent: 1_200 }]);
    // Untagging and deleting a movement both take it out.
    tag_entry(&mut store, taxi, None, None).unwrap();
    delete_entry(&store, tacos).unwrap();
    let s = view(&mut store, trip.id).spending;
    assert_eq!((s.total, s.other, s.items.len()), (8_000, 0, 1));
    assert_eq!(store.entry_row(night1).unwrap().unwrap().wish_item_id, Some(hotel));
}

#[test]
fn tags_are_validated_and_completion_tags_its_movements() {
    let (mut store, account, category) = seed();
    let a = save_wish(&mut store, None, WishDraft { items: Some(vec![item("X", 100)]), ..draft(Some(category)) }).unwrap();
    let b = save_wish(&mut store, None, draft(Some(category))).unwrap();
    let a_item = view(&mut store, a.id).items[0].id;
    let entry = spend(&store, account, -100);
    assert!(matches!(tag_entry(&mut store, entry, Some(b.id), Some(a_item)), Err(ServiceError::NotFound)), "the item must belong to the goal");
    assert!(matches!(tag_entry(&mut store, entry, None, Some(a_item)), Err(ServiceError::InvalidWish("Choose the goal of the line item"))));
    assert!(matches!(tag_entry(&mut store, 9_999, Some(a.id), None), Err(ServiceError::NotFound)));
    assert!(matches!(tag_entry(&mut store, entry, Some(9_999), None), Err(ServiceError::NotFound)));
    let done = complete_wish(&mut store, a.id, account, today()).unwrap();
    assert_eq!((done.entries[0].wish_id, done.entries[0].wish_item_id), (Some(a.id), Some(a_item)));
    assert_eq!(view(&mut store, a.id).spending.total, 100);
}
