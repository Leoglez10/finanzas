use super::*;
use finanzas_core::service::{BudgetCategory, BudgetGroup, BudgetView};
use overview::{button, card, label, muted};
use theme::{badge, header_bar, headline, icon_button, money_big, titles};
use rune::text_input;

const COLUMN: f32 = 110.0;

/// Text shown in an Asignado field: whole amounts without decimals.
pub fn input_text(cents: i64) -> String {
    if cents % 100 == 0 { (cents / 100).to_string() } else { format!("{}.{:02}", cents / 100, cents % 100) }
}

fn cell(content: El) -> El { row().w(COLUMN).justify_end().center_y().child(content) }
fn amount(value: i64, color: Color) -> El { text(money::format(value)).font(type_::BODY).color(color) }
fn available_color(value: i64, p: Palette) -> Color { if value < 0 { p.coral } else if value > 0 { p.emerald } else { p.muted } }
fn visible(group: &BudgetGroup) -> Vec<&BudgetCategory> { group.categories.iter().filter(|c| !c.hidden).collect() }

fn hero(b: &BudgetView, p: Palette) -> El {
    let negative = b.to_budget < 0;
    let (fg, note) = if negative { (p.coral, p.muted) } else { (p.card, p.card.alpha(0.85)) };
    card(p).grow(2.0).fill(if negative { p.card } else { p.petroleum }).border(1.0, if negative { p.coral } else { p.petroleum })
        .child(row().w_full().justify_between()
            .child(col().gap(8.0).min_w(0.0)
                .child(text("Disponible para asignar").font(theme::CARD_TITLE).color(fg))
                .child(text(money::format(b.to_budget)).key("to-assign").font(theme::HERO).color(fg))
                .child(text(b.hint.clone()).font(type_::BODY).color(note).wrap()))
            .child(icon(&theme::PIE_ART, 64.0)))
}
fn stat(title: &str, value: i64, glyph: &'static rune::Icon, tint: Color, p: Palette) -> El {
    card(p).child(row().gap(10.0).center_y().child(badge(glyph, 14.0, tint)).child(muted(title, p))).child(money_big(value, false, p.ink, p))
}

fn category_row(s: &State, c: &BudgetCategory, income: bool, p: Palette) -> El {
    let id = c.id;
    let line = row().w_full().gap(8.0).px(16.0).py(8.0).center_y()
        .child(col().grow(1.0).min_w(0.0).gap(4.0)
            .child({ let name = c.name.clone(); theme::tappable(label(c.name.clone(), p).truncate().px(4.0).py(1.0), p).key(format!("budget-name-{id}"))
                .help(format!("Ver movimientos de {}", c.name)).on_press(move |s: &mut State| s.show_movements(&name)) })
            .child(if income { el().h(0.0) } else { progress(c.spent_share, p) }));
    if income { return line.child(cell(amount(c.activity, p.emerald))); }
    let failed = s.budget_error.as_ref().is_some_and(|(row, _)| *row == Some(id));
    let field = match s.budget_inputs.get(&id) {
        Some((edit, _)) => text_input(edit, type_::BODY).key(format!("budget-input-{id}")).label(format!("Asignado {}", c.name)).single_line().color(p.ink)
            .on_edit(move |s: &mut State| s.budget_edit(id)).on_submit(move |s: &mut State, _| s.commit_budget(id))
            .padding(8.0).w(COLUMN).fill(p.canvas).border(1.0, if failed { p.coral } else { p.border }).radius(8.0),
        None => el().w(COLUMN),
    };
    line.child(field).child(cell(amount(c.activity, p.muted))).child(cell(amount(c.available, available_color(c.available, p))))
}
fn progress(share: f64, p: Palette) -> El {
    let f = share.clamp(0.0, 1.0) as f32;
    row().w_full().h(6.0).radius(3.0).fill(p.border)
        .child(el().grow(f).h(6.0).radius(3.0).fill(if share > 1.0 { p.coral } else if share >= 0.9 { p.amber } else { p.petroleum }))
        .child(el().grow(1.0 - f).h(5.0))
}

