use super::*;
use finanzas_core::service::{Overview, UpcomingView};
use theme::{badge, card_title, dot, money_big};

pub fn label(value: impl Into<String>, p: Palette) -> El { text(value.into()).font(type_::BODY).color(p.ink) }
pub fn muted(value: impl Into<String>, p: Palette) -> El { text(value.into()).font(type_::BODY).color(p.muted) }
pub fn card(p: Palette) -> El { col().grow(1.0).padding(20.0).gap(12.0).radius(18.0).fill(p.card).border(1.0, p.border) }
pub fn button(name: &'static str, key: &'static str, action: fn(&mut State), p: Palette) -> El {
    el().key(key).role("button").label(name).focusable(true).padding(8.0).radius(8.0).fill(p.card).hover(p.selected)
        .on_press(action).child(label(name, p))
}
fn icon_button(i: &'static rune::Icon, name: &'static str, key: &'static str, action: fn(&mut State), p: Palette) -> El {
    el().key(key).role("button").label(name).focusable(true).size(32.0, 32.0).radius(10.0).center().border(1.0, p.border).hover(p.selected)
        .on_press(action).child(icon(i, 16.0).color(p.ink))
}
fn primary(i: &'static rune::Icon, name: &'static str, key: &'static str, action: fn(&mut State), p: Palette) -> El {
    row().key(key).role("button").label(name).focusable(true).gap(6.0).px(14.0).py(8.0).radius(10.0).center_y().fill(p.petroleum).hover(p.petroleum.alpha(0.85))
        .on_press(action).child(icon(i, 14.0).color(p.card)).child(text(name.to_owned()).font(type_::LABEL).color(p.card))
}
fn secondary(i: &'static rune::Icon, name: &'static str, key: &'static str, action: fn(&mut State), p: Palette) -> El {
    row().key(key).role("button").label(name).focusable(true).gap(6.0).px(14.0).py(8.0).radius(10.0).center_y().border(1.0, p.border).hover(p.selected)
        .on_press(action).child(icon(i, 14.0).color(p.ink)).child(text(name.to_owned()).font(type_::LABEL).color(p.ink))
}
/// Card heading with an optional subtitle and a flat illustration on the right.
fn heading(title: &str, note: Option<String>, art: &'static rune::Icon, p: Palette) -> El {
    let mut left = col().gap(3.0).child(card_title(title, p));
    if let Some(note) = note { left = left.child(muted(note, p)); }
    row().w_full().justify_between().child(left).child(icon(art, 52.0))
}
fn bar(value: i64, total: i64, color: Color, p: Palette) -> El {
    let f = chart_fraction(value, total);
    row().w_full().h(7.0).radius(3.5).fill(p.border)
        .child(el().grow(f).h(7.0).radius(3.5).fill(color))
        .child(el().grow(1.0 - f).h(7.0))
}
/// A progress bar with a thin marker at `reference` (0–1), like "referencia: 100%".
fn marked_bar(fraction: f32, reference: f32, color: Color, p: Palette) -> El {
    let f = fraction.clamp(0.0, 1.0);
    let r = reference.clamp(0.0, 1.0);
    let track = row().w_full().h(10.0).radius(5.0).fill(p.border)
        .child(el().grow(f).h(10.0).radius(5.0).fill(color)).child(el().grow(1.0 - f).h(10.0));
    let tick = |h: f32| row().w_full().h(h).child(el().grow(r)).child(el().w(3.0).h(h).radius(1.5).fill(p.ink)).child(el().grow(1.0 - r));
    col().w_full().gap(2.0).child(tick(8.0)).child(track)
}

fn header(s: &State, p: Palette) -> El {
    row().w_full().px(18.0).py(14.0).gap(10.0).radius(18.0).fill(p.card).border(1.0, p.border).center_y()
        .child(icon_button(&theme::LEFT, "Mes anterior", "previous-month", |s| s.set_month(month::previous(s.month)), p))
        .child(icon_button(&theme::RIGHT, "Mes siguiente", "next-month", |s| s.set_month(month::next(s.month)), p))
        .child(text(month::spanish_title(s.month)).key("month-title").font(theme::HEADLINE).color(p.ink).pl(6.0))
        .child(el().grow(1.0))
        .child(if s.month == s.today { el() } else { button("Hoy", "today", |s| s.set_month(s.today), p) })
        .child(secondary(&theme::PLUS, "Ingreso", "add-income", |s| s.open_sheet(false, None), p))
        .child(primary(&theme::MINUS, "Gasto", "add-expense", |s| s.open_sheet(true, None), p))
}

