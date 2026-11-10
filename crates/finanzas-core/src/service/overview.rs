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
pub fn overview(store: &Store, target: MonthKey, chart_months: u32) -> Result<Overview> {
    let mut chart_start = target;
    for _ in 1..chart_months { chart_start = month::previous(chart_start); }
    let start = chart_start.min(month::previous(target));
    let months = envelope::months(&store.envelope_inputs()?, start, target);
    let current_budget = months.last().expect("target month included");
    let current = summary::MonthSummary::from(current_budget);
    let previous = summary::MonthSummary::from(&months[months.len()-2]);
    let chart = if chart_months == 0 { Vec::new() } else { months.iter().filter(|m| m.month >= chart_start).map(|m| { let s = summary::MonthSummary::from(m); ChartPoint { month: m.month, income: s.income, expenses: s.expenses } }).collect() };
    let names: HashMap<_, _> = store.list_categories()?.into_iter().map(|c| (c.id, c.name)).collect();
    let mut spending: Vec<_> = current.spending.iter().map(|(id, spent)| CategoryProgress { category_id: *id, name: names.get(id).cloned().unwrap_or_default(), spent: *spent, budgeted: current_budget.rows.iter().find(|r| r.category_id == *id).map_or(0, |r| r.budgeted) }).collect();
    spending.sort_by(|a,b| b.spent.cmp(&a.spent).then_with(|| a.name.cmp(&b.name)));
    let budget_progress = spending.iter().take(5).map(|p| CategoryProgress { category_id: p.category_id, name: p.name.clone(), spent: p.spent, budgeted: p.budgeted }).collect();
    spending.truncate(6);
    let balances = store.account_balances()?;
    let accounts = store.list_accounts()?;
    let open: Vec<_> = accounts.iter().filter(|a| !a.closed).collect();
    let (has_account, has_categorized_income, has_budget) = store.first_run_checks()?;
    Ok(Overview { total_balance: open.iter().map(|a| balances.get(&a.id).copied().unwrap_or(0)).sum(), open_account_count: open.len(), current: (&current).into(), previous: (&previous).into(), income_change: summary::relative_change(current.income, previous.income), expenses_change: summary::relative_change(current.expenses, previous.expenses), savings_rate_change: match (current.savings_rate, previous.savings_rate) { (Some(a), Some(b)) if b != 0.0 => Some((a-b)/b.abs()), _ => None }, chart, spending_total: current.expenses, spending, recent_entries: store.recent_entries(8)?, budget_spent: current.expenses, budget_assigned: current.budgeted, budget_progress, first_run: FirstRun { has_account, has_categorized_income, has_budget } })
}
