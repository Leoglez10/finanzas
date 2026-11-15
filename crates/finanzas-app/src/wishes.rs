use super::*;
use finanzas_core::service::{Wish, WishDraft, WishView};
use rune::{text_input, PopupItem, PopupMenu};
use sheet::{button, field};

#[derive(Clone, Copy, PartialEq, Eq)]
enum WishStep { Purchase, Context }
type WishValues = (String, String, String, String, Option<Id>);

pub struct WishSheet {
    pub id: Option<Id>, pub name: TextEdit, pub price: TextEdit,
    pub url: TextEdit, pub notes: TextEdit, pub category: Option<Id>,
    step: WishStep, tried_purchase: bool, tried_save: bool,
    details_open: bool, discard_confirm: bool, initial: WishValues,
}
const NAME_ERROR: &str = "Escribe el nombre de lo que quieres comprar.";
const PRICE_ERROR: &str = "Escribe un precio mayor que cero. Usa 11,000 o 11,000.50; coma para miles y punto para decimales.";
const PRICE_HELP: &str = "Ej. 11,000.50 · coma para miles, punto para decimales.";
const URL_ERROR: &str = "Usa un enlace completo que empiece por https:// o deja este campo vacío.";
fn decimal(cents: i64) -> String { format!("{}.{:02}", cents / 100, cents % 100) }
// Preserve reservation-input behavior: grouping belongs only to target prices.
fn parse_amount(text: &str) -> Option<i64> {
    let text = text.trim();
    if text.is_empty() || !text.chars().all(|c| c.is_ascii_digit() || c == '.') || text.split('.').nth(1).is_some_and(|s| s.len() > 2) { return None; }
    money::cents_from(text)
}
fn parse_price(text: &str) -> Option<i64> {
    let text = text.trim();
    // Prices are displayed as "$11,000.00"; accept that same shape when pasted back.
    let text = text.strip_prefix('$').map_or(text, str::trim_start);
    let (whole, fraction) = text.split_once('.').map_or((text, None), |(w, f)| (w, Some(f)));
    let digits = |value: &str| !value.is_empty() && value.bytes().all(|c| c.is_ascii_digit());
    if fraction.is_some_and(|f| !digits(f) || f.len() > 2) { return None; }
    let mut groups = whole.split(',');
    let first = groups.next()?;
    if !digits(first) { return None; }
    if whole.contains(',') && (first.len() > 3 || !groups.all(|g| g.len() == 3 && digits(g))) { return None; }
    money::cents_from(text).filter(|cents| *cents > 0)
}
fn valid_url(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() { return true; }
    let Some(rest) = value.strip_prefix("https://").or_else(|| value.strip_prefix("http://")) else { return false };
    !rest.split(['/', '?', '#']).next().unwrap_or("").is_empty()
        && !value.chars().any(|c| c.is_whitespace() || c.is_control())
}
impl WishSheet {
    fn new(wish: Option<&Wish>) -> Self {
        let mut sheet = Self { id: wish.map(|w| w.id), name: TextEdit::new(wish.map_or("", |w| &w.name)),
            price: TextEdit::new(wish.map(|w| decimal(w.price)).unwrap_or_default()),
            url: TextEdit::new(wish.map_or("", |w| &w.url)), notes: TextEdit::new(wish.map_or("", |w| &w.notes)),
            category: wish.and_then(|w| w.category_id), step: WishStep::Purchase,
            tried_purchase: false, tried_save: false, details_open: wish.is_some_and(|w| !w.url.is_empty() || !w.notes.is_empty()),
            discard_confirm: false, initial: Default::default() };
        sheet.initial = sheet.values();
        sheet
    }
    fn values(&self) -> WishValues {
        (self.name.text.clone(), parse_price(&self.price.text).map(money::format).unwrap_or_else(|| self.price.text.clone()),
            self.url.text.clone(), self.notes.text.clone(), self.category)
    }
    fn first_control(&self) -> &'static str { if self.step == WishStep::Purchase { "wish-name" } else { "wish-category" } }
    fn trigger(&self) -> String { self.id.map_or_else(|| "wish-new".into(), |id| format!("wish-edit-{id}")) }
    fn name_error(&self) -> Option<&'static str> { (self.tried_purchase && self.name.text.trim().is_empty()).then_some(NAME_ERROR) }
    fn price_error(&self) -> Option<&'static str> { (self.tried_purchase && parse_price(&self.price.text).is_none()).then_some(PRICE_ERROR) }
    fn url_error(&self) -> Option<&'static str> { (self.tried_save && !valid_url(&self.url.text)).then_some(URL_ERROR) }
    fn draft(&self) -> Option<WishDraft> {
        if self.name.text.trim().is_empty() || !valid_url(&self.url.text) { return None; }
        Some(WishDraft { name:self.name.text.trim().into(), price:parse_price(&self.price.text)?,
            url:self.url.text.trim().into(), notes:self.notes.text.clone(), category_id:self.category })
    }
}
impl State {
    fn wishes_key_now(&self) -> WishesKey { (self.version, self.proximos_gen, self.today_date, self.wishes_archived) }
    fn wishes_fresh(&self) -> bool { self.route == Route::Deseos && self.wishes.is_some() && self.wishes_key == Some(self.wishes_key_now()) }
    fn visible_wish(&self, id: Id) -> Option<&Wish> {
        if !self.wishes_fresh() { return None; }
        self.wishes.as_ref()?.iter().find(|v| v.wish.id == id && !v.wish.archived).map(|v| &v.wish)
    }
    pub fn open_wish(&mut self, id: Option<Id>) {
        if self.mutation_active() || !self.wishes_fresh() || id.is_some_and(|id| self.visible_wish(id).is_none()) { return; }
        self.wish_sheet = Some(WishSheet::new(id.and_then(|id| self.visible_wish(id))));
        self.mutation_error = None;
        self.wish_saved = false;
    }
    fn next_wish(&mut self, tx: &mut Tx) {
        if self.mutation_active() { return; }
        let Some(sh) = &mut self.wish_sheet else { return };
        sh.tried_purchase = true;
        if sh.name_error().is_some() { tx.focus("wish-name", true); return; }
        let Some(price) = parse_price(&sh.price.text) else { tx.focus("wish-price", true); return };
        sh.price.replace_text(money::format(price).trim_start_matches('$'));
        sh.step = WishStep::Context;
        tx.focus("wish-category", true);
    }
    fn save_wish(&mut self, tx: &mut Tx) {
        if self.mutation_active() { return; }
        if self.wish_sheet.as_ref().is_some_and(|sh| sh.step == WishStep::Purchase) { self.next_wish(tx); return; }
        let Some(sh) = &mut self.wish_sheet else { return };
        sh.tried_save = true;
        sh.tried_purchase = true;
        if sh.name_error().is_some() || sh.price_error().is_some() {
            sh.step = WishStep::Purchase;
            tx.focus(if sh.name_error().is_some() { "wish-name" } else { "wish-price" }, true);
            return;
        }
        if sh.url_error().is_some() { sh.details_open = true; tx.focus("wish-url", true); return; }
        let Some(draft) = sh.draft() else { return };
        let id = sh.id;
        if !self.wishes_fresh() || id.is_some_and(|id| self.visible_wish(id).is_none()) {
            self.mutation_error = Some("Los datos cambiaron o se están actualizando. Revisa el deseo y vuelve a intentar.".into());
            return;
        }
        self.wish_mutation(data::Mutation::SaveWish(id, draft), MutationTarget::WishSheet);
    }
    pub fn request_wish_close(&mut self, tx: &mut Tx) {
        if self.mutation_active() { return; }
        let Some(sh) = &mut self.wish_sheet else { return };
        if sh.discard_confirm {
            sh.discard_confirm = false;
            tx.focus(sh.first_control(), true);
        } else if sh.values() != sh.initial {
            sh.discard_confirm = true;
            tx.focus("wish-keep-editing", true);
        } else { self.dismiss_wish(tx); }
    }
    fn dismiss_wish(&mut self, tx: &mut Tx) {
        if self.mutation_active() { return; }
        if let Some(sh) = self.wish_sheet.take() { tx.focus(sh.trigger(), true); }
    }
    fn wish_mutation(&mut self, mutation: data::Mutation, target: MutationTarget) {
        if self.mutation_active() || !self.wishes_fresh() { return; }
        self.mutation_error = None;
        self.wish_saved = false;
        self.mutation_target = Some(target);
        let worker = Arc::clone(&self.worker);
        self.mutation.request(mutation, move |m| data::mutate(worker,m));
    }
    fn reserve_wish(&mut self, id: Id, release: bool) {
        if self.visible_wish(id).is_none() { return; }
        let amount = if release { Some(0) } else { self.wish_inputs.get(&id).and_then(|e| parse_amount(&e.text)) };
        let Some(amount) = amount.filter(|a| *a >= 0) else { self.wish_saved = false; self.mutation_error = Some("Escribe un importe de cero o más, con hasta dos decimales.".into()); return };
        self.wish_mutation(data::Mutation::ReserveWish(id,amount,self.today_date),MutationTarget::WishRow);
    }
}

