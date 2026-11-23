//! Debts: credit cards (with optional terms) and any other account that owes money.
//! Everything a card's terms enable is optional: missing terms only leave those numbers out.

use super::{Result, ServiceError};
use crate::{store::Store, Cents, Id};
use chrono::{Datelike, Months, NaiveDate};
use serde::{Deserialize, Serialize};

/// ponytail: Mexican VAT charged on interest; make it a setting if the app is used elsewhere.
const IVA: f64 = 1.16;

/// Card terms; every field may be empty. Rates are basis points (4500 = 45%).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CardTerms {
    pub credit_limit: Option<Cents>, pub cut_day: Option<u32>, pub due_day: Option<u32>,
    pub annual_rate: Option<i64>, pub min_payment: Option<i64>,
}
/// A purchase split into monthly charges; `annual_rate` 0 means without interest (MSI).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installment { pub id: Id, pub account_id: Id, pub name: String, pub total: Cents, pub months: u32, pub start_date: NaiveDate, pub annual_rate: i64 }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallmentDraft { pub account_id: Id, pub name: String, pub total: Cents, pub months: u32, pub start_date: NaiveDate, #[serde(default)] pub annual_rate: i64 }
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallmentView { pub installment: Installment, pub monthly: Cents, pub billed: u32, pub left: u32, pub remaining: Cents }
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebtView {
    pub account_id: Id, pub name: String, pub is_card: bool, pub terms: Option<CardTerms>,
    /// What is owed now, including installments not billed yet.
    pub debt: Cents,
    pub available: Option<Cents>, pub utilization: Option<f64>,
    pub last_cut: Option<NaiveDate>, pub next_cut: Option<NaiveDate>, pub due_date: Option<NaiveDate>, pub overdue: bool,
    /// Owed at the last cut, without installments still to come.
    pub statement: Option<Cents>,
    /// What is left to pay of the statement so it carries no interest.
    pub to_avoid_interest: Option<Cents>, pub minimum: Option<Cents>, pub interest_if_minimum: Option<Cents>,
    pub installments: Vec<InstallmentView>,
    /// Terms not filled in, by field name, so the UI can say what they would unlock.
    pub missing: Vec<&'static str>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebtsView { pub total: Cents, pub monthly_installments: Cents, pub debts: Vec<DebtView> }

fn invalid(message: &'static str) -> ServiceError { ServiceError::InvalidDebt(message) }

/// `day` of the month `months` after (or before) `date`'s month, moved to the month's last day when it is shorter.
fn on_day(date: NaiveDate, months: i32, day: u32) -> NaiveDate {
    let first = date.with_day(1).unwrap();
    let first = if months >= 0 { first + Months::new(months as u32) } else { first - Months::new(months.unsigned_abs()) };
    (0..4).find_map(|back| first.with_day(day.saturating_sub(back).max(1))).unwrap()
}
/// The first date strictly after `date` that falls on `day`.
fn next_after(date: NaiveDate, day: u32) -> NaiveDate { let this = on_day(date, 0, day); if this > date { this } else { on_day(date, 1, day) } }
fn last_on_or_before(date: NaiveDate, day: u32) -> NaiveDate { let this = on_day(date, 0, day); if this <= date { this } else { on_day(date, -1, day) } }

fn monthly_rate(annual_bp: i64) -> f64 { annual_bp as f64 / 10_000.0 / 12.0 * IVA }
fn payment(p: &Installment) -> f64 {
    let r = monthly_rate(p.annual_rate);
    if r == 0.0 { p.total as f64 / p.months as f64 } else { p.total as f64 * r / (1.0 - (1.0 + r).powi(-(p.months as i32))) }
}
/// Principal still owed after `k` monthly charges.
fn remaining(p: &Installment, k: u32) -> f64 {
    if k >= p.months { return 0.0; }
    let r = monthly_rate(p.annual_rate);
    if r == 0.0 { return (p.total - p.total * k as i64 / p.months as i64) as f64; }
    let g = (1.0 + r).powi(k as i32);
    p.total as f64 * g - payment(p) * (g - 1.0) / r
}
/// Charge dates: the first cycle day on or after the purchase, then monthly.
fn charge_date(p: &Installment, cycle: u32, k: u32) -> NaiveDate {
    let first = last_on_or_before(p.start_date, cycle);
    let first = if first < p.start_date { next_after(p.start_date - chrono::Days::new(1), cycle) } else { first };
    on_day(first, k as i32, cycle)
}
fn billed(p: &Installment, cycle: u32, at: NaiveDate) -> u32 { (0..p.months).take_while(|&k| charge_date(p, cycle, k) <= at).count() as u32 }

pub fn debts(store: &Store, today: NaiveDate) -> Result<DebtsView> {
    let terms = store.card_terms()?;
    let plans = store.installments()?;
    let balances = store.account_balances()?;
    let mut list = Vec::new();
    for account in store.list_accounts()?.into_iter().filter(|a| !a.closed) {
        let balance = balances.get(&account.id).copied().unwrap_or(0);
        let card = terms.get(&account.id).cloned();
        if card.is_none() && balance >= 0 { continue; }
        let t = card.clone().unwrap_or_default();
        let own: Vec<&Installment> = plans.iter().filter(|p| p.account_id == account.id).collect();
        let debt = (-balance).max(0);
        let mut view = DebtView {
            account_id: account.id, name: account.name, is_card: card.is_some(), terms: card.clone(), debt,
            available: t.credit_limit.map(|l| l - debt), utilization: t.credit_limit.map(|l| debt as f64 / l as f64),
            last_cut: None, next_cut: None, due_date: None, overdue: false, statement: None, to_avoid_interest: None, minimum: None, interest_if_minimum: None,
            installments: own.iter().map(|p| {
                let n = billed(p, t.cut_day.unwrap_or(p.start_date.day()), today);
                InstallmentView { installment: (*p).clone(), monthly: payment(p).round() as Cents, billed: n, left: p.months - n, remaining: remaining(p, n).round() as Cents }
            }).collect(),
            missing: if card.is_some() { [("creditLimit", t.credit_limit.is_none()), ("cutDay", t.cut_day.is_none()), ("dueDay", t.due_day.is_none()), ("annualRate", t.annual_rate.is_none()), ("minPayment", t.min_payment.is_none())].into_iter().filter(|m| m.1).map(|m| m.0).collect() } else { Vec::new() },
        };
        if let Some(cut) = t.cut_day {
            let last = last_on_or_before(today, cut);
            let previous = on_day(last, -1, cut);
            let next = next_after(last, cut);
            // Installments are in the ledger in full: keep out what is not billed yet, add this cycle's interest.
            let mut adjust = 0.0;
            for p in &own {
                let n = billed(p, cut, last);
                adjust -= remaining(p, n);
                if n >= 1 && charge_date(p, cut, n - 1) > previous { adjust += payment(p) - (remaining(p, n - 1) - remaining(p, n)); }
            }
            let statement = ((-store.balance_through(view.account_id, last)?) as f64 + adjust).round().max(0.0) as Cents;
            let paid = store.inflows_between(view.account_id, last, today)?;
            let pending = (statement - paid).max(0);
            let minimum_full = t.min_payment.map(|bp| statement * bp / 10_000);
            view.last_cut = Some(last); view.next_cut = Some(next); view.statement = Some(statement); view.to_avoid_interest = Some(pending);
            view.minimum = minimum_full.map(|m| (m - paid).max(0).min(pending));
            view.interest_if_minimum = t.annual_rate.zip(minimum_full).map(|(bp, m)| ((statement - m) as f64 * monthly_rate(bp)).round() as Cents);
            if let Some(due_day) = t.due_day {
                let due = next_after(last, due_day);
                view.overdue = today > due && pending > 0;
                view.due_date = Some(if today > due && pending == 0 { next_after(next, due_day) } else { due });
            }
        }
        list.push(view);
    }
    // Highest rate first (pay it down first), unknown rates last, then the biggest debt.
    list.sort_by_key(|d| (std::cmp::Reverse(d.terms.as_ref().and_then(|t| t.annual_rate).unwrap_or(-1)), std::cmp::Reverse(d.debt)));
    Ok(DebtsView {
        total: list.iter().map(|d| d.debt).sum(),
        monthly_installments: list.iter().flat_map(|d| &d.installments).filter(|p| p.left > 0).map(|p| p.monthly).sum(),
        debts: list,
    })
}

/// `None` stops treating the account as a credit card (its installments stay).
pub fn set_card_terms(store: &Store, account_id: Id, terms: Option<CardTerms>) -> Result<()> {
    if !store.account_exists(account_id)? { return Err(ServiceError::NotFound); }
    if let Some(t) = &terms {
        if t.credit_limit.is_some_and(|l| l <= 0) { return Err(invalid("El límite debe ser mayor a cero")); }
        if [t.cut_day, t.due_day].into_iter().flatten().any(|d| !(1..=31).contains(&d)) { return Err(invalid("El día debe estar entre 1 y 31")); }
        if t.annual_rate.is_some_and(|r| !(0..=100_000).contains(&r)) { return Err(invalid("La tasa debe estar entre 0% y 1000%")); }
        if t.min_payment.is_some_and(|m| !(0..=10_000).contains(&m)) { return Err(invalid("El pago mínimo debe estar entre 0% y 100%")); }
    }
    Ok(store.put_card_terms(account_id, terms.as_ref())?)
}

pub fn save_installment(store: &mut Store, id: Option<Id>, draft: InstallmentDraft) -> Result<Installment> {
    let name = draft.name.trim();
    if name.is_empty() { return Err(invalid("Ponle un nombre a la compra")); }
    if draft.total <= 0 { return Err(invalid("El monto debe ser mayor a cero")); }
    if !(1..=120).contains(&draft.months) { return Err(invalid("Los meses deben estar entre 1 y 120")); }
    if !(0..=100_000).contains(&draft.annual_rate) { return Err(invalid("La tasa debe estar entre 0% y 1000%")); }
    if !store.account_exists(draft.account_id)? { return Err(ServiceError::NotFound); }
    if let Some(id) = id { if !store.installments()?.iter().any(|i| i.id == id) { return Err(ServiceError::NotFound); } }
    let mut plan = Installment { id: id.unwrap_or(0), account_id: draft.account_id, name: name.into(), total: draft.total, months: draft.months, start_date: draft.start_date, annual_rate: draft.annual_rate };
    plan.id = store.put_installment(&plan)?;
    Ok(plan)
}

/// Returns the removed plan, for undo.
pub fn delete_installment(store: &mut Store, id: Id) -> Result<Installment> { store.remove_installment(id)?.ok_or(ServiceError::NotFound) }
