use std::sync::Mutex;

use finanzas_core::store::Store;
use serde::Serialize;
use tauri::State;

struct AppState {
    store: Option<Mutex<Store>>,
    startup_error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountOverview {
    id: i64,
    name: String,
    off_budget: bool,
    closed: bool,
    sort_order: f64,
    balance: i64,
}

#[tauri::command]
fn startup_error(state: State<'_, AppState>) -> Option<String> {
    state.startup_error.clone()
}

#[tauri::command]
fn accounts_overview(state: State<'_, AppState>) -> Result<Vec<AccountOverview>, String> {
    let store = state.store.as_ref().ok_or("La base de datos no está disponible")?
        .lock().map_err(|e| e.to_string())?;
    let accounts = store.list_accounts().map_err(|e| e.to_string())?;
    let balances = store.account_balances().map_err(|e| e.to_string())?;
    Ok(accounts.into_iter().map(|account| AccountOverview {
        id: account.id,
        name: account.name,
        off_budget: account.off_budget,
        closed: account.closed,
        sort_order: account.sort_order,
        balance: balances.get(&account.id).copied().unwrap_or(0),
    }).collect())
}

#[tauri::command]
fn data_version(state: State<'_, AppState>) -> Result<i64, String> {
    let store = state.store.as_ref().ok_or("La base de datos no está disponible")?
        .lock().map_err(|e| e.to_string())?;
    store.data_version().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = Store::default_path()
        .ok_or_else(|| "No se pudo determinar la ubicación de la base de datos".to_string())
        .and_then(|path| Store::open(&path)
            .map_err(|e| format!("{}: {e}", path.display())));
    let state = match result {
        Ok(store) => AppState { store: Some(Mutex::new(store)), startup_error: None },
        Err(error) => AppState { store: None, startup_error: Some(error) },
    };
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![startup_error, accounts_overview, data_version])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación");
}