pub fn poll(s: &mut State, tx: &mut Tx, changed: &mut bool) {
    // The "Deseo guardado." status belongs to the save that just happened on this screen.
    if s.route != Route::Deseos { s.wish_saved = false; }
    if let Some((key,result)) = s.wishes_latest.take() && key == s.wishes_key_now() {
        match result {
            Ok(views) => {
                for view in &views { s.wish_inputs.entry(view.wish.id).or_insert_with(|| TextEdit::new(decimal(view.wish.reserved))); }
                s.wishes = Some(views); s.wishes_key = Some(key);
            }
            Err(e) => s.mutation_error = Some(e),
        }
        tx.instant(); *changed = true;
    }
    if s.route == Route::Deseos && s.version.is_some() {
        let key = s.wishes_key_now();
        let worker = Arc::clone(&s.worker);
        s.wishes_latest.request_if_changed(&key,move |key| {
            let result = (|| {
                let mut guard = worker.lock().map_err(|e| e.to_string())?;
                let store = guard.as_mut().ok_or("No se pudo abrir la base de datos")?;
                finanzas_core::service::list_wishes(store,key.2,key.3).map_err(|e| e.to_string())
            })();
            (key,result)
        });
    }
}
fn status(code: &str) -> &str {
    match code {
        "affordable" => "Cabe en tu presupuesto y saldo registrado",
        "budget_shortfall" => "Falta presupuesto para esta compra",
        "cash_shortfall" => "El saldo no cubre la compra y los pagos próximos",
        "reservation_deficit" => "Hay reservas sin respaldo en esta categoría",
        "budget_overassigned" => "Tu presupuesto tiene dinero asignado de más",
        "archived" => "Archivado · reserva liberada",
        _ => "Faltan datos para evaluar la compra",
    }
}
fn amount(value: Option<i64>) -> String { value.map(money::format).unwrap_or_else(|| "Sin datos".into()) }
fn wish_card(s: &State, v: &WishView, p: Palette) -> El {
    let w = &v.wish;
    let id = w.id;
    let category = s.categories.iter().find(|c| Some(c.id) == w.category_id).map_or("Sin categoría", |c| c.name.as_str());
    let mut card = col().w_full().gap(10.0).padding(20.0).radius(12.0).fill(p.card).border(1.0,p.border)
        .child(text(w.name.clone()).font(type_::TITLE).color(p.ink).wrap_anywhere().w_full())
        .child(text(format!("{} · {} · Apartado {}",money::format(w.price),category,money::format(w.reserved))).font(type_::BODY).color(p.ink).wrap().w_full())
        .child(text(status(&v.evaluation.status)).key(format!("wish-status-{id}")).font(type_::BODY).color(if v.evaluation.status == "affordable" { p.emerald } else { p.amber }).wrap().w_full())
        .child(text(format!("Te falta en presupuesto: {} · Te falta en saldo: {}",amount(v.evaluation.budget_shortfall),amount(v.evaluation.cash_shortfall))).font(type_::BODY).color(p.ink).wrap().w_full())
        .child(text(format!("Pagos registrados (30 días y vencidos): {}",money::format(v.evaluation.upcoming_outflows))).font(type_::BODY).color(p.muted).wrap().w_full());
    for reason in &v.evaluation.reasons { card = card.child(text(reason.clone()).font(type_::BODY).color(p.muted).wrap().w_full()); }
    if !w.notes.is_empty() { card = card.child(text(w.notes.clone()).font(type_::BODY).color(p.ink).wrap().w_full()); }
    if !w.url.is_empty() {
        let url = w.url.clone();
        card = card.child(button("Ver producto",&format!("wish-link-{id}"),p).help(w.url.clone()).on_press_tx(move |_: &mut State,tx| { let _ = tx.open_url(&url); }));
    }
    if !w.archived {
        let input = s.wish_inputs.get(&id).expect("inputs prepared with list");
        card = card.child(field("Importe total que quieres apartar (0 libera fondos)",text_input(input,type_::BODY).key(format!("wish-reserve-input-{id}")).label("Importe a apartar").single_line().color(p.ink).caret(p.petroleum,p.selected)
            .on_edit(move |s: &mut State| {
                if s.mutation_active() || !s.wishes_fresh() { return &mut s.blocked_edit; }
                s.wish_inputs.get_mut(&id).expect("visible wish input")
            }),p))
            .child(row().wrap().w_full().gap(8.0)
                .child(button("Apartar",&format!("wish-reserve-{id}"),p).on_press(move |s: &mut State| s.reserve_wish(id,false)))
                .child(button("Liberar",&format!("wish-release-{id}"),p).on_press(move |s: &mut State| s.reserve_wish(id,true)))
                .child(button("Editar",&format!("wish-edit-{id}"),p).on_press(move |s: &mut State| s.open_wish(Some(id))))
                .child(button("Archivar",&format!("wish-archive-{id}"),p).on_press(move |s: &mut State| {
                    if s.visible_wish(id).is_some() { s.wish_mutation(data::Mutation::ArchiveWish(id),MutationTarget::WishRow); }
                })));
    }
    if s.mutation_active() || !s.wishes_fresh() { card.inert() } else { card }
}
pub fn render(s: &State,p: Palette) -> El {
    let mut page = col().grow(1.0).min_w(0.0).h_full().padding(28.0).gap(16.0)
        .child(text("Deseos").font(type_::TITLE).color(p.ink))
        .child(text("Guarda lo que quieres comprar y aparta dinero sin registrar un gasto.").font(type_::BODY).color(p.muted).wrap().w_full())
        .child(row().wrap().w_full().gap(8.0).child(button("Nuevo deseo","wish-new",p).on_press(|s: &mut State| s.open_wish(None)))
            .child(button(if s.wishes_archived { "Ocultar archivados" } else { "Ver archivados" },"wish-history",p).on_press(|s: &mut State| {
                if !s.mutation_active() { s.wishes_archived = !s.wishes_archived; s.wish_saved = false; }
            })));
    if s.wish_saved { page = page.child(text("Deseo guardado.").key("wish-saved").role("status").font(type_::BODY).color(p.emerald)); }
    if let Some(error) = &s.mutation_error { page = page.child(text(error.clone()).key("wish-error").font(type_::BODY).color(p.coral).wrap().w_full()); }
    let mut list = col().w_full().grow(1.0).min_h(0.0).gap(14.0).scroll(rune::ScrollSpec {stick_bottom:false});
    if !s.wishes_fresh() { list = list.child(text("Cargando deseos y presupuesto…").key("wish-loading").font(type_::BODY).color(p.muted).wrap().w_full()); }
    else if let Some(views) = &s.wishes {
        if views.is_empty() { list = list.child(text("Todavía no tienes deseos. Añade una compra para conocer cuánto te falta.").key("wish-empty").font(type_::BODY).color(p.muted).wrap().w_full()); }
        for view in views { list = list.child(wish_card(s,view,p)); }
    }
    page.child(list)
}
fn edit(s: &mut State, pick: fn(&mut WishSheet)->&mut TextEdit) -> &mut TextEdit {
    if s.mutation_active() || s.wish_sheet.as_ref().is_none_or(|sh| sh.discard_confirm) { return &mut s.blocked_edit; }
    pick(s.wish_sheet.as_mut().expect("wish sheet present"))
}
fn help(value: &str, p: Palette) -> El { text(value.to_owned()).font(theme::HINT).color(p.ink).wrap().w_full() }
fn separator(p: Palette) -> El { el().w_full().h(1.0).fill(p.border) }
fn editor_field(key: &'static str, label: &str, value: &TextEdit,
    pick: fn(&mut WishSheet)->&mut TextEdit, error: Option<&str>, hint: &str, p: Palette) -> El {
    let mut input = text_input(value, if key == "wish-price" { theme::AMOUNT } else { type_::BODY })
        .key(key).label(label).description(error.unwrap_or(hint)).invalid(error.is_some()).color(p.ink).caret(p.petroleum,p.selected)
        .on_edit(move |s: &mut State| edit(s, pick));
    if key == "wish-notes" { input = input.h(96.0); }
    else { input = input.single_line().on_submit(|s: &mut State, tx| s.save_wish(tx)); }
    if key == "wish-name" { input = input.autofocus(); }
    // Rune draws input text from the content box's top edge, so only equal vertical padding
    // centres it: the artboard's 9pt band around a 13pt line inside a 44pt control is 13pt.
    let (pad_y, pad_x) = match key { "wish-notes" => (9.0, 12.0), "wish-price" => (5.0, 12.0), _ => (13.0, 12.0) };
    let mut field = col().w_full().gap(5.0).child(text(label.to_owned()).font(type_::BODY).color(p.ink))
        .child(input.w_full().min_h(44.0).px(pad_x).py(pad_y).radius(8.0).fill(p.canvas).border(1.0, if error.is_some() { p.coral } else { p.border }));
    if !hint.is_empty() { field = field.child(help(hint,p)); }
    if let Some(error) = error { field = field.child(text(error.to_owned()).key(format!("{key}-error")).role("alert").font(type_::BODY).color(p.coral).wrap().w_full()); }
    field
}
fn primary(label: &str, key: &str, p: Palette) -> El {
    el().key(key).role("button").label(label).focusable(true).min_h(44.0).padding(10.0).radius(10.0).fill(p.petroleum).center_y()
        .child(text(label.to_owned()).font(type_::BODY).color(p.canvas))
}
/// Square control carrying one glyph, centred on both axes.
fn glyph_button(glyph: &str, name: &str, key: &'static str, p: Palette, action: fn(&mut State, &mut Tx)) -> El {
    el().key(key).role("button").label(name).focusable(true).size(44.0, 44.0).radius(10.0).fill(p.selected).hover(p.border).center()
        .on_press_tx(action).child(text(glyph.to_owned()).font(theme::GLYPH).color(p.ink))
}
fn category_picker(s: &State, p: Palette) -> El {
    let sh = s.wish_sheet.as_ref().expect("wish sheet present");
    let category = s.categories.iter().find(|c| Some(c.id)==sh.category).map_or("Elegir después",|c| c.name.as_str());
    field("Categoría del presupuesto · Opcional", button(category,"wish-category",p).min_h(44.0)
        .description("Sirve para evaluar el presupuesto. No mueve dinero.").on_press_tx(|s: &mut State,tx| {
            if s.mutation_active() { return; }
            let mut menu = PopupMenu::new().item(PopupItem::new("none","Elegir después"));
            for group in s.groups.iter().filter(|g| !g.is_income) {
                for category in s.categories.iter().filter(|c| c.group_id == Some(group.id) && !c.hidden) {
                    menu = menu.item(PopupItem::new(category.id.to_string(),format!("{} · {}",group.name,category.name)));
                }
            }
            tx.popup_menu(menu,"wish-category",|s: &mut State,id,_| {
                if !s.mutation_active() && let (Some(sh),Some(id)) = (&mut s.wish_sheet,id) { sh.category = id.parse().ok(); }
            });
        }),p).child(help("Sirve para evaluar si la compra cabe en tu presupuesto. No mueve dinero.",p))
}
fn editor_body(s: &State, p: Palette) -> El {
    let sh = s.wish_sheet.as_ref().expect("wish sheet present");
    let mut body = col().key("wish-editor-body").w_full().min_h(0.0).shrink().scroll(rune::ScrollSpec {stick_bottom:false}).gap(24.0).padding(24.0);
    if sh.step == WishStep::Purchase {
        body = body.child(editor_field("wish-name","¿Qué quieres comprar? · Obligatorio",&sh.name,|sh| &mut sh.name,sh.name_error(),"",p))
            .child(editor_field("wish-price","Precio objetivo · Obligatorio",&sh.price,|sh| &mut sh.price,sh.price_error(),PRICE_HELP,p));
        if let Some(price) = parse_price(&sh.price.text) {
            body = body.child(text(format!("Importe reconocido: {}",money::format(price))).key("wish-price-recognized").role("status").font(type_::BODY).color(p.emerald).wrap().w_full());
        }
    } else {
        body = body.child(col().w_full().gap(8.0).pb(16.0)
            .child(help("Tu compra",p)).child(text(sh.name.text.trim().to_owned()).key("wish-review-name").font(theme::HEADLINE).color(p.ink).wrap_anywhere().w_full())
            .child(text(parse_price(&sh.price.text).map(money::format).unwrap_or_default()).key("wish-review-price").font(theme::AMOUNT).color(p.ink).wrap().w_full()).child(separator(p)))
            .child(category_picker(s,p))
            .child(button(if sh.details_open { "Ocultar enlace y detalles · Opcionales" } else { "Enlace y detalles · Opcionales" },"wish-details-toggle",p).min_h(44.0)
                .description(if sh.details_open { "Sección expandida" } else { "Sección contraída" }).expanded(sh.details_open).on_press_tx(|s: &mut State,tx| {
                    if s.mutation_active() { return; }
                    if let Some(sh) = &mut s.wish_sheet { sh.details_open = !sh.details_open; tx.focus(if sh.details_open { "wish-url" } else { "wish-details-toggle" }, true); }
                }));
        if sh.details_open {
            body = body.child(editor_field("wish-url","Enlace del producto · Opcional",&sh.url,|sh| &mut sh.url,sh.url_error(),"Déjalo vacío si todavía no tienes un enlace.",p))
                .child(editor_field("wish-notes","Detalles · Opcionales",&sh.notes,|sh| &mut sh.notes,None,"Modelo, accesorios o algo que quieras recordar.",p));
        }
        if sh.id.is_some() { body = body.child(help("Cambiar la categoría libera lo apartado.",p)); }
    }
    body
}
fn discard_dialog(p: Palette) -> El {
    let body = col().w_full().max_w(440.0).padding(24.0).gap(16.0).radius(14.0).fill(p.card).border(1.0,p.border)
        .child(text("¿Descartar los cambios?").font(theme::HEADLINE).color(p.ink).wrap().w_full())
        .child(help("Tienes cambios sin guardar en este deseo.",p))
        .child(row().wrap().w_full().gap(12.0)
            .child(primary("Seguir editando","wish-keep-editing",p).autofocus().on_press_tx(|s: &mut State,tx| s.request_wish_close(tx)))
            .child(button("Descartar","wish-discard",p).min_h(44.0).on_press_tx(|s: &mut State,tx| s.dismiss_wish(tx))));
    el().key("wish-discard-confirm").role("dialog").label("¿Descartar los cambios?").paint_order(110).raise()
        .absolute(0.0,0.0).w_full().h_full().padding(24.0).center().fill(Color::BLACK.alpha(0.4))
        .modal_tx(false,|s: &mut State,tx| s.request_wish_close(tx)).child(body)
}
pub fn sheet(s: &State,p: Palette) -> El {
    let sh = s.wish_sheet.as_ref().expect("wish sheet present");
    let busy = s.mutation_active();
    let title = if sh.id.is_some() { "Editar deseo" } else { "Nuevo deseo" };
    let description = if sh.step == WishStep::Purchase { "Primero, lo imprescindible." } else { "Revisa la compra y añade contexto si lo necesitas." };
    let mut steps = row().w_full().gap(16.0);
    for (step,label) in [(WishStep::Purchase,"1. La compra"),(WishStep::Context,"2. Contexto y revisión")] {
        let color = if sh.step == step { p.petroleum } else { p.muted };
        steps = steps.child(col().grow(1.0).min_w(0.0).gap(8.0).child(text(label).font(theme::CARD_TITLE).color(color).wrap().w_full())
            .child(el().w_full().h(2.0).fill(if sh.step == step { p.petroleum } else { p.border })));
    }
    let header = col().w_full().padding(24.0).gap(16.0)
        .child(row().w_full().justify_between().center_y().gap(12.0).child(text("DESEOS · FLUJO GUIADO").font(theme::CARD_TITLE).color(p.ink))
            .child(glyph_button("×","Cerrar formulario","wish-close",p,|s: &mut State,tx| s.request_wish_close(tx))))
        .child(steps).child(text(title).font(theme::HEADLINE).color(p.ink)).child(help(description,p));
    let mut actions = row().wrap().w_full().justify_between().gap(12.0)
        .child(button("Cancelar","wish-cancel",p).min_h(44.0).on_press_tx(|s: &mut State,tx| s.request_wish_close(tx)));
    let mut right = row().wrap().gap(8.0);
    if sh.step == WishStep::Context {
        right = right.child(button("Volver","wish-back",p).min_h(44.0).on_press_tx(|s: &mut State,tx| {
            if !s.mutation_active() && let Some(sh) = &mut s.wish_sheet { sh.step = WishStep::Purchase; tx.focus("wish-name",true); }
        })).child(primary(if busy { "Guardando…" } else { "Guardar deseo" },"wish-save",p).disabled(busy).default_button().on_press_tx(|s: &mut State,tx| s.save_wish(tx)));
    } else {
        right = right.child(primary("Continuar","wish-next",p).default_button().on_press_tx(|s: &mut State,tx| s.next_wish(tx)));
    }
    actions = actions.child(right);
    let mut footer = col().w_full().px(24.0).py(16.0).gap(12.0);
    if let Some(error) = &s.mutation_error {
        footer = footer.child(text(format!("No se pudo guardar. Conservamos tus datos. {error}")).key("wish-save-error").role("alert").font(type_::BODY).color(p.coral).wrap().w_full());
    }
    if busy { footer = footer.child(text("Guardando deseo…").key("wish-saving").role("status").font(type_::BODY).color(p.ink)); }
    footer = footer.child(actions).child(help("Después podrás apartar fondos. Guardar no registra un gasto.",p));
    let form = col().key("wish-form").w_full().min_h(0.0).shrink().radius(14.0).fill(p.card).border(1.0,p.border)
        .child(header).child(separator(p)).child(editor_body(s,p)).child(separator(p)).child(footer);
    let form = if busy || sh.discard_confirm { form.inert() } else { form };
    // The dialog hugs its content, like the artboard's `max-height`: the frame alone is definite, so it
    // caps at 760pt or the window and the body scrolls only when a step outgrows both.
    let form = col().w_full().max_w(580.0).h_full().max_h(760.0).justify_center().child(form);
    let mut root = el().key("wish-sheet").role("dialog").label(title).description(description).focus_return(sh.trigger())
        .paint_order(100).raise().absolute(0.0,0.0).w_full().h_full().padding(24.0).center().fill(Color::BLACK.alpha(0.4))
        .modal_tx(false,|s: &mut State,tx| s.request_wish_close(tx)).child(form);
    if sh.discard_confirm { root = root.child(discard_dialog(p)); }
    root
}

