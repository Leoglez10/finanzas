//! Menu-bar icon: balance as its title, the next payment, and a way back to the window.
//! While it exists, closing the window only hides it, so reminders keep firing.
use crate::AppState;
use chrono::Local;
use finanzas_core::{domain::money, service};
use tauri::{AppHandle, Manager, menu::{Menu, MenuItem, PredefinedMenuItem}, tray::TrayIconBuilder};

const ID: &str = "finanzas";

/// Shows the main window, optionally on a route (`proximos`, `resumen`…).
pub fn show(app: &AppHandle, route: Option<&str>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        if let Some(route) = route { let _ = window.eval(format!("location.hash = {route:?}")); }
    }
}

struct Summary { balance: String, next: Option<(String, bool)> }

fn summary(app: &AppHandle) -> Result<Summary, String> {
    let state = app.state::<AppState>();
    state.with_store(|s| {
        let balances = s.account_balances().map_err(|e| e.to_string())?;
        let open: i64 = s.list_accounts().map_err(|e| e.to_string())?.iter().filter(|a| !a.closed).map(|a| balances.get(&a.id).copied().unwrap_or(0)).sum();
        let next = service::upcoming(s, Local::now().date_naive(), 30).map_err(|e| e.to_string())?.into_iter().next().map(|u| {
            let when = if u.overdue { "vencido".to_owned() } else { u.date.format("%d/%m").to_string() };
            (format!("{}: {} · {when}", u.name, money::format(u.amount)), u.overdue)
        });
        Ok(Summary { balance: money::format(open), next })
    })
}

fn menu(app: &AppHandle, summary: &Summary) -> tauri::Result<Menu<tauri::Wry>> {
    let balance = MenuItem::with_id(app, "balance", format!("Saldo en cuentas: {}", summary.balance), false, None::<&str>)?;
    let next = match &summary.next {
        Some((text, overdue)) => MenuItem::with_id(app, "next", if *overdue { format!("⚠︎ {text}") } else { format!("Próximo: {text}") }, true, None::<&str>)?,
        None => MenuItem::with_id(app, "next", "Sin pagos en los próximos 30 días", false, None::<&str>)?,
    };
    let quick = MenuItem::with_id(app, "quick", "Captura rápida", true, Some(crate::menu::QUICK_SHORTCUT))?;
    let open = MenuItem::with_id(app, "open", "Abrir Finanzas", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir de Finanzas", true, Some("CmdOrCtrl+Q"))?;
    Menu::with_items(app, &[&balance, &next, &PredefinedMenuItem::separator(app)?, &quick, &open, &quit])
}

/// Creates the icon, or updates it when it already exists. Errors are logged, never fatal.
pub fn refresh(app: &AppHandle) {
    let enabled = app.state::<AppState>().settings.lock().map(|s| s.tray).unwrap_or(true);
    if !enabled { let _ = app.remove_tray_by_id(ID); return; }
    let summary = match summary(app) { Ok(s) => s, Err(e) => { eprintln!("Bandeja: {e}"); return; } };
    let result = (|| -> tauri::Result<()> {
        let menu = menu(app, &summary)?;
        if let Some(tray) = app.tray_by_id(ID) {
            tray.set_menu(Some(menu))?;
            tray.set_title(Some(&summary.balance))?;
        } else {
            let mut builder = TrayIconBuilder::with_id(ID).menu(&menu).title(&summary.balance).tooltip("Finanzas")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => show(app, None),
                    "next" => show(app, Some("proximos")),
                    "quick" => crate::menu::show_quick_window(app),
                    "quit" => app.exit(0),
                    _ => {},
                });
            if let Some(icon) = app.default_window_icon() { builder = builder.icon(icon.clone()); }
            builder.build(app)?;
        }
        Ok(())
    })();
    if let Err(e) = result { eprintln!("Bandeja: {e}"); }
}

/// The UI calls this after it changes data so the icon never shows a stale balance.
#[tauri::command(async)]
pub fn refresh_tray(app: AppHandle) { refresh(&app); }