fn group_card(s: &State, g: &BudgetGroup, p: Palette) -> El {
    let rows = visible(g);
    let sum = |f: fn(&BudgetCategory) -> i64| rows.iter().map(|c| f(c)).sum::<i64>();
    let (budgeted, activity, available) = (sum(|c| c.budgeted), sum(|c| c.activity), sum(|c| c.available));
    let mut header = row().w_full().gap(8.0).px(16.0).py(10.0).center_y()
        .child(row().grow(1.0).min_w(0.0).gap(8.0).center_y().child(theme::dot(if g.is_income { p.emerald } else { p.petroleum }))
            .child(text(g.name.clone()).font(theme::CARD_TITLE).color(p.ink).truncate()));
    header = if g.is_income { header.child(cell(amount(activity, p.emerald).key(format!("group-activity-{}", g.id)))) } else {
        header.child(cell(amount(budgeted, p.ink).key(format!("group-budgeted-{}", g.id)))).child(cell(amount(activity, p.ink).key(format!("group-activity-{}", g.id))))
            .child(cell(amount(available, available_color(available, p)).key(format!("group-available-{}", g.id))))
    };
    let mut c = col().w_full().gap(0.0).radius(18.0).fill(p.card).border(1.0, p.border).py(8.0).child(header);
    for category in rows { c = c.child(el().w_full().h(1.0).fill(p.border)).child(category_row(s, category, g.is_income, p)); }
    c
}

pub fn render(s: &State, p: Palette) -> El {
    let mut content = col().w_full().padding(24.0).gap(16.0)
        .child(header_bar(p)
            .child(icon_button(&theme::LEFT, "Mes anterior", "previous-month", |s| s.set_month(month::previous(s.month)), p))
            .child(icon_button(&theme::RIGHT, "Mes siguiente", "next-month", |s| s.set_month(month::next(s.month)), p))
            .child(titles("Presupuesto", headline(month::spanish_title(s.month), p).key("month-title"), p).pl(6.0))
            .child(el().grow(1.0))
            .child(if s.month == s.today { el() } else { button("Hoy", "today", |s| s.set_month(s.today), p) }));
    if let Some((_, error)) = &s.budget_error {
        content = content.child(text(error.clone()).key("budget-error").font(type_::BODY).color(p.coral));
    }
    let Some(b) = s.budget.as_ref().filter(|b| b.month == s.month) else {
        return col().grow(1.0).h_full().min_h(0.0).child(content.child(muted("Cargando…", p)));
    };
    content = content.child(row().w_full().gap(16.0).child(hero(b, p))
        .child(col().grow(1.0).gap(16.0).child(stat("Ingresos del mes", b.income, &theme::TREND_UP, p.emerald, p)).child(stat("Asignado", b.assigned, &theme::PIE, p.petroleum, p))));
    if b.uncategorized_income > 0 {
        content = content.child(row().w_full().padding(14.0).radius(14.0).fill(p.amber.alpha(0.12)).border(1.0, p.amber).child(text(format!("Tienes {} en ingresos sin categoría. Dales una categoría de ingreso en Movimientos para poder repartirlos.", money::format(b.uncategorized_income))).font(type_::BODY).color(p.amber)));
    }
    content = content.child(muted("Escribe cuánto asignas a cada categoría, o usa +100 / -50 para sumar o restar.", p))
        .child(row().w_full().gap(8.0).px(16.0).child(el().grow(1.0))
            .child(cell(muted("Asignado", p))).child(cell(muted("Actividad", p))).child(cell(muted("Disponible", p))));
    for g in b.groups.iter().filter(|g| !g.is_income && !visible(g).is_empty()) { content = content.child(group_card(s, g, p)); }
    for g in b.groups.iter().filter(|g| g.is_income && !visible(g).is_empty()) { content = content.child(group_card(s, g, p)); }
    col().grow(1.0).h_full().min_h(0.0).scroll(rune::ScrollSpec { stick_bottom: false }).child(content)
}
