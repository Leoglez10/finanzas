//! Money input and formatting.

use crate::models::Cents;

pub fn cents_from(input: &str) -> Option<Cents> {
    let cleaned: String = input.chars().filter(|c| !matches!(c, ' ' | '$' | ',' | '-')).collect();
    let mut parts = cleaned.split('.');
    let whole = parts.next()?;
    let fraction = parts.next();
    if parts.next().is_some() || whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) { return None; }
    let mut value = whole.parse::<Cents>().ok()?.checked_mul(100)?;
    if let Some(fraction) = fraction {
        if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) { return None; }
        let digits = fraction.as_bytes();
        value = value.checked_add(i64::from(digits[0] - b'0') * 10)?;
        if digits.len() > 1 { value = value.checked_add(i64::from(digits[1] - b'0'))?; }
        if digits.len() > 2 && digits[2] >= b'5' { value = value.checked_add(1)?; }
    }
    Some(value)
}

pub fn budgeted_from(input: &str, current: Cents) -> Option<Cents> {
    let text = input.trim();
    let value = cents_from(&text.replace('+', ""))?;
    match text.chars().next() {
        Some('+') => current.checked_add(value),
        Some('-') => Some(current.saturating_sub(value).max(0)),
        _ => Some(value),
    }
}

pub fn format(cents: Cents) -> String {
    let magnitude = cents.unsigned_abs();
    let whole = (magnitude / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) { grouped.push(','); }
        grouped.push(ch);
    }
    format!("{}${}.{:02}", if cents < 0 { "-" } else { "" }, grouped, magnitude % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typed_amounts_to_positive_cents() {
        for (text, expected) in [("125.50", 12_550), ("$1,250.5", 125_050), (" 80 ", 8_000), ("0.07", 7), ("-40", 4_000), ("12.345", 1_235)] {
            assert_eq!(cents_from(text), Some(expected));
        }
    }

    #[test]
    fn rejects_invalid_amounts() {
        for text in ["", "abc", "1.2.3"] { assert_eq!(cents_from(text), None); }
    }

    #[test]
    fn budget_input_sets_or_adjusts_the_assigned_amount() {
        for (text, current, expected) in [("500", 20_000, 50_000), ("+100", 20_000, 30_000), ("-50", 20_000, 15_000), ("- 300", 20_000, 0), ("+$1,000.50", 0, 100_050)] {
            assert_eq!(budgeted_from(text, current), Some(expected));
        }
    }

    #[test]
    fn budget_input_rejects_invalid_text() { assert_eq!(budgeted_from("+abc", 20_000), None); }

    #[test]
    fn formatting_and_rounding_boundaries() {
        assert_eq!(format(123456), "$1,234.56");
        assert_eq!(format(-123456), "-$1,234.56");
        assert_eq!(cents_from("1.0049"), Some(100));
        assert_eq!(cents_from("1.999"), Some(200));
        assert_eq!(cents_from("1."), None);
        assert_eq!(cents_from("999999999999999999999999"), None);
    }
}
