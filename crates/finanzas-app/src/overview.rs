use super::*;
use finanzas_core::service::Overview;

pub fn label(value: impl Into<String>, p: Palette) -> El { text(value.into()).font(type_::BODY).color(p.ink) }
pub fn muted(value: impl Into<String>, p: Palette) -> El { text(value.into()).font(type_::BODY).color(p.muted) }
pub fn card(p: Palette) -> El { col().grow(1.0).padding(18.0).gap(10.0).radius(18.0).fill(p.card).border(1.0, p.border) }
pub fn button(name: &'static str, key: &'static str, action: fn(&mut State), p: Palette) -> El {
    el().key(key).role("button").label(name).focusable(true).padding(8.0).radius(8.0).fill(p.card).hover(p.selected)
        .on_press(action).child(label(name, p))
}
fn stat(title: &str, value: String, note: String, hero: bool, p: Palette) -> El {
    let fg = if hero { Color::WHITE } else { p.ink };
    card(p).fill(if hero { color("#155E75") } else { p.card })
        .child(text(title.to_owned()).font(type_::BODY).color(fg))
        .child(text(value).font(type_::TITLE).color(fg))
        .child(text(note).font(type_::BODY).color(if hero { Color::WHITE } else { p.muted }))
}
fn bar(value: i64, total: i64, color: Color, p: Palette) -> El {
    let f = chart_fraction(value, total);
    row().w_full().h(7.0).radius(3.5).fill(p.border)
        .child(el().grow(f).h(7.0).radius(3.5).fill(color))
        .child(el().grow(1.0 - f).h(7.0))
}
fn chart(o: &Overview, selected: u32, p: Palette) -> El {
    let emerald = p.emerald;
    let max = o.chart.iter().map(|v| v.income.max(v.expenses)).max().unwrap_or(0);
    let mut columns = row().w_full().h(110.0).gap(10.0).center_y();
    for point in &o.chart {
        let inc = chart_fraction(point.income, max) * 75.0;
        let exp = chart_fraction(point.expenses, max) * 75.0;
        let number = point.month % 100;
        columns = columns.child(col().grow(1.0).h_full().gap(5.0).center()
            .child(row().h(80.0).gap(4.0).center_y()
                .child(col().h(80.0).justify_end().child(el().w(13.0).h(inc).radius(3.0).fill(emerald)))
                .child(col().h(80.0).justify_end().child(el().w(13.0).h(exp).radius(3.0).fill(p.coral))))
            .child(muted(format!("{:02}/{}", number, point.month / 100 % 100), p)));
    }
    let points: Vec<_> = o.chart.iter().map(|v| (v.income, v.expenses)).collect();
    card(p).child(row().w_full().justify_between().center_y().child(label("Ingresos vs gastos", p))
        .child(row().gap(3.0).child(button("6M", "chart-6", |s| { s.chart_months = 6; s.refresh = true; }, p).role("radio").checked(selected == 6).fill(if selected == 6 { p.selected } else { Color::TRANSPARENT }))
            .child(button("12M", "chart-12", |s| { s.chart_months = 12; s.refresh = true; }, p).role("radio").checked(selected == 12).fill(if selected == 12 { p.selected } else { Color::TRANSPARENT }))))
        .child(columns).child(muted(format!("Flujo neto promedio: {}", money::format(metrics::net_average(&points))), p))
}
fn spending(o: &Overview, p: Palette) -> El {
    let mut c = card(p).child(label("Gasto por categoría", p)).child(muted(money::format(o.spending_total), p));
    if o.spending.is_empty() { c = c.child(muted("Sin gastos este mes", p)); }
    for item in &o.spending {
        let pct = if o.spending_total > 0 { item.spent as f64 / o.spending_total as f64 * 100.0 } else { 0.0 };
        c = c.child(row().w_full().justify_between().child(label(item.name.clone(), p)).child(muted(format!("{pct:.0}%"), p)))
            .child(bar(item.spent, o.spending_total, p.petroleum, p));
    }
    c
}
fn budget(o: &Overview, p: Palette) -> El {
    let ratio = if o.budget_assigned > 0 { o.budget_spent as f64 / o.budget_assigned as f64 } else { 0.0 };
    let tint = if ratio > 1.0 { p.coral } else if ratio >= 0.9 { p.amber } else { p.petroleum };
    let mut c = card(p).child(label("Presupuesto del mes", p))
        .child(text(money::format(o.budget_spent)).font(type_::TITLE).color(p.ink))
        .child(muted(format!("de {}", money::format(o.budget_assigned)), p))
        .child(bar(o.budget_spent, o.budget_assigned, tint, p))
        .child(muted(if o.budget_assigned == 0 { "Sin asignar".into() } else { format!("{:.0}% usado · {} {}", ratio * 100.0, money::format((o.budget_assigned - o.budget_spent).abs()), if ratio > 1.0 { "excedido" } else { "restante" }) }, p));
    for item in &o.budget_progress {
        c = c.child(row().w_full().justify_between().child(label(item.name.clone(), p)).child(muted(format!("{} / {}", money::format(item.spent), money::format(item.budgeted)), p)));
    }
    c
}
fn recent(o: &Overview, p: Palette) -> El {
    let mut c = card(p).child(label("Movimientos recientes", p));
    c = c.child(row().w_full().gap(8.0).child(muted("Descripción", p).w(160.0)).child(muted("Categoría", p).w(135.0)).child(muted("Cuenta", p).w(160.0)).child(muted("Fecha", p).w(90.0)).child(muted("Monto", p).w(100.0)));
    for e in &o.recent_entries {
        c = c.child(row().w_full().gap(8.0)
            .child(label(e.payee_name.as_deref().filter(|v| !v.is_empty()).unwrap_or(if e.notes.is_empty() { "Movimiento" } else { &e.notes }).to_owned(), p).w(160.0))
            .child(label(e.category_name.as_deref().unwrap_or("Sin categoría").to_owned(), p).w(135.0))
            .child(label(e.account_name.as_deref().unwrap_or("—").to_owned(), p).w(160.0))
            .child(label(e.date.format("%d.%m.%Y").to_string(), p).w(90.0))
            .child(text(money::format(e.amount)).font(type_::BODY).color(if e.amount < 0 { p.coral } else { p.petroleum }).w(100.0)));
    }
    c
}
pub fn render(s: &State, p: Palette) -> El {
    let mut content = col().w_full().padding(32.0).gap(16.0)
        .child(row().w_full().center_y().justify_between()
            .child(col().gap(5.0).child(text("Resumen").font(type_::TITLE).color(p.ink))
                .child(text(month::spanish_title(s.month)).key("month-title").font(type_::BODY).color(p.muted)))
            .child(row().gap(6.0).child(button("Mes anterior", "previous-month", |s| s.set_month(month::previous(s.month)), p))
                .child(button("Hoy", "today", |s| s.set_month(s.today), p))
                .child(button("Mes siguiente", "next-month", |s| s.set_month(month::next(s.month)), p))));
    if let Some(o) = &s.overview {
        let first = &o.first_run;
        if !(first.has_account && first.has_categorized_income && first.has_budget) {
            let mut guide = card(p).child(label("Primeros pasos", p));
            for (done, name) in [(first.has_account, "Crea una cuenta"), (first.has_categorized_income, "Registra tu ingreso"), (first.has_budget, "Asígnalo en el presupuesto")] {
                guide = guide.child(muted(format!("{}  {}", if done { "✓" } else { "○" }, name), p));
            }
            content = content.child(guide);
        }
        let savings = o.current.savings_rate.map_or("—".to_owned(), |v| format!("{:.0}%", v * 100.0));
        content = content.child(row().w_full().gap(12.0)
            .child(stat("Saldo total", money::format(o.total_balance), format!("{} cuenta(s) abierta(s)", o.open_account_count), true, p))
            .child(stat("Ingresos del mes", money::format(o.current.income), metrics::change_badge(o.income_change), false, p))
            .child(stat("Gastos del mes", money::format(o.current.expenses), metrics::change_badge(o.expenses_change), false, p))
            .child(stat("Tasa de ahorro", savings, if o.current.savings_rate.is_some() { metrics::change_badge(o.savings_rate_change) } else { "Sin ingresos este mes".into() }, false, p)))
            .child(row().w_full().gap(12.0).child(chart(o, s.chart_months, p)).child(spending(o, p)))
            .child(recent(o, p)).child(budget(o, p));
    } else { content = content.child(muted("Cargando…", p)); }
    col().grow(1.0).h_full().min_h(0.0).scroll(rune::ScrollSpec { stick_bottom: false }).child(content)
}
