mod data;
mod entry_helpers;
mod ledger;
mod metrics;
mod overview;

use std::sync::{Arc, Mutex};
use chrono::Local;
use finanzas_core::{domain::{money, month}, models::Id};
use rune::{AppSpec, Color, Cx, El, Latest, TextEdit, Tx, Waker, Key, KeyEvent, col, el, row, text, tok::type_};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route { Resumen, Presupuesto, Proximos, Todas, Cuenta(Id) }

type LedgerKey = (Option<Id>, String, Option<i64>);
type LedgerResult = (LedgerKey, Result<data::Ledger, String>);

struct State {
    route: Route,
    accounts: Vec<data::AccountBalance>,
    dark: Option<bool>,
    error: Option<String>,
    month: i32,
    today: i32,
    chart_months: u32,
    overview: Option<finanzas_core::service::Overview>,
    version: Option<i64>,
    worker: data::WorkerStore,
    latest: Latest<Result<Option<data::Snapshot>, String>>,
    refresh: bool,
    next_poll: f64,
    search: TextEdit,
    ledger: Option<data::Ledger>,
    ledger_latest: Latest<LedgerResult>,
    ledger_key: Option<LedgerKey>,
}

impl State {
    fn empty() -> Self {
        let today = month::month_of(Local::now().date_naive());
        Self { route: Route::Resumen, accounts: Vec::new(), dark: None, error: None,
            month: today, today, chart_months: 6, overview: None, version: None,
            worker: Arc::new(Mutex::new(None)), latest: Latest::default(), refresh: true, next_poll: 0.0,
            search: TextEdit::new(""), ledger: None, ledger_latest: Latest::default().debounce(std::time::Duration::from_millis(150)), ledger_key: None }
    }
    fn load() -> Self { Self::empty() }
    fn total(&self) -> i64 { self.accounts.iter().filter(|a| !a.account.closed).map(|a| a.balance).sum() }
    fn set_month(&mut self, month: i32) { self.month = month; self.refresh = true; }
    fn accept(&mut self, snapshot: data::Snapshot) {
        self.version = Some(snapshot.version); self.accounts = snapshot.accounts; self.overview = Some(snapshot.overview);
    }
}

fn scene(s: &mut State, seed: bool, dark: bool) {
    let result = std::thread::spawn(move || data::scene(seed)).join();
    *s = State::empty(); s.month = 202611; s.today = 202611; s.dark = Some(dark); s.refresh = false;
    match result {
        Ok(Ok((worker, snapshot))) => { s.worker = worker; s.accept(snapshot); },
        Ok(Err(error)) => s.error = Some(error),
        Err(_) => s.error = Some("No se pudo preparar la escena".into()),
    }
}
fn demo_light(s: &mut State) { scene(s, true, false); }
fn demo_dark(s: &mut State) { scene(s, true, true); }
fn empty_scene(s: &mut State) { scene(s, false, false); }
fn movements_scene(s: &mut State, dark: bool) {
    scene(s, true, dark);
    s.route = Route::Todas;
    let worker = Arc::clone(&s.worker);
    match std::thread::spawn(move || data::ledger(worker, None, "")).join() {
        Ok(Ok(ledger)) => { s.ledger = Some(ledger); s.ledger_key = Some((None, String::new(), s.version)); }
        Ok(Err(error)) => s.error = Some(error),
        Err(_) => s.error = Some("No se pudo preparar la escena".into()),
    }
}
fn movements_light(s: &mut State) { movements_scene(s, false); }
fn movements_dark(s: &mut State) { movements_scene(s, true); }

