use std::sync::{Arc, Mutex};
use chrono::NaiveDate;
use finanzas_core::{domain::month, models::{Account, Category, CategoryGroup, Entry, Frequency, Recurrence, Schedule, WeekendMode}, service::{overview, Overview}, store::Store};

pub struct AccountBalance { pub account: Account, pub balance: i64 }
pub struct Snapshot { pub accounts: Vec<AccountBalance>, pub overview: Overview, pub version: i64 }
pub type WorkerStore = Arc<Mutex<Option<Store>>>;

pub fn read(store: &Store, month: i32, chart_months: u32) -> Result<Snapshot, String> {
    let version = store.data_version().map_err(|e| e.to_string())?;
    let accounts = store.list_accounts().map_err(|e| e.to_string())?;
    let balances = store.account_balances().map_err(|e| e.to_string())?;
    let accounts = accounts.into_iter().map(|account| {
        let balance = balances.get(&account.id).copied().unwrap_or(0);
        AccountBalance { account, balance }
    }).collect();
    let overview = overview(store, month, chart_months).map_err(|e| e.to_string())?;
    Ok(Snapshot { accounts, overview, version })
}

pub fn fetch(worker: WorkerStore, month: i32, chart_months: u32, version: Option<i64>, force: bool) -> Result<Option<Snapshot>, String> {
    let mut guard = worker.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        let path = Store::default_path().ok_or("No se pudo determinar la ruta de la base de datos")?;
        *guard = Some(Store::open(path).map_err(|e| e.to_string())?);
    }
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
    let mut m = 202611;
    for index in 0..7 {
        let date = NaiveDate::from_ymd_opt(m / 100, (m % 100) as u32, 5).ok_or("Fecha inválida")?;
        store.insert_entry(&Entry { id: 0, date, amount: 320_000 + index * 5_000, notes: "Ingreso mensual".into(), cleared: true, account_id: Some(account), category_id: Some(income), payee_id: None, schedule_id: None }).map_err(db)?;
        for (i, category) in categories.iter().enumerate() {
            store.insert_entry(&Entry { id: 0, date, amount: -(50_000 - i as i64 * 5_000 + index * 500), notes: "Movimiento".into(), cleared: true, account_id: Some(if i == 4 { debt } else { account }), category_id: Some(*category), payee_id: None, schedule_id: None }).map_err(db)?;
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

