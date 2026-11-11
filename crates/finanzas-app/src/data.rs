use std::sync::{Arc, Mutex};
use chrono::NaiveDate;
use finanzas_core::{domain::month, models::{Account, Category, CategoryGroup, Entry, Frequency, Recurrence, Schedule, WeekendMode}, service::{overview, Overview}, store::Store};

pub struct AccountBalance { pub account: Account, pub balance: i64 }
pub struct Snapshot { pub accounts: Vec<AccountBalance>, pub overview: Overview, pub version: i64, pub groups: Vec<CategoryGroup>, pub categories: Vec<Category> }
pub type WorkerStore = Arc<Mutex<Option<Store>>>;

pub enum Mutation { Save(Option<i64>, finanzas_core::service::NewEntry), Delete(i64) }
pub fn mutate(worker: WorkerStore, mutation: Mutation) -> Result<(), String> {
    let guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_ref().ok_or("No se pudo abrir la base de datos")?;
    match mutation {
        Mutation::Save(None, entry) => { finanzas_core::service::add_entry(store, entry).map_err(|e| e.to_string())?; }
        Mutation::Save(Some(id), entry) => {
            let changes = finanzas_core::service::EntryChanges { date: Some(entry.date), amount: Some(entry.amount), account_id: Some(entry.account_id), payee: Some(entry.payee.unwrap_or_default()), category_id: Some(entry.category_id), notes: Some(entry.notes), cleared: Some(entry.cleared) };
            finanzas_core::service::update_entry(store, id, changes).map_err(|e| e.to_string())?;
        }
        Mutation::Delete(id) => { finanzas_core::service::delete_entry(store, id).map_err(|e| e.to_string())?; }
    }
    Ok(())
}
pub fn suggestions(worker: WorkerStore, text: String) -> Result<Vec<(String, Option<i64>)>, String> {
    let guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_ref().ok_or("No se pudo abrir la base de datos")?;
    store.payee_suggestions(&text, 8).map_err(|e| e.to_string())?.into_iter().map(|p| Ok((p.name, store.suggested_category(p.id).map_err(|e| e.to_string())?))).collect()
}
pub struct Ledger { pub rows: Vec<finanzas_core::store::EntryRow>, pub total: i64 }
pub fn ledger(worker: WorkerStore, account: Option<i64>, search: &str) -> Result<Ledger, String> {
    let guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_ref().ok_or("No se pudo abrir la base de datos")?;
    let filter = finanzas_core::store::EntryFilter { account_id: account, search: Some(search.to_owned()), ..Default::default() };
    let (rows, _) = store.entries(&filter).map_err(|e| e.to_string())?;
    let total = rows.iter().map(|row| row.amount).sum();
    Ok(Ledger { rows, total })
}

pub fn read(store: &Store, month: i32, chart_months: u32) -> Result<Snapshot, String> {
    let version = store.data_version().map_err(|e| e.to_string())?;
    let accounts = store.list_accounts().map_err(|e| e.to_string())?;
    let balances = store.account_balances().map_err(|e| e.to_string())?;
    let accounts = accounts.into_iter().map(|account| {
        let balance = balances.get(&account.id).copied().unwrap_or(0);
        AccountBalance { account, balance }
    }).collect();
    let overview = overview(store, month, chart_months).map_err(|e| e.to_string())?;
    let groups = store.list_category_groups().map_err(|e| e.to_string())?;
    let categories = store.list_categories().map_err(|e| e.to_string())?;
    Ok(Snapshot { accounts, overview, version, groups, categories })
}

pub fn open_default(worker: &WorkerStore) -> Result<(), String> {
    let mut guard = worker.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        let path = Store::default_path().ok_or("No se pudo determinar la ruta de la base de datos")?;
        *guard = Some(Store::open(path).map_err(|e| e.to_string())?);
    }
    Ok(())
}

pub fn fetch(worker: WorkerStore, month: i32, chart_months: u32, version: Option<i64>, force: bool) -> Result<Option<Snapshot>, String> {
    let guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_ref().ok_or("No se pudo abrir la base de datos")?;
    if !force && version == Some(store.data_version().map_err(|e| e.to_string())?) { return Ok(None); }
    read(store, month, chart_months).map(Some)
}

pub fn scene(seed: bool) -> Result<(WorkerStore, Snapshot), String> {
    let store = Store::open_in_memory().map_err(|e| e.to_string())?;
    if seed { seed_demo(&store)?; }
    let snapshot = read(&store, 202611, 6)?;
    Ok((Arc::new(Mutex::new(Some(store))), snapshot))
}

