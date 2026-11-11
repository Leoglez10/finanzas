use std::sync::{Arc, Mutex};
use chrono::NaiveDate;
use finanzas_core::{domain::month, models::{Account, Category, CategoryGroup, Entry, Frequency, Recurrence, Schedule, WeekendMode}, service::{overview, Overview}, store::Store};

pub struct AccountBalance { pub account: Account, pub balance: i64 }
pub struct Snapshot { pub accounts: Vec<AccountBalance>, pub overview: Overview, pub version: i64, pub groups: Vec<CategoryGroup>, pub categories: Vec<Category> }
pub type WorkerStore = Arc<Mutex<Option<Store>>>;

pub enum Mutation {
    Save(Option<i64>, finanzas_core::service::NewEntry), Delete(i64), Budget(i32, i64, String),
    PostSchedule(i64, NaiveDate), SkipSchedule(i64), SaveSchedule(Option<i64>, ScheduleDraft), DeleteSchedule(i64),
    CreateAccount(AccountDraft),
}

/// What the "Nueva cuenta" sheet collects. `opening` is signed cents; `None` means no opening entry.
#[derive(Clone, Debug, PartialEq)]
pub struct AccountDraft { pub name: String, pub off_budget: bool, pub opening: Option<i64>, pub date: NaiveDate }
fn create_account(store: &mut Store, draft: &AccountDraft) -> Result<(), String> {
    let db = |e: rusqlite::Error| e.to_string();
    store.transaction(|store| {
        let order = store.list_accounts()?.iter().map(|a| a.sort_order).fold(0.0, f64::max) + 1.0;
        let id = store.insert_account(&Account { id: 0, name: draft.name.trim().to_owned(), off_budget: draft.off_budget, closed: false, sort_order: order })?;
        if let Some(amount) = draft.opening.filter(|v| *v != 0) {
            store.insert_entry(&Entry { id: 0, date: draft.date, amount, notes: "Saldo inicial".into(), cleared: true, account_id: Some(id), category_id: None, payee_id: None, schedule_id: None })?;
        }
        Ok(())
    }).map_err(db)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportKind { SwiftData, Actual }
/// Imports into the (empty) open store. The source is only ever read; an Actual backup `.zip` is
/// unpacked into a scratch directory that is removed afterwards.
pub fn import(worker: WorkerStore, kind: ImportKind, source: std::path::PathBuf) -> Result<finanzas_core::store::ImportSummary, String> {
    use finanzas_core::store::ImportError;
    let mut guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_mut().ok_or("No se pudo abrir la base de datos")?;
    let explain = |e: ImportError| match e {
        ImportError::StoreNotEmpty => "Ya tienes datos: la base de datos de destino no está vacía.".to_owned(),
        other => format!("No se pudo importar: {other}"),
    };
    match kind {
        ImportKind::SwiftData => store.import_swiftdata(&source).map_err(explain),
        ImportKind::Actual => {
            let scratch = std::env::temp_dir().join(format!("finanzas-import-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos())));
            std::fs::create_dir_all(&scratch).map_err(|e| format!("No se pudo importar: {e}"))?;
            let result = finanzas_core::store::extract_actual(&source, &scratch).and_then(|db| store.import_actual(&db)).map_err(explain);
            let _ = std::fs::remove_dir_all(&scratch);
            result
        }
    }
}

/// Reminders for payments coming up, planned on the worker. Disabled settings never reach here.
pub fn reminders(worker: WorkerStore, now: chrono::NaiveDateTime, days: i64, hour: u32) -> Result<Vec<finanzas_core::service::ReminderView>, String> {
    let guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_ref().ok_or("No se pudo abrir la base de datos")?;
    finanzas_core::service::reminders(store, now, days, hour).map_err(|e| e.to_string())
}

/// What the schedule sheet collects; the sign of `amount` already says expense or income.
#[derive(Clone, Debug, PartialEq)]
pub struct ScheduleDraft {
    pub name: String, pub amount: i64, pub account_id: i64, pub category_id: Option<i64>, pub payee: String,
    pub frequency: Frequency, pub interval: u32, pub start: NaiveDate, pub skip_weekend: bool, pub auto_post: bool,
}
/// Builds the row to store. A rule that did not change keeps its progress (`next_date`, `completed`);
/// a changed rule starts over from its first occurrence.
pub fn apply_draft(existing: Option<Schedule>, payee_id: Option<i64>, draft: &ScheduleDraft) -> Schedule {
    let mut schedule = existing.unwrap_or(Schedule { id: 0, name: String::new(), amount: 0, auto_post: false, completed: false, next_date: None, recurrence: None, account_id: None, category_id: None, payee_id: None });
    let old = schedule.recurrence.take();
    let rule = Recurrence { start: draft.start, frequency: draft.frequency, interval: draft.interval.max(1), skip_weekend: draft.skip_weekend,
        weekend_mode: old.as_ref().map_or(WeekendMode::After, |r| r.weekend_mode), end_date: old.as_ref().and_then(|r| r.end_date) };
    if old.as_ref() != Some(&rule) { schedule.next_date = None; schedule.completed = false; }
    schedule.recurrence = Some(rule);
    schedule.name = draft.name.trim().to_owned(); schedule.amount = draft.amount; schedule.auto_post = draft.auto_post;
    schedule.account_id = Some(draft.account_id); schedule.category_id = draft.category_id; schedule.payee_id = payee_id;
    schedule
}
pub fn mutate(worker: WorkerStore, mutation: Mutation) -> Result<(), String> {
    let mut guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_mut().ok_or("No se pudo abrir la base de datos")?;
    let service_err = |e: finanzas_core::service::ServiceError| e.to_string();
    match mutation {
        Mutation::PostSchedule(id, today) => { finanzas_core::service::post_schedule(store, id, today).map_err(service_err)?; }
        Mutation::SkipSchedule(id) => { finanzas_core::service::skip_schedule(store, id, None).map_err(service_err)?; }
        Mutation::DeleteSchedule(id) => { finanzas_core::service::delete_schedule(store, id).map_err(service_err)?; }
        Mutation::CreateAccount(draft) => create_account(store, &draft)?,
        Mutation::SaveSchedule(id, draft) => {
            let payee_id = store.find_or_create_payee(&draft.payee).map_err(|e| e.to_string())?;
            let existing = match id { Some(id) => Some(store.get_schedule(id).map_err(|e| e.to_string())?.ok_or("El pago programado ya no existe")?), None => None };
            let schedule = apply_draft(existing, payee_id, &draft);
            if id.is_some() { finanzas_core::service::update_schedule(store, &schedule).map_err(service_err)?; } else { finanzas_core::service::create_schedule(store, &schedule).map_err(service_err)?; }
        }
        _ => mutate_ledger(store, mutation)?,
    }
    Ok(())
}
fn mutate_ledger(store: &Store, mutation: Mutation) -> Result<(), String> {
    match mutation {
        Mutation::Save(None, entry) => { finanzas_core::service::add_entry(store, entry).map_err(|e| e.to_string())?; }
        Mutation::Save(Some(id), entry) => {
            let changes = finanzas_core::service::EntryChanges { date: Some(entry.date), amount: Some(entry.amount), account_id: Some(entry.account_id), payee: Some(entry.payee.unwrap_or_default()), category_id: Some(entry.category_id), notes: Some(entry.notes), cleared: Some(entry.cleared) };
            finanzas_core::service::update_entry(store, id, changes).map_err(|e| e.to_string())?;
        }
        Mutation::Delete(id) => { finanzas_core::service::delete_entry(store, id).map_err(|e| e.to_string())?; }
        Mutation::Budget(month, category, text) => {
            // `None` means the text is not an amount; nothing was written.
            finanzas_core::service::set_budget_from_text(store, month, category, &text).map_err(|e| e.to_string())?.ok_or("Importe inválido")?;
        }
        _ => unreachable!("schedule mutations are handled by `mutate`"),
    }
    Ok(())
}
pub struct Proximos { pub items: Vec<finanzas_core::service::UpcomingView>, pub schedules: Vec<finanzas_core::service::ScheduleView> }
pub fn proximos(worker: WorkerStore, today: NaiveDate) -> Result<Proximos, String> {
    let guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_ref().ok_or("No se pudo abrir la base de datos")?;
    let items = finanzas_core::service::upcoming(store, today, 30).map_err(|e| e.to_string())?;
    let mut schedules = finanzas_core::service::schedule_views(store).map_err(|e| e.to_string())?;
    schedules.sort_by(|a, b| a.schedule.name.cmp(&b.schedule.name));
    Ok(Proximos { items, schedules })
}
/// Posts every automatic payment that is due; returns how many entries it created.
pub fn auto_post(worker: WorkerStore, today: NaiveDate) -> Result<usize, String> {
    let mut guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_mut().ok_or("No se pudo abrir la base de datos")?;
    finanzas_core::service::auto_post_due(store, today).map_err(|e| e.to_string())
}
pub fn suggestions(worker: WorkerStore, text: String) -> Result<Vec<(String, Option<i64>)>, String> {
    let guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_ref().ok_or("No se pudo abrir la base de datos")?;
    store.payee_suggestions(&text, 8).map_err(|e| e.to_string())?.into_iter().map(|p| Ok((p.name, store.suggested_category(p.id).map_err(|e| e.to_string())?))).collect()
}
pub fn budget(worker: WorkerStore, month: i32) -> Result<finanzas_core::service::BudgetView, String> {
    let guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_ref().ok_or("No se pudo abrir la base de datos")?;
    finanzas_core::service::budget_month(store, month).map_err(|e| e.to_string())
}
pub struct Ledger { pub rows: Vec<finanzas_core::store::EntryRow>, pub total: i64 }
pub fn ledger(worker: WorkerStore, account: Option<i64>, search: &str) -> Result<Ledger, String> {
    let guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_ref().ok_or("No se pudo abrir la base de datos")?;
    let filter = finanzas_core::store::EntryFilter { account_id: account, search: Some(search.to_owned()), ..Default::default() };
    let (rows, _) = store.entries(&filter).map_err(|e| e.to_string())?;
    let total = rows.iter().map(|row| row.amount).sum();
    Ok(Ledger { rows, total })
}

pub fn read(store: &Store, month: i32, chart_months: u32) -> Result<Snapshot, String> {
    let version = store.data_version().map_err(|e| e.to_string())?;
    let accounts = store.list_accounts().map_err(|e| e.to_string())?;
    let balances = store.account_balances().map_err(|e| e.to_string())?;
    let accounts = accounts.into_iter().map(|account| {
        let balance = balances.get(&account.id).copied().unwrap_or(0);
        AccountBalance { account, balance }
    }).collect();
    let overview = overview(store, month, chart_months).map_err(|e| e.to_string())?;
    let groups = store.list_category_groups().map_err(|e| e.to_string())?;
    let categories = store.list_categories().map_err(|e| e.to_string())?;
    Ok(Snapshot { accounts, overview, version, groups, categories })
}

pub fn open_default(worker: &WorkerStore) -> Result<(), String> {
    let mut guard = worker.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        let path = Store::default_path().ok_or("No se pudo determinar la ruta de la base de datos")?;
        *guard = Some(Store::open(path).map_err(|e| e.to_string())?);
    }
    Ok(())
}

pub fn fetch(worker: WorkerStore, month: i32, chart_months: u32, version: Option<i64>, force: bool) -> Result<Option<Snapshot>, String> {
    let guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_ref().ok_or("No se pudo abrir la base de datos")?;
    if !force && version == Some(store.data_version().map_err(|e| e.to_string())?) { return Ok(None); }
    read(store, month, chart_months).map(Some)
}

pub fn scene(seed: bool) -> Result<(WorkerStore, Snapshot), String> {
    let store = Store::open_in_memory().map_err(|e| e.to_string())?;
    if seed { seed_demo(&store)?; }
    let snapshot = read(&store, 202611, 6)?;
    Ok((Arc::new(Mutex::new(Some(store))), snapshot))
}

fn seed_demo(store: &Store) -> Result<(), String> {
    let db = |e: rusqlite::Error| e.to_string();
    let account = store.insert_account(&Account { id: 0, name: "Cuenta principal".into(), off_budget: false, closed: false, sort_order: 0.0 }).map_err(db)?;
    let debt = store.insert_account(&Account { id: 0, name: "Tarjeta".into(), off_budget: false, closed: false, sort_order: 1.0 }).map_err(db)?;
    let income_group = store.insert_category_group(&CategoryGroup { id: 0, name: "Ingresos".into(), is_income: true, sort_order: 0.0 }).map_err(db)?;
    let expense_group = store.insert_category_group(&CategoryGroup { id: 0, name: "Gastos".into(), is_income: false, sort_order: 1.0 }).map_err(db)?;
    let income = store.insert_category(&Category { id: 0, name: "Sueldo".into(), hidden: false, sort_order: 0.0, group_id: Some(income_group) }).map_err(db)?;
    let mut categories = Vec::new();
    for (i, name) in ["Vivienda", "Alimentación", "Transporte", "Ocio", "Servicios", "Salud"].iter().enumerate() {
        let id = store.insert_category(&Category { id: 0, name: (*name).into(), hidden: false, sort_order: i as f64, group_id: Some(expense_group) }).map_err(db)?;
        store.set_budget(202611, id, (80_000 - i as i64 * 8_000).max(20_000)).map_err(db)?;
        categories.push(id);
    }
    let income_payee = store.find_or_create_payee("Empresa SA").map_err(db)?.ok_or("Comercio inválido")?;
    let mut payees = Vec::new();
    for name in ["Renta", "Supermercado La Esquina", "Uber", "Netflix", "CFE", "Farmacia"] {
        payees.push(store.find_or_create_payee(name).map_err(db)?.ok_or("Comercio inválido")?);
    }
    let mut m = 202611;
    for index in 0..7 {
        let date = NaiveDate::from_ymd_opt(m / 100, (m % 100) as u32, 5).ok_or("Fecha inválida")?;
        store.insert_entry(&Entry { id: 0, date, amount: 320_000 + index * 5_000, notes: "Ingreso mensual".into(), cleared: true, account_id: Some(account), category_id: Some(income), payee_id: Some(income_payee), schedule_id: None }).map_err(db)?;
        for (i, category) in categories.iter().enumerate() {
            store.insert_entry(&Entry { id: 0, date, amount: -(50_000 - i as i64 * 5_000 + index * 500), notes: if i == 1 { "Despensa".into() } else { String::new() }, cleared: true, account_id: Some(if i == 4 { debt } else { account }), category_id: Some(*category), payee_id: Some(payees[i]), schedule_id: None }).map_err(db)?;
        }
        m = month::previous(m);
    }
    let date = NaiveDate::from_ymd_opt(2026, 11, 20).ok_or("Fecha inválida")?;
    store.insert_schedule(&Schedule { id: 0, name: "Pago mensual".into(), amount: -10_000, auto_post: false, completed: false, next_date: Some(date), recurrence: Some(Recurrence { start: date, frequency: Frequency::Monthly, interval: 1, skip_weekend: false, weekend_mode: WeekendMode::After, end_date: None }), account_id: Some(account), category_id: Some(categories[0]), payee_id: None }).map_err(db)?;
    Ok(())
}

/// Extra scheduled payments for the Próximos pagos scenes: one overdue, one automatic, one income.
pub fn seed_schedules(worker: &WorkerStore) -> Result<(), String> {
    let guard = worker.lock().map_err(|e| e.to_string())?;
    let store = guard.as_ref().ok_or("No se pudo abrir la base de datos")?;
    let db = |e: rusqlite::Error| e.to_string();
    let accounts = store.list_accounts().map_err(db)?;
    let categories = store.list_categories().map_err(db)?;
    let account = |name: &str| accounts.iter().find(|a| a.name == name).map(|a| a.id);
    let category = |name: &str| categories.iter().find(|c| c.name == name).map(|c| c.id);
    let d = |y, m, day| NaiveDate::from_ymd_opt(y, m, day).ok_or("Fecha inválida".to_owned());
    let rule = |start: NaiveDate, frequency| Recurrence { start, frequency, interval: 1, skip_weekend: false, weekend_mode: WeekendMode::After, end_date: None };
    let add = |name: &str, amount: i64, start: NaiveDate, next: Option<NaiveDate>, frequency, auto_post: bool, account_name: &str, category_name: &str, payee: &str| -> Result<(), String> {
        let payee_id = store.find_or_create_payee(payee).map_err(db)?;
        store.insert_schedule(&Schedule { id: 0, name: name.into(), amount, auto_post, completed: false, next_date: next, recurrence: Some(rule(start, frequency)),
            account_id: account(account_name), category_id: category(category_name), payee_id }).map_err(db)?;
        Ok(())
    };
    add("Renta", -80_000, d(2026, 10, 5)?, Some(d(2026, 11, 5)?), Frequency::Monthly, false, "Cuenta principal", "Vivienda", "Renta")?;
    add("Netflix", -2_000, d(2026, 11, 14)?, None, Frequency::Monthly, true, "Tarjeta", "Ocio", "Netflix")?;
    add("Nómina", 320_000, d(2026, 11, 30)?, None, Frequency::Monthly, false, "Cuenta principal", "Sueldo", "Empresa SA")?;
    Ok(())
}

#[cfg(test)]
mod tests {
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
    fn ledger_filters_and_sums_the_matching_rows() {
        let (worker, _) = scene(true).unwrap();
        let all = ledger(Arc::clone(&worker), None, "").unwrap();
        assert!(all.rows.len() >= 20);
        assert!(all.rows.iter().all(|row| row.payee_name.as_ref().is_some_and(|name| !name.is_empty())));
        let income = ledger(Arc::clone(&worker), None, "Ingreso mensual").unwrap();
        assert_eq!(income.rows.len(), 7);
        assert_eq!(income.total, income.rows.iter().map(|r| r.amount).sum::<i64>());
        let missing = ledger(worker, None, "no-such-transaction").unwrap();
        assert_eq!(missing.total, 0);
        assert!(missing.rows.is_empty());
    }

    #[test]
    fn external_write_refreshes_snapshot_but_unchanged_poll_does_not() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target").join(format!("worker-refresh-{}.db", std::process::id()));
        let first = Store::open(&path).unwrap();
        let second = Store::open(&path).unwrap();
        let worker = Arc::new(Mutex::new(Some(first)));
        let before = fetch(Arc::clone(&worker), 202611, 6, None, true).unwrap().unwrap();
        assert!(fetch(Arc::clone(&worker), 202611, 6, Some(before.version), false).unwrap().is_none());
        second.insert_account(&Account { id: 0, name: "Nueva".into(), off_budget: false, closed: false, sort_order: 0.0 }).unwrap();
        let after = fetch(worker, 202611, 6, Some(before.version), false).unwrap().unwrap();
        assert!(after.accounts.iter().any(|account| account.account.name == "Nueva"));
        assert_ne!(after.version, before.version);
    }
}

