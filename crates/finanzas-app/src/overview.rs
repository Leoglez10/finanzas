use super::*;
use finanzas_core::service::{Overview, UpcomingView};
use chrono::Datelike;
use theme::{badge, card_title, dot, icon_button, money_big, primary, secondary, tappable};

pub fn label(value: impl Into<String>, p: Palette) -> El { text(value.into()).font(type_::BODY).color(p.ink) }
pub fn muted(value: impl Into<String>, p: Palette) -> El { text(value.into()).font(type_::BODY).color(p.muted) }
pub fn card(p: Palette) -> El { col().grow(1.0).padding(20.0).gap(12.0).radius(18.0).fill(p.card).border(1.0, p.border) }
pub fn button(name: &'static str, key: &'static str, action: fn(&mut State), p: Palette) -> El {
    el().key(key).role("button").label(name).focusable(true).padding(8.0).radius(8.0).fill(p.card).hover(p.selected).press_scale(0.96)
        .on_press(action).child(label(name, p))
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
    let title = text(month::spanish_title(s.month)).key("month-title").font(theme::HEADLINE).color(p.ink);
    let title = if s.month == s.today { title.pl(6.0) } else {
        tappable(title.px(6.0).py(2.0), p).label("Ir al mes actual").help("Volver al mes actual").on_press(|s: &mut State| s.set_month(s.today))
    };
    row().w_full().px(18.0).py(14.0).gap(10.0).radius(18.0).fill(p.card).border(1.0, p.border).center_y().window_drag()
        .child(icon_button(&theme::LEFT, "Mes anterior", "previous-month", |s| s.set_month(month::previous(s.month)), p).help("Mes anterior"))
        .child(icon_button(&theme::RIGHT, "Mes siguiente", "next-month", |s| s.set_month(month::next(s.month)), p).help("Mes siguiente"))
        .child(title)
        .child(el().grow(1.0))
        .child(if s.month == s.today { el() } else { button("Hoy", "today", |s| s.set_month(s.today), p) })
        .child(secondary(&theme::PLUS, "Ingreso", "add-income", |s| s.open_sheet(false, None), p).help("Registrar un ingreso"))
        .child(primary(&theme::MINUS, "Gasto", "add-expense", |s| s.open_sheet(true, None), p).help("Registrar un gasto"))
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
    let segment = |share: f32, c: Color, tip: String| el().grow(share).h(10.0).radius(5.0).fill(c).help(tip);
    let mut track = row().w_full().h(10.0).gap(3.0);
    for (share, c, name, value) in [(spent_share, spent_color, "Gastado", f.spent), (pending_share, pending_color, "Por pagar", f.pending), (left_share, left_color, "Sobra", f.left)] {
        if share > 0.0 { track = track.child(segment(share, c, format!("{name}: {}", money::format(value)))); }
    }
    if spent_share + pending_share + left_share <= 0.0 { track = track.child(el().grow(1.0).h(10.0).radius(5.0).fill(p.border)); }
    let pct = |v: i64| match f.percent_of_income(v) { Some(v) if v > 0 => format!(" · {v}%"), _ => String::new() };
    let legend = |key: &'static str, title: &str, c: Option<Color>, value: String, share: String, tint: Color, foot: Option<String>, tip: &'static str, go: fn(&mut State)| {
        let mut t = row().gap(6.0).center_y();
        if let Some(c) = c { t = t.child(dot(c)); }
        let mut l = col().grow(1.0).gap(4.0).padding(8.0).child(t.child(muted(title.to_owned(), p)).child(icon(&theme::ARROW, 11.0).color(p.muted)))
            .child(row().gap(2.0).items_end().child(text(value).font(type_::TITLE).color(tint)).child(muted(share, p)));
        if let Some(foot) = foot { l = l.child(text(foot).font(type_::LABEL).color(p.muted)); }
        tappable(l.key(key), p).label(title.to_owned()).help(tip).on_press(go)
    };
    card(p).key("forecast").grow(0.0)
        .child(row().w_full().justify_between()
            .child(col().gap(10.0).child(row().gap(8.0).center_y().child(card_title("Saldo previsto", p))
                    .child(tappable(text(format!("· Saldo en cuentas {}", money::format(o.total_balance))).key("total-balance").font(type_::LABEL).color(p.muted).px(4.0).py(2.0), p)
                        .help("Ver todas las cuentas").on_press(|s: &mut State| s.show_movements(""))))
                .child(row().gap(18.0).items_end().child(money_big(f.left, true, if f.left < 0 { p.coral } else { p.ink }, p).key("forecast-left"))
                    .child(col().gap(3.0).pb(8.0).child(note).child(muted("estimado tras pagar lo programado del mes", p)))))
            .child(icon(&theme::WALLET_ART, 76.0)))
        .child(track)
        .child(row().w_full().gap(4.0)
            .child(legend("legend-income", "Ingresos", None, money::format(f.income), String::new(), p.ink, Some(metrics::change_badge(o.income_change)), "Ver movimientos", |s| s.show_movements("")))
            .child(legend("legend-spent", "Gastado", Some(spent_color), format!("− {}", money::format(f.spent)), pct(f.spent), p.ink, Some(metrics::change_badge(o.expenses_change)), "Ver movimientos", |s| s.show_movements("")))
            .child(legend("legend-pending", "Por pagar", Some(pending_color), format!("− {}", money::format(f.pending)), pct(f.pending), p.ink, None, "Ver próximos pagos", |s| s.route = Route::Proximos))
            .child(legend("legend-left", "= Sobra", Some(left_color), money::format(f.left), pct(f.left), left_color, None, "Ver el presupuesto", |s| s.route = Route::Presupuesto)))
}

