mod features;
mod reminder_window;
mod settings;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use std::time::Duration;
use chrono::Local;
use finanzas_core::{domain::money, service::{self, EntryChanges, NewEntry}, store::{EntryFilter, EntryRow, Store}, Category, CategoryGroup};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use tauri_plugin_notification::NotificationExt;

pub(crate) struct AppState {
    store: Option<Arc<Mutex<Store>>>,
    revision: Arc<AtomicU64>,
    startup_error: Option<String>,
    settings: Arc<Mutex<settings::Settings>>,
    settings_path: Option<PathBuf>,
}
impl AppState {
    fn with_store<T>(&self, f: impl FnOnce(&mut Store) -> Result<T, String>) -> Result<T, String> {
        let mut store = self.store.as_ref().ok_or("La base de datos no está disponible")?.lock().map_err(|e| e.to_string())?;
        f(&mut store)
    }
    fn changed(&self) { self.revision.fetch_add(1, Ordering::Relaxed); }
    fn save_settings(&self, settings: &settings::Settings) -> Result<(), String> {
        settings::save(self.settings_path.as_ref().ok_or("Ajustes no disponibles")?, settings)
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountOverview { id: i64, name: String, off_budget: bool, closed: bool, sort_order: f64, balance: i64 }
#[derive(Serialize)]
struct EntriesResult { rows: Vec<EntryRow>, total: usize, sum: i64 }
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Filter { account_id: Option<i64>, search: Option<String>, limit: Option<usize> }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CategoryGroupView { id: i64, name: String, is_income: bool, categories: Vec<Category> }

#[tauri::command(async)]
fn startup_error(state: State<'_, AppState>) -> Option<String> { state.startup_error.clone() }
#[tauri::command(async)]
fn accounts_overview(state: State<'_, AppState>) -> Result<Vec<AccountOverview>, String> {
    state.with_store(|store| {
        let accounts = store.list_accounts().map_err(|e| e.to_string())?;
        let balances = store.account_balances().map_err(|e| e.to_string())?;
        Ok(accounts.into_iter().map(|a| AccountOverview { id: a.id, name: a.name, off_budget: a.off_budget, closed: a.closed, sort_order: a.sort_order, balance: balances.get(&a.id).copied().unwrap_or(0) }).collect())
    })
}
/// Changes when this app or anyone else (the MCP server) writes; the UI polls it to refresh.
#[tauri::command(async)]
fn data_version(state: State<'_, AppState>) -> Result<String, String> {
    state.with_store(|s| Ok(format!("{}:{}", s.data_version().map_err(|e| e.to_string())?, state.revision.load(Ordering::Relaxed))))
}
#[tauri::command(async)]
fn overview(state: State<'_, AppState>, month: i32, chart_months: u32) -> Result<service::Overview, String> {
    state.with_store(|s| service::overview(s, month, chart_months).map_err(|e| e.to_string()))
}
#[tauri::command(async)]
fn entries(state: State<'_, AppState>, filter: Filter) -> Result<EntriesResult, String> {
    state.with_store(|s| {
        let mut f = EntryFilter { account_id: filter.account_id, search: filter.search, limit: filter.limit, ..Default::default() };
        let (rows, total) = s.entries(&f).map_err(|e| e.to_string())?;
        f.limit = None;
        // ponytail: sums in Rust; move to SQL SUM if ledgers grow past ~50k rows
        let sum = s.entries(&f).map_err(|e| e.to_string())?.0.iter().map(|r| r.amount).sum();
        Ok(EntriesResult { rows, total, sum })
    })
}
#[tauri::command(async)]
fn add_entry(state: State<'_, AppState>, entry: NewEntry) -> Result<EntryRow, String> {
    let row = state.with_store(|s| service::add_entry(s, entry).map_err(|e| e.to_string()))?; state.changed(); Ok(row)
}
#[tauri::command(async)]
fn update_entry(state: State<'_, AppState>, id: i64, entry: NewEntry) -> Result<EntryRow, String> {
    let changes = EntryChanges { date: Some(entry.date), amount: Some(entry.amount), account_id: Some(entry.account_id), payee: Some(entry.payee.unwrap_or_default()), category_id: Some(entry.category_id), notes: Some(entry.notes), cleared: None };
    let row = state.with_store(|s| service::update_entry(s, id, changes).map_err(|e| e.to_string()))?; state.changed(); Ok(row)
}
#[tauri::command(async)]
fn delete_entry(state: State<'_, AppState>, id: i64) -> Result<EntryRow, String> {
    let row = state.with_store(|s| service::delete_entry(s, id).map_err(|e| e.to_string()))?; state.changed(); Ok(row)
}
#[tauri::command(async)]
fn payee_suggestions(state: State<'_, AppState>, prefix: String) -> Result<Vec<String>, String> {
    state.with_store(|s| Ok(s.payee_suggestions(&prefix, 8).map_err(|e| e.to_string())?.into_iter().map(|p| p.name).collect()))
}
#[tauri::command(async)]
fn suggested_category(state: State<'_, AppState>, payee_name: String) -> Result<Option<i64>, String> {
    state.with_store(|s| match s.payee_named(&payee_name).map_err(|e| e.to_string())? { Some(p) => s.suggested_category(p.id).map_err(|e| e.to_string()), None => Ok(None) })
}
#[tauri::command(async)]
fn categories_grouped(state: State<'_, AppState>) -> Result<Vec<CategoryGroupView>, String> {
    state.with_store(|s| {
        let categories = s.list_categories().map_err(|e| e.to_string())?;
        let groups: Vec<CategoryGroup> = s.list_category_groups().map_err(|e| e.to_string())?;
        Ok(groups.into_iter().map(|g| CategoryGroupView { id: g.id, name: g.name, is_income: g.is_income, categories: categories.iter().filter(|c| c.group_id == Some(g.id)).cloned().collect() }).collect())
    })
}
#[tauri::command(async)]
fn wishes(state: State<'_, AppState>) -> Result<Vec<service::WishView>, String> {
    state.with_store(|s| service::list_wishes(s, Local::now().date_naive(), false).map_err(|e| e.to_string()))
}
#[tauri::command(async)]
fn today() -> String { Local::now().date_naive().to_string() }
#[tauri::command(async)]
fn parse_amount(text: String) -> Option<i64> { money::cents_from(&text) }

/// Opens the store and its settings; failures become a startup error the UI shows instead of crashing.
fn open_state() -> AppState {
    let revision = Arc::new(AtomicU64::new(0));
    let path = Store::default_path();
    let settings_path = path.as_ref().and_then(|p| p.parent()).map(|dir| dir.join("settings.json"));
    let (loaded, settings_error) = match settings_path.as_deref().map(settings::load) {
        Some(Ok(value)) => (value, None),
        Some(Err(e)) => (settings::Settings::default(), Some(format!("Ajustes inválidos: {e}"))),
        None => (settings::Settings::default(), None),
    };
    if let Some(e) = &settings_error { eprintln!("{e}"); }
    let settings = Arc::new(Mutex::new(loaded));
    let opened = path.ok_or_else(|| "No se pudo determinar la ubicación de la base de datos".to_string())
        .and_then(|path| Store::open(&path).map_err(|e| format!("{}: {e}", path.display())));
    match opened {
        Ok(store) => AppState { store: Some(Arc::new(Mutex::new(store))), revision, startup_error: None, settings, settings_path },
        Err(error) => AppState { store: None, revision, startup_error: Some(error), settings, settings_path },
    }
}

/// Posts automatic payments now and every hour.
fn spawn_auto_post(store: Arc<Mutex<Store>>, revision: Arc<AtomicU64>) {
    std::thread::spawn(move || loop {
        match store.lock() {
            Ok(mut store) => match service::auto_post_due(&mut store, Local::now().date_naive()) {
                Ok(count) if count > 0 => { revision.fetch_add(1, Ordering::Relaxed); },
                Ok(_) => {},
                Err(e) => eprintln!("No se pudieron registrar pagos automáticos: {e}"),
            },
            Err(e) => eprintln!("No se pudieron registrar pagos automáticos: {e}"),
        }
        std::thread::sleep(Duration::from_secs(3600));
    });
}

/// Checks reminders every minute and shows each one once as a system notification.
// ponytail: fires only while the process runs; the tray (T4) keeps it running with the window closed.
fn spawn_reminders(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut previous = Local::now().naive_local() - chrono::Duration::minutes(1);
        loop {
            let now = Local::now().naive_local();
            let state = app.state::<AppState>();
            if let Ok(mut settings) = state.settings.lock() && settings.enabled {
                let (days, hour) = (settings.days, settings.hour);
                match state.with_store(|s| service::reminders(s, previous, days, hour).map_err(|e| e.to_string())) {
                    Ok(reminders) => for r in reminders {
                        if reminder_window::due(&r.id, r.fire_at, previous, now, &settings.sent) {
                            match app.notification().builder().title(&r.title).body(&r.body).show() {
                                Ok(()) => { settings.sent.insert(r.id); if let Err(e) = state.save_settings(&settings) { eprintln!("No se pudo guardar el aviso: {e}"); } },
                                Err(e) => eprintln!("No se pudo enviar el aviso: {e}"),
                            }
                        }
                    },
                    Err(e) => eprintln!("No se pudieron calcular los avisos: {e}"),
                }
            }
            previous = now;
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = open_state();
    if let Some(store) = &state.store { spawn_auto_post(Arc::clone(store), Arc::clone(&state.revision)); }
    tauri::Builder::default().plugin(tauri_plugin_dialog::init()).plugin(tauri_plugin_notification::init()).manage(state)
        .setup(|app| { spawn_reminders(app.handle().clone()); Ok(()) })
        .invoke_handler(tauri::generate_handler![startup_error, accounts_overview, data_version, overview, entries, add_entry, update_entry, delete_entry, payee_suggestions, suggested_category, categories_grouped, wishes, today, parse_amount,
            features::budget, features::assign_budget, features::create_account, features::change_category, features::change_group, features::create_category, features::upcoming, features::schedules, features::create_schedule, features::schedule_action, features::get_settings, features::save_settings, features::import_availability, features::import_data])
        .run(tauri::generate_context!()).expect("error al iniciar la aplicación");
}