/// Scheduled outflows still owed in the month on screen; overdue ones count only for the current month.
fn pending(s: &State) -> Vec<&UpcomingView> {
    let Some(data) = &s.proximos else { return Vec::new() };
    data.items.iter().filter(|i| i.amount < 0 && (month::month_of(i.date) == s.month || (i.overdue && s.month == s.today))).collect()
}

fn forecast_card(o: &Overview, s: &State, p: Palette) -> El {
    let f = metrics::forecast(o.current.income, o.current.expenses, pending(s).iter().map(|i| i.amount));
    let (spent_share, pending_share, left_share) = f.shares();
    let spent_color = p.petroleum;
    let pending_color = color("#3F8CFF");
    let left_color = if f.left < 0 { p.coral } else { p.emerald };
    let note = match f.percent_of_income(f.left) {
        Some(pct) if f.left >= 0 => row().gap(4.0).center_y().child(icon(&theme::TREND_UP, 13.0).color(p.emerald)).child(text(format!("{pct}% del ingreso libre")).font(type_::LABEL).color(p.emerald)),
        Some(_) | None if f.left < 0 => row().gap(4.0).center_y().child(icon(&theme::TREND_DOWN, 13.0).color(p.coral)).child(text("Gastos por encima del ingreso").font(type_::LABEL).color(p.coral)),
        _ => row().child(text("Sin ingresos este mes").font(type_::LABEL).color(p.muted)),
    };
    let mut track = row().w_full().h(10.0).gap(3.0);
    for (share, c) in [(spent_share, spent_color), (pending_share, pending_color), (left_share, left_color)] {
        if share > 0.0 { track = track.child(el().grow(share).h(10.0).radius(5.0).fill(c)); }
    }
    if spent_share + pending_share + left_share <= 0.0 { track = track.child(el().grow(1.0).h(10.0).radius(5.0).fill(p.border)); }
    let pct = |v: i64| f.percent_of_income(v).map_or(String::new(), |v| format!(" · {v}%"));
    let legend = |title: &str, c: Option<Color>, value: String, share: String, tint: Color, foot: Option<String>| {
        let mut t = row().gap(6.0).center_y();
        if let Some(c) = c { t = t.child(dot(c)); }
        let mut l = col().grow(1.0).gap(4.0).child(t.child(muted(title.to_owned(), p)))
            .child(row().gap(2.0).items_end().child(text(value).font(type_::TITLE).color(tint)).child(muted(share, p)));
        if let Some(foot) = foot { l = l.child(text(foot).font(type_::LABEL).color(p.muted)); }
        l
    };
    card(p).key("forecast").grow(0.0)
        .child(row().w_full().justify_between()
            .child(col().gap(10.0).child(row().gap(8.0).center_y().child(card_title("Saldo previsto", p))
                    .child(text(format!("· Saldo en cuentas {}", money::format(o.total_balance))).key("total-balance").font(type_::LABEL).color(p.muted)))
                .child(row().gap(18.0).items_end().child(money_big(f.left, true, if f.left < 0 { p.coral } else { p.ink }, p).key("forecast-left"))
                    .child(col().gap(3.0).pb(8.0).child(note).child(muted("estimado tras pagar lo programado del mes", p)))))
            .child(icon(&theme::WALLET_ART, 76.0)))
        .child(track)
        .child(row().w_full().gap(12.0)
            .child(legend("Ingresos", None, money::format(f.income), String::new(), p.ink, Some(metrics::change_badge(o.income_change))))
            .child(legend("Gastado", Some(spent_color), format!("− {}", money::format(f.spent)), pct(f.spent), p.ink, Some(metrics::change_badge(o.expenses_change))))
            .child(legend("Por pagar", Some(pending_color), format!("− {}", money::format(f.pending)), pct(f.pending), p.ink, None))
            .child(legend("= Sobra", Some(left_color), money::format(f.left), pct(f.left.max(0)), left_color, None)))
}