fn demo(s: &mut State,dark: bool) {
    scene(s,true,dark); s.route=Route::Deseos;
    let worker=Arc::clone(&s.worker);
    let category=s.categories.iter().find(|c| c.name=="Ocio").map(|c| c.id);
    let today=s.today_date;
    let result=std::thread::spawn(move || -> Result<Vec<WishView>,String> {
        let mut guard=worker.lock().map_err(|e| e.to_string())?;
        let store=guard.as_mut().ok_or("No se pudo abrir la base de datos")?;
        let wish=finanzas_core::service::save_wish(store,None,WishDraft {name:"PlayStation 5".into(),price:50_000,url:"https://example.com/playstation".into(),notes:"Con lector y un mando adicional".into(),category_id:category}).map_err(|e| e.to_string())?;
        finanzas_core::service::reserve_wish(store,wish.id,5_000,today).map_err(|e| e.to_string())?;
        finanzas_core::service::list_wishes(store,today,false).map_err(|e| e.to_string())
    }).join();
    match result {
        Ok(Ok(views)) => { for v in &views {s.wish_inputs.insert(v.wish.id,TextEdit::new(decimal(v.wish.reserved)));} s.wishes=Some(views); s.wishes_key=Some(s.wishes_key_now()); }
        Ok(Err(e)) => s.error=Some(e),
        Err(_) => s.error=Some("No se pudo preparar la escena".into()),
    }
}
pub fn light_scene(s: &mut State) {demo(s,false);}
pub fn dark_scene(s: &mut State) {demo(s,true);}
fn wizard_scene(s: &mut State, dark: bool, step: WishStep, discard: bool) {
    demo(s,dark);
    s.open_wish(None);
    if let Some(sh) = &mut s.wish_sheet {
        sh.name.replace_text("PlayStation 5");
        sh.price.replace_text(if step == WishStep::Purchase { "11,000" } else { "11,000.00" });
        sh.step = step;
        sh.discard_confirm = discard;
    }
}
pub fn sheet_scene(s: &mut State) { wizard_scene(s,false,WishStep::Purchase,false); }
pub fn sheet_dark_scene(s: &mut State) { wizard_scene(s,true,WishStep::Purchase,false); }
pub fn review_scene(s: &mut State) { wizard_scene(s,true,WishStep::Context,false); }
pub fn discard_scene(s: &mut State) { wizard_scene(s,true,WishStep::Purchase,true); }
