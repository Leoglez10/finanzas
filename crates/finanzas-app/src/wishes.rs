use super::*;
use finanzas_core::service::{Wish, WishDraft, WishView};
use rune::{text_input, PopupItem, PopupMenu};
use sheet::{button, field};

pub struct WishSheet {
    pub id: Option<Id>, pub name: TextEdit, pub price: TextEdit,
    pub url: TextEdit, pub notes: TextEdit, pub category: Option<Id>,
}
fn decimal(cents: i64) -> String { format!("{}.{:02}", cents / 100, cents % 100) }
fn parse_amount(text: &str) -> Option<i64> {
    let text = text.trim();
    if text.is_empty() || !text.chars().all(|c| c.is_ascii_digit() || c == '.') || text.split('.').nth(1).is_some_and(|s| s.len() > 2) { return None; }
    money::cents_from(text)
}
impl WishSheet {
    fn new(wish: Option<&Wish>) -> Self {
        Self { id: wish.map(|w| w.id), name: TextEdit::new(wish.map_or("", |w| &w.name)),
            price: TextEdit::new(wish.map(|w| decimal(w.price)).unwrap_or_default()),
            url: TextEdit::new(wish.map_or("", |w| &w.url)), notes: TextEdit::new(wish.map_or("", |w| &w.notes)),
            category: wish.and_then(|w| w.category_id) }
    }
    fn draft(&self) -> Option<WishDraft> {
        let price = parse_amount(&self.price.text).filter(|v| *v > 0)?;
        let name = self.name.text.trim();
        let url = self.url.text.trim();
        if name.is_empty() || (!url.is_empty() && !url.starts_with("http://") && !url.starts_with("https://")) { return None; }
        Some(WishDraft { name:name.into(), price, url:url.into(), notes:self.notes.text.clone(), category_id:self.category })
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
    }
    fn save_wish(&mut self) {
        if self.mutation_active() || !self.wishes_fresh() { return; }
        let Some(sh) = &self.wish_sheet else { return };
        let Some(draft) = sh.draft() else { return };
        if sh.id.is_some_and(|id| self.visible_wish(id).is_none()) { return; }
        let mutation = data::Mutation::SaveWish(sh.id, draft);
        self.wish_mutation(mutation, MutationTarget::WishSheet);
    }
    fn wish_mutation(&mut self, mutation: data::Mutation, target: MutationTarget) {
        if self.mutation_active() || !self.wishes_fresh() { return; }
        self.mutation_error = None;
        self.mutation_target = Some(target);
        let worker = Arc::clone(&self.worker);
        self.mutation.request(mutation, move |m| data::mutate(worker,m));
    }
    fn reserve_wish(&mut self, id: Id, release: bool) {
        if self.visible_wish(id).is_none() { return; }
        let amount = if release { Some(0) } else { self.wish_inputs.get(&id).and_then(|e| parse_amount(&e.text)) };
        let Some(amount) = amount.filter(|a| *a >= 0) else { self.mutation_error = Some("Escribe un importe de cero o más, con hasta dos decimales.".into()); return };
        self.wish_mutation(data::Mutation::ReserveWish(id,amount,self.today_date),MutationTarget::WishRow);
    }
}