fn setup(s: &mut State, waker: Waker) { s.latest.set_waker(waker.clone()); s.ledger_latest.set_waker(waker); }
fn poll(s: &mut State, now: f64, tx: &mut Tx) -> bool {
    let mut changed = false;
    if let Some(result) = s.latest.take() {
        match result {
            Ok(Some(snapshot)) => { s.accept(snapshot); s.error = None; tx.instant(); changed = true; }
            Ok(None) => {}
            Err(error) => { s.error = Some(error); tx.instant(); changed = true; }
        }
    }
    if s.refresh || (now >= s.next_poll && !s.latest.is_pending()) {
        let force = s.refresh;
        s.refresh = false;
        s.next_poll = now + 2.0;
        let worker = Arc::clone(&s.worker);
        let input = (s.month, s.chart_months, s.version, force);
        s.latest.request(input, move |(month, chart, version, force)| data::fetch(worker, month, chart, version, force));
    }
    if let Some((key, result)) = s.ledger_latest.take() {
        match result { Ok(ledger) => { s.ledger = Some(ledger); s.ledger_key = Some(key); tx.instant(); changed = true; }, Err(error) => { s.error = Some(error); changed = true; } }
    }
    if s.overview.is_some() && matches!(s.route, Route::Todas | Route::Cuenta(_)) {
        let account = match s.route { Route::Cuenta(id) => Some(id), _ => None };
        let key = (account, s.search.text.clone(), s.version);
        let worker = Arc::clone(&s.worker);
        if s.ledger_key.as_ref() != Some(&key) && s.ledger.is_some() { s.ledger = None; changed = true; }
        s.ledger_latest.request_if_changed(&key, move |key| { let result = data::ledger(worker, key.0, &key.1); (key, result) });
    }
    tx.wake_at(if s.next_poll <= now { now + 0.25 } else { s.next_poll });
    changed
}
fn on_key(s: &mut State, ev: &KeyEvent, _: &mut Tx) -> bool {
    if s.route != Route::Resumen || !(ev.mods.cmd || ev.mods.ctrl) || ev.mods.alt || ev.mods.shift { return false; }
    match ev.key {
        Key::Left => { s.set_month(month::previous(s.month)); true }
        Key::Right => { s.set_month(month::next(s.month)); true }
        _ => false,
    }
}

// These are the adaptive colors in the original app's FinanceTheme.
#[derive(Clone, Copy)]
struct Palette { canvas: Color, card: Color, border: Color, petroleum: Color, emerald: Color, coral: Color, amber: Color, ink: Color, muted: Color, selected: Color }
fn color(hex: &str) -> Color { rune::color::hex(hex).unwrap_or(Color::BLACK) }
fn palette(dark: bool) -> Palette {
    if dark {
        Palette { canvas: color("#151D27"), card: color("#1E2936"), border: color("#354453"), petroleum: color("#67C6DF"), emerald: color("#6DDBAD"), coral: color("#FF9A8F"), amber: color("#F5C46B"), ink: color("#F3F7F9"), muted: color("#A9BAC6"), selected: color("#304657") }
    } else {
        Palette { canvas: color("#F4F7FA"), card: color("#FFFFFF"), border: color("#DEE7ED"), petroleum: color("#155E75"), emerald: color("#147D59"), coral: color("#BE493D"), amber: color("#986000"), ink: color("#202F3A"), muted: color("#6C7E8A"), selected: color("#E2F0F4") }
    }
}

fn nav(label: &str, key: String, route: Route, selected: bool, balance: Option<i64>, p: Palette) -> El {
    let mut content = row().w_full().h(38.0).px(12.0).center_y().justify_between().gap(8.0)
        .child(text(label.to_owned()).font(type_::BODY).color(if selected { p.petroleum } else { p.ink }));
    if let Some(amount) = balance {
        content = content.child(text(money::format(amount)).font(type_::BODY).color(if amount < 0 { p.coral } else { p.muted }));
    }
    content.key(key).role("button").label(label).focusable(true).radius(8.0)
        .fill(if selected { p.selected } else { Color::TRANSPARENT })
        .hover(p.selected).on_press(move |s: &mut State| s.route = route)
}

fn view(s: &State, cx: &mut Cx<State>) -> El {
    let p = palette(s.dark.unwrap_or_else(|| cx.system_dark()));
    if let Some(error) = &s.error {
        return col().w_full().h_full().center().gap(12.0).fill(p.canvas)
            .child(text("No se pudo abrir la base de datos").font(type_::TITLE).color(p.ink))
            .child(text(error.clone()).font(type_::BODY).color(p.muted));
    }
    let mut sidebar = col().w(280.0).h_full().padding(20.0).gap(3.0).fill(p.card)
        .child(row().gap(12.0).center_y().pb(24.0)
            .child(el().size(42.0, 42.0).radius(12.0).fill(color("#155E75")).center().child(text("F").font(type_::TITLE).color(Color::WHITE)))
            .child(col().gap(3.0).child(text("Finanzas").font(type_::TITLE).color(p.ink)).child(text("Tu dinero, en orden").font(type_::BODY).color(p.muted))))
        .child(nav("Resumen", "resumen".into(), Route::Resumen, s.route == Route::Resumen, None, p))
        .child(nav("Presupuesto", "presupuesto".into(), Route::Presupuesto, s.route == Route::Presupuesto, None, p))
        .child(nav("Próximos pagos", "proximos".into(), Route::Proximos, s.route == Route::Proximos, None, p))
        .child(text("CUENTAS").font(type_::SECTION).color(p.muted).pt(24.0).pb(7.0).px(12.0))
        .child(nav("Todas", "todas".into(), Route::Todas, s.route == Route::Todas, Some(s.total()), p));
    for a in s.accounts.iter().filter(|a| !a.account.closed) {
        let route = Route::Cuenta(a.account.id);
        sidebar = sidebar.child(nav(&a.account.name, format!("cuenta-{}", a.account.id), route, s.route == route, Some(a.balance), p));
    }
    sidebar = sidebar.child(text("CERRADAS").font(type_::SECTION).color(p.muted).pt(24.0).pb(7.0).px(12.0));
    for a in s.accounts.iter().filter(|a| a.account.closed) {
        let route = Route::Cuenta(a.account.id);
        sidebar = sidebar.child(nav(&a.account.name, format!("cuenta-{}", a.account.id), route, s.route == route, Some(a.balance), p));
    }
    let title = match s.route {
        Route::Resumen => "Resumen", Route::Presupuesto => "Presupuesto", Route::Proximos => "Próximos pagos", Route::Todas => "Todas las cuentas",
        Route::Cuenta(id) => s.accounts.iter().find(|a| a.account.id == id).map_or("Cuenta", |a| a.account.name.as_str()),
    };
    let detail = if s.route == Route::Resumen { overview::render(s, p) } else if matches!(s.route, Route::Todas | Route::Cuenta(_)) { ledger::render(s, p, title) } else {
        col().grow(1.0).h_full().padding(48.0).gap(12.0)
            .child(text(title.to_owned()).font(type_::TITLE).color(p.ink))
            .child(text("Tu espacio de finanzas").font(type_::BODY).color(p.muted))
    };
    row().w_full().h_full().fill(p.canvas)
        .child(sidebar)
        .child(el().w(1.0).h_full().fill(p.border))
        .child(detail)
}

