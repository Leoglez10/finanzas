use super::{Result, ServiceError};
use crate::{Entry, Id, Schedule, domain::schedule, store::{EntryRow, Store}};
use chrono::{NaiveDate, NaiveDateTime};
use std::collections::HashMap;
use serde::Serialize;

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
pub fn create_schedule(store: &Store, schedule: &Schedule) -> Result<Id> { Ok(store.insert_schedule(schedule)?) }
pub fn update_schedule(store: &Store, schedule: &Schedule) -> Result<()> { store.get_schedule(schedule.id)?.ok_or(ServiceError::NotFound)?; Ok(store.update_schedule(schedule)?) }
pub fn delete_schedule(store: &Store, id: Id) -> Result<()> { store.get_schedule(id)?.ok_or(ServiceError::NotFound)?; Ok(store.delete_schedule(id)?) }