pub fn poll(s: &mut State, tx: &mut Tx, changed: &mut bool) {
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
        card = card.child(field("Importe total que quieres apartar (0 libera fondos)",text_input(input,type_::BODY).key(format!("wish-reserve-input-{id}")).label("Importe a apartar").single_line().color(p.ink)
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
                if !s.mutation_active() { s.wishes_archived = !s.wishes_archived; }
            })));
    if let Some(error) = &s.mutation_error { page = page.child(text(error.clone()).key("wish-error").font(type_::BODY).color(p.coral).wrap().w_full()); }
    let mut list = col().w_full().grow(1.0).min_h(0.0).gap(14.0).scroll(rune::ScrollSpec {stick_bottom:false});
    if !s.wishes_fresh() { list = list.child(text("Cargando deseos y presupuesto…").key("wish-loading").font(type_::BODY).color(p.muted).wrap().w_full()); }
    else if let Some(views) = &s.wishes {
        if views.is_empty() { list = list.child(text("Todavía no tienes deseos. Añade una compra para conocer cuánto te falta.").key("wish-empty").font(type_::BODY).color(p.muted).wrap().w_full()); }
        for view in views { list = list.child(wish_card(s,view,p)); }
    }
    page.child(list)
}
fn edit(s: &mut State,pick: fn(&mut WishSheet)->&mut TextEdit) -> &mut TextEdit {
    if s.mutation_active() || s.wish_sheet.is_none() { return &mut s.blocked_edit; }
    pick(s.wish_sheet.as_mut().expect("wish sheet present"))
}
pub fn sheet(s: &State,p: Palette) -> El {
    let sh = s.wish_sheet.as_ref().expect("wish sheet present");
    let input = |key: &'static str,label: &'static str,value: &TextEdit,pick: fn(&mut WishSheet)->&mut TextEdit| {
        field(label,text_input(value,type_::BODY).key(key).label(label).single_line().color(p.ink).on_edit(move |s: &mut State| edit(s,pick)),p)
    };
    let category = s.categories.iter().find(|c| Some(c.id)==sh.category).map_or("Sin categoría",|c| c.name.as_str());
    let picker = button(category,"wish-category",p).on_press_tx(|s: &mut State,tx| {
        if s.mutation_active() { return; }
        let mut menu = PopupMenu::new().item(PopupItem::new("none","Sin categoría"));
        for group in s.groups.iter().filter(|g| !g.is_income) {
            for category in s.categories.iter().filter(|c| c.group_id == Some(group.id) && !c.hidden) {
                menu = menu.item(PopupItem::new(category.id.to_string(),format!("{} · {}",group.name,category.name)));
            }
        }
        tx.popup_menu(menu,"wish-category",|s: &mut State,id,_| {
            if !s.mutation_active() && let (Some(sh),Some(id)) = (&mut s.wish_sheet,id) { sh.category = id.parse().ok(); }
        });
    });
    let mut body = col().w_full().max_h(570.0).scroll(rune::ScrollSpec {stick_bottom:false}).gap(12.0)
        .child(input("wish-name","Nombre",&sh.name,|sh| &mut sh.name))
        .child(input("wish-price","Precio objetivo",&sh.price,|sh| &mut sh.price))
        .child(input("wish-url","Enlace (http o https, opcional)",&sh.url,|sh| &mut sh.url))
        .child(input("wish-notes","Detalles",&sh.notes,|sh| &mut sh.notes))
        .child(field("Categoría del presupuesto",picker,p))
        .child(text("Nombre y precio mayor que cero son obligatorios. Usa hasta dos decimales. Cambiar la categoría libera lo apartado. Guarda primero; después puedes apartar fondos.").font(type_::BODY).color(p.muted).wrap().w_full());
    if let Some(error)=&s.mutation_error { body=body.child(text(error.clone()).font(type_::BODY).color(p.coral).wrap().w_full()); }
    let mut save=button("Guardar","wish-save",p);
    if sh.draft().is_some() && !s.mutation_active() { save=save.default_button().on_press(|s: &mut State| s.save_wish()); }
    else { save=save.disabled(true); }
    let form=col().w(580.0).padding(24.0).gap(16.0).radius(14.0).fill(p.card)
        .child(text(if sh.id.is_some() {"Editar deseo"} else {"Nuevo deseo"}).font(type_::TITLE).color(p.ink))
        .child(body).child(row().justify_between().child(button("Cancelar","wish-cancel",p).on_press(|s: &mut State| if !s.mutation_active() {s.wish_sheet=None;})).child(save));
    let form=if s.mutation_active() {form.inert()} else {form};
    el().key("wish-sheet").paint_order(100).raise().absolute(0.0,0.0).w_full().h_full().center().fill(Color::BLACK.alpha(0.4))
        .modal(false,|s: &mut State| if !s.mutation_active() {s.wish_sheet=None;}).child(form)
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
pub fn sheet_scene(s: &mut State) {demo(s,false);s.open_wish(Some(1));}
