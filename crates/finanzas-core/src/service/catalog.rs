//! Accounts, categories and groups: the writes the app and the MCP server share.

use super::{set_card_terms, CardTerms, Result, ServiceError};
use crate::{store::Store, Account, Category, CategoryGroup, Cents, Entry, Id};
use chrono::NaiveDate;

fn name(text: &str) -> Result<String> {
    let text = text.trim();
    if text.is_empty() { Err(ServiceError::Invalid("Escribe un nombre")) } else { Ok(text.into()) }
}

/// A new account at the end of the list. A non-zero `opening` becomes a "Saldo inicial" movement on `date`;
/// `card` marks it as a credit card with those terms.
pub fn create_account(store: &mut Store, account_name: &str, off_budget: bool, opening: Option<Cents>, date: NaiveDate, card: Option<CardTerms>) -> Result<Account> {
    let account_name = name(account_name)?;
    store.transaction(|s| {
        let order = s.list_accounts()?.iter().map(|a| a.sort_order).fold(0.0, f64::max) + 1.0;
        let mut account = Account { id: 0, name: account_name, off_budget, closed: false, sort_order: order };
        account.id = s.insert_account(&account)?;
        if let Some(amount) = opening.filter(|v| *v != 0) {
            s.insert_entry(&Entry { id: 0, date, amount, notes: "Saldo inicial".into(), cleared: true, account_id: Some(account.id), category_id: None, payee_id: None, schedule_id: None })?;
        }
        if card.is_some() { set_card_terms(s, account.id, card)?; }
        Ok(account)
    })
}

/// A new category at the end of its group.
pub fn create_category(store: &mut Store, group_id: Id, category_name: &str) -> Result<Category> {
    let category_name = name(category_name)?;
    if store.get_category_group(group_id)?.is_none() { return Err(ServiceError::NotFound); }
    let order = store.list_categories()?.iter().filter(|c| c.group_id == Some(group_id)).map(|c| c.sort_order).fold(0.0_f64, f64::max) + 1.0;
    let mut category = Category { id: 0, name: category_name, hidden: false, sort_order: order, group_id: Some(group_id) };
    category.id = store.insert_category(&category)?;
    Ok(category)
}

/// Renames and/or hides a category; `None` keeps that field.
pub fn update_category(store: &mut Store, id: Id, new_name: Option<&str>, hidden: Option<bool>) -> Result<Category> {
    let mut category = store.get_category(id)?.ok_or(ServiceError::NotFound)?;
    if let Some(n) = new_name { category.name = name(n)?; }
    if let Some(h) = hidden { category.hidden = h; }
    store.update_category(&category)?;
    Ok(category)
}

/// Renames a group, or creates an expense group at the end when `id` is None.
pub fn save_category_group(store: &mut Store, id: Option<Id>, group_name: &str) -> Result<CategoryGroup> {
    let group_name = name(group_name)?;
    match id {
        Some(id) => {
            let mut group = store.get_category_group(id)?.ok_or(ServiceError::NotFound)?;
            group.name = group_name;
            store.update_category_group(&group)?;
            Ok(group)
        }
        None => {
            let order = store.list_category_groups()?.iter().filter(|g| !g.is_income).map(|g| g.sort_order).fold(0.0_f64, f64::max) + 1.0;
            let mut group = CategoryGroup { id: 0, name: group_name, is_income: false, sort_order: order };
            group.id = store.insert_category_group(&group)?;
            Ok(group)
        }
    }
}
