use chrono::NaiveDate;
use finanzas_core::models::Id;

pub fn parse_date(text: &str) -> Option<NaiveDate> {
    let b = text.as_bytes();
    if b.len() != 10 || b[2] != b'/' || b[5] != b'/' || !b.iter().enumerate().all(|(i, c)| i == 2 || i == 5 || c.is_ascii_digit()) { return None; }
    NaiveDate::from_ymd_opt(text[6..10].parse().ok()?, text[3..5].parse().ok()?, text[0..2].parse().ok()?)
}
pub fn header_label(search: &str) -> &'static str {
    if search.trim().is_empty() { "Saldo de los movimientos" } else { "Total de la búsqueda" }
}
pub fn valid_sheet(amount: &str, account: Option<Id>, date: &str) -> bool {
    finanzas_core::domain::money::cents_from(amount).is_some() && account.is_some() && parse_date(date).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dates_require_exact_valid_day_month_year() {
        assert_eq!(parse_date("29/02/2024"), NaiveDate::from_ymd_opt(2024, 2, 29));
        for text in ["29/02/2023", "1/02/2024", "01-02-2024", "01/13/2024", "01/02/2024 "] { assert_eq!(parse_date(text), None); }
    }
    #[test]
    fn filtered_header_depends_on_nonempty_search() {
        assert_eq!(header_label(""), "Saldo de los movimientos");
        assert_eq!(header_label("  "), "Saldo de los movimientos");
        assert_eq!(header_label("café"), "Total de la búsqueda");
    }
    #[test]
    fn saving_requires_valid_amount_account_and_date() {
        assert!(valid_sheet("10.25", Some(1), "10/11/2026"));
        assert!(!valid_sheet("bad", Some(1), "10/11/2026"));
        assert!(!valid_sheet("10", None, "10/11/2026"));
        assert!(!valid_sheet("10", Some(1), "31/02/2026"));
    }
}
