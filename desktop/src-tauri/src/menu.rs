//! Native menu bar and the quick-capture window.
//! Menu items carry the frontend command id (desktop/src/lib/commands.svelte.ts); choosing one
//! emits `menu` with that id to the main window, which runs it. Accelerators live here only.
use tauri::{AppHandle, Emitter, Manager, menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu}};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const QUICK_SHORTCUT: &str = "Alt+Shift+F";

fn item(app: &AppHandle, id: &str, label: &str, accel: Option<&str>) -> tauri::Result<MenuItem<tauri::Wry>> {
    MenuItem::with_id(app, id, label, true, accel)
}

pub fn build(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let sep = || PredefinedMenuItem::separator(app);
    let finanzas = Submenu::with_items(app, "Finanzas", true, &[
        &PredefinedMenuItem::about(app, Some("Acerca de Finanzas"), Some(AboutMetadata::default()))?, &sep()?,
        &item(app, "go-ajustes", "Ajustes…", Some("CmdOrCtrl+,"))?, &sep()?,
        &PredefinedMenuItem::hide(app, Some("Ocultar Finanzas"))?, &PredefinedMenuItem::hide_others(app, Some("Ocultar otros"))?, &sep()?,
        &PredefinedMenuItem::quit(app, Some("Salir de Finanzas"))?,
    ])?;
    let archivo = Submenu::with_items(app, "Archivo", true, &[
        &item(app, "new-gasto", "Nuevo gasto", Some("CmdOrCtrl+N"))?,
        &item(app, "new-ingreso", "Nuevo ingreso", Some("CmdOrCtrl+Shift+N"))?,
        &item(app, "quick", "Captura rápida", Some(QUICK_SHORTCUT))?,
        &item(app, "new-wish", "Nuevo deseo", None)?,
        &item(app, "new-schedule", "Programar pago", None)?, &sep()?,
        &item(app, "import", "Importar CSV, Excel o PDF…", Some("CmdOrCtrl+I"))?,
        &sep()?, &PredefinedMenuItem::close_window(app, Some("Cerrar ventana"))?,
    ])?;
    // Undo/redo/cut/copy/paste stay native so text fields behave; the app's ⌘Z runs in the webview
    // first and only when no field is being edited.
    let edicion = Submenu::with_items(app, "Edición", true, &[
        &PredefinedMenuItem::undo(app, Some("Deshacer"))?, &PredefinedMenuItem::redo(app, Some("Rehacer"))?, &sep()?,
        &PredefinedMenuItem::cut(app, Some("Cortar"))?, &PredefinedMenuItem::copy(app, Some("Copiar"))?, &PredefinedMenuItem::paste(app, Some("Pegar"))?,
        &PredefinedMenuItem::select_all(app, Some("Seleccionar todo"))?, &sep()?,
        &item(app, "search", "Buscar movimientos", Some("CmdOrCtrl+F"))?,
    ])?;
    let ver = Submenu::with_items(app, "Ver", true, &[
        &item(app, "go-resumen", "Resumen", Some("CmdOrCtrl+1"))?,
        &item(app, "go-movimientos", "Movimientos", Some("CmdOrCtrl+2"))?,
        &item(app, "go-presupuesto", "Presupuesto", Some("CmdOrCtrl+3"))?,
        &item(app, "go-proximos", "Próximos pagos", Some("CmdOrCtrl+4"))?,
        &item(app, "go-deseos", "Deseos", Some("CmdOrCtrl+5"))?, &sep()?,
        &item(app, "theme", "Cambiar entre claro y oscuro", Some("CmdOrCtrl+Shift+L"))?,
        &PredefinedMenuItem::fullscreen(app, Some("Pantalla completa"))?,
    ])?;
    let ventana = Submenu::with_items(app, "Ventana", true, &[
        &item(app, "palette", "Paleta de comandos", Some("CmdOrCtrl+K"))?, &sep()?,
        &PredefinedMenuItem::minimize(app, Some("Minimizar"))?, &PredefinedMenuItem::maximize(app, Some("Zoom"))?,
    ])?;
    Menu::with_items(app, &[&finanzas, &archivo, &edicion, &ver, &ventana])
}

/// Sends a command id to the main window (shown first, since menus also work while it is hidden).
pub fn dispatch(app: &AppHandle, id: &str) {
    if id == "quick" { show_quick_window(app); return; }
    crate::tray::show(app, None);
    if let Err(e) = app.emit_to("main", "menu", id) { eprintln!("Menú: {e}"); }
}

pub fn show_quick_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("quick") {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("quick-open", ());
    }
}

/// ⌥⇧F from any app toggles quick capture.
pub fn register_shortcut(app: &AppHandle) {
    let result = QUICK_SHORTCUT.parse::<Shortcut>().map_err(|e| e.to_string()).and_then(|shortcut| {
        app.global_shortcut().on_shortcut(shortcut, |app, _, event| {
            if event.state() != ShortcutState::Pressed { return; }
            match app.get_webview_window("quick") {
                Some(w) if w.is_visible().unwrap_or(false) => { let _ = w.hide(); }
                _ => show_quick_window(app),
            }
        }).map_err(|e| e.to_string())
    });
    if let Err(e) = result { eprintln!("No se pudo registrar {QUICK_SHORTCUT}: {e}"); }
}

#[tauri::command]
pub fn show_quick(app: AppHandle) { show_quick_window(&app); }
#[tauri::command]
pub fn hide_quick(app: AppHandle) { if let Some(w) = app.get_webview_window("quick") { let _ = w.hide(); } }
