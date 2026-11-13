use super::*;
use finanzas_core::{models::Frequency, service::ScheduleView};
use rune::{PopupItem, PopupMenu, text_input};
use sheet::{button, field};

const FREQUENCIES: [(Frequency, &str, &str); 4] = [(Frequency::Daily, "Diario", "freq-daily"), (Frequency::Weekly, "Semanal", "freq-weekly"), (Frequency::Monthly, "Mensual", "freq-monthly"), (Frequency::Yearly, "Anual", "freq-yearly")];

pub struct ScheduleSheet {
    pub id: Option<Id>, pub expense: bool, pub name: TextEdit, pub amount: TextEdit, pub account: Option<Id>, pub payee: TextEdit,
    pub category: Option<Id>, pub frequency: Frequency, pub interval: u32, pub start: TextEdit, pub skip_weekend: bool, pub auto_post: bool,
}
impl ScheduleSheet {
    pub fn new(s: &State, view: Option<&ScheduleView>) -> Self {
        let open = |id: Id| s.accounts.iter().any(|a| a.account.id == id && !a.account.closed);
        let first_open = s.accounts.iter().find(|a| !a.account.closed).map(|a| a.account.id);
        let Some(v) = view else {
            return Self { id: None, expense: true, name: TextEdit::new(""), amount: TextEdit::new(""), account: first_open, payee: TextEdit::new(""), category: None,
                frequency: Frequency::Monthly, interval: 1, start: TextEdit::new(s.today_date.format("%d/%m/%Y").to_string()), skip_weekend: false, auto_post: false };
        };
        let sc = &v.schedule;
        let start = sc.recurrence.as_ref().map(|r| r.start).or(sc.next_date).unwrap_or(s.today_date);
        let units = sc.amount.unsigned_abs();
        Self { id: Some(sc.id), expense: sc.amount < 0, name: TextEdit::new(sc.name.clone()), amount: TextEdit::new(format!("{}.{:02}", units / 100, units % 100)),
            account: sc.account_id.filter(|id| open(*id)).or(first_open), payee: TextEdit::new(v.payee.clone().unwrap_or_default()), category: sc.category_id,
            frequency: sc.recurrence.as_ref().map_or(Frequency::Monthly, |r| r.frequency), interval: sc.recurrence.as_ref().map_or(1, |r| r.interval.clamp(1, 52)),
            start: TextEdit::new(start.format("%d/%m/%Y").to_string()), skip_weekend: sc.recurrence.as_ref().is_some_and(|r| r.skip_weekend), auto_post: sc.auto_post }
    }
    pub fn valid(&self) -> bool { self.draft().is_some() }
    /// `None` until the name, amount, account and first date are all usable.
    pub fn draft(&self) -> Option<data::ScheduleDraft> {
        let cents = money::amount_from(&self.amount.text)?;
        Some(data::ScheduleDraft { name: Some(self.name.text.trim()).filter(|n| !n.is_empty())?.to_owned(), amount: if self.expense { -cents } else { cents }, account_id: self.account?,
            category_id: self.category, payee: self.payee.text.clone(), frequency: self.frequency, interval: self.interval.clamp(1, 52),
            start: entry_helpers::parse_date(&self.start.text)?, skip_weekend: self.skip_weekend, auto_post: self.auto_post })
    }
}

pub fn confirm(s: &State, p: Palette) -> El {
    let mut content = col().w(420.0).padding(24.0).gap(18.0).radius(18.0).fill(p.card)
        .child(text("¿Eliminar este pago programado?").font(type_::TITLE).color(p.ink));
    if let Some(v) = s.schedule_confirm.and_then(|id| s.visible_schedule(id)) {
        let account = v.account.as_deref().unwrap_or("Sin cuenta");
        content = content.child(text(format!("{} · {} · {}", v.schedule.name, account, money::format(v.schedule.amount))).key("schedule-confirm-details").font(type_::BODY).color(p.ink))
            .child(text("Los movimientos ya registrados se conservan.").font(type_::BODY).color(p.muted));
    }
    if let Some(error) = &s.mutation_error { content = content.child(text(error.clone()).font(type_::BODY).color(p.coral)); }
    content = content.child(row().justify_between().child(button("Cancelar", "schedule-delete-cancel", p).on_press(|s: &mut State| if !s.mutation_active() { s.schedule_confirm = None }))
        .child(button("Eliminar", "schedule-delete-confirm", p).on_press(|s: &mut State| s.delete_schedule_confirmed())));
    let content = if s.mutation_active() { content.inert() } else { content };
    el().key("confirm-schedule-delete").paint_order(100).raise().absolute(0.0, 0.0).w_full().h_full().center().fill(Color::BLACK.alpha(0.4)).modal(false, |s: &mut State| if !s.mutation_active() { s.schedule_confirm = None }).child(content)
}

