mod features;
mod menu;
mod prefs;
mod reminder_window;
mod settings;
mod tray;

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
    /// The default database; other profiles live beside it.
    home: Option<PathBuf>,
    profile: Mutex<String>,
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
fn overview(state: State<'_, AppState>, month: i32, chart_months: u32, span: Option<u32>) -> Result<service::Overview, String> {
    state.with_store(|s| service::overview_span(s, month, span.unwrap_or(1), chart_months).map_err(|e| e.to_string()))
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
fn wishes(state: State<'_, AppState>, archived: Option<bool>) -> Result<Vec<service::WishView>, String> {
    state.with_store(|s| service::list_wishes(s, Local::now().date_naive(), archived.unwrap_or(false)).map_err(|e| e.to_string()))
}
#[tauri::command(async)]
fn save_wish(state: State<'_, AppState>, id: Option<i64>, draft: service::WishDraft) -> Result<finanzas_core::service::Wish, String> {
    let wish = state.with_store(|s| service::save_wish(s, id, draft).map_err(|e| e.to_string()))?; state.changed(); Ok(wish)
}
/// `amount` is the new total set aside, not a delta (0 releases everything).
#[tauri::command(async)]
fn tag_entry(state: State<'_, AppState>, id: i64, wish_id: Option<i64>, item_id: Option<i64>) -> Result<finanzas_core::store::EntryRow, String> {
    let row = state.with_store(|s| service::tag_entry(s, id, wish_id, item_id).map_err(|e| e.to_string()))?; state.changed(); Ok(row)
}
#[tauri::command(async)]
fn complete_wish(state: State<'_, AppState>, id: i64, account_id: i64) -> Result<service::CompletedWish, String> {
    let done = state.with_store(|s| service::complete_wish(s, id, account_id, Local::now().date_naive()).map_err(|e| e.to_string()))?; state.changed(); Ok(done)
}
#[tauri::command(async)]
fn set_wish_item_paid(state: State<'_, AppState>, id: i64, paid: bool) -> Result<finanzas_core::service::Wish, String> {
    let wish = state.with_store(|s| service::set_wish_item_paid(s, id, paid, Local::now().date_naive()).map_err(|e| e.to_string()))?; state.changed(); Ok(wish)
}
#[tauri::command(async)]
fn reserve_wish(state: State<'_, AppState>, id: i64, amount: i64) -> Result<finanzas_core::service::Wish, String> {
    let wish = state.with_store(|s| service::reserve_wish(s, id, amount, Local::now().date_naive()).map_err(|e| e.to_string()))?; state.changed(); Ok(wish)
}
#[tauri::command(async)]
fn archive_wish(state: State<'_, AppState>, id: i64) -> Result<finanzas_core::service::Wish, String> {
    let wish = state.with_store(|s| service::archive_wish(s, id).map_err(|e| e.to_string()))?; state.changed(); Ok(wish)
}
/// Undo for archive/buy: writes the wish back exactly as it was a moment ago.
#[tauri::command(async)]
fn restore_wish(state: State<'_, AppState>, wish: finanzas_core::service::Wish) -> Result<(), String> {
    state.with_store(|s| service::restore_wish(s, &wish).map(|_| ()).map_err(|e| e.to_string()))?; state.changed(); Ok(())
}
/// QA scripts (desktop/qa) report through this; only debug builds print.
#[tauri::command]
fn qa_log(line: String) {
    #[cfg(debug_assertions)]
    eprintln!("QA {line}");
    #[cfg(not(debug_assertions))]
    let _ = line;
}
/// QA scripts wait through Rust: a hidden or covered webview suspends its own timers.
#[tauri::command(async)]
fn qa_sleep(ms: u64) {
    #[cfg(debug_assertions)]
    std::thread::sleep(Duration::from_millis(ms.min(10_000)));
    #[cfg(not(debug_assertions))]
    let _ = ms;
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
    // Reopen the last profile; fall back to Personal if its file is gone.
    let profile = settings.lock().map(|s| s.profile.clone()).unwrap_or_default();
    let profile_path = path.as_deref().and_then(|home| Store::profile_path_beside(home, &profile)).filter(|p| p.exists());
    let (open_path, profile) = match profile_path { Some(p) => (Some(p), profile), None => (path.clone(), "personal".to_owned()) };
    let opened = open_path.ok_or_else(|| "No se pudo determinar la ubicación de la base de datos".to_string())
        .and_then(|path| Store::open(&path).map_err(|e| format!("{}: {e}", path.display())));
    let (store, startup_error) = match opened { Ok(store) => (Some(Arc::new(Mutex::new(store))), None), Err(error) => (None, Some(error)) };
    AppState { store, revision, startup_error, settings, settings_path, home: path, profile: Mutex::new(profile) }
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
            tray::refresh(&app);
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = open_state();
    if let Some(store) = &state.store { spawn_auto_post(Arc::clone(store), Arc::clone(&state.revision)); }
    tauri::Builder::default().plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None)).plugin(tauri_plugin_opener::init()).plugin(tauri_plugin_global_shortcut::Builder::new().build()).plugin(tauri_plugin_updater::Builder::new().build()).plugin(tauri_plugin_process::init()).plugin(tauri_plugin_dialog::init()).plugin(tauri_plugin_notification::init()).manage(state)
        .setup(|app| {
            // The window comes from tauri.conf.json ("create": false) so QA runs can keep a covered window alive.
            let config = app.config().app.windows.first().cloned().ok_or("falta la ventana principal en tauri.conf.json")?;
            #[allow(unused_mut)]
            let mut window = tauri::WebviewWindowBuilder::from_config(app.handle(), &config)?;
            #[cfg(debug_assertions)]
            if std::env::var_os("FINANZAS_DEV_EVAL").is_some() { window = window.background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled); }
            window.build()?;
            if let Some(quick) = app.config().app.windows.iter().find(|w| w.label == "quick").cloned() {
                tauri::WebviewWindowBuilder::from_config(app.handle(), &quick)?.build()?;
            }
            app.set_menu(menu::build(app.handle())?)?;
            app.on_menu_event(|app, event| menu::dispatch(app, event.id().as_ref()));
            menu::register_shortcut(app.handle());
            tray::refresh(app.handle());
            spawn_reminders(app.handle().clone());
            Ok(())
        })
        // With the menu-bar icon on, closing the window only hides it so reminders keep arriving.
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } if window.label() == "main" && window.app_handle().tray_by_id("finanzas").is_some() => {
                api.prevent_close();
                let _ = window.hide();
            }
            // Quick capture behaves like Spotlight: clicking elsewhere puts it away.
            tauri::WindowEvent::Focused(false) if window.label() == "quick" => { let _ = window.hide(); }
            _ => {}
        })
        // Debug builds only: FINANZAS_DEV_EVAL runs a script after the page loads, to drive the real UI in checks.
        .on_page_load(|_webview, _payload| {
            #[cfg(debug_assertions)]
            if _payload.event() == tauri::webview::PageLoadEvent::Finished && _webview.label() == "main" && let Ok(js) = std::env::var("FINANZAS_DEV_EVAL") && let Err(e) = _webview.eval(&js) { eprintln!("FINANZAS_DEV_EVAL: {e}"); }
        })
        .invoke_handler(tauri::generate_handler![startup_error, accounts_overview, data_version, overview, entries, add_entry, update_entry, delete_entry, payee_suggestions, suggested_category, categories_grouped, wishes, save_wish, reserve_wish, set_wish_item_paid, complete_wish, tag_entry, archive_wish, restore_wish, today, parse_amount, qa_log, qa_sleep,
            features::budget, features::assign_budget, features::create_account, features::debts, features::set_card_terms, features::save_installment, features::delete_installment, features::change_category, features::change_group, features::create_category, features::upcoming, features::schedules, features::save_schedule, features::post_schedule, features::skip_schedule, features::delete_schedule, features::restore_schedule, prefs::get_settings, prefs::update_settings, prefs::profiles, prefs::switch_profile, prefs::create_profile, prefs::rename_profile, prefs::delete_profile, prefs::test_notification, prefs::agents, prefs::database_path, features::import_availability, features::import_data, features::read_csv, features::pick_csv, features::import_entries, tray::refresh_tray, menu::show_quick, menu::hide_quick])
        .build(tauri::generate_context!()).expect("error al iniciar la aplicación")
        .run(|app, event| {
            // Clicking the Dock icon brings a hidden window back.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event { tray::show(app, None); }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
