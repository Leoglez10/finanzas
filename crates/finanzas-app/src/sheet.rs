use super::*;
use finanzas_core::store::EntryRow;
use rune::{PopupItem, PopupMenu, text_input};

pub struct Sheet {
    pub id: Option<Id>, pub expense: bool, pub cleared: bool, pub amount: TextEdit, pub account: Option<Id>,
    pub payee: TextEdit, pub category: Option<Id>, pub date: TextEdit, pub notes: TextEdit,
}
impl Sheet {
    pub fn new(s: &State, expense: bool, entry: Option<&EntryRow>) -> Self {
        let account = entry.and_then(|e| e.account_id).or_else(|| match s.route { Route::Cuenta(id) if s.accounts.iter().any(|a| a.account.id == id && !a.account.closed) => Some(id), _ => s.accounts.iter().find(|a| !a.account.closed).map(|a| a.account.id) });
        let date = entry.map(|e| e.date.format("%d/%m/%Y").to_string()).unwrap_or_else(|| if s.dark.is_some() { "10/11/2026".into() } else { Local::now().format("%d/%m/%Y").to_string() });
        let amount = entry.map(|e| format!("{}.{:02}", e.amount.unsigned_abs() / 100, e.amount.unsigned_abs() % 100)).unwrap_or_default();
        let category = if entry.is_none() && !expense { s.categories.iter().find(|c| !c.hidden && s.groups.iter().any(|g| g.is_income && Some(g.id) == c.group_id)).map(|c| c.id) } else { entry.and_then(|e| e.category_id) };
        Self { id: entry.map(|e| e.id), expense: entry.map_or(expense, |e| e.amount < 0), cleared: entry.is_none_or(|e| e.cleared), amount: TextEdit::new(amount), account,
            payee: TextEdit::new(entry.and_then(|e| e.payee_name.as_deref()).unwrap_or("")), category,
            date: TextEdit::new(date), notes: TextEdit::new(entry.map(|e| e.notes.as_str()).unwrap_or("")) }
    }
    pub fn valid(&self) -> bool { entry_helpers::valid_sheet(&self.amount.text, self.account, &self.date.text) }
}
pub fn field(label: &str, input: El, p: Palette) -> El {
    col().w_full().gap(5.0).child(text(label.to_owned()).font(type_::BODY).color(p.ink)).child(input.padding(9.0).w_full().fill(p.canvas).border(1.0, p.border).radius(8.0))
}
pub fn button(name: &str, key: &str, p: Palette) -> El {
    el().key(key).role("button").label(name).focusable(true).padding(10.0).radius(8.0).fill(p.selected).hover(p.border).child(text(name.to_owned()).font(type_::BODY).color(p.ink))
}
pub fn confirm(s: &State, p: Palette) -> El {
    let mut content = col().w(420.0).padding(24.0).gap(18.0).radius(18.0).fill(p.card)
        .child(text("¿Eliminar este movimiento?").font(type_::TITLE).color(p.ink));
    if let Some(entry) = s.confirm.and_then(|id| s.visible_entry(id)) {
        content = content.child(text(format!("{} · {} · {} · {}", entry.date.format("%d/%m/%Y"), entry.account_name.as_deref().unwrap_or("Sin cuenta"), entry.payee_name.as_deref().unwrap_or("Sin comercio"), money::format(entry.amount))).font(type_::BODY).color(p.ink));
    }
    if let Some(error) = &s.mutation_error { content = content.child(text(error.clone()).font(type_::BODY).color(p.coral)); }
    content = content.child(row().justify_between().child(button("Cancelar", "delete-cancel", p).on_press(|s: &mut State| if !s.mutation_active() { s.confirm = None }))
        .child(button("Eliminar", "delete-confirm", p).on_press(|s: &mut State| s.delete_selected())));
    let content = if s.mutation_active() { content.inert() } else { content };
    el().key("confirm-delete").paint_order(100).raise().absolute(0.0, 0.0).w_full().h_full().center().fill(Color::BLACK.alpha(0.4)).modal(false, |s: &mut State| if !s.mutation_active() { s.confirm = None }).child(content)
}
pub fn render(s: &State, p: Palette, dark: bool) -> El {
    let sheet = s.sheet.as_ref().expect("sheet present");
    let account_label = s.accounts.iter().find(|a| Some(a.account.id) == sheet.account).map_or("Selecciona una cuenta", |a| a.account.name.as_str());
    let category_label = s.categories.iter().find(|c| Some(c.id) == sheet.category).map_or("Sin categoría", |c| c.name.as_str());
    let mut account_picker = button(account_label, "account-picker", p);
    account_picker = account_picker.on_press_tx(|s: &mut State, tx| {
        if s.mutation_active() { return; }
        let mut menu = PopupMenu::new();
        for a in s.accounts.iter().filter(|a| !a.account.closed) { menu = menu.item(PopupItem::new(a.account.id.to_string(), a.account.name.clone())); }
        tx.popup_menu(menu, "account-picker", |s: &mut State, id, _| if !s.mutation_active() && let (Some(sheet), Some(id)) = (&mut s.sheet, id) { sheet.account = id.parse().ok(); });
    });
    let category_picker = button(category_label, "category-picker", p).on_press_tx(|s: &mut State, tx| {
        if s.mutation_active() { return; }
        let mut menu = PopupMenu::new().item(PopupItem::new("none", "Sin categoría"));
        for group in &s.groups {
            let mut sub = PopupMenu::new();
            for category in s.categories.iter().filter(|c| !c.hidden && c.group_id == Some(group.id)) { sub = sub.item(PopupItem::new(category.id.to_string(), category.name.clone())); }
            menu = menu.submenu(group.name.clone(), sub);
        }
        tx.popup_menu(menu, "category-picker", |s: &mut State, id, _| if !s.mutation_active() && let (Some(sheet), Some(id)) = (&mut s.sheet, id) { sheet.category = id.parse().ok(); });
    });
    let toggle = |name: &'static str, expense: bool| button(name, name, p).fill(if sheet.expense == expense { p.selected } else { Color::TRANSPARENT })
        .on_press(move |s: &mut State| { if s.mutation_active() { return; } let category = if expense { None } else { s.categories.iter().find(|c| !c.hidden && s.groups.iter().any(|g| Some(g.id) == c.group_id && g.is_income)).map(|c| c.id) }; if let Some(sh) = &mut s.sheet { sh.expense = expense; sh.category = category; } });
    let amount = text_input(&sheet.amount, type_::BODY).key("entry-amount").label("Monto").autofocus().single_line().color(p.ink).on_edit(|s: &mut State| if s.mutation_active() { &mut s.blocked_edit } else { &mut s.sheet.as_mut().unwrap().amount }).on_submit(|s: &mut State, _| s.submit());
    let payee = text_input(&sheet.payee, type_::BODY).key("entry-payee").label("Comercio").single_line().color(p.ink).on_edit(|s: &mut State| if s.mutation_active() { &mut s.blocked_edit } else { &mut s.sheet.as_mut().unwrap().payee }).on_submit(|s: &mut State, _| s.submit());
    let date = text_input(&sheet.date, type_::BODY).key("entry-date").label("Fecha (dd/mm/aaaa)").single_line().color(p.ink).on_edit(|s: &mut State| if s.mutation_active() { &mut s.blocked_edit } else { &mut s.sheet.as_mut().unwrap().date }).on_submit(|s: &mut State, _| s.submit());
    let notes = text_input(&sheet.notes, type_::BODY).key("entry-notes").label("Notas").single_line().color(p.ink).on_edit(|s: &mut State| if s.mutation_active() { &mut s.blocked_edit } else { &mut s.sheet.as_mut().unwrap().notes }).on_submit(|s: &mut State, _| s.submit());
    let mut form = col().w(540.0).max_h(720.0).scroll(rune::ScrollSpec { stick_bottom: false }).padding(24.0).gap(12.0).fill(p.card).radius(18.0).border(1.0, p.border)
        .child(text(if sheet.id.is_some() { "Editar movimiento" } else { "Nuevo movimiento" }).font(type_::TITLE).color(p.ink))
        .child(row().gap(8.0).child(toggle("Gasto", true)).child(toggle("Ingreso", false)))
        .child(field("Monto", amount, p)).child(field("Cuenta", account_picker, p)).child(field("Comercio", payee, p));
    if !sheet.payee.text.is_empty() {
        for (name, category) in &s.suggestions {
            let n = name.clone(); let c = *category;
            form = form.child(button(name, &format!("suggest-{name}"), p).on_press(move |s: &mut State| if !s.mutation_active() && let Some(sh) = &mut s.sheet { sh.payee.replace_text(n.clone()); sh.category = c; }));
        }
    }
    form = form.child(field("Categoría", category_picker, p)).child(field("Fecha (dd/mm/aaaa)", date, p)).child(field("Notas", notes, p));
    if let Some(error) = &s.mutation_error { form = form.child(text(error.clone()).font(type_::BODY).color(p.coral)); }
    let save_ink = if dark { p.canvas } else { Color::WHITE };
    let mut save = el().key("entry-save").role("button").label("Guardar").focusable(true).padding(10.0).radius(8.0).fill(p.border).child(text("Guardar").font(type_::BODY).color(save_ink));
    if sheet.valid() && !s.mutation_active() { save = save.fill(p.petroleum).default_button().on_press(|s: &mut State| s.submit()); } else { save = save.disabled(true); }
    form = form.child(row().w_full().justify_between().child(button("Cancelar", "entry-cancel", p).on_press(|s: &mut State| if !s.mutation_active() { s.sheet = None })).child(save));
    let form = if s.mutation_active() { form.inert() } else { form };
    el().key("entry-sheet").paint_order(100).raise().absolute(0.0, 0.0).w_full().h_full().center().fill(Color::BLACK.alpha(0.4)).modal(false, |s: &mut State| if !s.mutation_active() { s.sheet = None }).child(form)
}
