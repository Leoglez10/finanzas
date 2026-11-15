use super::*;
use rune::{Env, Runtime};

fn settle(rt: &mut Runtime<State>) {
    for _ in 0..120 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); }
}
fn animate(rt: &mut Runtime<State>) { for _ in 0..60 { rt.frame(1.0 / 60.0); } }
fn click(rt: &mut Runtime<State>, key: &str) {
    animate(rt);
    for _ in 0..8 {
        let Some((target_x,target_y))=rt.find(key) else { break };
        if rt.elements().iter().find(|e| e.key.as_deref()==Some(key)).is_none_or(|e| {
            let [x,y,w,h]=e.visible;
            target_x>=x && target_x<x+w && target_y>=y && target_y<y+h
        }) { break; }
        let Some((x,y))=rt.find("wish-editor-body") else { break };
        rt.wheel(x,y,if target_y<y {180.0} else {-180.0});rt.frame(0.0);
    }
    let (x,y) = rt.find(key).unwrap_or_else(|| panic!("missing {key}"));
    rt.pointer_down(x,y); rt.pointer_up(x,y); rt.frame(0.0);
}
fn fill(rt: &mut Runtime<State>, key: &str, value: &str) {
    click(rt,key);
    rt.key(KeyEvent { key:Key::Command('a'), mods:rune::Mods {cmd:true,..Default::default()} });
    rt.key(KeyEvent { key:if value.is_empty() { Key::Backspace } else { Key::Text(value.into()) }, mods:Default::default() });
    rt.frame(0.0);
}
fn runtime_with_size(size: (f32,f32)) -> Runtime<State> {
    let mut s = State::empty(); scene(&mut s,true,false);
    let mut rt = Runtime::new(s,view,size,Env::new(false,1.0));
    rt.poll = Some(poll); rt.on_key = Some(on_key); rt.frame(0.0); rt
}
fn runtime() -> Runtime<State> { runtime_with_size((1200.0,900.0)) }
fn editor(rt: &mut Runtime<State>) {
    click(rt,"deseos"); settle(rt); click(rt,"wish-new");
}
fn text_of(rt: &Runtime<State>, key: &str) -> Option<String> {
    rt.elements().into_iter().find(|e| e.key.as_deref()==Some(key)).and_then(|e| e.text)
}
fn focused_key(rt: &Runtime<State>) -> Option<String> {
    rt.elements().into_iter().find(|e| e.focused).and_then(|e| e.key)
}
fn saved(rt: &Runtime<State>) -> Vec<finanzas_core::service::WishView> {
    finanzas_core::service::list_wishes(rt.state.worker.lock().unwrap().as_mut().unwrap(),rt.state.today_date,true).unwrap()
}
fn purchase(rt: &mut Runtime<State>, price: &str) {
    fill(rt,"wish-name","PlayStation 5"); fill(rt,"wish-price",price);
    click(rt,"wish-next");
}
#[test]
fn grouped_price_advances_to_review_and_saves_only_after_confirmation() {
    let mut rt=runtime(); editor(&mut rt);
    assert!(rt.find("wish-save").is_none());
    purchase(&mut rt,"11,000");
    assert!(rt.find("wish-name").is_none());
    assert_eq!(focused_key(&rt).as_deref(),Some("wish-category"));
    assert_eq!(text_of(&rt,"wish-review-price").as_deref(),Some("$11,000.00"));
    assert!(saved(&rt).is_empty());
    assert!(rt.find("wish-url").is_none(),"optional fields start collapsed");
    click(&mut rt,"wish-save"); settle(&mut rt);
    assert!(rt.state.wish_sheet.is_none());
    assert_eq!(focused_key(&rt).as_deref(),Some("wish-new"));
    let wishes=saved(&rt);
    assert_eq!(wishes.len(),1);
    assert_eq!((wishes[0].wish.name.as_str(),wishes[0].wish.price),("PlayStation 5",1_100_000));
    assert_eq!((wishes[0].wish.reserved,wishes[0].wish.category_id),(0,None));
    assert_eq!((wishes[0].wish.url.as_str(),wishes[0].wish.notes.as_str()),("",""));
    assert!(rt.elements().iter().all(|e| e.text.as_deref()!=Some("Tema claro")));
}
#[test]
fn supported_target_price_formats_save_exact_cents_without_ledger_changes() {
    for (input,cents) in [("11000",1_100_000),("11,000.50",1_100_050),("0.01",1),("$11,000",1_100_000),("$ 11,000.50",1_100_050)] {
        let mut rt=runtime();
        let before=data::ledger(Arc::clone(&rt.state.worker),None,"").unwrap();
        editor(&mut rt);purchase(&mut rt,input);click(&mut rt,"wish-save");settle(&mut rt);
        assert_eq!(saved(&rt)[0].wish.price,cents,"{input}");
        let after=data::ledger(Arc::clone(&rt.state.worker),None,"").unwrap();
        assert_eq!((after.rows.len(),after.total),(before.rows.len(),before.total));
    }
}
#[test]
fn saved_status_clears_after_other_wish_actions_and_navigation() {
    let mut rt=runtime();
    let category=rt.state.categories.iter().find(|c| c.name=="Ocio").unwrap().id;
    editor(&mut rt);purchase(&mut rt,"100");
    rt.state.wish_sheet.as_mut().unwrap().category=Some(category);
    click(&mut rt,"wish-save");settle(&mut rt);
    assert!(rt.find("wish-saved").is_some());
    fill(&mut rt,"wish-reserve-input-1","1");click(&mut rt,"wish-reserve-1");settle(&mut rt);
    assert!(rt.find("wish-saved").is_none(),"stale after reserve");
    click(&mut rt,"wish-edit-1");fill(&mut rt,"wish-name","PS5");click(&mut rt,"wish-next");click(&mut rt,"wish-save");settle(&mut rt);
    assert!(rt.find("wish-saved").is_some());
    click(&mut rt,"resumen");settle(&mut rt);click(&mut rt,"deseos");settle(&mut rt);
    assert!(rt.find("wish-saved").is_none(),"stale after route change");
}
#[test]
fn primary_actions_use_the_primary_radius_token() {
    let mut rt=runtime();editor(&mut rt);animate(&mut rt);
    let radius=|rt:&Runtime<State>,key:&str| rt.elements().into_iter().find(|e| e.key.as_deref()==Some(key)).unwrap().radius;
    assert_eq!(radius(&rt,"wish-next"),10.0);
    purchase(&mut rt,"100");animate(&mut rt);
    assert_eq!(radius(&rt,"wish-save"),10.0);
}
#[test]
fn compact_window_keeps_actions_visible_and_keyboard_focus_in_the_modal() {
    let (width,height)=(800.0,600.0);
    let mut rt=runtime_with_size((width,height));editor(&mut rt);
    purchase(&mut rt,"11,000");click(&mut rt,"wish-details-toggle");animate(&mut rt);
    for key in ["wish-close","wish-cancel","wish-back","wish-save"] {
        let [x,y,w,h]=rt.elements().into_iter().find(|e|e.key.as_deref()==Some(key)).unwrap().rect;
        assert!(x>=0.0 && y>=0.0 && x+w<=width && y+h<=height,"clipped {key}: {x},{y},{w},{h}");
    }
    rt.update(|_,tx|tx.focus("wish-save",true));rt.frame(0.0);
    rt.key(KeyEvent {key:Key::Tab,mods:Default::default()});rt.frame(0.0);
    assert_eq!(focused_key(&rt).as_deref(),Some("wish-close"));
}
#[test]
fn invalid_required_fields_show_specific_errors_and_do_not_write() {
    let mut rt=runtime(); editor(&mut rt);
    click(&mut rt,"wish-next");
    assert_eq!(text_of(&rt,"wish-name-error").as_deref(),Some("Escribe el nombre de lo que quieres comprar."));
    assert!(text_of(&rt,"wish-price-error").is_some());
    assert_eq!(focused_key(&rt).as_deref(),Some("wish-name"));
    fill(&mut rt,"wish-name","PlayStation 5");
    for price in ["11,00","11.000,50","0","-10","1.234",""] {
        fill(&mut rt,"wish-price",price); click(&mut rt,"wish-next");
        assert!(rt.find("wish-price-error").is_some(),"missing price error for {price}");
        assert!(rt.find("wish-name-error").is_none());
        assert!(rt.find("wish-next").is_some());
        assert!(saved(&rt).is_empty());
    }
    fill(&mut rt,"wish-price","11000"); click(&mut rt,"wish-next");
    assert_eq!(text_of(&rt,"wish-review-price").as_deref(),Some("$11,000.00"));
}
#[test]
fn details_back_navigation_and_editing_preserve_existing_wish_behavior() {
    let mut rt=runtime(); editor(&mut rt); purchase(&mut rt,"500.50");
    click(&mut rt,"wish-details-toggle");
    fill(&mut rt,"wish-url","https://example.com/ps5");
    fill(&mut rt,"wish-notes","Con mando\nY lector");
    click(&mut rt,"wish-back");
    assert_eq!(rt.state.wish_sheet.as_ref().unwrap().price.text,"500.50");
    click(&mut rt,"wish-next");
    assert_eq!(rt.state.wish_sheet.as_ref().unwrap().notes.text,"Con mando\nY lector");
    click(&mut rt,"wish-save"); settle(&mut rt);
    assert_eq!(saved(&rt)[0].wish.url,"https://example.com/ps5");
    assert_eq!(saved(&rt)[0].wish.notes,"Con mando\nY lector");
    click(&mut rt,"wish-edit-1");
    fill(&mut rt,"wish-name","PlayStation 5 edición digital");
    click(&mut rt,"wish-next"); click(&mut rt,"wish-save"); settle(&mut rt);
    let wishes=saved(&rt);
    assert_eq!(wishes.len(),1);
    assert_eq!((wishes[0].wish.id,wishes[0].wish.price),(1,50_050));
    assert_eq!(wishes[0].wish.name,"PlayStation 5 edición digital");
}
#[test]
fn invalid_optional_url_keeps_input_and_explains_how_to_fix_it() {
    let mut rt=runtime(); editor(&mut rt); purchase(&mut rt,"500");
    click(&mut rt,"wish-details-toggle");
    fill(&mut rt,"wish-url","javascript:alert(1)");
    click(&mut rt,"wish-save");
    assert!(rt.find("wish-url-error").is_some());
    assert_eq!(rt.state.wish_sheet.as_ref().unwrap().url.text,"javascript:alert(1)");
    assert!(saved(&rt).is_empty());
    fill(&mut rt,"wish-url",""); click(&mut rt,"wish-save"); settle(&mut rt);
    assert_eq!(saved(&rt).len(),1);
}
#[test]
fn close_cancel_and_escape_protect_dirty_input_but_unchanged_edit_closes() {
    let mut rt=runtime(); editor(&mut rt);
    fill(&mut rt,"wish-name","Sin guardar");
    click(&mut rt,"wish-cancel");
    assert!(rt.find("wish-discard-confirm").is_some());
    assert_eq!(focused_key(&rt).as_deref(),Some("wish-keep-editing"));
    click(&mut rt,"wish-keep-editing");
    assert_eq!(rt.state.wish_sheet.as_ref().unwrap().name.text,"Sin guardar");
    click(&mut rt,"wish-close");
    assert!(rt.find("wish-discard-confirm").is_some());
    rt.key(KeyEvent {key:Key::Escape,mods:Default::default()});rt.frame(0.0);
    assert!(rt.find("wish-discard-confirm").is_none());
    assert!(rt.state.wish_sheet.is_some());
    rt.key(KeyEvent {key:Key::Escape,mods:Default::default()});rt.frame(0.0);
    assert!(rt.find("wish-discard-confirm").is_some());
    click(&mut rt,"wish-discard");
    assert!(rt.state.wish_sheet.is_none());assert!(saved(&rt).is_empty());
    click(&mut rt,"wish-new");
    purchase(&mut rt,"500");click(&mut rt,"wish-save");settle(&mut rt);
    click(&mut rt,"wish-edit-1");click(&mut rt,"wish-close");
    assert!(rt.state.wish_sheet.is_none(),"unchanged edit closes without confirmation");
}
#[test]
fn busy_save_is_single_shot_and_store_failure_retains_the_draft() {
    let mut rt=runtime();editor(&mut rt);purchase(&mut rt,"500");
    let worker=Arc::clone(&rt.state.worker);
    let mut guard=worker.lock().unwrap();
    let store=guard.as_mut().unwrap();
    let category=rt.state.categories.iter().find(|c| c.name=="Ocio").unwrap().id;
    // A category that disappeared while the form was open must yield a recoverable backend error.
    store.delete_category(category).unwrap();
    rt.state.wish_sheet.as_mut().unwrap().category=Some(category);
    click(&mut rt,"wish-save");
    assert_eq!(text_of(&rt,"wish-saving").as_deref(),Some("Guardando deseo…"));
    rt.key(KeyEvent {key:Key::Escape,mods:Default::default()});rt.frame(0.0);
    assert!(rt.state.wish_sheet.is_some());
    drop(guard);settle(&mut rt);
    assert!(rt.find("wish-save-error").is_some());
    assert_eq!(rt.state.wish_sheet.as_ref().unwrap().name.text,"PlayStation 5");
    assert!(saved(&rt).is_empty());
    rt.state.wish_sheet.as_mut().unwrap().category=None;
    let guard=worker.lock().unwrap();
    click(&mut rt,"wish-save");click(&mut rt,"wish-save");
    drop(guard);settle(&mut rt);
    assert_eq!(saved(&rt).len(),1);
}
#[test]
fn wishes_reserve_release_and_archive_use_public_controls() {
    let mut rt = runtime();
    let category = rt.state.categories.iter().find(|c| c.name == "Ocio").unwrap().id;
    {
        let mut guard = rt.state.worker.lock().unwrap();
        finanzas_core::service::save_wish(guard.as_mut().unwrap(),None,finanzas_core::service::WishDraft {
            name:"PlayStation".into(),price:10_000,url:String::new(),notes:String::new(),category_id:Some(category)
        }).unwrap();
    }
    click(&mut rt,"deseos"); settle(&mut rt);
    fill(&mut rt,"wish-reserve-input-1","50");
    click(&mut rt,"wish-reserve-1"); settle(&mut rt);
    assert_eq!(rt.state.wishes.as_ref().unwrap()[0].wish.reserved,5_000);
    for input in ["11,000","-1","1.234"] {
        fill(&mut rt,"wish-reserve-input-1",input);click(&mut rt,"wish-reserve-1");settle(&mut rt);
        assert_eq!(saved(&rt)[0].wish.reserved,5_000,"reservation contract changed for {input}");
        assert!(rt.find("wish-error").is_some());
    }
    fill(&mut rt,"wish-reserve-input-1","999999");
    click(&mut rt,"wish-reserve-1"); settle(&mut rt);
    assert_eq!(rt.state.wishes.as_ref().unwrap()[0].wish.reserved,5_000);
    assert!(rt.find("wish-error").is_some());
    click(&mut rt,"wish-release-1"); settle(&mut rt);
    assert_eq!(rt.state.wishes.as_ref().unwrap()[0].wish.reserved,0);
    click(&mut rt,"wish-archive-1"); settle(&mut rt);
    assert!(rt.find("wish-empty").is_some());
}
