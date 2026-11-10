//! Recurrence expansion.

use chrono::{Datelike, Duration, Months, NaiveDate, Weekday};
use crate::models::{Frequency, Recurrence, WeekendMode};

impl Recurrence {
    pub fn occurrences(&self, from: NaiveDate, through: NaiveDate) -> Vec<NaiveDate> {
        if from > through { return Vec::new(); }
        let limit = through.checked_add_signed(Duration::days(3)).unwrap_or(NaiveDate::MAX);
        let mut result = Vec::new();
        let mut n = 0_u32;
        while let Some(steps) = n.checked_mul(self.interval.max(1)) {
            let raw = match self.frequency {
                Frequency::Daily => self.start.checked_add_signed(Duration::days(i64::from(steps))),
                Frequency::Weekly => self.start.checked_add_signed(Duration::days(i64::from(steps) * 7)),
                Frequency::Monthly => self.start.checked_add_months(Months::new(steps)),
                Frequency::Yearly => steps.checked_mul(12).and_then(|m| self.start.checked_add_months(Months::new(m))),
            };
            let Some(raw) = raw else { break };
            if raw > limit || self.end_date.is_some_and(|end| raw > end) { break; }
            let mut adjusted = raw;
            if self.skip_weekend {
                while matches!(adjusted.weekday(), Weekday::Sat | Weekday::Sun) {
                    let delta = if self.weekend_mode == WeekendMode::After { 1 } else { -1 };
                    let Some(next) = adjusted.checked_add_signed(Duration::days(delta)) else { break };
                    adjusted = next;
                }
            }
            if adjusted >= from && adjusted <= through && result.last() != Some(&adjusted) { result.push(adjusted); }
            let Some(next) = n.checked_add(1) else { break }; n = next;
        }
        result
    }
    pub fn next_on_or_after(&self, date: NaiveDate) -> Option<NaiveDate> {
        let through = date.checked_add_months(Months::new(24)).unwrap_or(NaiveDate::MAX);
        self.occurrences(date, through).into_iter().next()
    }
    pub fn describe(&self) -> String {
        let interval = self.interval.max(1);
        let (singular, plural) = match self.frequency { Frequency::Daily => ("Diario", "días"), Frequency::Weekly => ("Semanal", "semanas"), Frequency::Monthly => ("Mensual", "meses"), Frequency::Yearly => ("Anual", "años") };
        if interval == 1 { singular.to_owned() } else { format!("Cada {interval} {plural}") }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Frequency, WeekendMode};
    fn d(y: i32, m: u32, day: u32) -> NaiveDate { NaiveDate::from_ymd_opt(y,m,day).unwrap() }
    fn rule(start: NaiveDate, frequency: Frequency) -> Recurrence { Recurrence { start, frequency, interval: 1, skip_weekend: false, weekend_mode: WeekendMode::After, end_date: None } }
    #[test] fn monthly_occurrences_within_range() { let r=rule(d(2026,4,28),Frequency::Monthly); assert_eq!(r.occurrences(d(2026,10,1),d(2026,12,31)),vec![d(2026,10,28),d(2026,11,28),d(2026,12,28)]); }
    #[test] fn month_end_clamps_without_drift() { let r=rule(d(2026,1,31),Frequency::Monthly); assert_eq!(r.occurrences(d(2026,1,1),d(2026,4,30)),vec![d(2026,1,31),d(2026,2,28),d(2026,3,31),d(2026,4,30)]); }
    #[test] fn interval_and_weekly() { let mut r=rule(d(2026,10,1),Frequency::Weekly); r.interval=2; assert_eq!(r.occurrences(d(2026,10,1),d(2026,10,31)),vec![d(2026,10,1),d(2026,10,15),d(2026,10,29)]); }
    #[test] fn weekend_moves_after_or_before() { let mut r=rule(d(2026,11,28),Frequency::Monthly); r.skip_weekend=true; assert_eq!(r.next_on_or_after(d(2026,11,1)),Some(d(2026,11,30))); r.weekend_mode=WeekendMode::Before; assert_eq!(r.next_on_or_after(d(2026,11,1)),Some(d(2026,11,27))); }
    #[test] fn end_date_stops_occurrences() { let mut r=rule(d(2026,10,5),Frequency::Monthly); r.end_date=Some(d(2026,11,5)); assert_eq!(r.occurrences(d(2026,10,1),d(2027,1,31)),vec![d(2026,10,5),d(2026,11,5)]); assert_eq!(r.next_on_or_after(d(2026,11,6)),None); }
    #[test] fn weekend_shift_never_repeats_a_date() { let mut r=rule(d(2026,10,16),Frequency::Daily); r.skip_weekend=true; assert_eq!(r.occurrences(d(2026,10,16),d(2026,10,20)),vec![d(2026,10,16),d(2026,10,19),d(2026,10,20)]); r.start=d(2026,10,15); r.weekend_mode=WeekendMode::Before; assert_eq!(r.occurrences(d(2026,10,15),d(2026,10,19)),vec![d(2026,10,15),d(2026,10,16),d(2026,10,19)]); }
}
