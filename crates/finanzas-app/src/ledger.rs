use super::*;
use rune::{GrowthSpec, ScrollSpec, text_input};

fn action(name: &str, key: &str, color: Color) -> El {
    el().key(key).role("button").label(name).focusable(true).padding(10.0).radius(10.0).fill(color).child(text(name.to_owned()).font(type_::BODY).color(Color::WHITE))
}

fn cell(value: impl Into<String>, width: f32, color: Color) -> El {
    text(value.into()).font(type_::BODY).color(color).w(width).shrink().truncate()
}

pub fn render(s: &State, p: Palette, title: &str) -> El {
    let ledger = s.ledger.as_ref();
    let show_account = s.route == Route::Todas && s.accounts.iter().filter(|a| !a.account.closed).count() > 1;
    let mut content = col().grow(1.0).h_full().min_h(0.0).padding(24.0).gap(16.0)
        .child(text(title.to_owned()).key("ledger-title").font(type_::TITLE).color(p.ink))
        .child(text("Tu historial de movimientos").font(type_::BODY).color(p.muted))
        .child(row().w_full().padding(20.0).gap(8.0).justify_between().center_y().fill(p.card).border(1.0, p.border).radius(18.0)
            .child(col().gap(8.0).child(text(entry_helpers::header_label(&s.search.text)).font(type_::BODY).color(p.muted))
                .child(text(ledger.map_or("—".into(), |l| money::format(l.total))).font(type_::TITLE).color(p.ink))
                .child(text(ledger.map_or("Cargando…".into(), |l| format!("{} movimientos", l.rows.len()))).font(type_::BODY).color(p.muted)))
            .child(row().gap(8.0).child(action("Agregar gasto", "add-expense", color("#BE493D")).on_press(|s: &mut State| s.open_sheet(true, None)))
                .child(action("Agregar ingreso", "add-income", color("#147D59")).on_press(|s: &mut State| s.open_sheet(false, None)))))
        .child(text_input(&s.search, type_::BODY).key("ledger-search").single_line().on_edit(|s: &mut State| &mut s.search)
            .placeholder("Buscar comercio, categoría o nota", p.muted).padding(10.0).fill(p.card).border(1.0, p.border).radius(8.0));
    let mut header = row().w_full().gap(8.0).px(12.0).py(8.0).child(cell("Fecha", 90.0, p.muted));
    if show_account { header = header.child(cell("Cuenta", 135.0, p.muted)); }
    header = header.child(cell("Comercio", 150.0, p.muted)).child(cell("Categoría", 150.0, p.muted))
        .child(cell("Notas", 130.0, p.muted)).child(cell("Monto", 110.0, p.muted));
    let mut table = col().grow(1.0).min_h(0.0).gap(0.0).child(header);
    if let Some(ledger) = ledger {
        if ledger.rows.is_empty() {
            table = table.child(col().grow(1.0).center().gap(8.0)
                .child(text("Sin movimientos").font(type_::TITLE).color(p.ink))
                .child(text("Agrega uno con ⌘N o importa desde Actual en Archivo.").font(type_::BODY).color(p.muted)));
        } else {
            let rows = col().key("ledger-rows").growth(GrowthSpec::default()).virtual_rows(ledger.rows.len(), 40.0,
                |s: &State, i| format!("entry-{}", s.ledger.as_ref().unwrap().rows[i].id).into(),
                move |s: &State, i| {
                    let e = &s.ledger.as_ref().unwrap().rows[i];
                    let id = e.id;
                    let mut r = row().w_full().h(40.0).gap(8.0).px(12.0).center_y()
                        .child(cell(e.date.format("%d.%m.%Y").to_string(), 90.0, p.ink));
                    if show_account { r = r.child(cell(e.account_name.as_deref().unwrap_or(""), 135.0, p.ink)); }
                    r.fill(if s.selected == Some(id) { p.selected } else { Color::TRANSPARENT }).hover(p.selected)
                        .on_press(move |s: &mut State| if !s.mutation_active() { s.selected = Some(id) })
                        .on_double_press(move |s: &mut State| s.open_sheet(true, Some(id)))
                        .on_secondary_press(move |s: &mut State, press, tx| {
                            if s.mutation_active() { return; }
                            s.selected = Some(id);
                            let menu = rune::PopupMenu::new().item(rune::PopupItem::new("edit", "Editar")).item(rune::PopupItem::new("delete", "Eliminar"));
                            tx.popup_menu(menu, press, move |s: &mut State, picked, _| { if !s.mutation_active() { match picked { Some("edit") => s.open_sheet(true, Some(id)), Some("delete") if s.visible_entry(id).is_some() => s.confirm = Some(id), _ => {} } } });
                        })
                        .child(cell(e.payee_name.as_deref().unwrap_or(""), 150.0, p.ink))
                        .child(cell(e.category_name.as_deref().unwrap_or("⚠ Sin categoría"), 150.0, if e.category_name.is_none() { p.amber } else { p.ink }))
                        .child(cell(e.notes.clone(), 130.0, p.muted))
                        .child(cell(money::format(e.amount), 110.0, if e.amount < 0 { p.coral } else { p.emerald }))
                });
            table = table.child(col().grow(1.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).child(rows));
        }
    }
    content = content.child(table);
    content
}
