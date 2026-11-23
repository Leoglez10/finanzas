use chrono::NaiveDate;
use finanzas_core::{service::*, store::Store, *};

fn day(y: i32, m: u32, d: u32) -> NaiveDate { NaiveDate::from_ymd_opt(y, m, d).unwrap() }
fn account(store: &Store, name: &str, closed: bool) -> Id {
    store.insert_account(&Account { id: 0, name: name.into(), off_budget: false, closed, sort_order: 0.0 }).unwrap()
}
fn entry(store: &Store, account: Id, date: NaiveDate, amount: Cents) {
    store.insert_entry(&Entry { id: 0, date, amount, notes: String::new(), cleared: true, account_id: Some(account), category_id: None, payee_id: None, schedule_id: None }).unwrap();
}
fn full_terms() -> CardTerms { CardTerms { credit_limit: Some(5_000_000), cut_day: Some(5), due_day: Some(25), annual_rate: Some(6000), min_payment: Some(1000) } }

#[test]
fn a_card_without_terms_is_listed_with_its_debt_and_what_is_missing() {
    let store = Store::open_in_memory().unwrap();
    let card = account(&store, "Tarjeta", false);
    entry(&store, card, day(2026, 11, 2), -120_000);
    set_card_terms(&store, card, Some(CardTerms::default())).unwrap();
    let view = debts(&store, day(2026, 11, 10)).unwrap();
    let d = &view.debts[0];
    assert!(d.is_card);
    assert_eq!((d.debt, view.total), (120_000, 120_000));
    assert_eq!(d.missing, vec!["creditLimit", "cutDay", "dueDay", "annualRate", "minPayment"]);
    assert_eq!((d.available, d.statement, d.to_avoid_interest, d.minimum, d.interest_if_minimum, d.due_date), (None, None, None, None, None, None));
}

#[test]
fn full_terms_give_statement_payments_and_interest() {
    let store = Store::open_in_memory().unwrap();
    let card = account(&store, "Tarjeta", false);
    entry(&store, card, day(2026, 11, 2), -300_000);
    entry(&store, card, day(2026, 11, 8), -100_000);
    entry(&store, card, day(2026, 11, 9), 50_000);
    set_card_terms(&store, card, Some(full_terms())).unwrap();
    let d = debts(&store, day(2026, 11, 10)).unwrap().debts.remove(0);
    assert_eq!(d.debt, 350_000);
    assert_eq!((d.available, d.utilization), (Some(4_650_000), Some(0.07)));
    assert_eq!((d.last_cut, d.next_cut, d.due_date, d.overdue), (Some(day(2026, 11, 5)), Some(day(2026, 12, 5)), Some(day(2026, 11, 25)), false));
    assert_eq!(d.statement, Some(300_000));
    assert_eq!(d.to_avoid_interest, Some(250_000));
    assert_eq!(d.minimum, Some(0), "the 500 paid already covers the 300 minimum");
    // (3,000 − 300 minimum) × 60% / 12 × 1.16 IVA
    assert_eq!(d.interest_if_minimum, Some(15_660));
    assert!(d.missing.is_empty());
}

#[test]
fn installments_leave_only_the_billed_part_in_the_statement() {
    let mut store = Store::open_in_memory().unwrap();
    let card = account(&store, "Tarjeta", false);
    entry(&store, card, day(2026, 10, 1), -120_000);
    set_card_terms(&store, card, Some(full_terms())).unwrap();
    let plan = save_installment(&mut store, None, InstallmentDraft { account_id: card, name: "Laptop".into(), total: 120_000, months: 12, start_date: day(2026, 10, 1), annual_rate: 0 }).unwrap();
    let view = debts(&store, day(2026, 11, 10)).unwrap();
    let d = &view.debts[0];
    assert_eq!(d.debt, 120_000, "the whole purchase uses the credit limit");
    assert_eq!(d.statement, Some(20_000), "two monthly charges billed (Oct 5 and Nov 5)");
    let p = &d.installments[0];
    assert_eq!((p.installment.id, p.monthly, p.billed, p.left, p.remaining), (plan.id, 10_000, 2, 10, 100_000));
    assert_eq!(view.monthly_installments, 10_000);
}

#[test]
fn installments_with_interest_amortize_and_add_this_months_interest() {
    let mut store = Store::open_in_memory().unwrap();
    let card = account(&store, "Tarjeta", false);
    entry(&store, card, day(2026, 10, 1), -120_000);
    set_card_terms(&store, card, Some(full_terms())).unwrap();
    save_installment(&mut store, None, InstallmentDraft { account_id: card, name: "Tele".into(), total: 120_000, months: 12, start_date: day(2026, 10, 1), annual_rate: 2400 }).unwrap();
    let d = debts(&store, day(2026, 11, 10)).unwrap().debts.remove(0);
    let p = &d.installments[0];
    // r = 24% / 12 × 1.16 = 2.32% a month
    assert_eq!(p.monthly, 11_571);
    // With interest less principal is paid off than with MSI: about 1,022.21 is left after two charges.
    assert_eq!(p.remaining, 102_221);
    // Two principals billed (1,200 − 1,022.21) plus the second charge's interest (2.32% of 1,112.12).
    assert_eq!(d.statement, Some(20_359));
}

