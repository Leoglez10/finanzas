//! Plain data types. Money is always integer cents; dates are calendar days;
//! months are `yyyymm` integers (e.g. 202611).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

pub type Cents = i64;
pub type Id = i64;
/// `yyyy * 100 + mm`.
pub type MonthKey = i32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekendMode {
    Before,
    #[default]
    After,
}

/// Same field names and raw values as the Swift `Recurrence` JSON, so stored
/// rules round-trip between both apps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recurrence {
    pub start: NaiveDate,
    pub frequency: Frequency,
    /// Values below 1 behave as 1.
    pub interval: u32,
    #[serde(default)]
    pub skip_weekend: bool,
    #[serde(default)]
    pub weekend_mode: WeekendMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: Id,
    pub name: String,
    pub off_budget: bool,
    pub closed: bool,
    pub sort_order: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryGroup {
    pub id: Id,
    pub name: String,
    pub is_income: bool,
    pub sort_order: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: Id,
    pub name: String,
    pub hidden: bool,
    pub sort_order: f64,
    pub group_id: Option<Id>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payee {
    pub id: Id,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: Id,
    pub date: NaiveDate,
    /// Signed: negative = outflow.
    pub amount: Cents,
    pub notes: String,
    pub cleared: bool,
    pub account_id: Option<Id>,
    pub category_id: Option<Id>,
    pub payee_id: Option<Id>,
    pub schedule_id: Option<Id>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    pub id: Id,
    pub name: String,
    pub amount: Cents,
    pub auto_post: bool,
    pub completed: bool,
    /// `None` means "first occurrence of the recurrence".
    pub next_date: Option<NaiveDate>,
    pub recurrence: Option<Recurrence>,
    pub account_id: Option<Id>,
    pub category_id: Option<Id>,
    pub payee_id: Option<Id>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetAmount {
    pub id: Id,
    pub month: MonthKey,
    pub amount: Cents,
    pub carryover: bool,
    pub category_id: Option<Id>,
}
