//! Upcoming scheduled payments.

use std::collections::HashMap;
use chrono::{Duration, NaiveDate};
use crate::models::{Cents, Id, Schedule};

pub fn effective_next_date(schedule: &Schedule) -> Option<NaiveDate> {
    if schedule.completed { return None; }
    let recurrence = schedule.recurrence.as_ref()?;
    schedule.next_date.or_else(|| recurrence.next_on_or_after(recurrence.start))
}
pub fn advance(schedule: &mut Schedule) {
    let Some(current) = effective_next_date(schedule) else { return };
    let next = current.succ_opt().and_then(|day| schedule.recurrence.as_ref()?.next_on_or_after(day));
    if let Some(date) = next { schedule.next_date = Some(date); } else { schedule.completed = true; }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpcomingItem { pub schedule_id: Id, pub name: String, pub date: NaiveDate, pub amount: Cents, pub account_id: Option<Id>, pub auto_post: bool, pub overdue: bool, pub projected_balance: Option<Cents> }
pub fn upcoming(schedules: &[Schedule], balances: &HashMap<Id, Cents>, today: NaiveDate, days: i64) -> Vec<UpcomingItem> {
    let Some(through) = today.checked_add_signed(Duration::days(days)) else { return Vec::new() };
    let mut items = Vec::new();
    for s in schedules {
        if let (Some(next), Some(rule)) = (effective_next_date(s), &s.recurrence) {
            for date in rule.occurrences(next, through) {
                items.push(UpcomingItem { schedule_id: s.id, name: s.name.clone(), date, amount: s.amount, account_id: s.account_id, auto_post: s.auto_post, overdue: date < today, projected_balance: None });
            }
        }
    }
    items.sort_by(|a,b| a.date.cmp(&b.date).then_with(|| a.name.cmp(&b.name)));
    let mut running = balances.clone();
    for item in &mut items {
        if let Some(id) = item.account_id { let balance = running.entry(id).or_default(); *balance += item.amount; item.projected_balance = Some(*balance); }
    }
    items
}
pub fn due_dates(schedule: &Schedule, today: NaiveDate, cap: usize) -> Vec<NaiveDate> {
    if !schedule.auto_post || schedule.completed { return Vec::new(); }
    let mut copy = schedule.clone();
    let mut dates = Vec::new();
    while dates.len() < cap.min(500) {
        let Some(date) = effective_next_date(&copy) else { break };
        if date > today { break; }
        dates.push(date);
        advance(&mut copy);
    }
    dates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Frequency, Recurrence, WeekendMode};
    fn d(y:i32,m:u32,day:u32)->NaiveDate { NaiveDate::from_ymd_opt(y,m,day).unwrap() }
    fn schedule(id:Id,name:&str,start:NaiveDate,frequency:Frequency,amount:Cents,account:Id)->Schedule { Schedule { id,name:name.into(),amount,auto_post:false,completed:false,next_date:None,recurrence:Some(Recurrence {start,frequency,interval:1,skip_weekend:false,weekend_mode:WeekendMode::After,end_date:None}),account_id:Some(account),category_id:None,payee_id:None } }
    #[test] fn upcoming_lists_occurrences_with_projected_balance_per_account() { let mut gym=schedule(1,"Gym",d(2026,5,11),Frequency::Monthly,-55000,1); gym.next_date=Some(d(2026,10,11)); let mut rent=schedule(2,"Renta",d(2026,1,20),Frequency::Monthly,-30000,1); rent.next_date=Some(d(2026,10,20)); let weekly=schedule(3,"Súper",d(2026,10,16),Frequency::Weekly,-1000,2); let mut done=gym.clone(); done.completed=true; let items=upcoming(&[gym,rent,weekly,done],&HashMap::from([(1,100000),(2,5000)]),d(2026,10,15),14); assert_eq!(items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),vec!["Gym","Súper","Renta","Súper"]); assert_eq!(items.iter().map(|i| i.date).collect::<Vec<_>>(),vec![d(2026,10,11),d(2026,10,16),d(2026,10,20),d(2026,10,23)]); assert_eq!(items.iter().map(|i| i.projected_balance).collect::<Vec<_>>(),vec![Some(45000),Some(4000),Some(15000),Some(3000)]); assert_eq!(items.iter().map(|i| i.overdue).collect::<Vec<_>>(),vec![true,false,false,false]); }
    #[test] fn schedule_without_stored_next_date_starts_at_its_first_occurrence() { let s=schedule(1,"Súper",d(2026,10,16),Frequency::Weekly,-1000,1); assert_eq!(effective_next_date(&s),Some(d(2026,10,16))); }
    #[test] fn skip_advances_to_the_following_occurrence() { let mut s=schedule(1,"Gym",d(2026,10,11),Frequency::Monthly,-55000,1); advance(&mut s); assert_eq!(s.next_date,Some(d(2026,11,11))); }
    #[test] fn passing_the_last_occurrence_completes_the_schedule() { let mut s=schedule(1,"Regalo",d(2026,12,24),Frequency::Monthly,-80000,1); s.recurrence.as_mut().unwrap().end_date=Some(d(2026,12,24)); advance(&mut s); assert!(s.completed); assert!(upcoming(&[s],&HashMap::new(),d(2026,12,1),60).is_empty()); }
    #[test] fn daily_weekend_shifted_schedule_lists_each_date_once() { let mut s=schedule(1,"Diario",d(2026,10,16),Frequency::Daily,-100,2); s.recurrence.as_mut().unwrap().skip_weekend=true; let items=upcoming(&[s],&HashMap::from([(2,5000)]),d(2026,10,15),5); assert_eq!(items.iter().map(|i| i.date).collect::<Vec<_>>(),vec![d(2026,10,16),d(2026,10,19),d(2026,10,20)]); assert_eq!(items.iter().map(|i| i.projected_balance).collect::<Vec<_>>(),vec![Some(4900),Some(4800),Some(4700)]); }
    #[test]
    fn advance_without_an_effective_date_preserves_schedule() {
        let mut s = schedule(1, "No recurrence", d(2026, 10, 1), Frequency::Monthly, -100, 1);
        s.recurrence = None;
        advance(&mut s);
        assert!(!s.completed);
    }
    #[test]
    fn auto_post_caps_at_500() {
        let mut s = schedule(1, "Daily", d(2020, 1, 1), Frequency::Daily, -1, 1);
        s.auto_post = true;
        assert_eq!(due_dates(&s, d(2022, 1, 1), 501).len(), 500);
    }
    #[test] fn auto_post_records_every_due_occurrence_of_automatic_schedules_only() { let mut s=schedule(1,"iCloud",d(2026,8,28),Frequency::Monthly,-1000,1); s.auto_post=true; assert_eq!(due_dates(&s,d(2026,10,15),500),vec![d(2026,8,28),d(2026,9,28)]); assert_eq!(due_dates(&s,d(2026,10,15),1).len(),1); advance(&mut s); advance(&mut s); assert_eq!(s.next_date,Some(d(2026,10,28))); assert!(due_dates(&s,d(2026,10,15),500).is_empty()); s.auto_post=false; assert!(due_dates(&s,d(2026,10,15),500).is_empty()); s.auto_post=true; s.completed=true; assert!(due_dates(&s,d(2026,10,15),500).is_empty()); }
}
