use chrono::NaiveDate;
use finanzas_core::{service::*, store::Store};

fn day() -> NaiveDate { NaiveDate::from_ymd_opt(2026, 11, 10).unwrap() }
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
const SVG: &[u8] = b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1 1\"/>";

#[test]
fn accounts_have_a_default_look_until_a_preset_icon_and_color_are_chosen() {
    let mut store = Store::open_in_memory().unwrap();
    let bank = create_account(&mut store, "Nu", false, None, day(), None).unwrap();
    let saving = create_account(&mut store, "Ahorro", true, None, day(), None).unwrap();
    let styles = account_styles(&store).unwrap();
    assert!(styles.get(&bank.id).is_none(), "no row until something is chosen");
    set_account_style(&store, bank.id, Some("bank"), Some("violet")).unwrap();
    set_account_style(&store, saving.id, Some("piggy"), None).unwrap();
    let styles = account_styles(&store).unwrap();
    assert_eq!((styles[&bank.id].icon.as_deref(), styles[&bank.id].color.as_deref(), styles[&bank.id].image.as_deref()), (Some("bank"), Some("violet"), None));
    assert_eq!((styles[&saving.id].icon.as_deref(), styles[&saving.id].color.as_deref()), (Some("piggy"), None));
    // None clears that field back to the default.
    set_account_style(&store, bank.id, None, Some("green")).unwrap();
    assert_eq!((account_styles(&store).unwrap()[&bank.id].icon.as_deref(), account_styles(&store).unwrap()[&bank.id].color.as_deref()), (None, Some("green")));
    assert!(ACCOUNT_ICONS.contains(&"bank") && ACCOUNT_COLORS.contains(&"violet"));
    assert!(set_account_style(&store, bank.id, Some("rocket"), None).is_err());
    assert!(set_account_style(&store, bank.id, None, Some("#ff0000")).is_err());
    assert!(set_account_style(&store, 999, Some("bank"), None).is_err());
}

#[test]
fn own_images_are_sniffed_limited_kept_with_the_style_and_removed_with_the_account() {
    let mut store = Store::open_in_memory().unwrap();
    let bank = create_account(&mut store, "BBVA", false, None, day(), None).unwrap();
    set_account_style(&store, bank.id, Some("bank"), Some("blue")).unwrap();
    set_account_image(&store, bank.id, Some(PNG)).unwrap();
    let style = &account_styles(&store).unwrap()[&bank.id];
    assert!(style.image.as_deref().unwrap().starts_with("data:image/png;base64,"), "{:?}", style.image);
    assert_eq!(style.icon.as_deref(), Some("bank"), "the icon stays as the fallback");
    set_account_image(&store, bank.id, Some(SVG)).unwrap();
    assert!(account_styles(&store).unwrap()[&bank.id].image.as_deref().unwrap().starts_with("data:image/svg+xml;base64,"));
    assert!(set_account_image(&store, bank.id, Some(b"GIF89a....")).is_err(), "only PNG, JPEG, WebP or SVG");
    assert!(set_account_image(&store, bank.id, Some(&vec![0u8; 600 * 1024])).is_err(), "too big");
    // Setting the icon or color keeps the image.
    set_account_style(&store, bank.id, Some("card"), Some("blue")).unwrap();
    assert!(account_styles(&store).unwrap()[&bank.id].image.is_some());
    set_account_image(&store, bank.id, None).unwrap();
    assert!(account_styles(&store).unwrap()[&bank.id].image.is_none());
    set_account_image(&store, bank.id, Some(PNG)).unwrap();
    store.delete_account(bank.id).unwrap();
    assert!(account_styles(&store).unwrap().is_empty(), "the style goes with its account");
}

#[test]
fn a_new_account_can_start_with_its_style() {
    let mut store = Store::open_in_memory().unwrap();
    let acc = create_account(&mut store, "Nu", false, None, day(), None).unwrap();
    set_account_style(&store, acc.id, Some("card"), Some("violet")).unwrap();
    assert_eq!(account_styles(&store).unwrap()[&acc.id].color.as_deref(), Some("violet"));
}
