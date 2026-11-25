use super::{Result, ServiceError};
use crate::{Entry, Frequency, Id, Recurrence, Schedule, WeekendMode, domain::schedule, store::{EntryRow, Store}};
use chrono::{Months, NaiveDate, NaiveDateTime};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleView { pub schedule: Schedule, pub next_date: Option<NaiveDate>, pub label: String, pub payee: Option<String>, pub account: Option<String> }
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpcomingView { pub schedule_id: Id, pub name: String, pub date: NaiveDate, pub amount: i64, pub account_id: Option<Id>, pub account: Option<String>, pub auto_post: bool, pub overdue: bool, pub projected_balance: Option<i64> }
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderView { pub id: String, pub title: String, pub body: String, pub fire_at: NaiveDateTime }
fn account_names(store: &Store) -> Result<HashMap<Id, String>> { Ok(store.list_accounts()?.into_iter().map(|a| (a.id, a.name)).collect()) }
fn upcoming_items(store: &Store, today: NaiveDate, days: i64) -> Result<Vec<schedule::UpcomingItem>> { Ok(schedule::upcoming(&store.list_schedules()?, &store.account_balances()?, today, days)) }
pub fn schedule_views(store: &Store) -> Result<Vec<ScheduleView>> {
    let accounts = account_names(store)?;
    let payees: HashMap<_, _> = store.list_payees()?.into_iter().map(|p| (p.id, p.name)).collect();
    Ok(store.list_schedules()?.into_iter().map(|schedule| {
        let next_date = schedule::effective_next_date(&schedule);
        let label = if schedule.completed { "Terminado".to_owned() } else { let mut label = schedule.recurrence.as_ref().map(|r| r.describe()).unwrap_or_default(); if let Some(date) = next_date { label.push_str(&format!(" · próximo {}", date.format("%d.%m.%Y"))); }
        if schedule.auto_post { label.push_str(" · automático"); } label };
        let account = schedule.account_id.and_then(|id| accounts.get(&id).cloned());
        let payee = schedule.payee_id.and_then(|id| payees.get(&id).cloned());
        ScheduleView { schedule, next_date, label, payee, account }
    }).collect())
}
pub fn upcoming(store: &Store, today: NaiveDate, days: i64) -> Result<Vec<UpcomingView>> {
    let names = account_names(store)?;
    Ok(upcoming_items(store, today, days)?.into_iter().map(|i| UpcomingView { schedule_id: i.schedule_id, name: i.name, date: i.date, amount: i.amount, account_id: i.account_id, account: i.account_id.and_then(|id| names.get(&id).cloned()), auto_post: i.auto_post, overdue: i.overdue, projected_balance: i.projected_balance }).collect())
}
pub fn reminders(store: &Store, now: NaiveDateTime, days_before: i64, hour: u32) -> Result<Vec<ReminderView>> {
    Ok(crate::domain::reminders::plan(&upcoming_items(store, now.date(), 60)?, &account_names(store)?, days_before, hour, now).into_iter().map(|r| ReminderView { id: r.id, title: r.title, body: r.body, fire_at: r.fire_at }).collect())
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkipResult { pub previous_next_date: Option<NaiveDate>, pub next_date: Option<NaiveDate>, pub completed: bool, pub skipped: usize }
fn post(store: &Store, id: Id) -> Result<EntryRow> {
    let mut s = store.get_schedule(id)?.ok_or(ServiceError::NotFound)?;
    let date = schedule::effective_next_date(&s).ok_or(ServiceError::CompletedSchedule)?;
    let category_id = match (s.category_id, s.payee_id) { (None, Some(id)) => store.suggested_category(id)?, (category, _) => category };
    let entry = Entry { id: 0, date, amount: s.amount, notes: String::new(), cleared: false, account_id: s.account_id, category_id, payee_id: s.payee_id, schedule_id: Some(id) };
    let entry_id = store.insert_entry(&entry)?;
    schedule::advance(&mut s);
    store.update_schedule(&s)?;
    store.entry_row(entry_id)?.ok_or(ServiceError::NotFound)
}
pub fn post_schedule(store: &mut Store, id: Id, _today: NaiveDate) -> Result<EntryRow> { store.transaction(|store| post(store, id)) }
pub fn skip_schedule(store: &mut Store, id: Id, through: Option<NaiveDate>) -> Result<SkipResult> {
    store.transaction(|store| {
        let mut s = store.get_schedule(id)?.ok_or(ServiceError::NotFound)?;
        let previous = schedule::effective_next_date(&s).ok_or(ServiceError::CompletedSchedule)?;
        let mut skipped = 0;
        while skipped < 500 {
            let Some(next) = schedule::effective_next_date(&s) else { break };
            if through.is_some_and(|end| next > end) { break; }
            schedule::advance(&mut s);
            skipped += 1;
            if through.is_none() { break; }
        }
        store.update_schedule(&s)?;
        Ok(SkipResult { previous_next_date: Some(previous), next_date: schedule::effective_next_date(&s), completed: s.completed, skipped })
    })
}
pub fn auto_post_due(store: &mut Store, today: NaiveDate) -> Result<usize> {
    store.transaction(|store| {
        let mut count = 0;
        for s in store.list_schedules()?.into_iter().filter(|s| s.auto_post && !s.completed) {
            for _ in 0..500 {
                let current = store.get_schedule(s.id)?.ok_or(ServiceError::NotFound)?;
                if !schedule::effective_next_date(&current).is_some_and(|date| date <= today) { break; }
                post(store, s.id)?;
                count += 1;
            }
        }
        Ok(count)
    })
}
/// What a schedule editor collects; the sign of `amount` already says expense or income.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDraft {
    pub name: String, pub amount: i64, pub account_id: Id, pub category_id: Option<Id>, pub payee: String,
    pub frequency: Frequency, pub interval: u32, pub start: NaiveDate, pub skip_weekend: bool, pub auto_post: bool,
}
/// Builds the row to store. A rule that did not change keeps its progress (`next_date`, `completed`);
/// a changed rule starts over from its first occurrence.
pub fn apply_draft(existing: Option<Schedule>, payee_id: Option<Id>, draft: &ScheduleDraft) -> Schedule {
    let mut schedule = existing.unwrap_or(Schedule { id: 0, name: String::new(), amount: 0, auto_post: false, completed: false, next_date: None, recurrence: None, account_id: None, category_id: None, payee_id: None });
    let old_next = crate::domain::schedule::effective_next_date(&schedule);
    let old = schedule.recurrence.take();
    let rule = Recurrence { start: draft.start, frequency: draft.frequency, interval: draft.interval.max(1), skip_weekend: draft.skip_weekend,
        weekend_mode: old.as_ref().map_or(WeekendMode::After, |r| r.weekend_mode), end_date: old.as_ref().and_then(|r| r.end_date) };
    if old.as_ref() != Some(&rule) {
        schedule.next_date = old.as_ref().zip(old_next).and_then(|(old, next)| carried_next_date(old, next, &rule));
        schedule.completed = false;
    }
    schedule.recurrence = Some(rule);
    schedule.name = draft.name.trim().to_owned(); schedule.amount = draft.amount; schedule.auto_post = draft.auto_post;
    schedule.account_id = Some(draft.account_id); schedule.category_id = draft.category_id; schedule.payee_id = payee_id;
    schedule
}
/// Where an edited rule picks up: the new date nearest to the old next one, never repeating
/// a date the old rule already passed (so moving the day doesn't bring back paid months).
fn carried_next_date(old: &Recurrence, old_next: NaiveDate, rule: &Recurrence) -> Option<NaiveDate> {
    let day_before = old_next.pred_opt()?;
    let passed = old.occurrences(day_before.checked_sub_months(Months::new(24))?, day_before).last().copied();
    let from = passed.and_then(|d| d.succ_opt()).unwrap_or(rule.start);
    let through = old_next.checked_add_months(Months::new(24))?;
    rule.occurrences(from, through).into_iter().min_by_key(|d| (*d - old_next).num_days().abs())
}
/// Creates (`id` None) or edits a schedule from a draft; returns its id.
pub fn save_schedule(store: &mut Store, id: Option<Id>, draft: &ScheduleDraft) -> Result<Id> {
    store.transaction(|store| {
        let payee_id = store.find_or_create_payee(&draft.payee)?;
        let existing = match id { Some(id) => Some(store.get_schedule(id)?.ok_or(ServiceError::NotFound)?), None => None };
        let schedule = apply_draft(existing, payee_id, draft);
        match id { Some(id) => { store.update_schedule(&schedule)?; Ok(id) }, None => Ok(store.insert_schedule(&schedule)?) }
    })
}
pub fn create_schedule(store: &Store, schedule: &Schedule) -> Result<Id> { Ok(store.insert_schedule(schedule)?) }
pub fn update_schedule(store: &Store, schedule: &Schedule) -> Result<()> { store.get_schedule(schedule.id)?.ok_or(ServiceError::NotFound)?; Ok(store.update_schedule(schedule)?) }
pub fn delete_schedule(store: &Store, id: Id) -> Result<()> { store.get_schedule(id)?.ok_or(ServiceError::NotFound)?; Ok(store.delete_schedule(id)?) }

