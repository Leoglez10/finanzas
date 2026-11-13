//! T10c: Ajustes, Nueva cuenta, importers and in-app reminders.
//! Every store is in memory; files live under `target/t10c-tests`. No test opens the default database.
use super::*;
use rune::{Env, FileDialogKind, Runtime};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

const ACTUAL_SQL: &str = include_str!("t10c_actual.sql");
const SWIFT_SQL: &str = "CREATE TABLE ZACCOUNT(Z_PK, ZNAME, ZOFFBUDGET, ZCLOSED, ZSORTORDER);
    CREATE TABLE ZCATEGORYGROUP(Z_PK,ZNAME,ZISINCOME,ZSORTORDER);
    CREATE TABLE ZCATEGORY(Z_PK,ZNAME,ZHIDDEN,ZSORTORDER,ZGROUP);
    CREATE TABLE ZPAYEE(Z_PK,ZNAME);
    CREATE TABLE ZLEDGERENTRY(Z_PK,ZAMOUNT,ZCLEARED,ZACCOUNT,ZCATEGORY,ZPAYEE,ZSCHEDULE,ZDATE,ZNOTES);
    CREATE TABLE ZSCHEDULE(Z_PK,ZNAME,ZAMOUNT,ZAUTOPOST,ZCOMPLETED,ZACCOUNT,ZCATEGORY,ZPAYEE,ZRECURRENCEDATA,ZNEXTDATE);
    CREATE TABLE ZBUDGETAMOUNT(Z_PK,ZAMOUNT,ZCARRYOVER,ZMONTH,ZCATEGORY);
    INSERT INTO ZACCOUNT VALUES (20,'Banco importado',NULL,NULL,1);
    INSERT INTO ZCATEGORYGROUP VALUES (30,'Food',NULL,1);
    INSERT INTO ZCATEGORY VALUES (40,'Groceries',NULL,1,30);
    INSERT INTO ZPAYEE VALUES (50,'Shop');
    INSERT INTO ZSCHEDULE VALUES (60,'Rent',-100,1,NULL,20,40,50,CAST('{\"start\":801036000,\"frequency\":\"monthly\",\"interval\":1,\"skipWeekend\":false,\"weekendMode\":\"after\",\"endDate\":801036000}' AS BLOB),801036000);
    INSERT INTO ZLEDGERENTRY VALUES (70,-100,1,20,40,50,60,801036000.5,NULL), (71,-250,1,20,40,50,NULL,801036000.5,'x');
    INSERT INTO ZBUDGETAMOUNT VALUES (80,500,1,202605,40);";

static COUNTER: AtomicU32 = AtomicU32::new(0);
fn scratch_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/t10c-tests")
        .join(format!("{}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
fn sqlite_fixture(sql: &str, name: &str) -> PathBuf {
    let path = scratch_dir().join(name);
    rusqlite::Connection::open(&path).unwrap().execute_batch(sql).unwrap();
    path
}

fn runtime_of(mut state: State) -> Runtime<State> {
    state.settings_path = Some(scratch_dir().join("settings.json"));
    let mut rt = Runtime::new(state, view, (1200.0, 800.0), Env::new(false, 1.0));
    rt.on_key = Some(on_key); rt.poll = Some(poll); rt.frame(0.0); rt
}
fn runtime(scene: fn(&mut State)) -> Runtime<State> { let mut state = State::empty(); scene(&mut state); runtime_of(state) }
fn settle(rt: &mut Runtime<State>) { for _ in 0..150 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); } }
fn animate(rt: &mut Runtime<State>) { for _ in 0..60 { rt.frame(1.0 / 60.0); } }
fn click(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("missing {key}"));
    rt.pointer_down(x, y); rt.pointer_up(x, y); rt.frame(0.0);
}
fn press(rt: &mut Runtime<State>, key: Key, mods: rune::Mods) { rt.key(KeyEvent { key, mods }); rt.frame(0.0); }
fn cmd() -> rune::Mods { rune::Mods { cmd: true, ..Default::default() } }
fn fill(rt: &mut Runtime<State>, key: &str, value: &str) {
    click(rt, key);
    rt.key(KeyEvent { key: Key::Command('a'), mods: cmd() });
    rt.key(KeyEvent { key: Key::Text(value.into()), mods: Default::default() });
    rt.frame(0.0);
}
fn text_of(rt: &Runtime<State>, key: &str) -> Option<String> { rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).and_then(|e| e.text) }
fn has_text(rt: &Runtime<State>, wanted: &str) -> bool { rt.elements().iter().any(|e| e.text.as_deref().is_some_and(|t| t.contains(wanted))) }
fn open_ajustes(rt: &mut Runtime<State>) { click(rt, "ajustes"); animate(rt); }
fn account_names(rt: &Runtime<State>) -> Vec<String> { rt.state.accounts.iter().map(|a| a.account.name.clone()).collect() }