fn payments_card(s: &State, p: Palette) -> El {
    let all = pending(s);
    let total: i64 = all.iter().map(|i| -i.amount).sum();
    let overdue = all.iter().filter(|i| i.overdue).count();
    let note = if s.proximos.is_none() { "Cargando…".to_owned() } else if all.is_empty() { "Nada pendiente este mes".to_owned() } else if overdue > 0 { format!("{} pendiente(s) · {overdue} vencido(s)", all.len()) } else { format!("{} pendiente(s)", all.len()) };
    let mut c = card(p).key("payments").grow(0.0)
        .child(row().w_full().justify_between()
            .child(col().gap(6.0).child(card_title("Pagos por hacer", p)).child(money_big(total, false, p.ink, p).key("payments-total")).child(muted(note, p)))
            .child(icon(&theme::CALENDAR_ART, 56.0)));
    let mut days: Vec<NaiveDate> = all.iter().map(|i| i.date).collect();
    days.dedup();
    let picked = s.payment_day.filter(|d| days.contains(d));
    if !days.is_empty() {
        let mut chips = row().w_full().gap(6.0);
        for day in days.iter().take(5).copied() {
            let count = all.iter().filter(|i| i.date == day).count();
            let on = picked == Some(day);
            let late = all.iter().any(|i| i.date == day && i.overdue);
            let tint = if late { p.amber } else { p.petroleum };
            let (fill, fg) = if on { (tint, p.card) } else { (if late { p.amber.alpha(0.12) } else { Color::TRANSPARENT }, if late { p.amber } else { p.ink }) };
            let mut dots = row().gap(2.0).center();
            for _ in 0..count.min(5) { dots = dots.child(el().size(3.0, 3.0).radius(1.5).fill(if on { p.card } else { tint })); }
            let chip = col().key(format!("payment-day-{day}")).grow(1.0).py(7.0).gap(3.0).radius(10.0).center().fill(fill).border(1.0, if on || late { tint } else { p.border })
                .child(text(day.format("%d").to_string()).font(type_::TITLE).color(fg))
                .child(text(proximos::MONTHS[day.month0() as usize].to_owned()).font(type_::LABEL).color(if on { p.card } else { p.muted }))
                .child(dots);
            chips = chips.child(tappable(chip, p).label(format!("Pagos del {}", day.format("%d/%m"))).help(format!("{count} pago(s) el {} · clic para filtrar", day.format("%d/%m")))
                .on_press(move |s: &mut State| s.payment_day = if s.payment_day == Some(day) { None } else { Some(day) }));
        }
        c = c.child(chips);
    }
    let items: Vec<_> = all.iter().filter(|i| picked.is_none_or(|d| i.date == d)).collect();
    let fresh = s.payments_fresh() && !s.mutation_active();
    for item in items.iter().take(6) {
        let (id, date) = (item.schedule_id, item.date);
        let actionable = fresh && s.proximos.as_ref().is_some_and(|d| d.schedules.iter().any(|v| v.schedule.id == id && v.next_date == Some(date)));
        let ring_color = if item.overdue { p.amber } else { p.border };
        let ring = el().key(format!("pay-{id}-{date}")).size(22.0, 22.0).radius(11.0).center().border(1.5, ring_color).label(format!("Marcar {} como pagado", item.name));
        let ring = if actionable {
            ring.role("button").focusable(true).hover(p.emerald.alpha(0.25)).press_scale(0.9).help("Marcar como pagado (se registra hoy)")
                .child(icon(&theme::CHECK, 12.0).color(p.emerald.alpha(0.55))).on_press(move |s: &mut State| s.schedule_action(id, true))
        } else { ring.help("Solo se puede pagar la siguiente fecha de este pago") };
        let body = row().grow(1.0).min_w(0.0).gap(10.0).center_y().padding(4.0)
            .child(col().grow(1.0).min_w(0.0).gap(2.0).child(label(item.name.clone(), p).truncate())
                .child(text(if item.overdue { format!("Vencido · {}", date.format("%d/%m")) } else { date.format("%d/%m").to_string() }).font(type_::LABEL).color(if item.overdue { p.amber } else { p.muted })))
            .child(text(money::format(-item.amount)).font(type_::TITLE).color(p.ink));
        c = c.child(row().w_full().gap(8.0).center_y().key(format!("payment-{id}-{date}")).child(ring)
            .child(tappable(body, p).label(item.name.clone()).help("Ver en próximos pagos").on_press(|s: &mut State| s.route = Route::Proximos)));
    }
    if items.len() > 6 { c = c.child(muted(format!("y {} más", items.len() - 6), p)); }
    if picked.is_some() {
        c = c.child(tappable(row().key("payment-day-clear").w_full().py(6.0).center().child(text("Mostrar todos los días").font(type_::LABEL).color(p.petroleum)), p)
            .on_press(|s: &mut State| s.payment_day = None));
    }
    c.child(tappable(row().key("see-payments").label("Ver próximos pagos").w_full().py(8.0).gap(6.0).center()
        .child(text("Ver próximos pagos").font(type_::LABEL).color(p.petroleum)).child(icon(&theme::ARROW, 13.0).color(p.petroleum)), p)
        .on_press(|s: &mut State| s.route = Route::Proximos))
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
        let name = item.name.clone();
        if f > 0.0 {
            stacked = stacked.child(el().grow(f).h(12.0).radius(4.0).fill(color(PALETTE[n % PALETTE.len()])).hover(color(PALETTE[n % PALETTE.len()]).alpha(0.7))
                .help(format!("{} · {}", item.name, money::format(item.spent))).on_press(move |s: &mut State| s.show_movements(&name)));
        }
    }
    c = c.child(stacked).child(text("En qué se va el dinero · clic para ver los movimientos").font(type_::LABEL).color(p.muted).pt(4.0));
    for (n, item) in o.spending.iter().enumerate() {
        let pct = if o.spending_total > 0 { item.spent as f64 / o.spending_total as f64 * 100.0 } else { 0.0 };
        let name = item.name.clone();
        let line = row().key(format!("spend-{}", item.category_id)).w_full().gap(8.0).px(6.0).py(7.0).center_y()
            .child(dot(color(PALETTE[n % PALETTE.len()]))).child(label(item.name.clone(), p).grow(1.0).min_w(0.0).truncate())
            .child(muted(format!("{pct:.0}%"), p).w(40.0).shrink())
            .child(text(money::format(item.spent)).font(type_::TITLE).color(p.ink))
            .child(icon(&theme::ARROW, 12.0).color(p.muted));
        c = c.child(tappable(line, p).label(item.name.clone()).help(format!("Ver movimientos de {}", item.name)).on_press(move |s: &mut State| s.show_movements(&name)));
    }
    c
}
fn chart(o: &Overview, s: &State, p: Palette) -> El {
    let selected = s.chart_months;
    let max = o.chart.iter().map(|v| v.income.max(v.expenses)).max().unwrap_or(0);
    let mut columns = row().w_full().h(150.0).gap(4.0).center_y();
    for point in &o.chart {
        let inc = chart_fraction(point.income, max) * 104.0;
        let exp = chart_fraction(point.expenses, max) * 104.0;
        let m = point.month;
        let current = m == s.month;
        let column = col().key(format!("chart-month-{m}")).grow(1.0).h_full().gap(6.0).py(4.0).center().radius(10.0)
            .fill(if current { p.selected } else { Color::TRANSPARENT })
            .child(row().h(110.0).gap(3.0).center_y()
                .child(col().h(110.0).justify_end().child(el().w(10.0).h(inc).radius(3.0).fill(p.emerald)))
                .child(col().h(110.0).justify_end().child(el().w(10.0).h(exp).radius(3.0).fill(p.coral))))
            .child(text(format!("{:02}/{}", m % 100, m / 100 % 100)).font(type_::LABEL).color(if current { p.petroleum } else { p.muted }));
        columns = columns.child(tappable(column, p).label(month::spanish_title(m))
            .help(format!("{} · Ingresos {} · Gastos {} · clic para abrir", month::spanish_title(m), money::format(point.income), money::format(point.expenses)))
            .on_press(move |s: &mut State| s.set_month(m)));
    }
    let points: Vec<_> = o.chart.iter().map(|v| (v.income, v.expenses)).collect();
    let toggle = |name: &'static str, key: &'static str, months: u32, action: fn(&mut State)| {
        el().key(key).role("radio").label(name).focusable(true).checked(selected == months).px(10.0).py(5.0).radius(8.0).press_scale(0.95).hover(p.selected)
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
        .child(tappable(row().w_full().key("budget-open").child(heading("Presupuesto del mes", Some("Gastado de lo asignado · abrir".into()), &theme::BARS_ART, p)), p)
            .help("Abrir el presupuesto").on_press(|s: &mut State| s.route = Route::Presupuesto))
        .child(row().gap(6.0).items_end().child(money_big(o.budget_spent, false, p.ink, p)).child(muted(format!("de {}", money::format(o.budget_assigned)), p).pb(3.0)));
    if o.budget_assigned == 0 {
        return c.child(tappable(row().key("budget-assign").w_full().py(8.0).gap(6.0).center().child(text("Asignar presupuesto").font(type_::LABEL).color(p.petroleum)).child(icon(&theme::ARROW, 13.0).color(p.petroleum)), p)
            .on_press(|s: &mut State| s.route = Route::Presupuesto));
    }
    // The bar spans 0–150% of the budget, with the marker at 100%.
    c = c.child(row().gap(4.0).center_y().child(text(format!("{:.0}% usado", ratio * 100.0)).font(type_::LABEL).color(tint))
            .child(muted(format!("· {} {}", money::format((o.budget_assigned - o.budget_spent).abs()), if ratio > 1.0 { "excedido" } else { "restante" }), p)))
        .child(marked_bar((ratio / 1.5) as f32, 1.0 / 1.5, tint, p).help("La marca indica el 100% del presupuesto"))
        .child(row().w_full().justify_between().child(muted("0%", p)).child(muted("150%", p)));
    if !o.budget_progress.is_empty() { c = c.child(text("Categorías principales").font(type_::LABEL).color(p.ink).pt(4.0)); }
    for item in &o.budget_progress {
        let share = if item.budgeted > 0 { item.spent as f64 / item.budgeted as f64 } else { 0.0 };
        let bar_tint = if share > 1.0 { p.coral } else if share >= 0.9 { p.amber } else { p.petroleum };
        let name = item.name.clone();
        c = c.child(tappable(col().key(format!("budget-cat-{}", item.category_id)).w_full().gap(5.0).padding(6.0)
            .child(row().w_full().justify_between().child(label(item.name.clone(), p).truncate()).child(muted(format!("{} / {}", money::format(item.spent), money::format(item.budgeted)), p)))
            .child(bar(item.spent, item.budgeted, bar_tint, p)), p)
            .help(format!("{:.0}% usado · clic para ver los movimientos", share * 100.0)).on_press(move |s: &mut State| s.show_movements(&name)));
    }
    c
}
fn recent(o: &Overview, p: Palette) -> El {
    let mut c = card(p).key("recent").child(row().w_full().justify_between().center_y().child(card_title("Movimientos recientes", p))
        .child(tappable(row().key("see-movements").label("Ver todos").gap(6.0).px(10.0).py(5.0).center_y()
            .child(text("Ver todos").font(type_::LABEL).color(p.petroleum)).child(icon(&theme::ARROW, 13.0).color(p.petroleum)), p)
            .on_press(|s: &mut State| s.show_movements(""))));
    if o.recent_entries.is_empty() { return c.child(muted("Aún no hay movimientos", p)); }
    for e in &o.recent_entries {
        let income = e.amount >= 0;
        let tint = if income { p.emerald } else { p.coral };
        let name = e.payee_name.as_deref().filter(|v| !v.is_empty()).unwrap_or(if e.notes.is_empty() { "Movimiento" } else { &e.notes }).to_owned();
        let query = name.clone();
        let line = row().key(format!("recent-{}", e.id)).w_full().gap(12.0).px(8.0).py(6.0).center_y()
            .child(badge(if income { &theme::TREND_UP } else { &theme::TREND_DOWN }, 14.0, tint))
            .child(col().grow(1.0).min_w(0.0).gap(2.0)
                .child(label(name.clone(), p).truncate())
                .child(muted(format!("{} · {}", e.category_name.as_deref().unwrap_or("Sin categoría"), e.account_name.as_deref().unwrap_or("—")), p).truncate()))
            .child(muted(e.date.format("%d/%m/%Y").to_string(), p).w(84.0).shrink())
            .child(row().w(110.0).justify_end().child(text(money::format(e.amount)).font(type_::TITLE).color(tint)));
        c = c.child(tappable(line, p).label(name).help("Buscar en movimientos").on_press(move |s: &mut State| s.show_movements(&query)));
    }
    c
}
/// "New version available" (with how to get it) or "you were updated to X"; closable for this session.
fn update_banner(s: &State, p: Palette) -> Option<El> {
    if s.update_dismissed { return None; }
    let link = |name: &'static str, key: &'static str, url: String| el().key(key).role("button").label(name).focusable(true).padding(8.0).radius(8.0).fill(p.petroleum).press_scale(0.96)
        .on_press_tx(move |_: &mut State, tx: &mut Tx| { let _ = tx.open_url(&url); }).child(text(name.to_owned()).font(type_::BODY).color(Color::WHITE));
    let (title, note, action) = if let Some(v) = &s.update_available {
        (format!("Nueva versión disponible: {v}"), format!("Tienes la {}. Descárgala o, si instalaste con Homebrew, ejecuta: brew upgrade finanzas", update::CURRENT),
            link("Descargar", "update-download", format!("{}/latest", update::RELEASES)))
    } else if let Some(v) = &s.updated_to {
        (format!("Finanzas se actualizó a la versión {v}"), "Tus datos siguen intactos.".to_owned(), link("Ver novedades", "update-notes", format!("{}/tag/v{v}", update::RELEASES)))
    } else { return None; };
    Some(card(p).key("update-banner").fill(p.petroleum.alpha(0.10)).border(1.0, p.petroleum)
        .child(row().w_full().center_y().justify_between().gap(12.0)
            .child(col().gap(2.0).grow(1.0).child(card_title(&title, p)).child(muted(note, p).wrap().w_full()))
            .child(row().gap(8.0).child(action).child(button("Cerrar", "update-dismiss", |s| s.update_dismissed = true, p)))))
}
/// Payment reminders planned by the worker; each one can be dismissed for this session.
pub const BANNER_ROWS: usize = 3;
fn reminders_banner(s: &State, p: Palette) -> Option<El> {
    if !s.settings.enabled { return None; }
    let pending: Vec<_> = s.reminders.iter().filter(|r| !s.dismissed.contains(&r.id)).collect();
    if pending.is_empty() { return None; }
    let collapsed = s.reminders_collapsed;
    let title = if collapsed { format!("Recordatorios de pagos · {}", pending.len()) } else { "Recordatorios de pagos".to_owned() };
    let mut banner = card(p).key("reminders-banner").fill(p.amber.alpha(0.12)).border(1.0, p.amber)
        .child(row().w_full().center_y().justify_between()
            .child(tappable(row().key("reminders-collapse").gap(8.0).px(4.0).py(2.0).center_y().child(icon(&theme::CLOCK, 16.0).color(p.amber)).child(card_title(&title, p))
                .child(icon(if collapsed { &theme::RIGHT } else { &theme::LEFT }, 12.0).color(p.muted)), p)
                .label(if collapsed { "Mostrar recordatorios" } else { "Ocultar recordatorios" }).help(if collapsed { "Mostrar" } else { "Ocultar" })
                .on_press(|s: &mut State| s.reminders_collapsed = !s.reminders_collapsed))
            .child(button("Descartar todos", "dismiss-all", |s| { let ids: Vec<_> = s.reminders.iter().map(|r| r.id.clone()).collect(); s.dismissed.extend(ids); }, p)));
    if collapsed { return Some(banner); }
    for r in pending.iter().take(BANNER_ROWS) {
        let id = r.id.clone();
        banner = banner.child(row().w_full().center_y().justify_between().gap(12.0).key(format!("reminder-{}", r.id))
            .child(tappable(col().gap(2.0).padding(4.0).child(label(r.title.clone(), p)).child(muted(r.body.clone(), p)).child(muted(format!("Aviso: {}", r.fire_at.format("%d/%m/%Y %H:%M")), p)), p)
                .help("Ver en próximos pagos").on_press(|s: &mut State| s.route = Route::Proximos))
            .child(el().key(format!("dismiss-{}", r.id)).role("button").label("Descartar").focusable(true).padding(8.0).radius(8.0).fill(p.card).hover(p.selected).press_scale(0.96)
                .on_press(move |s: &mut State| { s.dismissed.insert(id.clone()); }).child(label("Descartar", p))));
    }
    if pending.len() > BANNER_ROWS { banner = banner.child(muted(format!("y {} más", pending.len() - BANNER_ROWS), p)); }
    Some(banner)
}
pub fn render(s: &State, p: Palette) -> El {
    let mut content = col().w_full().padding(24.0).gap(16.0).child(header(s, p));
    if let Some(banner) = update_banner(s, p) { content = content.child(banner); }
    if let Some(banner) = reminders_banner(s, p) { content = content.child(banner); }
    if let Some(o) = &s.overview {
        let first = &o.first_run;
        if !(first.has_account && first.has_categorized_income && first.has_budget) {
            let mut guide = card(p).child(card_title("Primeros pasos", p));
            type Step = (bool, &'static str, fn(&mut State));
            let steps: [Step; 3] = [
                (first.has_account, "Crea una cuenta", |s| s.open_account_sheet()),
                (first.has_categorized_income, "Registra tu ingreso", |s| s.open_sheet(false, None)),
                (first.has_budget, "Asígnalo en el presupuesto", |s| s.route = Route::Presupuesto)];
            for (done, name, go) in steps {
                let step = row().gap(8.0).px(6.0).py(4.0).center_y().child(el().size(14.0, 14.0).radius(7.0).fill(if done { p.emerald } else { Color::TRANSPARENT }).border(1.5, if done { p.emerald } else { p.border }))
                    .child(if done { muted(name, p) } else { label(name, p) });
                guide = guide.child(if done { step } else { tappable(step.child(icon(&theme::ARROW, 12.0).color(p.petroleum)), p).label(name).on_press(go) });
            }
            content = content.child(guide);
        }
        content = content.child(row().w_full().gap(16.0)
            .child(col().grow(1.0).min_w(0.0).gap(16.0)
                .child(forecast_card(o, s, p))
                .child(row().w_full().gap(16.0).child(spending(o, p)).child(chart(o, s, p))))
            .child(col().w(330.0).gap(16.0).child(payments_card(s, p)).child(budget(o, p))))
            .child(recent(o, p));
    } else { content = content.child(muted("Cargando…", p)); }
    col().grow(1.0).h_full().min_h(0.0).scroll(rune::ScrollSpec { stick_bottom: false }).child(content)
}