#[cfg(test)]
mod draft_tests {
    use super::*;

    fn draft() -> ScheduleDraft { ScheduleDraft { name: "  Gimnasio ".into(), amount: -35_000, account_id: 1, category_id: None, payee: String::new(), frequency: Frequency::Monthly, interval: 1, start: NaiveDate::from_ymd_opt(2026, 11, 15).unwrap(), skip_weekend: false, auto_post: false } }

    #[test]
    fn new_schedule_starts_at_its_first_occurrence() {
        let s = apply_draft(None, None, &draft());
        assert_eq!((s.name.as_str(), s.amount, s.next_date, s.completed), ("Gimnasio", -35_000, None, false));
        assert_eq!(s.recurrence.as_ref().map(|r| (r.frequency, r.interval, r.weekend_mode)), Some((Frequency::Monthly, 1, WeekendMode::After)));
    }
    #[test]
    fn editing_keeps_progress_only_while_the_rule_is_unchanged() {
        let mut s = apply_draft(None, None, &draft());
        s.next_date = NaiveDate::from_ymd_opt(2026, 12, 15);
        let kept = apply_draft(Some(s.clone()), Some(7), &ScheduleDraft { amount: -40_000, ..draft() });
        assert_eq!((kept.next_date, kept.amount, kept.payee_id), (NaiveDate::from_ymd_opt(2026, 12, 15), -40_000, Some(7)));
        s.completed = true;
        let reset = apply_draft(Some(s), None, &ScheduleDraft { interval: 2, ..draft() });
        assert_eq!((reset.next_date, reset.completed), (None, false));
    }
    #[test]
    fn moving_the_day_keeps_progress_at_the_nearest_new_date() {
        let day = |m, d| NaiveDate::from_ymd_opt(2026, m, d);
        let mut s = apply_draft(None, None, &ScheduleDraft { start: day(5, 13).unwrap(), ..draft() });
        s.next_date = day(12, 13);
        let later = apply_draft(Some(s.clone()), None, &ScheduleDraft { start: day(5, 15).unwrap(), ..draft() });
        assert_eq!(later.next_date, day(12, 15));
        let earlier = apply_draft(Some(s), None, &ScheduleDraft { start: day(5, 11).unwrap(), ..draft() });
        assert_eq!(earlier.next_date, day(12, 11));
    }
    #[test]
    fn save_schedule_creates_then_edits_and_rejects_a_missing_id() {
        let mut store = Store::open_in_memory().unwrap();
        let account = store.insert_account(&crate::Account { id: 0, name: "Cuenta".into(), off_budget: false, closed: false, sort_order: 0.0 }).unwrap();
        let id = save_schedule(&mut store, None, &ScheduleDraft { account_id: account, payee: "Gym".into(), ..draft() }).unwrap();
        let saved = store.get_schedule(id).unwrap().unwrap();
        assert_eq!((saved.name.as_str(), saved.account_id, saved.payee_id.is_some()), ("Gimnasio", Some(account), true));
        save_schedule(&mut store, Some(id), &ScheduleDraft { account_id: account, amount: -50_000, ..draft() }).unwrap();
        assert_eq!(store.get_schedule(id).unwrap().unwrap().amount, -50_000);
        assert!(matches!(save_schedule(&mut store, Some(id + 99), &ScheduleDraft { account_id: account, ..draft() }), Err(ServiceError::NotFound)));
        assert_eq!(store.list_schedules().unwrap().len(), 1);
    }
}
