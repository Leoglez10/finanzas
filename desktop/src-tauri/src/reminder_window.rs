use chrono::NaiveDateTime;
use std::collections::HashSet;

pub fn due(id: &str, fire: NaiveDateTime, since: NaiveDateTime, now: NaiveDateTime, sent: &HashSet<String>) -> bool {
    fire > since && fire <= now && !sent.contains(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn includes_only_unsent_reminders_in_the_check_window() {
        let time = |minute| chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap().and_hms_opt(9, minute, 0).unwrap();
        let mut sent = HashSet::new();
        assert!(due("one", time(1), time(0), time(2), &sent));
        assert!(!due("one", time(0), time(0), time(2), &sent));
        assert!(due("one", time(2), time(0), time(2), &sent));
        assert!(!due("one", time(3), time(0), time(2), &sent));
        sent.insert("one".to_string());
        assert!(!due("one", time(1), time(0), time(2), &sent));
    }
}
