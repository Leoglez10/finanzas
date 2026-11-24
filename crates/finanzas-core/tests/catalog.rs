use chrono::NaiveDate;
use finanzas_core::{service::*, store::Store};

fn day() -> NaiveDate { NaiveDate::from_ymd_opt(2026, 11, 10).unwrap() }

#[test]
fn accounts_go_last_with_their_opening_balance_and_optional_card() {
    let mut store = Store::open_in_memory().unwrap();
    let first = create_account(&mut store, " Banco ", false, Some(150_000), day(), None).unwrap();
    let card = create_account(&mut store, "Visa", false, Some(-30_000), day(), Some(CardTerms { credit_limit: Some(1_000_000), ..CardTerms::default() })).unwrap();
    let empty = create_account(&mut store, "Ahorro", true, Some(0), day(), None).unwrap();
    assert_eq!((first.name.as_str(), first.sort_order, card.sort_order, empty.off_budget), ("Banco", 1.0, 2.0, true));
    let balances = store.account_balances().unwrap();
    assert_eq!((balances[&first.id], balances[&card.id], balances[&empty.id]), (150_000, -30_000, 0));
    let (rows, _) = store.entries(&finanzas_core::store::EntryFilter { account_id: Some(empty.id), ..Default::default() }).unwrap();
    assert!(rows.is_empty(), "a zero opening balance adds no movement");
    let debts = debts(&store, day()).unwrap();
    assert_eq!((debts.debts[0].name.as_str(), debts.debts[0].available), ("Visa", Some(970_000)));
    assert!(create_account(&mut store, "  ", false, None, day(), None).is_err());
    // A bad card term leaves no half-created account.
    assert!(create_account(&mut store, "Mala", false, Some(-100), day(), Some(CardTerms { cut_day: Some(40), ..CardTerms::default() })).is_err());
    assert_eq!(store.list_accounts().unwrap().len(), 3);
}

#[test]
fn categories_and_groups_are_created_renamed_and_hidden() {
    let mut store = Store::open_in_memory().unwrap();
    let group = save_category_group(&mut store, None, "Hogar").unwrap();
    let second = save_category_group(&mut store, None, "Ocio").unwrap();
    assert_eq!((group.is_income, group.sort_order, second.sort_order), (false, 1.0, 2.0));
    let renamed = save_category_group(&mut store, Some(group.id), "Casa").unwrap();
    assert_eq!(renamed.name, "Casa");
    let rent = create_category(&mut store, group.id, "Renta").unwrap();
    let power = create_category(&mut store, group.id, "Luz").unwrap();
    assert_eq!((rent.sort_order, power.sort_order, power.group_id), (1.0, 2.0, Some(group.id)));
    let hidden = update_category(&mut store, power.id, Some("Electricidad"), Some(true)).unwrap();
    assert_eq!((hidden.name.as_str(), hidden.hidden), ("Electricidad", true));
    let kept = update_category(&mut store, power.id, None, Some(false)).unwrap();
    assert_eq!((kept.name.as_str(), kept.hidden), ("Electricidad", false));
    assert!(create_category(&mut store, 999, "X").is_err());
    assert!(create_category(&mut store, group.id, " ").is_err());
    assert!(update_category(&mut store, 999, Some("X"), None).is_err());
    assert!(save_category_group(&mut store, Some(999), "X").is_err());
}
