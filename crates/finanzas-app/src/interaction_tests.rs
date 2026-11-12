//! Resumen interactions: everything on the dashboard leads somewhere or acts. In-memory stores only.
use super::*;
use rune::{Env, Runtime};

fn dashboard() -> Runtime<State> {
    let mut state = State::empty(); demo_light(&mut state);
    let mut rt = Runtime::new(state, view, (1440.0, 1800.0), Env::new(false, 1.0));
    rt.on_key = Some(on_key); rt.poll = Some(poll); rt.frame(0.0); settle(&mut rt); rt
}
fn settle(rt: &mut Runtime<State>) { for _ in 0..150 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); } }
fn click(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("missing {key}"));
    rt.pointer_down(x, y); rt.pointer_up(x, y); rt.frame(0.0);
}
fn keys(rt: &Runtime<State>, prefix: &str) -> Vec<String> {
    rt.elements().into_iter().filter_map(|e| e.key).filter(|k| k.starts_with(prefix)).collect()
}
fn next_date(rt: &Runtime<State>, name: &str) -> Option<NaiveDate> {
    rt.state.proximos.as_ref()?.schedules.iter().find(|v| v.schedule.name == name)?.next_date
}

#[test]
fn ticking_a_payment_on_resumen_registers_it() {
    let mut rt = dashboard();
    let before = next_date(&rt, "Netflix").expect("Netflix scheduled");
    let id = rt.state.proximos.as_ref().unwrap().schedules.iter().find(|v| v.schedule.name == "Netflix").unwrap().schedule.id;
    click(&mut rt, &format!("pay-{id}-{before}"));
    settle(&mut rt);
    assert_eq!(rt.state.route, Route::Resumen, "stays on the dashboard");
    assert!(next_date(&rt, "Netflix").is_some_and(|d| d > before), "the occurrence moved on after registering");
}

#[test]
fn a_day_chip_filters_the_payment_list_and_toggles_off() {
    let mut rt = dashboard();
    let all = keys(&rt, "payment-").into_iter().filter(|k| !k.starts_with("payment-day")).count();
    assert!(all >= 2);
    let day = NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
    click(&mut rt, &format!("payment-day-{day}")); settle(&mut rt);
    let shown: Vec<_> = keys(&rt, "payment-").into_iter().filter(|k| !k.starts_with("payment-day")).collect();
    assert!(!shown.is_empty() && shown.iter().all(|k| k.ends_with(&day.to_string())), "{shown:?}");
    click(&mut rt, &format!("payment-day-{day}")); settle(&mut rt);
    assert_eq!(keys(&rt, "payment-").into_iter().filter(|k| !k.starts_with("payment-day")).count(), all);
}

#[test]
fn category_and_recent_rows_open_filtered_movements() {
    let mut rt = dashboard();
    let first = rt.state.overview.as_ref().unwrap().spending[0].clone_parts();
    click(&mut rt, &format!("spend-{}", first.0));
    assert_eq!(rt.state.route, Route::Todas);
    assert_eq!(rt.state.search.text, first.1);
    // The sidebar's "Todas" shows everything again.
    click(&mut rt, "todas");
    assert_eq!(rt.state.search.text, "");
}

#[test]
fn chart_columns_and_legend_navigate() {
    let mut rt = dashboard();
    click(&mut rt, "chart-month-202609");
    assert_eq!(rt.state.month, 202609);
    let mut rt = dashboard();
    click(&mut rt, "legend-pending");
    assert_eq!(rt.state.route, Route::Proximos);
}

#[test]
fn reminders_banner_collapses_and_expands() {
    let mut state = State::empty(); reminders_scene(&mut state);
    let mut rt = Runtime::new(state, view, (1440.0, 1800.0), Env::new(false, 1.0));
    rt.poll = Some(poll); rt.frame(0.0); settle(&mut rt);
    assert!(!keys(&rt, "reminder-").is_empty());
    click(&mut rt, "reminders-collapse"); settle(&mut rt);
    assert!(keys(&rt, "reminder-").is_empty() && rt.find("reminders-banner").is_some());
    click(&mut rt, "reminders-collapse"); settle(&mut rt);
    assert!(!keys(&rt, "reminder-").is_empty());
}

trait Parts { fn clone_parts(&self) -> (Id, String); }
impl Parts for finanzas_core::service::CategoryProgress { fn clone_parts(&self) -> (Id, String) { (self.category_id, self.name.clone()) } }