#[test]
fn due_date_after_paying_moves_on_and_unpaid_is_overdue() {
    let store = Store::open_in_memory().unwrap();
    let card = account(&store, "Tarjeta", false);
    entry(&store, card, day(2026, 11, 2), -300_000);
    set_card_terms(&store, card, Some(full_terms())).unwrap();
    let late = debts(&store, day(2026, 11, 28)).unwrap().debts.remove(0);
    assert_eq!((late.overdue, late.due_date, late.to_avoid_interest), (true, Some(day(2026, 11, 25)), Some(300_000)));
    entry(&store, card, day(2026, 11, 20), 300_000);
    let paid = debts(&store, day(2026, 11, 28)).unwrap().debts.remove(0);
    assert_eq!((paid.overdue, paid.due_date, paid.to_avoid_interest), (false, Some(day(2026, 12, 25)), Some(0)));
}

#[test]
fn a_cut_day_past_the_month_end_falls_on_its_last_day() {
    let store = Store::open_in_memory().unwrap();
    let card = account(&store, "Tarjeta", false);
    set_card_terms(&store, card, Some(CardTerms { cut_day: Some(31), ..CardTerms::default() })).unwrap();
    let d = debts(&store, day(2027, 3, 10)).unwrap().debts.remove(0);
    assert_eq!((d.last_cut, d.next_cut), (Some(day(2027, 2, 28)), Some(day(2027, 3, 31))));
}

#[test]
fn loans_are_negative_accounts_and_highest_rate_goes_first() {
    let store = Store::open_in_memory().unwrap();
    let loan = account(&store, "Préstamo", false);
    entry(&store, loan, day(2026, 1, 1), -900_000);
    let cheap = account(&store, "Tarjeta barata", false);
    entry(&store, cheap, day(2026, 11, 1), -10_000);
    set_card_terms(&store, cheap, Some(CardTerms { annual_rate: Some(2000), ..CardTerms::default() })).unwrap();
    let pricey = account(&store, "Tarjeta cara", false);
    entry(&store, pricey, day(2026, 11, 1), -10_000);
    set_card_terms(&store, pricey, Some(CardTerms { annual_rate: Some(7000), ..CardTerms::default() })).unwrap();
    let bank = account(&store, "Banco", false);
    entry(&store, bank, day(2026, 11, 1), 50_000);
    let closed = account(&store, "Vieja", true);
    entry(&store, closed, day(2026, 11, 1), -50_000);
    let view = debts(&store, day(2026, 11, 10)).unwrap();
    assert_eq!(view.debts.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), vec!["Tarjeta cara", "Tarjeta barata", "Préstamo"]);
    assert!(!view.debts[2].is_card && view.debts[2].missing.is_empty());
    assert_eq!(view.total, 920_000);
}

#[test]
fn invalid_terms_and_plans_are_refused_and_removal_returns_the_old_values() {
    let mut store = Store::open_in_memory().unwrap();
    let card = account(&store, "Tarjeta", false);
    assert!(set_card_terms(&store, card, Some(CardTerms { cut_day: Some(40), ..CardTerms::default() })).is_err());
    assert!(set_card_terms(&store, card, Some(CardTerms { credit_limit: Some(0), ..CardTerms::default() })).is_err());
    assert!(set_card_terms(&store, 999, Some(CardTerms::default())).is_err());
    let bad = InstallmentDraft { account_id: card, name: "X".into(), total: 1_000, months: 0, start_date: day(2026, 1, 1), annual_rate: 0 };
    assert!(save_installment(&mut store, None, bad.clone()).is_err());
    assert!(save_installment(&mut store, None, InstallmentDraft { months: 3, name: " ".into(), ..bad.clone() }).is_err());
    let plan = save_installment(&mut store, None, InstallmentDraft { months: 3, ..bad }).unwrap();
    assert_eq!(delete_installment(&mut store, plan.id).unwrap(), plan);
    assert!(delete_installment(&mut store, plan.id).is_err());
    set_card_terms(&store, card, Some(CardTerms::default())).unwrap();
    set_card_terms(&store, card, None).unwrap();
    assert!(debts(&store, day(2026, 11, 10)).unwrap().debts.is_empty(), "no longer a card and no debt");
}