// ---- Settings ----
#[test]
fn settings_round_trip_defaults_and_clamping() {
    let path = scratch_dir().join("settings.json");
    assert_eq!(settings::load(&path).unwrap(), settings::Settings { enabled: true, days: 2, hour: 9, hidden_profiles: Vec::new(), dark: None });
    let saved = settings::Settings { enabled: false, days: 5, hour: 7, hidden_profiles: Vec::new(), dark: None };
    settings::save(&path, &saved).unwrap();
    assert_eq!(settings::load(&path).unwrap(), saved);
    std::fs::write(&path, r#"{"days":3}"#).unwrap();
    assert_eq!(settings::load(&path).unwrap(), settings::Settings { enabled: true, days: 3, hour: 9, hidden_profiles: Vec::new(), dark: None });
    std::fs::write(&path, r#"{"days":99,"hour":40}"#).unwrap();
    assert_eq!(settings::load(&path).unwrap(), settings::Settings { enabled: true, days: 14, hour: 23, hidden_profiles: Vec::new(), dark: None });
    // A legacy Tauri file carries a `sent` list that must not break loading.
    std::fs::write(&path, r#"{"enabled":false,"days":4,"sent":["pago-1"]}"#).unwrap();
    assert_eq!(settings::load(&path).unwrap(), settings::Settings { enabled: false, days: 4, hour: 9, hidden_profiles: Vec::new(), dark: None });
    std::fs::write(&path, "not json").unwrap();
    assert!(settings::load(&path).is_err());
}
#[test]
fn load_from_reads_settings_next_to_the_database_and_enables_timers() {
    let dir = scratch_dir();
    std::fs::write(dir.join("settings.json"), r#"{"enabled":false,"days":5,"hour":7}"#).unwrap();
    let state = State::load_from(Some(dir.join("finanzas.db")));
    assert_eq!(state.settings_path, Some(dir.join("settings.json")));
    assert_eq!(state.settings, settings::Settings { enabled: false, days: 5, hour: 7, hidden_profiles: Vec::new(), dark: None });
    assert!(state.hourly && state.open_default && state.settings_error.is_none());
    assert!(!State::empty().hourly);
    let broken = scratch_dir(); std::fs::write(broken.join("settings.json"), "{{").unwrap();
    let state = State::load_from(Some(broken.join("finanzas.db")));
    assert_eq!(state.settings, settings::Settings::default());
    assert!(state.settings_error.as_deref().is_some_and(|e| e.starts_with("Ajustes inválidos")));
}
#[test]
fn ajustes_controls_change_and_persist_settings() {
    let mut rt = runtime(demo_light); open_ajustes(&mut rt);
    let path = rt.state.settings_path.clone().unwrap();
    assert_eq!(rt.state.route, Route::Ajustes);
    assert_eq!(text_of(&rt, "days-value").as_deref(), Some("Avisar 2 día(s) antes"));
    assert_eq!(text_of(&rt, "hour-value").as_deref(), Some("a las 9:00"));
    click(&mut rt, "days-plus"); click(&mut rt, "hour-minus");
    assert_eq!((rt.state.settings.days, rt.state.settings.hour), (3, 8));
    assert_eq!(settings::load(&path).unwrap(), rt.state.settings);
    click(&mut rt, "reminders-toggle");
    assert!(!rt.state.settings.enabled);
    assert!(!settings::load(&path).unwrap().enabled);
    // Disabled: the steppers do nothing.
    click(&mut rt, "days-plus");
    assert_eq!(rt.state.settings.days, 3);
    click(&mut rt, "reminders-toggle");
    assert!(rt.state.settings.enabled);
    rt.state.settings.days = 14; click(&mut rt, "days-plus"); assert_eq!(rt.state.settings.days, 14);
    rt.state.settings.days = 0; click(&mut rt, "days-minus"); assert_eq!(rt.state.settings.days, 0);
    rt.state.settings.hour = 23; click(&mut rt, "hour-plus"); assert_eq!(rt.state.settings.hour, 23);
}
#[test]
fn settings_write_failure_is_shown() {
    let mut rt = runtime(demo_light); open_ajustes(&mut rt);
    let blocker = scratch_dir().join("blocker"); std::fs::write(&blocker, "x").unwrap();
    rt.state.settings_path = Some(blocker.join("settings.json"));
    click(&mut rt, "reminders-toggle");
    assert!(text_of(&rt, "settings-error").is_some_and(|t| t.starts_with("No se pudo guardar los ajustes")));
}
#[test]
fn sidebar_entry_and_command_comma_open_ajustes() {
    let mut rt = runtime(demo_light);
    click(&mut rt, "ajustes"); assert_eq!(rt.state.route, Route::Ajustes);
    click(&mut rt, "resumen"); assert_eq!(rt.state.route, Route::Resumen);
    press(&mut rt, Key::Command(','), cmd());
    assert_eq!(rt.state.route, Route::Ajustes);
    // Not while a sheet is open.
    click(&mut rt, "resumen"); click(&mut rt, "new-account");
    press(&mut rt, Key::Command(','), cmd());
    assert_eq!(rt.state.route, Route::Resumen);
}

// ---- Nueva cuenta ----
#[test]
fn creating_an_account_adds_it_to_the_sidebar_with_its_opening_balance() {
    let mut rt = runtime(demo_light);
    let before = rt.state.accounts.len();
    let max_before = rt.state.accounts.iter().map(|a| a.account.sort_order).fold(0.0, f64::max);
    click(&mut rt, "new-account"); animate(&mut rt);
    assert!(rt.state.account_sheet.is_some());
    fill(&mut rt, "account-name", "Ahorros"); fill(&mut rt, "account-opening", "1,500.50");
    click(&mut rt, "account-save"); settle(&mut rt);
    assert!(rt.state.account_sheet.is_none());
    assert_eq!(rt.state.accounts.len(), before + 1);
    let created = rt.state.accounts.iter().find(|a| a.account.name == "Ahorros").expect("account listed");
    assert_eq!((created.balance, created.account.off_budget, created.account.closed), (150_050, false, false));
    assert_eq!(created.account.sort_order, max_before + 1.0);
    let key = format!("cuenta-{}", created.account.id);
    assert!(rt.find(&key).is_some(), "sidebar row {key}");
    assert!(has_text(&rt, &money::format(150_050)));
    // The opening balance is a ledger entry dated today.
    let ledger = data::ledger(Arc::clone(&rt.state.worker), Some(created.account.id), "").unwrap();
    assert_eq!(ledger.rows.len(), 1);
    assert_eq!((ledger.rows[0].amount, ledger.rows[0].notes.as_str(), ledger.rows[0].date), (150_050, "Saldo inicial", rt.state.today_date));
}
#[test]
fn off_budget_type_and_negative_opening_balance() {
    let mut rt = runtime(empty_scene);
    click(&mut rt, "new-account"); animate(&mut rt);
    fill(&mut rt, "account-name", "  Hipoteca "); fill(&mut rt, "account-opening", "-2500");
    click(&mut rt, "account-type-off");
    click(&mut rt, "account-save"); settle(&mut rt);
    let a = rt.state.accounts.iter().find(|a| a.account.name == "Hipoteca").expect("trimmed name");
    assert_eq!((a.balance, a.account.off_budget), (-250_000, true));
}
#[test]
fn blank_opening_balance_creates_no_entry_and_invalid_input_blocks_saving() {
    let mut rt = runtime(empty_scene);
    click(&mut rt, "new-account"); animate(&mut rt);
    click(&mut rt, "account-save"); settle(&mut rt);
    assert!(rt.state.account_sheet.is_some() && rt.state.accounts.is_empty(), "empty name cannot be saved");
    fill(&mut rt, "account-name", "Efectivo"); fill(&mut rt, "account-opening", "abc");
    click(&mut rt, "account-save"); settle(&mut rt);
    assert!(rt.state.account_sheet.is_some() && rt.state.accounts.is_empty(), "bad opening balance cannot be saved");
    fill(&mut rt, "account-opening", "5"); press(&mut rt, Key::Backspace, Default::default());
    assert_eq!(rt.state.account_sheet.as_ref().unwrap().opening.text, "");
    click(&mut rt, "account-save"); settle(&mut rt);
    assert_eq!(account_names(&rt), vec!["Efectivo".to_owned()]);
    assert_eq!(rt.state.accounts[0].balance, 0);
    assert!(data::ledger(Arc::clone(&rt.state.worker), None, "").unwrap().rows.is_empty());
}
#[test]
fn account_sheet_cancels_with_escape_and_cancel_button() {
    let mut rt = runtime(empty_scene);
    click(&mut rt, "new-account"); animate(&mut rt);
    press(&mut rt, Key::Escape, Default::default());
    assert!(rt.state.account_sheet.is_none());
    click(&mut rt, "new-account"); animate(&mut rt); click(&mut rt, "account-cancel");
    assert!(rt.state.account_sheet.is_none() && rt.state.accounts.is_empty());
}

// ---- Reminders ----
fn banner_ids(rt: &Runtime<State>) -> Vec<String> {
    rt.elements().into_iter().filter_map(|e| e.key).filter_map(|k| k.strip_prefix("reminder-").map(str::to_owned)).collect()
}
#[test]
fn reminders_appear_when_enabled_and_hide_when_disabled() {
    let mut rt = runtime(reminders_scene); animate(&mut rt);
    assert!(rt.find("reminders-banner").is_some());
    assert!(!rt.state.reminders.is_empty());
    assert_eq!(banner_ids(&rt).len(), rt.state.reminders.len().min(overview::BANNER_ROWS));
    assert!(has_text(&rt, "Pago próximo: Netflix"));
    assert!(has_text(&rt, "14/11/2026"), "dd/MM/yyyy dates");
    rt.state.settings.enabled = false; rt.frame(0.0); settle(&mut rt);
    assert!(rt.find("reminders-banner").is_none());
    assert!(rt.state.reminders.is_empty());
}
#[test]
fn enabling_reminders_computes_them_on_the_worker() {
    let mut state = State::empty(); reminders_scene(&mut state);
    state.settings.enabled = false; state.reminders.clear();
    let mut rt = runtime_of(state); settle(&mut rt);
    assert!(rt.find("reminders-banner").is_none() && rt.state.reminders.is_empty());
    rt.state.settings.enabled = true; settle(&mut rt);
    assert!(rt.find("reminders-banner").is_some());
    // A different lead time moves the alert times, so the list is read again.
    let before: Vec<_> = rt.state.reminders.iter().map(|r| r.fire_at).collect();
    rt.state.settings.days = 0; settle(&mut rt);
    let after: Vec<_> = rt.state.reminders.iter().map(|r| r.fire_at).collect();
    assert_ne!(before, after);
}
#[test]
fn dismissed_reminders_stay_dismissed_after_a_refresh() {
    let mut rt = runtime(reminders_scene); animate(&mut rt);
    let ids = banner_ids(&rt); assert!(ids.len() >= 2);
    let gone = ids[0].clone();
    click(&mut rt, &format!("dismiss-{gone}")); settle(&mut rt);
    assert!(!banner_ids(&rt).contains(&gone));
    assert_eq!(banner_ids(&rt).len(), (rt.state.reminders.len() - 1).min(overview::BANNER_ROWS));
    rt.state.refresh = true; settle(&mut rt);
    assert!(!banner_ids(&rt).contains(&gone));
    click(&mut rt, "dismiss-all"); animate(&mut rt);
    assert!(rt.find("reminders-banner").is_none());
}
#[test]
fn only_load_enables_the_hourly_reminder_timer() {
    let mut rt = runtime(reminders_scene); settle(&mut rt);
    assert!(!rt.state.hourly);
    let ticks = rt.state.reminders_gen;
    rt.frame(3700.0); settle(&mut rt);
    assert_eq!(rt.state.reminders_gen, ticks, "scenes never tick");
    let mut state = State::empty(); reminders_scene(&mut state); state.hourly = true;
    let mut rt = runtime_of(state); settle(&mut rt);
    let first = rt.state.reminders_gen; assert!(first >= 1, "computed at startup");
    rt.frame(3700.0); settle(&mut rt);
    assert!(rt.state.reminders_gen > first, "and hourly");
}

// ---- Import ----
fn pick(rt: &mut Runtime<State>, button: &str, path: Option<&Path>) {
    click(rt, button);
    let (asked, _) = rt.file_dialog().expect("a file dialog is requested");
    assert_eq!(asked, FileDialogKind::Open);
    rt.resolve_file_dialog(path.map(Path::to_path_buf)).unwrap();
    animate(rt);
}
fn confirm_import(rt: &mut Runtime<State>) { click(rt, "import-confirm"); settle(rt); }

#[test]
fn swiftdata_import_confirms_then_reports_counts_and_leaves_the_source_untouched() {
    let source = sqlite_fixture(SWIFT_SQL, "Finanzas.store");
    let before = std::fs::read(&source).unwrap();
    let mut rt = runtime(empty_scene); open_ajustes(&mut rt);
    pick(&mut rt, "import-swiftdata", Some(&source));
    assert!(rt.state.import_confirm.is_some());
    assert!(text_of(&rt, "import-confirm-source").is_some_and(|t| t.contains(source.to_str().unwrap())));
    assert!(text_of(&rt, "import-confirm-destination").is_some_and(|t| t.contains(&rt.state.db_label)));
    settle(&mut rt);
    assert!(rt.state.accounts.is_empty(), "nothing is imported before the user confirms");
    confirm_import(&mut rt);
    assert_eq!(text_of(&rt, "import-result").as_deref(), Some("Importación completa: 1 cuenta, 1 categoría, 1 comercio, 2 movimientos, 1 pago programado y 1 presupuesto."));
    assert_eq!(account_names(&rt), vec!["Banco importado".to_owned()]);
    assert!(rt.find(&format!("cuenta-{}", rt.state.accounts[0].account.id)).is_some());
    assert_eq!(std::fs::read(&source).unwrap(), before, "source bytes unchanged");
}
#[test]
fn actual_import_reports_counts_and_leaves_the_source_untouched() {
    let source = sqlite_fixture(ACTUAL_SQL, "db.sqlite");
    let before = std::fs::read(&source).unwrap();
    let mut rt = runtime(empty_scene); open_ajustes(&mut rt);
    pick(&mut rt, "import-actual", Some(&source));
    assert!(text_of(&rt, "import-confirm-source").is_some_and(|t| t.contains("db.sqlite")));
    confirm_import(&mut rt);
    assert_eq!(text_of(&rt, "import-result").as_deref(), Some("Importación completa: 2 cuentas, 2 categorías, 2 comercios, 6 movimientos, 3 pagos programados y 2 presupuestos."));
    assert_eq!(rt.state.accounts.len(), 2);
    assert_eq!(std::fs::read(&source).unwrap(), before);
}
#[test]
fn cancelling_the_dialog_or_the_confirmation_imports_nothing() {
    let source = sqlite_fixture(SWIFT_SQL, "Finanzas.store");
    let mut rt = runtime(empty_scene); open_ajustes(&mut rt);
    pick(&mut rt, "import-swiftdata", None);
    assert!(rt.state.import_confirm.is_none());
    pick(&mut rt, "import-swiftdata", Some(&source));
    click(&mut rt, "import-cancel"); settle(&mut rt);
    assert!(rt.state.import_confirm.is_none() && rt.state.accounts.is_empty());
    assert!(rt.find("import-result").is_none());
    pick(&mut rt, "import-swiftdata", Some(&source));
    press(&mut rt, Key::Escape, Default::default());
    assert!(rt.state.import_confirm.is_none());
}
#[test]
fn import_into_a_database_with_data_shows_an_error_and_changes_nothing() {
    let source = sqlite_fixture(SWIFT_SQL, "Finanzas.store");
    let before = std::fs::read(&source).unwrap();
    let mut rt = runtime(demo_light); open_ajustes(&mut rt);
    let names = account_names(&rt);
    pick(&mut rt, "import-swiftdata", Some(&source));
    confirm_import(&mut rt);
    assert!(text_of(&rt, "import-error").is_some_and(|t| t.contains("Ya tienes datos")));
    assert!(rt.find("import-result").is_none());
    assert_eq!(account_names(&rt), names);
    assert_eq!(std::fs::read(&source).unwrap(), before);
}
#[test]
fn import_of_a_file_that_is_not_a_database_shows_the_error() {
    let source = scratch_dir().join("notes.txt");
    std::fs::write(&source, "esto no es una base de datos").unwrap();
    let before = std::fs::read(&source).unwrap();
    for button in ["import-swiftdata", "import-actual"] {
        let mut rt = runtime(empty_scene); open_ajustes(&mut rt);
        pick(&mut rt, button, Some(&source));
        confirm_import(&mut rt);
        assert!(text_of(&rt, "import-error").is_some_and(|t| t.starts_with("No se pudo importar")), "{button}");
        assert!(rt.state.accounts.is_empty());
        assert_eq!(std::fs::read(&source).unwrap(), before);
    }
}
#[test]
fn a_running_import_blocks_other_mutations() {
    let source = sqlite_fixture(SWIFT_SQL, "Finanzas.store");
    let mut rt = runtime(empty_scene); open_ajustes(&mut rt);
    pick(&mut rt, "import-swiftdata", Some(&source));
    let worker = Arc::clone(&rt.state.worker);
    let guard = worker.lock().unwrap();
    click(&mut rt, "import-confirm"); animate(&mut rt);
    assert!(rt.state.mutation_active());
    assert!(has_text(&rt, "Importando"));
    click(&mut rt, "new-account");
    assert!(rt.state.account_sheet.is_none());
    click(&mut rt, "import-actual");
    assert!(rt.file_dialog().is_none(), "no second import while one runs");
    drop(guard); settle(&mut rt);
    assert!(!rt.state.mutation_active());
    assert_eq!(account_names(&rt), vec!["Banco importado".to_owned()]);
}

// ---- Scenes ----
#[test]
fn scenes_build_without_the_default_store() {
    for scene in [ajustes_light as fn(&mut State), ajustes_dark, nueva_cuenta_scene, reminders_scene] {
        let mut state = State::empty(); scene(&mut state);
        assert!(state.error.is_none() && !state.open_default && !state.hourly);
    }
    let mut state = State::empty(); nueva_cuenta_scene(&mut state); assert!(state.account_sheet.is_some());
    let mut state = State::empty(); ajustes_light(&mut state); assert_eq!(state.route, Route::Ajustes);
    let mut state = State::empty(); reminders_scene(&mut state); assert!(state.reminders.len() >= 2 && state.route == Route::Resumen);
}

#[test]
fn profiles_create_switch_rename_hide_and_delete() {
    let dir = scratch_dir();
    let mut state = State::load_home(Some(dir.join("finanzas.db")));
    assert_eq!((state.profile.as_str(), state.profiles.clone()), ("personal", vec!["personal".to_owned()]));
    state.dark = Some(true); state.route = Route::Presupuesto;
    state.profile_edit = Some((None, TextEdit::new("Mi Tienda")));
    state.submit_profile_edit();
    assert_eq!((state.profile.as_str(), state.dark, state.route), ("mi-tienda", Some(true), Route::Resumen));
    assert_eq!(state.profiles, ["personal", "mi-tienda"]);
    assert_eq!(state.db_path, Some(dir.join("mi-tienda.db")));
    assert!(state.open_default && state.refresh && state.profile_edit.is_none());
    data::open(&state.worker, state.db_path.clone()).unwrap();
    assert!(dir.join("mi-tienda.db").exists());

    state.profile_edit = Some((None, TextEdit::new("!!")));
    state.submit_profile_edit();
    assert_eq!(state.profile_error.as_deref(), Some("Escribe un nombre con letras o números"));
    state.profile_edit = Some((None, TextEdit::new("Mi tienda")));
    state.submit_profile_edit();
    assert_eq!(state.profile_error.as_deref(), Some("Ya existe un perfil con ese nombre"));
    state.profile_edit = None;

    state.toggle_profile_hidden("mi-tienda");
    assert_eq!(state.settings.hidden_profiles, ["mi-tienda"]);
    assert_eq!(state.visible_profiles(), ["personal", "mi-tienda"], "the active profile stays visible");
    state.profile_edit = Some((Some("mi-tienda".into()), TextEdit::new("Tienda")));
    state.submit_profile_edit();
    assert_eq!((state.profile.as_str(), state.route), ("tienda", Route::Ajustes));
    assert_eq!(state.profiles, ["personal", "tienda"]);
    assert_eq!(state.settings.hidden_profiles, ["tienda"], "hidden follows the rename");
    assert!(dir.join("tienda.db").exists() && !dir.join("mi-tienda.db").exists());
    state.switch_to("personal".into());
    assert_eq!(state.visible_profiles(), ["personal"]);
    assert_eq!(settings::load(&dir.join("settings.json")).unwrap().hidden_profiles, ["tienda"]);

    state.switch_to("tienda".into());
    state.request_profile_delete("personal");
    assert!(state.profile_delete.is_none(), "personal cannot be deleted");
    state.request_profile_delete("tienda");
    assert_eq!(state.profile_delete.as_deref(), Some("tienda"));
    state.confirm_profile_delete();
    assert_eq!((state.profile.as_str(), state.profiles.clone()), ("personal", vec!["personal".to_owned()]));
    assert!(state.profile_delete.is_none() && state.settings.hidden_profiles.is_empty());
    assert!(!dir.join("tienda.db").exists() && !dir.join("finanzas.db").exists(), "only the deleted profile's file is removed");
}

#[test]
fn theme_choice_is_saved_and_restored() {
    let dir = scratch_dir();
    let mut state = State::load_home(Some(dir.join("finanzas.db")));
    assert_eq!(state.dark, None, "automatic by default");
    state.set_theme(Some(true));
    assert_eq!(state.dark, Some(true));
    assert_eq!(State::load_home(Some(dir.join("finanzas.db"))).dark, Some(true));
    state.set_theme(Some(false));
    assert_eq!(State::load_home(Some(dir.join("finanzas.db"))).dark, Some(false));
    state.set_theme(None);
    assert_eq!(State::load_home(Some(dir.join("finanzas.db"))).dark, None);
}

#[test]
fn agents_turn_connected_from_fresh_heartbeats() {
    use finanzas_core::agents::{self, Agent};
    let dir = scratch_dir();
    let mut state = State::load_home(Some(dir.join("finanzas.db")));
    assert!(!state.check_agents());
    assert_eq!(state.agents_connected, [false; 3]);
    agents::beat(&agents::heartbeat_dir(&dir.join("finanzas.db")), Agent::Codex).unwrap();
    assert!(state.check_agents(), "a change re-renders");
    assert_eq!(state.agents_connected, [false, true, false]);
    assert!(!state.check_agents());
    assert!(State::mcp_command().file_name().unwrap().to_string_lossy().starts_with("finanzas-mcp"));
}

#[test]
fn form_amounts_do_math_and_blank_is_zero() {
    assert!(entry_helpers::valid_sheet("", Some(1), "13/11/2026"), "blank amount is 0");
    assert!(entry_helpers::valid_sheet("550*2", Some(1), "13/11/2026"));
    assert!(!entry_helpers::valid_sheet("5/0", Some(1), "13/11/2026"));
    let mut sheet = account_sheet::AccountSheet::new();
    sheet.name.replace_text("Ahorros");
    sheet.opening.replace_text("-100*2");
    assert_eq!(sheet.draft(NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()).map(|d| d.opening), Some(Some(-20_000)));
}
