//! "Nueva cuenta": name, kind (on or off budget) and an optional opening balance.
use super::*;
use rune::text_input;
use sheet::{button, field};

pub struct AccountSheet { pub name: TextEdit, pub opening: TextEdit, pub off_budget: bool }
impl AccountSheet {
    pub fn new() -> Self { Self { name: TextEdit::new(""), opening: TextEdit::new(""), off_budget: false } }
    /// Blank means no opening balance; a leading `-` makes it a debt. `Err` while the text is not an amount.
    fn opening_cents(&self) -> Result<Option<i64>, ()> {
        let text = self.opening.text.trim();
        if text.is_empty() { return Ok(None); }
        let (negative, digits) = text.strip_prefix('-').map_or((false, text), |rest| (true, rest));
        if digits.contains('-') { return Err(()); }
        money::cents_from(digits).map(|cents| Some(if negative { -cents } else { cents })).ok_or(())
    }
    pub fn draft(&self, today: NaiveDate) -> Option<data::AccountDraft> {
        let name = Some(self.name.text.trim()).filter(|n| !n.is_empty())?.to_owned();
        Some(data::AccountDraft { name, off_budget: self.off_budget, opening: self.opening_cents().ok()?, date: today })
    }
}

fn edit_target(s: &mut State, pick: fn(&mut AccountSheet) -> &mut TextEdit) -> &mut TextEdit {
    if s.mutation_active() || s.account_sheet.is_none() { return &mut s.blocked_edit; }
    pick(s.account_sheet.as_mut().expect("checked above"))
}

pub fn render(s: &State, p: Palette, dark: bool) -> El {
    let sh = s.account_sheet.as_ref().expect("account sheet present");
    let input = |key: &'static str, label: &'static str, edit: &TextEdit, pick: fn(&mut AccountSheet) -> &mut TextEdit| {
        text_input(edit, type_::BODY).key(key).label(label).single_line().color(p.ink)
            .on_edit(move |s: &mut State| edit_target(s, pick)).on_submit(|s: &mut State, _| s.submit_account())
    };
    let kind = |name: &'static str, key: &'static str, off_budget: bool| button(name, key, p).role("radio").checked(sh.off_budget == off_budget)
        .fill(if sh.off_budget == off_budget { p.selected } else { Color::TRANSPARENT })
        .on_press(move |s: &mut State| if !s.mutation_active() && let Some(sh) = &mut s.account_sheet { sh.off_budget = off_budget; });
    let mut form = col().w(480.0).padding(24.0).gap(14.0).fill(p.card).radius(18.0).border(1.0, p.border)
        .child(col().gap(4.0).child(text("Nueva cuenta").key("account-sheet-title").font(type_::TITLE).color(p.ink))
            .child(text("Agrega una cuenta para registrar tus movimientos").font(type_::BODY).color(p.muted)))
        .child(field("Nombre", input("account-name", "Nombre", &sh.name, |sh| &mut sh.name).autofocus(), p))
        .child(field("Tipo", row().gap(8.0).child(kind("En presupuesto", "account-type-on", false)).child(kind("Fuera del presupuesto", "account-type-off", true)), p))
        .child(text(if sh.off_budget { "Inversiones, hipoteca…: su saldo cuenta en el total, pero sus movimientos no entran al presupuesto." } else { "Sus movimientos cuentan en el presupuesto." }).font(type_::BODY).color(p.muted).wrap().w_full())
        .child(field("Saldo inicial (opcional, usa - para una deuda)", input("account-opening", "Saldo inicial", &sh.opening, |sh| &mut sh.opening), p));
    if let Some(error) = &s.mutation_error { form = form.child(text(error.clone()).key("account-error").font(type_::BODY).color(p.coral)); }
    let save_ink = if dark { p.canvas } else { Color::WHITE };
    let mut save = el().key("account-save").role("button").label("Crear").focusable(true).padding(10.0).radius(8.0).fill(p.border).child(text("Crear").font(type_::BODY).color(save_ink));
    if sh.draft(s.today_date).is_some() && !s.mutation_active() { save = save.fill(p.petroleum).default_button().on_press(|s: &mut State| s.submit_account()); } else { save = save.disabled(true); }
    form = form.child(row().wrap().w_full().justify_between().child(button("Cancelar", "account-cancel", p).on_press(|s: &mut State| if !s.mutation_active() { s.account_sheet = None })).child(save));
    let form = if s.mutation_active() { form.inert() } else { form };
    el().key("account-sheet").paint_order(100).raise().absolute(0.0, 0.0).wrap().w_full().h_full().center().fill(Color::BLACK.alpha(0.4)).modal(false, |s: &mut State| if !s.mutation_active() { s.account_sheet = None }).child(form)
}