fn payments_card(s: &State, p: Palette) -> El {
    let items = pending(s);
    let total: i64 = items.iter().map(|i| -i.amount).sum();
    let overdue = items.iter().filter(|i| i.overdue).count();
    let note = if s.proximos.is_none() { "Cargando…".to_owned() } else if items.is_empty() { "Nada pendiente este mes".to_owned() } else if overdue > 0 { format!("{} pendiente(s) · {overdue} vencido(s)", items.len()) } else { format!("{} pendiente(s)", items.len()) };
    let mut c = card(p).key("payments").grow(0.0)
        .child(row().w_full().justify_between()
            .child(col().gap(6.0).child(card_title("Pagos por hacer", p)).child(money_big(total, false, p.ink, p).key("payments-total")).child(muted(note, p)))
            .child(icon(&theme::CALENDAR_ART, 56.0)));
    let mut days: Vec<NaiveDate> = items.iter().map(|i| i.date).collect();
    days.dedup();
    if !days.is_empty() {
        let mut chips = row().w_full().gap(6.0);
        for (n, day) in days.iter().take(5).enumerate() {
            let count = items.iter().filter(|i| i.date == *day).count().min(5);
            let first = n == 0;
            let late = items.iter().any(|i| i.date == *day && i.overdue);
            let tint = if late { p.amber } else { p.petroleum };
            let (fill, fg) = if first { (tint, Color::WHITE) } else { (Color::TRANSPARENT, p.ink) };
            let mut dots = row().gap(2.0).center();
            for _ in 0..count { dots = dots.child(el().size(3.0, 3.0).radius(1.5).fill(if first { Color::WHITE } else { p.muted })); }
            chips = chips.child(col().grow(1.0).py(8.0).gap(4.0).radius(10.0).center().fill(fill).border(1.0, if first { tint } else { p.border })
                .child(text(day.format("%d").to_string()).font(type_::TITLE).color(fg)).child(dots));
        }
        c = c.child(chips);
    }
    for item in items.iter().take(6) {
        let ring = el().size(16.0, 16.0).radius(8.0).border(1.5, if item.overdue { p.amber } else { p.border });
        c = c.child(row().w_full().gap(10.0).py(4.0).center_y().key(format!("payment-{}-{}", item.schedule_id, item.date))
            .child(ring)
            .child(col().grow(1.0).min_w(0.0).gap(2.0).child(label(item.name.clone(), p).truncate())
                .child(text(if item.overdue { format!("Vencido · {}", item.date.format("%d/%m")) } else { item.date.format("%d/%m").to_string() }).font(type_::LABEL).color(if item.overdue { p.amber } else { p.muted })))
            .child(text(money::format(-item.amount)).font(type_::TITLE).color(p.ink)));
    }
    if items.len() > 6 { c = c.child(muted(format!("y {} más", items.len() - 6), p)); }
    c.child(row().key("see-payments").role("button").label("Ver próximos pagos").focusable(true).w_full().py(8.0).radius(10.0).center().hover(p.selected)
        .on_press(|s: &mut State| s.route = Route::Proximos).child(text("Ver próximos pagos").font(type_::LABEL).color(p.petroleum)))
}

