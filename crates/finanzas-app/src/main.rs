use chrono::NaiveDate;
use finanzas_core::{domain::money, models::{Account, Entry, Id}, store::Store};
use rune::{AppSpec, Color, Cx, El, col, el, row, text, tok::type_};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route { Resumen, Presupuesto, Proximos, Todas, Cuenta(Id) }

struct AccountBalance { account: Account, balance: i64 }

struct State {
    route: Route,
    accounts: Vec<AccountBalance>,
    dark: Option<bool>,
    error: Option<String>,
}

impl State {
    fn empty() -> Self { Self { route: Route::Resumen, accounts: Vec::new(), dark: None, error: None } }

    fn load() -> Self {
        // Rune constructs the initial state before applying a scene. Avoid opening a real
        // database for deterministic headless scenes.
        if std::env::args_os().any(|arg| arg == "--scene") { return Self::empty(); }
        let result = Store::default_path().ok_or_else(|| "No se pudo determinar la ruta de la base de datos".to_owned())
            .and_then(|path| Store::open(path).map_err(|e| e.to_string()))
            .and_then(|store| Self::from_store(&store));
        result.unwrap_or_else(|error| Self { error: Some(error), ..Self::empty() })
    }

    fn from_store(store: &Store) -> Result<Self, String> {
        let accounts = store.list_accounts().map_err(|e| e.to_string())?;
        let balances = store.account_balances().map_err(|e| e.to_string())?;
        Ok(Self { accounts: accounts.into_iter().map(|account| {
            let balance = balances.get(&account.id).copied().unwrap_or(0);
            AccountBalance { account, balance }
        }).collect(), ..Self::empty() })
    }

    fn total(&self) -> i64 { self.accounts.iter().filter(|a| !a.account.closed).map(|a| a.balance).sum() }
}

fn demo(dark: bool) -> State {
    let result = (|| -> Result<State, String> {
        let store = Store::open_in_memory().map_err(|e| e.to_string())?;
        let date = NaiveDate::from_ymd_opt(2026, 11, 10).ok_or("Fecha inválida")?;
        for (name, amount, closed, order) in [("Cuenta principal", 257_450, false, 0.0), ("Tarjeta", -49_000, false, 1.0), ("Ahorros", 880_000, false, 2.0), ("Cuenta anterior", 12_500, true, 3.0)] {
            let id = store.insert_account(&Account { id: 0, name: name.into(), off_budget: false, closed, sort_order: order }).map_err(|e| e.to_string())?;
            store.insert_entry(&Entry { id: 0, date, amount, notes: String::new(), cleared: true, account_id: Some(id), category_id: None, payee_id: None, schedule_id: None }).map_err(|e| e.to_string())?;
        }
        let mut state = State::from_store(&store)?;
        state.dark = Some(dark);
        Ok(state)
    })();
    result.unwrap_or_else(|error| State { dark: Some(dark), error: Some(error), ..State::empty() })
}
fn demo_light(s: &mut State) { *s = demo(false); }
fn demo_dark(s: &mut State) { *s = demo(true); }
fn empty_scene(s: &mut State) { *s = State::empty(); s.dark = Some(false); }

// These are the adaptive colors in the original app's FinanceTheme.
#[derive(Clone, Copy)]
struct Palette { canvas: Color, card: Color, border: Color, petroleum: Color, coral: Color, ink: Color, muted: Color, selected: Color }
fn color(hex: &str) -> Color { rune::color::hex(hex).unwrap_or(Color::BLACK) }
fn palette(dark: bool) -> Palette {
    if dark {
        Palette { canvas: color("#151D27"), card: color("#1E2936"), border: color("#354453"), petroleum: color("#67C6DF"), coral: color("#FF9A8F"), ink: color("#F3F7F9"), muted: color("#A9BAC6"), selected: color("#304657") }
    } else {
        Palette { canvas: color("#F4F7FA"), card: color("#FFFFFF"), border: color("#DEE7ED"), petroleum: color("#155E75"), coral: color("#BE493D"), ink: color("#202F3A"), muted: color("#6C7E8A"), selected: color("#E2F0F4") }
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
    row().w_full().h_full().fill(p.canvas)
        .child(sidebar)
        .child(el().w(1.0).h_full().fill(p.border))
        .child(col().grow(1.0).h_full().padding(48.0).gap(12.0)
            .child(text(title.to_owned()).font(type_::TITLE).color(p.ink))
            .child(text("Tu espacio de finanzas").font(type_::BODY).color(p.muted)))
}

fn main() {
    rune::run(AppSpec::<State> {
        scenes: &[("demo-light", demo_light), ("demo-dark", demo_dark), ("empty", empty_scene)],
        ..AppSpec::new("Finanzas", (1200.0, 800.0), State::load, view)
    });
}

#[cfg(test)]
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
    fn chart_scales_values_and_handles_empty_data() {
        assert_eq!(chart_fraction(50, 100), 0.5);
        assert_eq!(chart_fraction(0, 0), 0.0);
        assert_eq!(chart_fraction(-1, 100), 0.0);
        assert_eq!(chart_fraction(200, 100), 1.0);
    }
}
