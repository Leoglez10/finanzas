use super::AppState;
use chrono::Local;
use finanzas_core::{service, store::{self, Store}, Account, Category, CategoryGroup, Schedule};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command(async)]
pub fn budget(state: State<'_, AppState>, month: i32) -> Result<service::BudgetView, String> {
    state.with_store(|s| service::budget_month(s, month).map_err(|e| e.to_string()))
}
#[tauri::command(async)]
pub fn assign_budget(state: State<'_, AppState>, month: i32, category_id: i64, text: String) -> Result<Option<i64>, String> {
    let result = state.with_store(|s| service::set_budget_from_text(s, month, category_id, &text).map_err(|e| e.to_string()))?;
    if result.is_some() { state.changed(); }
    Ok(result)
}
#[tauri::command(async)]
pub fn create_account(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let name = name.trim(); if name.is_empty() { return Err("Escribe un nombre".into()); }
    state.with_store(|s| { let order = s.list_accounts().map_err(|e| e.to_string())?.iter().map(|a| a.sort_order).reduce(f64::max).unwrap_or(0.0) + 1.0;
        s.insert_account(&Account { id: 0, name: name.into(), off_budget: false, closed: false, sort_order: order }).map_err(|e| e.to_string())?; Ok(()) })?;
    state.changed(); Ok(())
}
#[tauri::command(async)]
pub fn change_category(state: State<'_, AppState>, id: i64, name: Option<String>, hidden: Option<bool>) -> Result<(), String> {
    state.with_store(|s| { let mut c = s.get_category(id).map_err(|e| e.to_string())?.ok_or("Categoría no encontrada")?;
        if let Some(n) = name { if n.trim().is_empty() { return Err("Escribe un nombre".into()); } c.name = n.trim().into(); }
        if let Some(h) = hidden { c.hidden = h; } s.update_category(&c).map_err(|e| e.to_string()) })?;
    state.changed(); Ok(())
}
#[tauri::command(async)]
pub fn change_group(state: State<'_, AppState>, id: Option<i64>, name: String) -> Result<(), String> {
    let name = name.trim(); if name.is_empty() { return Err("Escribe un nombre".into()); }
    state.with_store(|s| { if let Some(id) = id { let mut g = s.get_category_group(id).map_err(|e| e.to_string())?.ok_or("Grupo no encontrado")?; g.name = name.into(); s.update_category_group(&g).map_err(|e| e.to_string())?; }
        else { let order = s.list_category_groups().map_err(|e| e.to_string())?.iter().filter(|g| !g.is_income).map(|g| g.sort_order).fold(0.0_f64, f64::max) + 1.0; s.insert_category_group(&CategoryGroup { id: 0, name: name.into(), is_income: false, sort_order: order }).map_err(|e| e.to_string())?; } Ok(()) })?;
    state.changed(); Ok(())
}
#[tauri::command(async)]
pub fn create_category(state: State<'_, AppState>, group_id: i64, name: String) -> Result<(), String> {
    let name = name.trim(); if name.is_empty() { return Err("Escribe un nombre".into()); }
    state.with_store(|s| { let order = s.list_categories().map_err(|e| e.to_string())?.iter().filter(|c| c.group_id == Some(group_id)).map(|c| c.sort_order).fold(0.0_f64, f64::max) + 1.0; s.insert_category(&Category { id: 0, name: name.into(), hidden: false, sort_order: order, group_id: Some(group_id) }).map_err(|e| e.to_string())?; Ok(()) })?;
    state.changed(); Ok(())
}
#[tauri::command(async)]
pub fn upcoming(state: State<'_, AppState>) -> Result<Vec<service::UpcomingView>, String> { state.with_store(|s| service::upcoming(s, Local::now().date_naive(), 30).map_err(|e| e.to_string())) }
#[tauri::command(async)]
pub fn schedules(state: State<'_, AppState>) -> Result<Vec<service::ScheduleView>, String> { state.with_store(|s| service::schedule_views(s).map_err(|e| e.to_string())) }
#[tauri::command(async)]
pub fn save_schedule(state: State<'_, AppState>, id: Option<i64>, draft: service::ScheduleDraft) -> Result<i64, String> {
    if draft.name.trim().is_empty() { return Err("Ponle un nombre".into()); }
    if draft.amount == 0 { return Err("Escribe un monto mayor a cero".into()); }
    if !(1..=52).contains(&draft.interval) { return Err("La frecuencia debe estar entre 1 y 52".into()); }
    let id = state.with_store(|s| { let id = service::save_schedule(s, id, &draft).map_err(|e| e.to_string())?; service::auto_post_due(s, Local::now().date_naive()).map_err(|e| e.to_string())?; Ok(id) })?;
    state.changed(); Ok(id)
}
#[derive(serde::Serialize)]
pub struct Posted { entry: store::EntryRow, before: Schedule }
/// Registers the next occurrence; returns the movement and the schedule as it was, for undo.
#[tauri::command(async)]
pub fn post_schedule(state: State<'_, AppState>, id: i64) -> Result<Posted, String> {
    let posted = state.with_store(|s| {
        let before = s.get_schedule(id).map_err(|e| e.to_string())?.ok_or("El pago programado ya no existe")?;
        let entry = service::post_schedule(s, id, Local::now().date_naive()).map_err(|e| e.to_string())?;
        Ok(Posted { entry, before })
    })?;
    state.changed(); Ok(posted)
}
/// Skips the next occurrence; returns the schedule as it was, for undo.
#[tauri::command(async)]
pub fn skip_schedule(state: State<'_, AppState>, id: i64) -> Result<Schedule, String> {
    let before = state.with_store(|s| { let before = s.get_schedule(id).map_err(|e| e.to_string())?.ok_or("El pago programado ya no existe")?; service::skip_schedule(s, id, None).map_err(|e| e.to_string())?; Ok(before) })?;
    state.changed(); Ok(before)
}
#[tauri::command(async)]
pub fn delete_schedule(state: State<'_, AppState>, id: i64) -> Result<Schedule, String> {
    let before = state.with_store(|s| { let before = s.get_schedule(id).map_err(|e| e.to_string())?.ok_or("El pago programado ya no existe")?; service::delete_schedule(s, id).map_err(|e| e.to_string())?; Ok(before) })?;
    state.changed(); Ok(before)
}
/// Undo for the three above: puts the schedule back exactly (a deleted one comes back with a new id).
#[tauri::command(async)]
pub fn restore_schedule(state: State<'_, AppState>, schedule: Schedule) -> Result<i64, String> {
    let id = state.with_store(|s| {
        if s.get_schedule(schedule.id).map_err(|e| e.to_string())?.is_some() { s.update_schedule(&schedule).map_err(|e| e.to_string())?; Ok(schedule.id) }
        else { s.insert_schedule(&Schedule { id: 0, ..schedule }).map_err(|e| e.to_string()) }
    })?;
    state.changed(); Ok(id)
}
#[tauri::command(async)]
pub fn import_availability(state: State<'_, AppState>) -> Result<(bool, bool), String> {
    let empty = state.with_store(|s| Ok(s.list_accounts().map_err(|e| e.to_string())?.is_empty() && s.entries(&store::EntryFilter { limit: Some(1), ..Default::default() }).map_err(|e| e.to_string())?.1 == 0))?;
    #[cfg(target_os = "macos")]
    let legacy = std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("Library/Application Support/Finanzas/Finanzas.store").is_file()).unwrap_or(false);
    #[cfg(not(target_os = "macos"))]
    let legacy = false;
    Ok((empty, legacy))
}
#[tauri::command(async)]
pub fn import_data(app: AppHandle, state: State<'_, AppState>, source: String) -> Result<Option<store::ImportSummary>, String> {
    let path = if source == "actual" {
        let picked = app.dialog().file().set_title("Selecciona el db.sqlite o el respaldo .zip de Actual").blocking_pick_file();
        let Some(file) = picked else { return Ok(None) }; let path = file.into_path().map_err(|e| e.to_string())?;
        let cache = app.path().app_cache_dir().map_err(|e| e.to_string())?.join(format!("actual-import-{}-{}", std::process::id(), Local::now().timestamp_nanos_opt().unwrap_or_default()));
        std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
        store::extract_actual(&path, &cache).map_err(|e| e.to_string())?
    } else if source == "legacy" && cfg!(target_os = "macos") {
        std::path::PathBuf::from(std::env::var_os("HOME").ok_or("No se encontró el directorio personal")?).join("Library/Application Support/Finanzas/Finanzas.store")
    } else { return Err("Origen desconocido".into()); };
    let summary = state.with_store(|s: &mut Store| { let result = if source == "legacy" { s.import_swiftdata(&path) } else { s.import_actual(&path) }; result.map_err(|e| match e { store::ImportError::StoreNotEmpty => "Ya tienes datos".into(), _ => e.to_string() }) })?;
    state.changed(); Ok(Some(summary))
}

#[derive(serde::Serialize)]
pub struct CsvFile { name: String, text: String }
/// Reads a dropped or picked CSV. Only .csv files up to 5 MB, so a stray drop never loads something huge.
fn read_csv_file(path: &std::path::Path) -> Result<CsvFile, String> {
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("csv")) { return Err("Solo puedo importar archivos .csv".into()); }
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > 5 * 1024 * 1024 { return Err("El archivo pesa más de 5 MB".into()); }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    // Some banks export Latin-1; fall back so accents do not break the whole file.
    let text = String::from_utf8(bytes.clone()).unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect());
    Ok(CsvFile { name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), text })
}
#[tauri::command(async)]
pub fn read_csv(path: String) -> Result<CsvFile, String> { read_csv_file(std::path::Path::new(&path)) }
#[tauri::command(async)]
pub fn pick_csv(app: AppHandle) -> Result<Option<CsvFile>, String> {
    let Some(file) = app.dialog().file().set_title("Elige el CSV de tu banco").add_filter("CSV", &["csv"]).blocking_pick_file() else { return Ok(None) };
    read_csv_file(&file.into_path().map_err(|e| e.to_string())?).map(Some)
}
/// Adds every row or none: a failure halfway leaves the ledger untouched. Returns the new ids for undo.
#[tauri::command(async)]
pub fn import_entries(state: State<'_, AppState>, entries: Vec<service::NewEntry>) -> Result<Vec<i64>, String> {
    let ids = state.with_store(|s| s.transaction(|s| entries.into_iter().map(|e| service::add_entry(s, e).map(|row| row.id)).collect::<Result<Vec<_>, _>>()).map_err(|e: service::ServiceError| e.to_string()))?;
    state.changed(); Ok(ids)
}
