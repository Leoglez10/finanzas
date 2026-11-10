//! Calendar month helpers.

use chrono::{Datelike, NaiveDate};
use crate::models::MonthKey;

pub fn month_of(date: NaiveDate) -> MonthKey { date.year() * 100 + date.month() as i32 }
pub fn next(month: MonthKey) -> MonthKey { if month % 100 == 12 { (month / 100 + 1) * 100 + 1 } else { month + 1 } }
pub fn previous(month: MonthKey) -> MonthKey { if month % 100 == 1 { (month / 100 - 1) * 100 + 12 } else { month - 1 } }
pub fn first_day(month: MonthKey) -> NaiveDate { NaiveDate::from_ymd_opt(month / 100, (month % 100) as u32, 1).expect("valid month") }
pub fn last_day(month: MonthKey) -> NaiveDate { first_day(next(month)).pred_opt().expect("valid month") }
pub fn parse(value: &str) -> Option<MonthKey> {
    let b = value.as_bytes();
    if b.len() != 7 || b[4] != b'-' || !b[..4].iter().chain(&b[5..]).all(u8::is_ascii_digit) { return None; }
    let year: i32 = value[..4].parse().ok()?; let month: i32 = value[5..].parse().ok()?;
    if !(1..=12).contains(&month) { return None; }
    Some(year * 100 + month)
}
pub fn format(month: MonthKey) -> String { format!("{:04}-{:02}", month / 100, month % 100) }
pub fn spanish_title(month: MonthKey) -> String {
    let names = ["Enero", "Febrero", "Marzo", "Abril", "Mayo", "Junio", "Julio", "Agosto", "Septiembre", "Octubre", "Noviembre", "Diciembre"];
    format!("{} de {}", names[(month % 100 - 1) as usize], month / 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    #[test]
    fn month_keys_step_across_years() {
        assert_eq!(next(202612), 202701); assert_eq!(previous(202701), 202612);
        assert_eq!(month_of(NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()), 202609);
    }
    #[test]
    fn month_format_and_validation() {
        assert_eq!(parse("2026-10"), Some(202610)); assert_eq!(parse("2026-13"), None);
        assert_eq!(format(202610), "2026-10"); assert_eq!(spanish_title(202610), "Octubre de 2026");
        assert_eq!(last_day(202402), NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
    }
}
