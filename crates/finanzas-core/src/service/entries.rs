use super::{Result, ServiceError};
use crate::{Cents, Entry, Id, store::{EntryRow, Store}};
use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewEntry {
    pub date: NaiveDate, pub amount: Cents, pub account_id: Id, pub payee: Option<String>,
    pub category_id: Option<Id>, #[serde(default)] pub notes: String, #[serde(default = "default_cleared")] pub cleared: bool,
}
fn default_cleared() -> bool { true }
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryChanges {
    pub date: Option<NaiveDate>, pub amount: Option<Cents>, pub account_id: Option<Id>,
    pub payee: Option<String>, pub category_id: Option<Option<Id>>, pub notes: Option<String>, pub cleared: Option<bool>,
}
fn row(store: &Store, id: Id) -> Result<EntryRow> { store.entry_row(id)?.ok_or(ServiceError::NotFound) }
fn validate_account(store: &Store, id: Id) -> Result<()> { store.get_account(id)?.ok_or(ServiceError::NotFound)?; Ok(()) }

pub fn add_entry(store: &Store, new: NewEntry) -> Result<EntryRow> {
    validate_account(store, new.account_id)?;
    let payee_id = new.payee.as_deref().map(|s| store.find_or_create_payee(s)).transpose()?.flatten();
    let category_id = match (new.category_id, payee_id) { (None, Some(id)) => store.suggested_category(id)?, (category, _) => category };
    let id = store.insert_entry(&Entry { id: 0, date: new.date, amount: new.amount, notes: new.notes, cleared: new.cleared, account_id: Some(new.account_id), category_id, payee_id, schedule_id: None })?;
    row(store, id)
}
pub fn update_entry(store: &Store, id: Id, changes: EntryChanges) -> Result<EntryRow> {
    let mut entry = store.get_entry(id)?.ok_or(ServiceError::NotFound)?;
    if let Some(account) = changes.account_id { validate_account(store, account)?; entry.account_id = Some(account); }
    if let Some(value) = changes.date { entry.date = value; }
    if let Some(value) = changes.amount { entry.amount = value; }
    if let Some(value) = changes.payee { entry.payee_id = store.find_or_create_payee(&value)?; }
    if let Some(value) = changes.category_id { entry.category_id = value; }
    if let Some(value) = changes.notes { entry.notes = value; }
    if let Some(value) = changes.cleared { entry.cleared = value; }
    store.update_entry(&entry)?;
    row(store, id)
}
pub fn delete_entry(store: &Store, id: Id) -> Result<EntryRow> {
    let deleted = row(store, id)?;
    store.delete_entry(id)?;
    Ok(deleted)
}