const PALETTE: [&str; 6] = ["#2F6FED", "#E8476A", "#34C78A", "#F5A524", "#8B5CF6", "#14B8C4"];
fn spending(o: &Overview, p: Palette) -> El {
    let mut c = card(p).key("spending")
        .child(heading("Gasto por categoría", Some("Total del mes".into()), &theme::PIE_ART, p))
        .child(money_big(o.spending_total, false, p.ink, p));
    if o.spending.is_empty() { return c.child(muted("Sin gastos este mes", p)); }
    let mut stacked = row().w_full().h(12.0).gap(2.0);
    for (n, item) in o.spending.iter().enumerate() {
        let f = chart_fraction(item.spent, o.spending_total);
        if f > 0.0 { stacked = stacked.child(el().grow(f).h(12.0).radius(4.0).fill(color(PALETTE[n % PALETTE.len()]))); }
    }
    c = c.child(stacked).child(text("En qué se va el dinero").font(type_::LABEL).color(p.ink).pt(4.0));
    for (n, item) in o.spending.iter().enumerate() {
        let pct = if o.spending_total > 0 { item.spent as f64 / o.spending_total as f64 * 100.0 } else { 0.0 };
        c = c.child(el().w_full().h(1.0).fill(p.border))
            .child(row().w_full().gap(8.0).center_y()
                .child(dot(color(PALETTE[n % PALETTE.len()]))).child(label(item.name.clone(), p).grow(1.0).min_w(0.0).truncate())
                .child(muted(format!("{pct:.0}%"), p).w(40.0).shrink())
                .child(text(money::format(item.spent)).font(type_::TITLE).color(p.ink)));
    }
    c
}
fn chart(o: &Overview, selected: u32, p: Palette) -> El {
    let max = o.chart.iter().map(|v| v.income.max(v.expenses)).max().unwrap_or(0);
    let mut columns = row().w_full().h(150.0).gap(8.0).center_y();
    for point in &o.chart {
        let inc = chart_fraction(point.income, max) * 110.0;
        let exp = chart_fraction(point.expenses, max) * 110.0;
        columns = columns.child(col().grow(1.0).h_full().gap(6.0).center()
            .child(row().h(115.0).gap(3.0).center_y()
                .child(col().h(115.0).justify_end().child(el().w(10.0).h(inc).radius(3.0).fill(p.emerald)))
                .child(col().h(115.0).justify_end().child(el().w(10.0).h(exp).radius(3.0).fill(p.coral))))
            .child(muted(format!("{:02}/{}", point.month % 100, point.month / 100 % 100), p)));
    }
    let points: Vec<_> = o.chart.iter().map(|v| (v.income, v.expenses)).collect();
    let toggle = |name: &'static str, key: &'static str, months: u32, action: fn(&mut State)| {
        el().key(key).role("radio").label(name).focusable(true).checked(selected == months).px(10.0).py(5.0).radius(8.0)
            .fill(if selected == months { p.card } else { Color::TRANSPARENT }).on_press(action)
            .child(text(name.to_owned()).font(type_::LABEL).color(if selected == months { p.ink } else { p.muted }))
    };
    let net = metrics::net_average(&points);
    card(p).key("chart")
        .child(row().w_full().justify_between().center_y().child(card_title("Ingresos vs gastos", p))
            .child(row().gap(2.0).padding(3.0).radius(10.0).fill(p.canvas)
                .child(toggle("6M", "chart-6", 6, |s| { s.chart_months = 6; s.refresh = true; }))
                .child(toggle("12M", "chart-12", 12, |s| { s.chart_months = 12; s.refresh = true; }))))
        .child(row().gap(14.0).child(row().gap(6.0).center_y().child(dot(p.emerald)).child(muted("Ingresos", p)))
            .child(row().gap(6.0).center_y().child(dot(p.coral)).child(muted("Gastos", p))))
        .child(columns)
        .child(row().w_full().justify_between().child(muted("Flujo neto promedio", p))
            .child(text(money::format(net)).font(type_::TITLE).color(if net < 0 { p.coral } else { p.emerald })))
}
fn budget(o: &Overview, p: Palette) -> El {
    let ratio = if o.budget_assigned > 0 { o.budget_spent as f64 / o.budget_assigned as f64 } else { 0.0 };
    let tint = if ratio > 1.0 { p.coral } else if ratio >= 0.9 { p.amber } else { p.emerald };
    let mut c = card(p).key("budget").grow(0.0)
        .child(heading("Presupuesto del mes", Some("Gastado de lo asignado".into()), &theme::BARS_ART, p))
        .child(row().gap(6.0).items_end().child(money_big(o.budget_spent, false, p.ink, p)).child(muted(format!("de {}", money::format(o.budget_assigned)), p).pb(3.0)));
    if o.budget_assigned == 0 { return c.child(muted("Sin asignar este mes", p)); }
    // The bar spans 0–150% of the budget, with the marker at 100%.
    c = c.child(row().gap(4.0).center_y().child(text(format!("{:.0}% usado", ratio * 100.0)).font(type_::LABEL).color(tint))
            .child(muted(format!("· {} {}", money::format((o.budget_assigned - o.budget_spent).abs()), if ratio > 1.0 { "excedido" } else { "restante" }), p)))
        .child(marked_bar((ratio / 1.5) as f32, 1.0 / 1.5, tint, p))
        .child(row().w_full().justify_between().child(muted("0%", p)).child(muted("150%", p)));
    if !o.budget_progress.is_empty() { c = c.child(text("Categorías principales").font(type_::LABEL).color(p.ink).pt(4.0)); }
    for item in &o.budget_progress {
        let share = if item.budgeted > 0 { item.spent as f64 / item.budgeted as f64 } else { 0.0 };
        let bar_tint = if share > 1.0 { p.coral } else if share >= 0.9 { p.amber } else { p.petroleum };
        c = c.child(col().w_full().gap(5.0)
            .child(row().w_full().justify_between().child(label(item.name.clone(), p).truncate()).child(muted(format!("{} / {}", money::format(item.spent), money::format(item.budgeted)), p)))
            .child(bar(item.spent, item.budgeted, bar_tint, p)));
    }
    c
}
fn recent(o: &Overview, p: Palette) -> El {
    let mut c = card(p).key("recent").child(row().w_full().justify_between().center_y().child(card_title("Movimientos recientes", p))
        .child(row().key("see-movements").role("button").label("Ver todos").focusable(true).px(10.0).py(5.0).radius(8.0).hover(p.selected)
            .on_press(|s: &mut State| s.route = Route::Todas).child(text("Ver todos").font(type_::LABEL).color(p.petroleum))));
    if o.recent_entries.is_empty() { return c.child(muted("Aún no hay movimientos", p)); }
    for e in &o.recent_entries {
        let income = e.amount >= 0;
        let tint = if income { p.emerald } else { p.coral };
        c = c.child(el().w_full().h(1.0).fill(p.border)).child(row().w_full().gap(12.0).center_y()
            .child(badge(if income { &theme::TREND_UP } else { &theme::TREND_DOWN }, 14.0, tint))
            .child(col().grow(1.0).min_w(0.0).gap(2.0)
                .child(label(e.payee_name.as_deref().filter(|v| !v.is_empty()).unwrap_or(if e.notes.is_empty() { "Movimiento" } else { &e.notes }).to_owned(), p).truncate())
                .child(muted(format!("{} · {}", e.category_name.as_deref().unwrap_or("Sin categoría"), e.account_name.as_deref().unwrap_or("—")), p).truncate()))
            .child(muted(e.date.format("%d/%m/%Y").to_string(), p).w(84.0).shrink())
            .child(row().w(110.0).justify_end().child(text(money::format(e.amount)).font(type_::TITLE).color(tint))));
    }
    c
}
/// Payment reminders planned by the worker; each one can be dismissed for this session.
pub const BANNER_ROWS: usize = 3;
fn reminders_banner(s: &State, p: Palette) -> Option<El> {
    if !s.settings.enabled { return None; }
    let pending: Vec<_> = s.reminders.iter().filter(|r| !s.dismissed.contains(&r.id)).collect();
    if pending.is_empty() { return None; }
    let mut banner = card(p).key("reminders-banner").fill(p.amber.alpha(0.12)).border(1.0, p.amber)
        .child(row().w_full().center_y().justify_between().child(row().gap(8.0).center_y().child(icon(&theme::CLOCK, 16.0).color(p.amber)).child(card_title("Recordatorios de pagos", p)))
            .child(button("Descartar todos", "dismiss-all", |s| { let ids: Vec<_> = s.reminders.iter().map(|r| r.id.clone()).collect(); s.dismissed.extend(ids); }, p)));
    for r in pending.iter().take(BANNER_ROWS) {
        let id = r.id.clone();
        banner = banner.child(row().w_full().center_y().justify_between().gap(12.0).key(format!("reminder-{}", r.id))
            .child(col().gap(2.0).child(label(r.title.clone(), p)).child(muted(r.body.clone(), p)).child(muted(format!("Aviso: {}", r.fire_at.format("%d/%m/%Y %H:%M")), p)))
            .child(el().key(format!("dismiss-{}", r.id)).role("button").label("Descartar").focusable(true).padding(8.0).radius(8.0).fill(p.card).hover(p.selected)
                .on_press(move |s: &mut State| { s.dismissed.insert(id.clone()); }).child(label("Descartar", p))));
    }
    if pending.len() > BANNER_ROWS { banner = banner.child(muted(format!("y {} más", pending.len() - BANNER_ROWS), p)); }
    Some(banner)
}
pub fn render(s: &State, p: Palette) -> El {
    let mut content = col().w_full().padding(24.0).gap(16.0).child(header(s, p));
    if let Some(banner) = reminders_banner(s, p) { content = content.child(banner); }
    if let Some(o) = &s.overview {
        let first = &o.first_run;
        if !(first.has_account && first.has_categorized_income && first.has_budget) {
            let mut guide = card(p).child(card_title("Primeros pasos", p));
            for (done, name) in [(first.has_account, "Crea una cuenta"), (first.has_categorized_income, "Registra tu ingreso"), (first.has_budget, "Asígnalo en el presupuesto")] {
                guide = guide.child(row().gap(8.0).center_y().child(el().size(14.0, 14.0).radius(7.0).fill(if done { p.emerald } else { Color::TRANSPARENT }).border(1.5, if done { p.emerald } else { p.border }))
                    .child(if done { muted(name, p) } else { label(name, p) }));
            }
            content = content.child(guide);
        }
        content = content.child(row().w_full().gap(16.0)
            .child(col().grow(1.0).min_w(0.0).gap(16.0)
                .child(forecast_card(o, s, p))
                .child(row().w_full().gap(16.0).child(spending(o, p)).child(chart(o, s.chart_months, p))))
            .child(col().w(330.0).gap(16.0).child(payments_card(s, p)).child(budget(o, p))))
            .child(recent(o, p));
    } else { content = content.child(muted("Cargando…", p)); }
    col().grow(1.0).h_full().min_h(0.0).scroll(rune::ScrollSpec { stick_bottom: false }).child(content)
}