fn main() {
    rune::run(AppSpec::<State> {
        scenes: &[("demo-light", demo_light), ("demo-dark", demo_dark), ("empty", empty_scene), ("movimientos-light", movements_light), ("movimientos-dark", movements_dark)],
        setup: Some(setup), poll: Some(poll), on_key: Some(on_key),
        ..AppSpec::new("Finanzas", (1200.0, 800.0), State::load, view)
    });
}

fn chart_fraction(value: i64, maximum: i64) -> f32 {
    if maximum <= 0 { return 0.0; }
    (value as f32 / maximum as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    #[test]
    fn sidebar_navigation_changes_route() {
        let mut rt = Runtime::new(State::empty(), view, (1200.0, 800.0), Env::new(false, 1.0));
        rt.frame(0.0);
        let (x, y) = rt.find("presupuesto").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
        assert_eq!(rt.state.route, Route::Presupuesto);
    }
    #[test]
    fn previous_month_changes_the_displayed_header() {
        let mut state = State::empty();
        state.month = 202611;
        let mut rt = Runtime::new(state, view, (1200.0, 800.0), Env::new(false, 1.0));
        rt.frame(0.0);
        let title = |rt: &Runtime<State>| rt.elements().into_iter().find(|e| e.key.as_deref() == Some("month-title")).and_then(|e| e.text).unwrap();
        assert_eq!(title(&rt), "Noviembre de 2026");
        let (x, y) = rt.find("previous-month").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
        assert_eq!(title(&rt), "Octubre de 2026");
    }
    #[test]
    fn all_accounts_shows_the_movements_screen() {
        let mut rt = Runtime::new(State::empty(), view, (1200.0, 800.0), Env::new(false, 1.0));
        rt.frame(0.0);
        let (x, y) = rt.find("todas").unwrap();
        rt.pointer_down(x, y); rt.pointer_up(x, y); rt.frame(0.0);
        assert!(rt.find("ledger-title").is_some());
    }
    #[test]
    fn chart_range_marks_the_selected_segment() {
        let mut state = State::empty();
        demo_light(&mut state);
        let mut rt = Runtime::new(state, view, (1200.0, 800.0), Env::new(false, 1.0));
        rt.frame(0.0);
        let fill = |rt: &Runtime<State>, key: &str| rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap().fill;
        assert_ne!(fill(&rt, "chart-6"), fill(&rt, "chart-12"));
        let old = fill(&rt, "chart-12");
        let (x, y) = rt.find("chart-12").unwrap();
        rt.pointer_down(x, y); rt.pointer_up(x, y);
        for _ in 0..60 { rt.frame(1.0 / 60.0); }
        assert_ne!(fill(&rt, "chart-12"), old);
        assert_eq!(rt.state.chart_months, 12);
    }
    #[test]
    fn chart_scales_values_and_handles_empty_data() {
        assert_eq!(chart_fraction(50, 100), 0.5);
        assert_eq!(chart_fraction(0, 0), 0.0);
        assert_eq!(chart_fraction(-1, 100), 0.0);
        assert_eq!(chart_fraction(200, 100), 1.0);
    }
}
