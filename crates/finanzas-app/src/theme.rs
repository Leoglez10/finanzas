//! Shared visual tokens for the dashboard look: type scale, large amounts and icons.
use super::*;
use rune::{Icon, tok::TextStyle};

pub const HEADLINE: TextStyle = TextStyle::new(22.0, 650.0);
pub const HERO: TextStyle = TextStyle::new(38.0, 700.0);
pub const AMOUNT: TextStyle = TextStyle::new(26.0, 700.0);
pub const CENTS_HERO: TextStyle = TextStyle::new(19.0, 500.0);
pub const CENTS: TextStyle = TextStyle::new(14.0, 500.0);
pub const CARD_TITLE: TextStyle = TextStyle::new(13.5, 600.0);

/// An amount with the cents set smaller, as in "$5,304" + ".28".
pub fn money_big(cents: i64, hero: bool, color: Color, p: Palette) -> El {
    let (whole, fraction) = metrics::split_money(cents);
    row().items_end().gap(1.0)
        .child(text(whole).font(if hero { HERO } else { AMOUNT }).color(color))
        .child(text(fraction).font(if hero { CENTS_HERO } else { CENTS }).color(p.muted).pb(if hero { 5.0 } else { 3.0 }))
}

pub fn card_title(value: &str, p: Palette) -> El { text(value.to_owned()).font(CARD_TITLE).color(p.ink) }

/// A tinted square holding an icon, used in the sidebar and as card illustrations.
pub fn badge(i: &'static Icon, size: f32, tint: Color) -> El {
    el().size(size * 1.7, size * 1.7).radius(size * 0.5).fill(tint.alpha(0.14)).center().child(icon(i, size).color(tint))
}

pub fn icon_button(i: &'static Icon, name: &'static str, key: &'static str, action: impl Fn(&mut State) + 'static, p: Palette) -> El {
    el().key(key).role("button").label(name).focusable(true).size(32.0, 32.0).radius(10.0).center().border(1.0, p.border).hover(p.selected).press_scale(0.94)
        .on_press(action).child(icon(i, 16.0).color(p.ink))
}
/// Filled call to action; `disabled` greys it out and drops the handler.
pub fn primary(i: &'static Icon, name: &str, key: impl Into<String>, action: impl Fn(&mut State) + 'static, p: Palette) -> El {
    row().key(key.into()).role("button").label(name).focusable(true).gap(6.0).px(14.0).py(8.0).radius(10.0).center_y().fill(p.petroleum).hover(p.petroleum.alpha(0.85)).press_scale(0.96)
        .on_press(action).child(icon(i, 14.0).color(p.card)).child(text(name.to_owned()).font(type_::LABEL).color(p.card))
}
pub fn primary_disabled(i: &'static Icon, name: &str, key: impl Into<String>, p: Palette) -> El {
    row().key(key.into()).role("button").label(name).focusable(true).disabled(true).gap(6.0).px(14.0).py(8.0).radius(10.0).center_y().fill(p.border)
        .child(icon(i, 14.0).color(p.muted)).child(text(name.to_owned()).font(type_::LABEL).color(p.muted))
}
pub fn secondary(i: &'static Icon, name: &str, key: impl Into<String>, action: impl Fn(&mut State) + 'static, p: Palette) -> El {
    row().key(key.into()).role("button").label(name).focusable(true).gap(6.0).px(14.0).py(8.0).radius(10.0).center_y().border(1.0, p.border).hover(p.selected).press_scale(0.96)
        .on_press(action).child(icon(i, 14.0).color(p.ink)).child(text(name.to_owned()).font(type_::LABEL).color(p.ink))
}
/// The rounded bar at the top of every screen.
pub fn header_bar(p: Palette) -> El { row().w_full().px(18.0).py(14.0).gap(12.0).radius(18.0).fill(p.card).border(1.0, p.border).center_y().window_drag() }
/// Hover wash and a slight press shrink for anything clickable.
pub fn tappable(e: El, p: Palette) -> El { e.role("button").focusable(true).radius(10.0).hover(p.selected).press_scale(0.98) }
/// Small caption over a large title inside the header bar.
pub fn titles(eyebrow: &str, title: El, p: Palette) -> El {
    col().gap(2.0).min_w(0.0).child(text(eyebrow.to_owned()).font(type_::LABEL).color(p.muted)).child(title)
}
pub fn headline(value: impl Into<String>, p: Palette) -> El { text(value.into()).font(HEADLINE).color(p.ink) }
/// Card title with a tinted icon in front.
pub fn icon_title(i: &'static Icon, value: &str, tint: Color, p: Palette) -> El {
    row().gap(10.0).center_y().child(badge(i, 14.0, tint)).child(card_title(value, p))
}

