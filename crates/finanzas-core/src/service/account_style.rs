//! How an account looks: a preset icon and color, or the user's own image. Logos are never shipped with the
//! app; an image only comes from the user (or an agent acting for them) and lives in their database.

use super::{Result, ServiceError};
use crate::{store::Store, Id};
use serde::Serialize;
use std::collections::HashMap;

/// Icon names the app draws (sprite ids `i-<name>` in desktop/index.html).
pub const ACCOUNT_ICONS: [&str; 12] = ["bank", "card", "cash", "wallet", "piggy", "chart", "house", "car", "gift", "shield", "star", "plane"];
/// Color names the app maps to its palette.
pub const ACCOUNT_COLORS: [&str; 10] = ["violet", "blue", "teal", "green", "lime", "yellow", "orange", "red", "pink", "gray"];
const MAX_IMAGE: usize = 512 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStyle {
    pub icon: Option<String>, pub color: Option<String>,
    /// A `data:` URL ready for an <img>.
    pub image: Option<String>,
}

fn invalid(message: &'static str) -> ServiceError { ServiceError::Invalid(message) }

/// Only accounts with something chosen are in the map.
pub fn account_styles(store: &Store) -> Result<HashMap<Id, AccountStyle>> {
    Ok(store.account_style_rows()?.into_iter().map(|(id, icon, color, image)| {
        (id, AccountStyle { icon, color, image: image.map(|(mime, bytes)| format!("data:{mime};base64,{}", base64(&bytes))) })
    }).collect())
}

/// Sets the icon and color; `None` puts that one back to the default. The image is kept.
pub fn set_account_style(store: &Store, account: Id, icon: Option<&str>, color: Option<&str>) -> Result<()> {
    if icon.is_some_and(|i| !ACCOUNT_ICONS.contains(&i)) { return Err(invalid("Icono desconocido")); }
    if color.is_some_and(|c| !ACCOUNT_COLORS.contains(&c)) { return Err(invalid("Color desconocido")); }
    if !store.account_exists(account)? { return Err(ServiceError::NotFound); }
    Ok(store.put_account_look(account, icon, color)?)
}

/// Sets the account's own image (PNG, JPEG, WebP or SVG, up to 512 KB), or removes it with `None`.
pub fn set_account_image(store: &Store, account: Id, image: Option<&[u8]>) -> Result<()> {
    let image = match image {
        None => None,
        Some(bytes) => {
            if bytes.len() > MAX_IMAGE { return Err(invalid("La imagen pesa más de 512 KB")); }
            Some((image_type(bytes).ok_or(invalid("Usa una imagen PNG, JPG, WebP o SVG"))?, bytes))
        }
    };
    if !store.account_exists(account)? { return Err(ServiceError::NotFound); }
    Ok(store.put_account_image(account, image)?)
}

/// The type from the file's first bytes, not its name.
fn image_type(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") { return Some("image/png"); }
    if b.starts_with(&[0xFF, 0xD8, 0xFF]) { return Some("image/jpeg"); }
    if b.len() > 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" { return Some("image/webp"); }
    let head = String::from_utf8_lossy(&b[..b.len().min(1024)]).trim_start_matches('\u{feff}').trim_start().to_ascii_lowercase();
    (head.starts_with("<svg") || (head.starts_with("<?xml") && head.contains("<svg"))).then_some("image/svg+xml")
}

// ponytail: base64 here so finanzas-core takes no new dependency for a few lines.
fn base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 { out.push(if i <= c.len() { A[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' }); }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_the_standard_alphabet_and_padding() {
        assert_eq!([&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar", &[0xFB, 0xFF]].map(super::base64), ["", "Zg==", "Zm8=", "Zm9v", "Zm9vYg==", "Zm9vYmE=", "Zm9vYmFy", "+/8="]);
    }
}
