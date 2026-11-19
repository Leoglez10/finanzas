//! Ajustes: preferences (settings.json) and profiles (one database per profile beside the default one).
use crate::{AppState, settings::Settings};
use finanzas_core::store::Store;
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

#[derive(Serialize)]
pub struct ProfileView { slug: String, label: String, hidden: bool, color: Option<String> }
#[derive(Serialize)]
pub struct Profiles { active: String, list: Vec<ProfileView> }

impl AppState {
    fn home(&self) -> Result<&std::path::Path, String> { self.home.as_deref().ok_or_else(|| "No hay base de datos predeterminada".into()) }
    fn update(&self, change: impl FnOnce(&mut Settings)) -> Result<Settings, String> {
        let mut settings = self.settings.lock().map_err(|e| e.to_string())?;
        change(&mut settings);
        *settings = settings.clone().clamped();
        self.save_settings(&settings)?;
        Ok(settings.clone())
    }
    /// Closes the current database and opens `slug`'s in its place; every command and the
    /// background jobs share the same handle, so they all follow.
    pub(crate) fn open_profile(&self, slug: &str) -> Result<(), String> {
        let path = Store::profile_path_beside(self.home()?, slug).ok_or("Nombre de perfil inválido")?;
        let opened = Store::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        *self.store.as_ref().ok_or("La base de datos no está disponible")?.lock().map_err(|e| e.to_string())? = opened;
        *self.profile.lock().map_err(|e| e.to_string())? = slug.to_owned();
        self.changed();
        Ok(())
    }
    fn active(&self) -> Result<String, String> { self.profile.lock().map(|p| p.clone()).map_err(|e| e.to_string()) }
}

#[tauri::command(async)]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> { state.settings.lock().map(|s| s.clone()).map_err(|e| e.to_string()) }

/// Merges the given fields into the settings. `sent` (reminders already shown) is never taken from the UI.
#[tauri::command(async)]
pub fn update_settings(app: AppHandle, state: State<'_, AppState>, patch: serde_json::Map<String, serde_json::Value>) -> Result<Settings, String> {
    let settings = state.update(|current| {
        let mut value = serde_json::to_value(&*current).unwrap_or_default();
        if let Some(object) = value.as_object_mut() { for (k, v) in patch { if k != "sent" { object.insert(k, v); } } }
        if let Ok(merged) = serde_json::from_value::<Settings>(value) { *current = merged; }
    })?;
    let autolaunch = app.autolaunch();
    let result = if settings.autostart { autolaunch.enable() } else { autolaunch.disable() };
    if let Err(e) = result { eprintln!("No se pudo cambiar el inicio automático: {e}"); }
    crate::tray::refresh(&app);
    Ok(settings)
}

#[tauri::command(async)]
pub fn profiles(state: State<'_, AppState>) -> Result<Profiles, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let list = Store::profiles_beside(state.home()?).into_iter().map(|slug| ProfileView {
        label: Store::profile_label(&slug), hidden: settings.hidden_profiles.contains(&slug), color: settings.profile_colors.get(&slug).cloned(), slug,
    }).collect();
    Ok(Profiles { active: state.active()?, list })
}

#[tauri::command(async)]
pub fn switch_profile(app: AppHandle, state: State<'_, AppState>, slug: String) -> Result<(), String> {
    state.open_profile(&slug)?;
    state.update(|s| s.profile = slug)?;
    crate::tray::refresh(&app);
    Ok(())
}

/// Creates a profile from a typed name and switches to it; returns its slug.
#[tauri::command(async)]
pub fn create_profile(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<String, String> {
    let slug = Store::profile_slug(&name).ok_or("Escribe un nombre con letras o números")?;
    if Store::profiles_beside(state.home()?).contains(&slug) { return Err("Ya existe un perfil con ese nombre".into()); }
    switch_profile(app, state, slug.clone())?;
    Ok(slug)
}

/// Renames a profile's database; the active one is closed during the move and reopened after.
#[tauri::command(async)]
pub fn rename_profile(state: State<'_, AppState>, from: String, name: String) -> Result<String, String> {
    let to = Store::profile_slug(&name).ok_or("Escribe un nombre con letras o números")?;
    if to == from { return Ok(to); }
    let home = state.home()?.to_owned();
    let active = state.active()? == from;
    {
        let mut store = state.store.as_ref().ok_or("La base de datos no está disponible")?.lock().map_err(|e| e.to_string())?;
        if active { *store = Store::open_in_memory().map_err(|e| e.to_string())?; }
        let moved = Store::rename_profile(&home, &from, &to);
        let reopen = if moved.is_ok() { &to } else { &from };
        if active { *store = Store::open(Store::profile_path_beside(&home, reopen).ok_or("Nombre de perfil inválido")?).map_err(|e| e.to_string())?; }
        moved?;
    }
    if active { *state.profile.lock().map_err(|e| e.to_string())? = to.clone(); }
    state.update(|s| {
        for p in s.hidden_profiles.iter_mut() { if *p == from { *p = to.clone(); } }
        if let Some(color) = s.profile_colors.remove(&from) { s.profile_colors.insert(to.clone(), color); }
        if s.profile == from { s.profile = to.clone(); }
    })?;
    state.changed();
    Ok(to)
}

/// Permanently deletes a profile (the UI asks first). The active one hands over to Personal.
#[tauri::command(async)]
pub fn delete_profile(app: AppHandle, state: State<'_, AppState>, slug: String) -> Result<(), String> {
    if state.active()? == slug { switch_profile(app, state.clone(), "personal".into())?; }
    Store::delete_profile(state.home()?, &slug)?;
    state.update(|s| { s.hidden_profiles.retain(|p| *p != slug); s.profile_colors.remove(&slug); })?;
    state.changed();
    Ok(())
}

/// Ajustes › Avisos: shows a sample so the user sees what reminders look like.
#[tauri::command(async)]
pub fn test_notification(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    app.notification().builder().title("Así se ven tus avisos").body("Renta: $800.00 de Cuenta principal vence mañana.").show().map_err(|e| e.to_string())
}