/// A thin colored dot for legends.
pub fn dot(c: Color) -> El { el().size(8.0, 8.0).radius(4.0).fill(c) }

macro_rules! line_icon {
    ($id:ident, $name:literal, $body:literal) => {
        pub static $id: Icon = Icon { name: $name, view: 24.0, body: concat!(r#"<g fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">"#, $body, "</g>") };
    };
}
line_icon!(GRID, "grid", r#"<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>"#);
line_icon!(PIE, "pie", r#"<path d="M21 12A9 9 0 1 1 12 3V12Z"/><path d="M15 3.5A9 9 0 0 1 20.5 9H15Z"/>"#);
line_icon!(CALENDAR, "calendar", r#"<rect x="3" y="4.5" width="18" height="16.5" rx="2.5"/><path d="M3 10H21M8 2.5V6.5M16 2.5V6.5"/>"#);
line_icon!(LIST, "list", r#"<path d="M8 6H21M8 12H21M8 18H21M3.5 6H4M3.5 12H4M3.5 18H4"/>"#);
line_icon!(CARD, "card", r#"<rect x="2.5" y="5" width="19" height="14" rx="2.5"/><path d="M2.5 10H21.5M6.5 15H10"/>"#);
line_icon!(PLUS, "plus", r#"<path d="M12 5V19M5 12H19"/>"#);
line_icon!(MINUS, "minus", r#"<path d="M5 12H19"/>"#);
line_icon!(GEAR, "gear", r#"<circle cx="12" cy="12" r="3"/><path d="M12 2.5V5M12 19V21.5M2.5 12H5M19 12H21.5M5.3 5.3L7 7M17 17L18.7 18.7M5.3 18.7L7 17M17 7L18.7 5.3"/>"#);
line_icon!(LEFT, "chevron-left", r#"<path d="M15 6L9 12L15 18"/>"#);
line_icon!(RIGHT, "chevron-right", r#"<path d="M9 6L15 12L9 18"/>"#);
line_icon!(TREND_UP, "trend-up", r#"<path d="M7 17L17 7M9 7H17V15"/>"#);
line_icon!(TREND_DOWN, "trend-down", r#"<path d="M7 7L17 17M17 9V17H9"/>"#);
line_icon!(BELL, "bell", r#"<path d="M6 16V11A6 6 0 0 1 18 11V16L19.5 18H4.5Z"/><path d="M10 21H14"/>"#);
line_icon!(DOWNLOAD, "download", r#"<path d="M12 4V15M7 10L12 15L17 10M4 20H20"/>"#);
line_icon!(DATABASE, "database", r#"<ellipse cx="12" cy="5.5" rx="8" ry="3"/><path d="M4 5.5V18.5C4 20.2 7.6 21.5 12 21.5S20 20.2 20 18.5V5.5M4 12C4 13.7 7.6 15 12 15S20 13.7 20 12"/>"#);
line_icon!(SEARCH, "search", r#"<circle cx="11" cy="11" r="7"/><path d="M20 20L16.2 16.2"/>"#);
line_icon!(REPEAT, "repeat", r#"<path d="M17 2L21 6L17 10M3 12V10A4 4 0 0 1 7 6H21M7 22L3 18L7 14M21 12V14A4 4 0 0 1 17 18H3"/>"#);
line_icon!(CHECK, "check", r#"<path d="M5 12.5L10 17.5L19 7"/>"#);
line_icon!(ARROW, "arrow-right", r#"<path d="M5 12H19M13 6L19 12L13 18"/>"#);
line_icon!(CLOCK, "clock", r#"<circle cx="12" cy="12" r="9"/><path d="M12 7V12L15 14"/>"#);

/// Flat colored illustrations; their bodies name their own colors, so tinting doesn't apply.
pub static WALLET_ART: Icon = Icon { name: "wallet-art", view: 48.0, body: r##"<rect x="9" y="7" width="26" height="15" rx="2" fill="#34C78A" transform="rotate(-8 22 14)"/><rect x="12" y="9" width="26" height="15" rx="2" fill="#5EDCA4" transform="rotate(-3 25 16)"/><rect x="4" y="15" width="40" height="27" rx="6" fill="#155E75"/><rect x="4" y="15" width="40" height="7" rx="3" fill="#1E7A96"/><rect x="29" y="25" width="15" height="10" rx="4" fill="#0E4556"/><circle cx="35" cy="30" r="2.2" fill="#C9D6E0"/>"## };
pub static CALENDAR_ART: Icon = Icon { name: "calendar-art", view: 48.0, body: r##"<rect x="5" y="9" width="38" height="33" rx="6" fill="#FFFFFF" stroke="#D5E0E8" stroke-width="1.5"/><path d="M5 15A6 6 0 0 1 11 9H37A6 6 0 0 1 43 15V19H5Z" fill="#2F6FED"/><rect x="13" y="5" width="4" height="9" rx="2" fill="#1B3F8C"/><rect x="31" y="5" width="4" height="9" rx="2" fill="#1B3F8C"/><rect x="11" y="24" width="6" height="5" rx="1.5" fill="#C7D7F5"/><rect x="21" y="24" width="6" height="5" rx="1.5" fill="#2F6FED"/><rect x="31" y="24" width="6" height="5" rx="1.5" fill="#C7D7F5"/><rect x="11" y="32" width="6" height="5" rx="1.5" fill="#C7D7F5"/><rect x="21" y="32" width="6" height="5" rx="1.5" fill="#C7D7F5"/><circle cx="36" cy="36" r="8" fill="#F5B83D"/><path d="M36 31.5V40.5M33.5 33.5H37.2A1.8 1.8 0 0 1 37.2 37.1H34.8A1.8 1.8 0 0 0 34.8 40.5H38.5" stroke="#8A5A00" stroke-width="1.4" fill="none" stroke-linecap="round"/>"## };
pub static PIE_ART: Icon = Icon { name: "pie-art", view: 48.0, body: r##"<circle cx="24" cy="26" r="17" fill="#E3ECF3"/><path d="M24 26V9A17 17 0 0 1 41 26Z" fill="#2F6FED"/><path d="M24 26H41A17 17 0 0 1 14 39.8Z" fill="#E8476A"/><path d="M24 26L14 39.8A17 17 0 0 1 24 9Z" fill="#34C78A"/><circle cx="24" cy="26" r="7" fill="#FFFFFF"/>"## };
pub static BARS_ART: Icon = Icon { name: "bars-art", view: 48.0, body: r##"<rect x="6" y="40" width="36" height="3" rx="1.5" fill="#C9D6E0"/><rect x="9" y="24" width="7" height="16" rx="2" fill="#34C78A"/><rect x="20" y="14" width="7" height="26" rx="2" fill="#2F6FED"/><rect x="31" y="20" width="7" height="20" rx="2" fill="#E8476A"/>"## };