fn seed_demo(store: &Store) -> Result<(), String> {
    let db = |e: rusqlite::Error| e.to_string();
    let account = store.insert_account(&Account { id: 0, name: "Cuenta principal".into(), off_budget: false, closed: false, sort_order: 0.0 }).map_err(db)?;
    let debt = store.insert_account(&Account { id: 0, name: "Tarjeta".into(), off_budget: false, closed: false, sort_order: 1.0 }).map_err(db)?;
    let income_group = store.insert_category_group(&CategoryGroup { id: 0, name: "Ingresos".into(), is_income: true, sort_order: 0.0 }).map_err(db)?;
    let expense_group = store.insert_category_group(&CategoryGroup { id: 0, name: "Gastos".into(), is_income: false, sort_order: 1.0 }).map_err(db)?;
    let income = store.insert_category(&Category { id: 0, name: "Sueldo".into(), hidden: false, sort_order: 0.0, group_id: Some(income_group) }).map_err(db)?;
    let mut categories = Vec::new();
    for (i, name) in ["Vivienda", "Alimentación", "Transporte", "Ocio", "Servicios", "Salud"].iter().enumerate() {
        let id = store.insert_category(&Category { id: 0, name: (*name).into(), hidden: false, sort_order: i as f64, group_id: Some(expense_group) }).map_err(db)?;
        store.set_budget(202611, id, (80_000 - i as i64 * 8_000).max(20_000)).map_err(db)?;
        categories.push(id);
    }
    let income_payee = store.find_or_create_payee("Empresa SA").map_err(db)?.ok_or("Comercio inválido")?;
    let mut payees = Vec::new();
    for name in ["Renta", "Supermercado La Esquina", "Uber", "Netflix", "CFE", "Farmacia"] {
        payees.push(store.find_or_create_payee(name).map_err(db)?.ok_or("Comercio inválido")?);
    }
    let mut m = 202611;
    for index in 0..7 {
        let date = NaiveDate::from_ymd_opt(m / 100, (m % 100) as u32, 5).ok_or("Fecha inválida")?;
        store.insert_entry(&Entry { id: 0, date, amount: 320_000 + index * 5_000, notes: "Ingreso mensual".into(), cleared: true, account_id: Some(account), category_id: Some(income), payee_id: Some(income_payee), schedule_id: None }).map_err(db)?;
        for (i, category) in categories.iter().enumerate() {
            store.insert_entry(&Entry { id: 0, date, amount: -(50_000 - i as i64 * 5_000 + index * 500), notes: if i == 1 { "Despensa".into() } else { String::new() }, cleared: true, account_id: Some(if i == 4 { debt } else { account }), category_id: Some(*category), payee_id: Some(payees[i]), schedule_id: None }).map_err(db)?;
        }
        m = month::previous(m);
    }
    let date = NaiveDate::from_ymd_opt(2026, 11, 20).ok_or("Fecha inválida")?;
    store.insert_schedule(&Schedule { id: 0, name: "Pago mensual".into(), amount: -10_000, auto_post: false, completed: false, next_date: Some(date), recurrence: Some(Recurrence { start: date, frequency: Frequency::Monthly, interval: 1, skip_weekend: false, weekend_mode: WeekendMode::After, end_date: None }), account_id: Some(account), category_id: Some(categories[0]), payee_id: None }).map_err(db)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_filters_and_sums_the_matching_rows() {
        let (worker, _) = scene(true).unwrap();
        let all = ledger(Arc::clone(&worker), None, "").unwrap();
        assert!(all.rows.len() >= 20);
        assert!(all.rows.iter().all(|row| row.payee_name.as_ref().is_some_and(|name| !name.is_empty())));
        let income = ledger(Arc::clone(&worker), None, "Ingreso mensual").unwrap();
        assert_eq!(income.rows.len(), 7);
        assert_eq!(income.total, income.rows.iter().map(|r| r.amount).sum::<i64>());
        let missing = ledger(worker, None, "no-such-transaction").unwrap();
        assert_eq!(missing.total, 0);
        assert!(missing.rows.is_empty());
    }

    #[test]
    fn external_write_refreshes_snapshot_but_unchanged_poll_does_not() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target").join(format!("worker-refresh-{}.db", std::process::id()));
        let first = Store::open(&path).unwrap();
        let second = Store::open(&path).unwrap();
        let worker = Arc::new(Mutex::new(Some(first)));
        let before = fetch(Arc::clone(&worker), 202611, 6, None, true).unwrap().unwrap();
        assert!(fetch(Arc::clone(&worker), 202611, 6, Some(before.version), false).unwrap().is_none());
        second.insert_account(&Account { id: 0, name: "Nueva".into(), off_budget: false, closed: false, sort_order: 0.0 }).unwrap();
        let after = fetch(worker, 202611, 6, Some(before.version), false).unwrap().unwrap();
        assert!(after.accounts.iter().any(|account| account.account.name == "Nueva"));
        assert_ne!(after.version, before.version);
    }
}