fn edit_target(s: &mut State, pick: fn(&mut ScheduleSheet) -> &mut TextEdit) -> &mut TextEdit {
    if s.mutation_active() || s.schedule_sheet.is_none() { return &mut s.blocked_edit; }
    pick(s.schedule_sheet.as_mut().expect("checked above"))
}
pub fn toggle(label: &str, key: &str, on: bool, p: Palette) -> El {
    let mark = el().size(18.0, 18.0).radius(5.0).border(1.0, if on { p.petroleum } else { p.muted }).fill(if on { p.petroleum } else { Color::TRANSPARENT }).center()
        .child(text(if on { "✓" } else { "" }.to_owned()).font(type_::BODY).color(p.canvas));
    row().key(key).role("checkbox").label(label).checked(on).focusable(true).gap(10.0).center_y().padding(8.0).radius(8.0).hover(p.selected)
        .child(mark).child(text(label.to_owned()).font(type_::BODY).color(p.ink))
}

pub fn render(s: &State, p: Palette, dark: bool) -> El {
    let sh = s.schedule_sheet.as_ref().expect("schedule sheet present");
    let account_label = s.accounts.iter().find(|a| Some(a.account.id) == sh.account).map_or("Selecciona una cuenta", |a| a.account.name.as_str());
    let category_label = s.categories.iter().find(|c| Some(c.id) == sh.category).map_or("Sin categoría", |c| c.name.as_str());
    let account_picker = button(account_label, "schedule-account", p).on_press_tx(|s: &mut State, tx| {
        if s.mutation_active() { return; }
        let mut menu = PopupMenu::new();
        for a in s.accounts.iter().filter(|a| !a.account.closed) { menu = menu.item(PopupItem::new(a.account.id.to_string(), a.account.name.clone())); }
        tx.popup_menu(menu, "schedule-account", |s: &mut State, id, _| if !s.mutation_active() && let (Some(sh), Some(id)) = (&mut s.schedule_sheet, id) { sh.account = id.parse().ok(); });
    });
    let category_picker = button(category_label, "schedule-category", p).on_press_tx(|s: &mut State, tx| {
        if s.mutation_active() { return; }
        let mut menu = PopupMenu::new().item(PopupItem::new("none", "Sin categoría"));
        for group in &s.groups {
            let mut sub = PopupMenu::new();
            for category in s.categories.iter().filter(|c| !c.hidden && c.group_id == Some(group.id)) { sub = sub.item(PopupItem::new(category.id.to_string(), category.name.clone())); }
            menu = menu.submenu(group.name.clone(), sub);
        }
        tx.popup_menu(menu, "schedule-category", |s: &mut State, id, _| if !s.mutation_active() && let (Some(sh), Some(id)) = (&mut s.schedule_sheet, id) { sh.category = id.parse().ok(); });
    });
    let kind = |name: &'static str, expense: bool| button(name, name, p).fill(if sh.expense == expense { p.selected } else { Color::TRANSPARENT })
        .on_press(move |s: &mut State| if !s.mutation_active() && let Some(sh) = &mut s.schedule_sheet { sh.expense = expense; });
    let input = |key: &'static str, label: &'static str, edit: &TextEdit, pick: fn(&mut ScheduleSheet) -> &mut TextEdit| {
        text_input(edit, type_::BODY).key(key).label(label).single_line().color(p.ink)
            .on_edit(move |s: &mut State| edit_target(s, pick)).on_submit(|s: &mut State, _| s.submit_schedule())
    };
    let mut frequency = row().gap(6.0);
    for (value, name, key) in FREQUENCIES {
        frequency = frequency.child(button(name, key, p).role("radio").checked(sh.frequency == value).fill(if sh.frequency == value { p.selected } else { Color::TRANSPARENT })
            .on_press(move |s: &mut State| if !s.mutation_active() && let Some(sh) = &mut s.schedule_sheet { sh.frequency = value; }));
    }
    let stepper = row().gap(8.0).center_y()
        .child(button("−", "interval-minus", p).label("Menos").on_press(|s: &mut State| if !s.mutation_active() && let Some(sh) = &mut s.schedule_sheet { sh.interval = sh.interval.saturating_sub(1).max(1); }))
        .child(text(format!("Cada {}", sh.interval)).key("interval-value").font(type_::BODY).color(p.ink))
        .child(button("+", "interval-plus", p).label("Más").on_press(|s: &mut State| if !s.mutation_active() && let Some(sh) = &mut s.schedule_sheet { sh.interval = (sh.interval + 1).min(52); }));
    let pair = |left: El, right: El| row().w_full().gap(12.0).child(col().grow(1.0).min_w(0.0).child(left)).child(col().grow(1.0).min_w(0.0).child(right));
    let mut body = col().w_full().max_h(560.0).scroll(rune::ScrollSpec { stick_bottom: false }).gap(12.0).px(24.0)
        .child(row().gap(8.0).child(kind("Gasto", true)).child(kind("Ingreso", false)))
        .child(pair(field("Nombre", input("schedule-name", "Nombre", &sh.name, |sh| &mut sh.name), p), field("Monto", input("schedule-amount", "Monto", &sh.amount, |sh| &mut sh.amount).autofocus(), p)))
        .child(pair(field("Cuenta", account_picker, p), field("Categoría", category_picker, p)))
        .child(pair(field("Comercio", input("schedule-payee", "Comercio", &sh.payee, |sh| &mut sh.payee), p), field("Primera fecha (dd/mm/aaaa)", input("schedule-date", "Primera fecha (dd/mm/aaaa)", &sh.start, |sh| &mut sh.start), p)))
        .child(text("Repetición").font(type_::SECTION).color(p.muted).pt(6.0))
        .child(pair(field("Se repite", frequency, p), field("Cada", stepper, p)))
        .child(toggle("Si cae en fin de semana, pasar al lunes", "skip-weekend", sh.skip_weekend, p).on_press(|s: &mut State| if !s.mutation_active() && let Some(sh) = &mut s.schedule_sheet { sh.skip_weekend = !sh.skip_weekend; }))
        .child(text("Automatización").font(type_::SECTION).color(p.muted).pt(6.0))
        .child(toggle("Registrar automáticamente", "auto-post", sh.auto_post, p).on_press(|s: &mut State| if !s.mutation_active() && let Some(sh) = &mut s.schedule_sheet { sh.auto_post = !sh.auto_post; }))
        .child(text("Los pagos automáticos se registran al abrir la app y cada hora, sin confirmación adicional.").font(type_::BODY).color(p.muted).pb(8.0));
    if let Some(error) = &s.mutation_error { body = body.child(text(error.clone()).key("schedule-error").font(type_::BODY).color(p.coral)); }
    let save_ink = if dark { p.canvas } else { Color::WHITE };
    let mut save = el().key("schedule-save").role("button").label("Guardar").focusable(true).padding(10.0).radius(8.0).fill(p.border).child(text("Guardar").font(type_::BODY).color(save_ink));
    if sh.valid() && !s.mutation_active() { save = save.fill(p.petroleum).default_button().on_press(|s: &mut State| s.submit_schedule()); } else { save = save.disabled(true); }
    let form = col().w(720.0).py(24.0).gap(14.0).fill(p.card).radius(18.0).border(1.0, p.border)
        .child(col().gap(4.0).px(24.0).child(text(if sh.id.is_some() { "Editar pago programado" } else { "Nuevo pago programado" }).key("schedule-sheet-title").font(type_::TITLE).color(p.ink))
            .child(text("Organiza los pagos que se repiten").font(type_::BODY).color(p.muted)))
        .child(body)
        .child(row().w_full().px(24.0).justify_between().child(button("Cancelar", "schedule-cancel", p).on_press(|s: &mut State| if !s.mutation_active() { s.schedule_sheet = None })).child(save));
    let form = if s.mutation_active() { form.inert() } else { form };
    el().key("schedule-sheet").paint_order(100).raise().absolute(0.0, 0.0).w_full().h_full().center().fill(Color::BLACK.alpha(0.4)).modal(false, |s: &mut State| if !s.mutation_active() { s.schedule_sheet = None }).child(form)
}
