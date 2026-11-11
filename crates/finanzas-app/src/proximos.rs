use super::*;
use finanzas_core::service::{ScheduleView, UpcomingView};
use overview::{card, label, muted};
use rune::ScrollSpec;

fn day(date: chrono::NaiveDate) -> String { date.format("%d.%m.%Y").to_string() }
fn amount_color(value: i64, p: Palette) -> Color { if value < 0 { p.coral } else { p.emerald } }

fn metric(title: &str, value: usize, key: &str, color: Color, p: Palette) -> El {
    card(p).child(muted(title, p)).child(text(value.to_string()).key(key).font(type_::TITLE).color(color))
}
fn pill(name: &str, color: Color) -> El {
    el().px(8.0).py(2.0).radius(8.0).fill(color.alpha(0.16)).child(text(name.to_owned()).font(type_::BODY).color(color))
}
fn action(name: &str, key: String, enabled: bool, press: impl Fn(&mut State) + 'static, p: Palette, dark: bool) -> El {
    let ink = if dark { p.canvas } else { Color::WHITE };
    let base = el().key(key).role("button").label(name).focusable(true).px(12.0).py(7.0).radius(8.0).child(text(name.to_owned()).font(type_::BODY).color(ink));
    if enabled { base.fill(p.petroleum).on_press(press) } else { base.fill(p.border).disabled(true) }
}
fn quiet(name: &str, key: String, enabled: bool, press: impl Fn(&mut State) + 'static, p: Palette) -> El {
    let base = sheet::button(name, &key, p);
    if enabled { base.on_press(press) } else { base.disabled(true) }
}
fn separator(p: Palette) -> El { el().w_full().h(1.0).fill(p.border) }
fn section(title: &str, p: Palette) -> El {
    col().w_full().gap(0.0).radius(18.0).fill(p.card).border(1.0, p.border).py(6.0).child(text(title.to_owned()).font(type_::BODY).color(p.muted).px(16.0).py(8.0))
}

fn details(view: Option<&ScheduleView>, categories: &[finanzas_core::models::Category]) -> String {
    let Some(v) = view else { return String::new() };
    let category = v.schedule.category_id.and_then(|id| categories.iter().find(|c| c.id == id)).map(|c| c.name.clone());
    let recurrence = v.schedule.recurrence.as_ref().map(|r| r.describe());
    [v.payee.clone(), v.account.clone(), category, recurrence].into_iter().flatten().collect::<Vec<_>>().join(" · ")
}

fn item_row(s: &State, item: &UpcomingView, view: Option<&ScheduleView>, enabled: bool, p: Palette, dark: bool) -> El {
    let id = item.schedule_id;
    // Only the pending occurrence can be acted on; later ones follow once it moves.
    let actionable = view.is_some_and(|v| v.next_date == Some(item.date));
    let tint = if item.overdue { p.amber } else { p.petroleum };
    let mut badge = col().w(112.0).gap(3.0).padding(10.0).radius(10.0).fill(tint.alpha(0.12))
        .child(text(day(item.date)).font(type_::BODY).color(if item.overdue { p.amber } else { p.muted }));
    if item.overdue { badge = badge.child(text("Vencido").font(type_::BODY).color(p.amber)); }
    let mut title = row().gap(8.0).center_y().child(label(item.name.clone(), p).truncate());
    if item.auto_post { title = title.child(pill("Automático", p.petroleum)); }
    let mut amounts = col().w(150.0).gap(3.0).items_end()
        .child(text(money::format(item.amount)).font(type_::BODY).color(amount_color(item.amount, p)));
    if let Some(balance) = item.projected_balance {
        amounts = amounts.child(text(format!("Saldo: {}", money::format(balance))).font(type_::BODY).color(if balance < 0 { p.coral } else { p.muted }));
    }
    let mut r = row().w_full().gap(14.0).px(16.0).py(10.0).center_y().child(badge)
        .child(col().grow(1.0).min_w(0.0).gap(4.0).child(title).child(muted(details(view, &s.categories), p).truncate())).child(amounts);
    if actionable {
        let enabled = enabled && !s.mutation_active();
        r = r.child(row().gap(6.0)
            .child(action("Registrar", format!("post-{id}"), enabled, move |s: &mut State| s.schedule_action(id, true), p, dark))
            .child(quiet("Saltar", format!("skip-{id}"), enabled, move |s: &mut State| s.schedule_action(id, false), p)));
    } else {
        r = r.child(el().w(142.0)); // keeps the amounts aligned with the rows that have buttons
    }
    r.key(format!("upcoming-{id}-{}", item.date))
}

