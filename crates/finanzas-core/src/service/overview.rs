use super::Result;
use crate::{Cents, Id, MonthKey, domain::{envelope, month, summary}, store::{EntryRow, Store}};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthSummaryView { pub income: Cents, pub expenses: Cents, pub budgeted: Cents, pub savings_rate: Option<f64> }
impl From<&summary::MonthSummary> for MonthSummaryView {
    fn from(s: &summary::MonthSummary) -> Self { Self { income: s.income, expenses: s.expenses, budgeted: s.budgeted, savings_rate: s.savings_rate } }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartPoint { pub month: MonthKey, pub income: Cents, pub expenses: Cents }
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryProgress { pub category_id: Id, pub name: String, pub spent: Cents, pub budgeted: Cents }
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstRun { pub has_account: bool, pub has_categorized_income: bool, pub has_budget: bool }
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub total_balance: Cents, pub open_account_count: usize,
    pub current: MonthSummaryView, pub previous: MonthSummaryView,
    pub income_change: Option<f64>, pub expenses_change: Option<f64>, pub savings_rate_change: Option<f64>,
    pub chart: Vec<ChartPoint>, pub spending_total: Cents, pub spending: Vec<CategoryProgress>,
    pub recent_entries: Vec<EntryRow>, pub budget_spent: Cents, pub budget_assigned: Cents, pub budget_progress: Vec<CategoryProgress>, pub first_run: FirstRun,
}
pub fn overview(store: &Store, target: MonthKey, chart_months: u32) -> Result<Overview> { overview_span(store, target, 1, chart_months) }

/// Adds up `span` months ending at `target` (a quarter, a year to date, a hand-picked range of months)
/// and compares them with the same number of months right before. The chart covers at least the span.
pub fn overview_span(store: &Store, target: MonthKey, span: u32, chart_months: u32) -> Result<Overview> {
    let span = span.max(1);
    let back = |n: u32| (1..n).fold(target, |m, _| month::previous(m));
    let span_start = back(span);
    let chart_start = back(chart_months.max(span));
    let start = chart_start.min(back(span * 2));
    let months = envelope::months(&store.envelope_inputs()?, start, target);
    let current_budget = months.last().expect("target month included");
    let previous_start = back(span * 2);
    let current = summed(months.iter().filter(|m| m.month >= span_start));
    let previous = summed(months.iter().filter(|m| m.month >= previous_start && m.month < span_start));
    let chart = if chart_months == 0 && span == 1 { Vec::new() } else { months.iter().filter(|m| m.month >= chart_start).map(|m| { let s = summary::MonthSummary::from(m); ChartPoint { month: m.month, income: s.income, expenses: s.expenses } }).collect() };
    let names: HashMap<_, _> = store.list_categories()?.into_iter().map(|c| (c.id, c.name)).collect();
    let mut spending: Vec<_> = current.spending.iter().map(|(id, spent)| CategoryProgress { category_id: *id, name: names.get(id).cloned().unwrap_or_default(), spent: *spent, budgeted: current_budget.rows.iter().find(|r| r.category_id == *id).map_or(0, |r| r.budgeted) }).collect();
    spending.sort_by(|a,b| b.spent.cmp(&a.spent).then_with(|| a.name.cmp(&b.name)));
    let mut budget_progress: Vec<_> = current_budget.rows.iter().filter(|r| r.budgeted > 0).map(|r| CategoryProgress { category_id: r.category_id, name: names.get(&r.category_id).cloned().unwrap_or_else(|| "—".into()), spent: -r.activity, budgeted: r.budgeted }).collect();
    budget_progress.sort_by_key(|row| std::cmp::Reverse(row.budgeted));
    budget_progress.truncate(5);
    spending.truncate(6);
    let balances = store.account_balances()?;
    let accounts = store.list_accounts()?;
    let open: Vec<_> = accounts.iter().filter(|a| !a.closed).collect();
    let (has_account, has_categorized_income, has_budget) = store.first_run_checks()?;
    Ok(Overview { total_balance: open.iter().map(|a| balances.get(&a.id).copied().unwrap_or(0)).sum(), open_account_count: open.len(), current: (&current).into(), previous: (&previous).into(), income_change: summary::relative_change(current.income, previous.income), expenses_change: summary::relative_change(current.expenses, previous.expenses), savings_rate_change: match (current.savings_rate, previous.savings_rate) { (Some(a), Some(b)) if b != 0.0 => Some((a-b)/b.abs()), _ => None }, chart, spending_total: current.expenses, spending, recent_entries: store.recent_entries(8)?, budget_spent: current.expenses, budget_assigned: current.budgeted, budget_progress, first_run: FirstRun { has_account, has_categorized_income, has_budget } })
}

fn summed<'a>(months: impl Iterator<Item = &'a envelope::BudgetMonth>) -> summary::MonthSummary {
    let mut total = summary::MonthSummary { income: 0, expenses: 0, budgeted: 0, savings_rate: None, spending: Vec::new() };
    let mut by_category: HashMap<Id, Cents> = HashMap::new();
    for m in months {
        let s = summary::MonthSummary::from(m);
        total.income += s.income; total.expenses += s.expenses; total.budgeted += s.budgeted;
        for (id, spent) in s.spending { *by_category.entry(id).or_default() += spent; }
    }
    total.savings_rate = (total.income > 0).then(|| (total.income as f64 - total.expenses as f64) / total.income as f64);
    total.spending = by_category.into_iter().collect();
    total
}
