//! Reminder planning.

use std::collections::HashMap;
use chrono::{Duration, NaiveDateTime};
use crate::models::Id;
use super::{money, schedule::UpcomingItem};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reminder { pub id: String, pub title: String, pub body: String, pub fire_at: NaiveDateTime }
pub fn plan(items: &[UpcomingItem], account_names: &HashMap<Id, String>, days_before: i64, hour: u32, now: NaiveDateTime) -> Vec<Reminder> {
    let mut result = Vec::new();
    for item in items {
        let Some(fire) = item.date.checked_sub_signed(Duration::days(days_before)).and_then(|d| d.and_hms_opt(hour, 0, 0)) else { continue };
        if fire <= now { continue; }
        let mut body = format!("{} el {}", money::format(item.amount), item.date.format("%d/%m/%Y"));
        if let Some(name) = item.account_id.and_then(|id| account_names.get(&id)) { body.push_str(&format!(" ({name})")); }
        if item.auto_post { body.push_str(" · se registrará automáticamente"); }
        result.push(Reminder { id: format!("pago-Schedule/{}-{}", item.schedule_id, item.date.format("%Y%m%d")), title: format!("Pago próximo: {}", item.name), body, fire_at: fire });
    }
    result.sort_by_key(|r| r.fire_at);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    fn date(y:i32,m:u32,d:u32)->NaiveDate { NaiveDate::from_ymd_opt(y,m,d).unwrap() }
    #[test] fn reminders_fire_days_before_at_the_chosen_hour_and_skip_the_past() {
        let item=|id,name:&str,date,auto_post| UpcomingItem { schedule_id:id,name:name.into(),date,amount:-1000,account_id:Some(1),auto_post,overdue:false,projected_balance:None };
        let items=[item(1,"Gym",date(2026,10,11),false),item(2,"iCloud",date(2026,10,28),true),item(1,"Gym",date(2026,11,11),false)];
        let p=plan(&items,&HashMap::from([(1,"Banco".into())]),2,9,date(2026,10,15).and_hms_opt(12,0,0).unwrap());
        assert_eq!(p.len(),2); assert_eq!(p[0].fire_at,date(2026,10,26).and_hms_opt(9,0,0).unwrap()); assert_eq!(p[1].fire_at,date(2026,11,9).and_hms_opt(9,0,0).unwrap());
        assert_eq!(p[0].title,"Pago próximo: iCloud"); assert_eq!(p[0].body,"-$10.00 el 28/10/2026 (Banco) · se registrará automáticamente"); assert_eq!(p[1].body,"-$10.00 el 11/11/2026 (Banco)"); assert_eq!(p[0].id,"pago-Schedule/2-20261028");
    }

    #[test]
    fn excludes_equal_fire_time_and_omits_missing_account() {
        let date = date(2026, 10, 28);
        let item = UpcomingItem { schedule_id: 5, name: "Test".into(), date, amount: -100, account_id: None, auto_post: false, overdue: false, projected_balance: None };
        let fire = date.and_hms_opt(9, 0, 0).unwrap();
        assert!(plan(std::slice::from_ref(&item), &HashMap::new(), 0, 9, fire).is_empty());
        let planned = plan(&[item], &HashMap::new(), 0, 9, date.and_hms_opt(8, 59, 0).unwrap());
        assert_eq!(planned[0].body, "-$1.00 el 28/10/2026");
    }
}
