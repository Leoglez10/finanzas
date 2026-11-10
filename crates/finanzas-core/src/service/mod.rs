//! Shared application services. Amounts are integer cents.

use crate::{Cents, Id, MonthKey, domain::{envelope, money, summary}, store::Store};
use serde::Serialize;
mod entries;
mod schedules;
mod overview;
pub use overview::*;
pub use schedules::*;
pub use entries::{add_entry, update_entry, delete_entry, EntryChanges, NewEntry};

#[derive(Debug)]
pub enum ServiceError {
    Store(rusqlite::Error),
    NotFound,
    IncomeCategory,
    CompletedSchedule,
}
impl From<rusqlite::Error> for ServiceError {
    fn from(error: rusqlite::Error) -> Self { Self::Store(error) }
}
impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::Store(e) => e.fmt(f), Self::NotFound => write!(f, "not found"), Self::IncomeCategory => write!(f, "Income categories cannot be budgeted"), Self::CompletedSchedule => write!(f, "Schedule has no remaining occurrence") }
    }
}
impl std::error::Error for ServiceError {}
pub type Result<T> = std::result::Result<T, ServiceError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetCategory {
    pub id: Id, pub name: String, pub hidden: bool,
    pub budgeted: Cents, pub activity: Cents, pub available: Cents, pub spent_share: f64,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetGroup { pub id: Id, pub name: String, pub is_income: bool, pub categories: Vec<BudgetCategory> }
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetView {
    pub month: MonthKey, pub to_budget: Cents, pub income: Cents, pub assigned: Cents,
    pub hint: String, pub uncategorized_income: Cents, pub groups: Vec<BudgetGroup>,
}

pub fn budget_month(store: &Store, month: MonthKey) -> Result<BudgetView> {
    let budget = envelope::month(&store.envelope_inputs()?, month);
    let mut groups = store.list_category_groups()?;
    groups.sort_by_key(|g| g.is_income);
    let categories = store.list_categories()?;
    let groups = groups.into_iter().map(|group| BudgetGroup {
        id: group.id, name: group.name, is_income: group.is_income,
        categories: categories.iter().filter(|c| c.group_id == Some(group.id)).filter_map(|c| {
            let row = budget.rows.iter().find(|row| row.category_id == c.id)?;
            Some(BudgetCategory { id: c.id, name: c.name.clone(), hidden: c.hidden, budgeted: row.budgeted, activity: row.activity, available: row.available, spent_share: row.spent_share() })
        }).collect(),
    }).collect();
    Ok(BudgetView { month, to_budget: budget.to_budget, income: budget.income, assigned: budget.budgeted, hint: summary::to_budget_hint(budget.to_budget), uncategorized_income: store.uncategorized_income(month)?, groups })
}

pub fn set_budget_from_text(store: &Store, month: MonthKey, category_id: Id, text: &str) -> Result<Option<Cents>> {
    let category = store.get_category(category_id)?.ok_or(ServiceError::NotFound)?;
    if let Some(group) = category.group_id.and_then(|id| store.get_category_group(id).transpose()).transpose()? {
        if group.is_income { return Err(ServiceError::IncomeCategory); }
    }
    let current = store.budget_rows()?.0.get(&(month, category_id)).copied().unwrap_or(0);
    let result = money::budgeted_from(text, current);
    if let Some(amount) = result { store.set_budget(month, category_id, amount)?; }
    Ok(result)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpendingCategory { pub category_id: Option<Id>, pub category: String, pub spent: Cents }
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpendingView { pub total: Cents, pub categories: Vec<SpendingCategory> }
pub fn spending_by_category(store: &Store, start: chrono::NaiveDate, end: chrono::NaiveDate, include_off_budget: bool) -> Result<SpendingView> {
    let categories: Vec<_> = store.spending_by_category(start, end, include_off_budget)?.into_iter().map(|(category_id, category, spent)| SpendingCategory { category_id, category, spent }).collect();
    Ok(SpendingView { total: categories.iter().map(|c| c.spent).sum(), categories })
}

#[cfg(test)]
mod tests;
