use super::*;
use rune::{Env, Runtime};

fn settle(rt: &mut Runtime<State>) {
    for _ in 0..120 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); }
}
fn click(rt: &mut Runtime<State>, key: &str) {
    let (x,y) = rt.find(key).unwrap_or_else(|| panic!("missing {key}"));
    rt.pointer_down(x,y); rt.pointer_up(x,y); rt.frame(0.0);
}
fn fill(rt: &mut Runtime<State>, key: &str, value: &str) {
    for _ in 0..60 { rt.frame(1.0 / 60.0); }
    click(rt,key);
    rt.key(KeyEvent { key:Key::Command('a'), mods:rune::Mods {cmd:true,..Default::default()} });
    rt.key(KeyEvent { key:Key::Text(value.into()), mods:Default::default() });
    rt.frame(0.0);
}
fn runtime() -> Runtime<State> {
    let mut s = State::empty(); scene(&mut s,true,false);
    let mut rt = Runtime::new(s,view,(1200.0,900.0),Env::new(false,1.0));
    rt.poll = Some(poll); rt.on_key = Some(on_key); rt.frame(0.0); rt
}
#[test]
fn wishes_navigation_form_validation_and_cancel() {
    let mut rt = runtime();
    click(&mut rt,"deseos"); settle(&mut rt);
    assert!(rt.find("wish-empty").is_some());
    click(&mut rt,"wish-new");
    assert!(rt.find("wish-sheet").is_some());
    fill(&mut rt,"wish-name","PlayStation");
    fill(&mut rt,"wish-price","500");
    fill(&mut rt,"wish-url","https://example.com/ps5");
    fill(&mut rt,"wish-notes","Con mando");
    click(&mut rt,"wish-save"); settle(&mut rt);
    assert!(rt.find("wish-sheet").is_none(), "error={:?}; fields={:?}",rt.state.mutation_error,rt.state.wish_sheet.as_ref().map(|sh| (&sh.name.text,&sh.price.text,&sh.url.text,&sh.notes.text)));
    assert!(rt.find("wish-edit-1").is_some());
    click(&mut rt,"wish-edit-1");
    fill(&mut rt,"wish-name","Sin guardar");
    click(&mut rt,"wish-cancel");
    click(&mut rt,"wish-edit-1");
    assert_eq!(rt.state.wish_sheet.as_ref().unwrap().name.text,"PlayStation");
    rt.key(KeyEvent {key:Key::Escape,mods:Default::default()}); rt.frame(0.0);
    assert!(rt.find("wish-sheet").is_none());
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
    fill(&mut rt,"wish-reserve-input-1","999999");
    click(&mut rt,"wish-reserve-1"); settle(&mut rt);
    assert_eq!(rt.state.wishes.as_ref().unwrap()[0].wish.reserved,5_000);
    assert!(rt.find("wish-error").is_some());
    click(&mut rt,"wish-release-1"); settle(&mut rt);
    assert_eq!(rt.state.wishes.as_ref().unwrap()[0].wish.reserved,0);
    click(&mut rt,"wish-archive-1"); settle(&mut rt);
    assert!(rt.find("wish-empty").is_some());
}
