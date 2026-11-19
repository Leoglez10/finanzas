use super::AppState;
use chrono::Local;
use finanzas_core::{service, store::{self, Store}, Account, Category, CategoryGroup, Recurrence, Schedule};
use serde::Deserialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use crate::settings::Settings;

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
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSchedule { name: String, amount: i64, auto_post: bool, recurrence: Recurrence, account_id: i64, category_id: Option<i64>, payee: String }
#[tauri::command(async)]
pub fn create_schedule(state: State<'_, AppState>, input: NewSchedule) -> Result<(), String> {
    if input.name.trim().is_empty() || !(1..=52).contains(&input.recurrence.interval) { return Err("Datos inválidos".into()); }
    state.with_store(|s| { let payee_id = s.find_or_create_payee(&input.payee).map_err(|e| e.to_string())?; let schedule = Schedule { id: 0, name: input.name.trim().into(), amount: input.amount, auto_post: input.auto_post, completed: false, next_date: None, recurrence: Some(input.recurrence), account_id: Some(input.account_id), category_id: input.category_id, payee_id }; service::create_schedule(s, &schedule).map_err(|e| e.to_string())?; service::auto_post_due(s, Local::now().date_naive()).map_err(|e| e.to_string())?; Ok(()) })?; state.changed(); Ok(())
}
#[tauri::command(async)]
pub fn schedule_action(state: State<'_, AppState>, id: i64, action: String) -> Result<(), String> {
    state.with_store(|s| { match action.as_str() { "post" => { service::post_schedule(s, id, Local::now().date_naive()).map_err(|e| e.to_string())?; }, "skip" => { service::skip_schedule(s, id, None).map_err(|e| e.to_string())?; }, "delete" => service::delete_schedule(s, id).map_err(|e| e.to_string())?, _ => return Err("Acción desconocida".into()) }; Ok(()) })?; state.changed(); Ok(())
}
#[tauri::command(async)]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> { state.settings.lock().map(|s| s.clone()).map_err(|e| e.to_string()) }
#[tauri::command(async)]
pub fn save_settings(state: State<'_, AppState>, enabled: bool, days: i64) -> Result<(), String> {
    if !(0..=14).contains(&days) { return Err("Días inválidos".into()); }
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?; settings.enabled = enabled; settings.days = days; state.save_settings(&settings)
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