fn schedule_row(s: &State, v: &ScheduleView, enabled: bool, p: Palette) -> El {
    let id = v.schedule.id;
    let selected = s.schedule_selected == Some(id);
    let enabled = enabled && !s.mutation_active();
    row().key(format!("schedule-{id}")).w_full().gap(12.0).px(16.0).py(10.0).center_y()
        .fill(if selected { p.selected } else { Color::TRANSPARENT }).hover(p.selected)
        .on_press(move |s: &mut State| if !s.mutation_active() { s.schedule_selected = Some(id) })
        .on_double_press(move |s: &mut State| s.open_schedule_sheet(Some(id)))
        .on_secondary_press(move |s: &mut State, press, tx| {
            if s.mutation_active() { return; }
            s.schedule_selected = Some(id);
            let menu = rune::PopupMenu::new().item(rune::PopupItem::new("edit", "Editar")).item(rune::PopupItem::new("delete", "Eliminar"));
            tx.popup_menu(menu, press, move |s: &mut State, picked, _| match picked { Some("edit") => s.open_schedule_sheet(Some(id)), Some("delete") => s.request_schedule_delete(id), _ => {} });
        })
        .child(col().grow(1.0).min_w(0.0).gap(4.0).child(label(v.schedule.name.clone(), p).truncate())
            .child(muted(if v.schedule.completed { "Terminado".to_owned() } else { v.label.clone() }, p).truncate()))
        .child(text(money::format(v.schedule.amount)).font(type_::BODY).color(amount_color(v.schedule.amount, p)).w(120.0).shrink().truncate())
        .child(quiet("Editar", format!("edit-{id}"), enabled, move |s: &mut State| s.open_schedule_sheet(Some(id)), p))
        .child(quiet("Eliminar", format!("delete-{id}"), enabled, move |s: &mut State| s.request_schedule_delete(id), p))
}

pub fn render(s: &State, p: Palette, dark: bool) -> El {
    let fresh = s.proximos_fresh();
    let mut content = col().w_full().padding(32.0).gap(16.0)
        .child(row().w_full().center_y().justify_between()
            .child(col().gap(5.0).child(text("Tus próximos compromisos").key("proximos-title").font(type_::TITLE).color(p.ink))
                .child(muted("Anticipa tus pagos y cuida tu saldo", p)))
            .child(action("Nuevo pago programado", "new-schedule".into(), !s.mutation_active(), |s: &mut State| s.open_schedule_sheet(None), p, dark)));
    if let Some(error) = &s.upcoming_error { content = content.child(text(error.clone()).key("upcoming-error").font(type_::BODY).color(p.coral)); }
    let Some(data) = &s.proximos else { return col().grow(1.0).h_full().min_h(0.0).child(content.child(muted("Cargando…", p))); };
    let (overdue, coming): (Vec<_>, Vec<_>) = data.items.iter().partition(|i| i.overdue);
    let view_of = |id| data.schedules.iter().find(|v| v.schedule.id == id);
    content = content.child(row().w_full().gap(12.0)
        .child(metric("Vencidos", overdue.len(), "metric-overdue", if overdue.is_empty() { p.emerald } else { p.amber }, p))
        .child(metric("Próximos 30 días", coming.len(), "metric-coming", p.ink, p))
        .child(metric("Pagos programados", data.schedules.len(), "metric-schedules", p.ink, p)));
    if !overdue.is_empty() {
        let mut card = section("Vencidos", p).key("section-overdue");
        for item in overdue { card = card.child(separator(p)).child(item_row(s, item, view_of(item.schedule_id), fresh, p, dark)); }
        content = content.child(card);
    }
    let mut card = section("Próximos 30 días", p).key("section-coming");
    if coming.is_empty() { card = card.child(separator(p)).child(muted("Nada programado", p).px(16.0).py(10.0)); }
    for item in coming { card = card.child(separator(p)).child(item_row(s, item, view_of(item.schedule_id), fresh, p, dark)); }
    content = content.child(card);
    let mut card = section("Pagos programados", p).key("section-schedules");
    if data.schedules.is_empty() { card = card.child(separator(p)).child(muted("Crea uno con «Nuevo pago programado» (⌘N)", p).px(16.0).py(10.0)); }
    for v in &data.schedules { card = card.child(separator(p)).child(schedule_row(s, v, fresh, p)); }
    content = content.child(card);
    col().grow(1.0).h_full().min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).child(content)
}

