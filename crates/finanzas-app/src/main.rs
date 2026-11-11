mod account_sheet;
mod ajustes;
mod budget;
mod data;
mod entry_helpers;
mod ledger;
mod metrics;
mod overview;
mod proximos;
mod schedule_sheet;
mod settings;
mod sheet;
#[cfg(test)]
mod t10c_tests;

use std::sync::{Arc, Mutex};
use chrono::{Local, NaiveDate};
use finanzas_core::{domain::{money, month}, models::Id};
use rune::{AppSpec, Color, Cx, El, Latest, TextEdit, Tx, Waker, Key, KeyEvent, col, el, row, text, tok::type_};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route { Resumen, Presupuesto, Proximos, Ajustes, Todas, Cuenta(Id) }

type LedgerKey = (Option<Id>, String, Option<i64>);
type LedgerResult = (LedgerKey, Result<data::Ledger, String>);
type Suggestions = Vec<(String, Option<Id>)>;
type SuggestionResult = (String, Result<Suggestions, String>);
#[derive(Clone, Copy)]
enum MutationTarget { Account, Sheet, Confirmation(Id), Budget(Id), ScheduleSheet, ScheduleConfirm(Id), ScheduleRow }
type ProximosKey = (Option<i64>, u64, NaiveDate);
type ProximosResult = (ProximosKey, Result<data::Proximos, String>);
type BudgetKey = (i32, Option<i64>, u64);
type BudgetResult = (BudgetKey, Result<finanzas_core::service::BudgetView, String>);
type RemindersKey = (Option<i64>, u64, bool, i64, u32, NaiveDate);
type RemindersResult = (RemindersKey, Result<Vec<finanzas_core::service::ReminderView>, String>);

struct State {
    route: Route,
    accounts: Vec<data::AccountBalance>,
    dark: Option<bool>,
    error: Option<String>,
    month: i32,
    today: i32,
    chart_months: u32,
    overview: Option<finanzas_core::service::Overview>,
    version: Option<i64>,
    worker: data::WorkerStore,
    open_default: bool,
    latest: Latest<Result<Option<data::Snapshot>, String>>,
    refresh: bool,
    next_poll: f64,
    search: TextEdit,
    ledger: Option<data::Ledger>,
    ledger_latest: Latest<LedgerResult>,
    ledger_key: Option<LedgerKey>,
    groups: Vec<finanzas_core::models::CategoryGroup>,
    categories: Vec<finanzas_core::models::Category>,
    sheet: Option<sheet::Sheet>,
    blocked_edit: TextEdit,
    selected: Option<Id>,
    confirm: Option<Id>,
    mutation: Latest<Result<(), String>>,
    mutation_target: Option<MutationTarget>,
    mutation_error: Option<String>,
    suggestions: Vec<(String, Option<Id>)>,
    suggestion_latest: Latest<SuggestionResult>,
    budget: Option<finanzas_core::service::BudgetView>,
    budget_latest: Latest<BudgetResult>,
    budget_gen: u64,
    budget_inputs: std::collections::HashMap<Id, (TextEdit, String)>,
    budget_error: Option<(Option<Id>, String)>,
    today_date: NaiveDate,
    proximos: Option<data::Proximos>,
    proximos_latest: Latest<ProximosResult>,
    proximos_key: Option<ProximosKey>,
    proximos_gen: u64,
    schedule_sheet: Option<schedule_sheet::ScheduleSheet>,
    schedule_selected: Option<Id>,
    schedule_confirm: Option<Id>,
    upcoming_error: Option<String>,
    auto_post: bool,
    auto_latest: Latest<Result<usize, String>>,
    next_auto: f64,
    settings: settings::Settings,
    settings_path: Option<std::path::PathBuf>,
    settings_error: Option<String>,
    hourly: bool,
    reminders: Vec<finanzas_core::service::ReminderView>,
    reminders_gen: u64,
    account_sheet: Option<account_sheet::AccountSheet>,
    import_confirm: Option<ajustes::ImportRequest>,
    import_latest: Latest<Result<finanzas_core::store::ImportSummary, String>>,
    importing: bool,
    import_outcome: Option<ajustes::ImportOutcome>,
    reminders_latest: Latest<RemindersResult>,
    next_reminders: f64,
    dismissed: std::collections::HashSet<String>,
    db_label: String,
}

impl State {
    fn empty() -> Self {
        let today = month::month_of(Local::now().date_naive());
        Self { route: Route::Resumen, accounts: Vec::new(), dark: None, error: None,
            month: today, today, chart_months: 6, overview: None, version: None,
            worker: Arc::new(Mutex::new(None)), open_default: false, latest: Latest::default(), refresh: true, next_poll: 0.0,
            search: TextEdit::new(""), ledger: None, ledger_latest: Latest::default().debounce(std::time::Duration::from_millis(150)), ledger_key: None, groups: Vec::new(), categories: Vec::new(), sheet: None, blocked_edit: TextEdit::new(""), selected: None, confirm: None, mutation: Latest::default(), mutation_target: None, mutation_error: None, suggestions: Vec::new(), suggestion_latest: Latest::default().debounce(std::time::Duration::from_millis(150)), budget: None, budget_latest: Latest::default(), budget_gen: 0, budget_inputs: Default::default(), budget_error: None,
            today_date: Local::now().date_naive(), proximos: None, proximos_latest: Latest::default(), proximos_key: None, proximos_gen: 0, schedule_sheet: None, schedule_selected: None, schedule_confirm: None, upcoming_error: None, auto_post: false, auto_latest: Latest::default(), next_auto: 0.0,
            settings: settings::Settings::default(), settings_path: None, settings_error: None, hourly: false, reminders: Vec::new(), reminders_gen: 0, account_sheet: None, import_confirm: None, import_latest: Latest::default(), importing: false, import_outcome: None, reminders_latest: Latest::default(), next_reminders: 0.0, dismissed: Default::default(), db_label: "base de datos en memoria".into() }
    }
    fn load() -> Self { Self::load_from(finanzas_core::store::Store::default_path()) }
    /// The real app: opens the default store, runs automatic payments and the hourly reminder check.
    /// Settings live next to the database file. Nothing else enables timers.
    fn load_from(db: Option<std::path::PathBuf>) -> Self {
        let mut state = Self::empty();
        state.open_default = true; state.auto_post = true; state.hourly = true;
        state.db_label = db.as_ref().map_or_else(|| "Base de datos predeterminada".into(), |p| p.display().to_string());
        state.settings_path = db.as_ref().and_then(|p| p.parent()).map(|dir| dir.join("settings.json"));
        if let Some(path) = &state.settings_path {
            match settings::load(path) { Ok(value) => state.settings = value, Err(e) => state.settings_error = Some(format!("Ajustes inválidos: {e}")) }
        }
        state
    }
    /// The clock reminders are planned against; scenes and tests keep theirs fixed.
    fn now(&self) -> chrono::NaiveDateTime { if self.open_default { Local::now().naive_local() } else { self.today_date.and_hms_opt(8, 0, 0).expect("valid time") } }
    fn reminders_key(&self) -> RemindersKey { (self.version, self.reminders_gen, self.settings.enabled, self.settings.days, self.settings.hour, self.today_date) }
    fn change_settings(&mut self, change: impl FnOnce(&mut settings::Settings)) {
        change(&mut self.settings);
        self.settings_error = None;
        if let Some(path) = &self.settings_path && let Err(e) = settings::save(path, &self.settings) { self.settings_error = Some(format!("No se pudo guardar los ajustes: {e}")); }
    }
    fn open_account_sheet(&mut self) {
        if self.mutation_active() { return; }
        self.account_sheet = Some(account_sheet::AccountSheet::new()); self.mutation_error = None;
    }
    fn submit_account(&mut self) {
        if self.mutation_active() { return; }
        let Some(draft) = self.account_sheet.as_ref().and_then(|sh| sh.draft(self.today_date)) else { return };
        let worker = Arc::clone(&self.worker);
        self.mutation_target = Some(MutationTarget::Account);
        self.mutation.request(data::Mutation::CreateAccount(draft), move |m| data::mutate(worker, m));
    }
    fn run_import(&mut self) {
        if self.mutation_active() { return; }
        let Some(request) = self.import_confirm.take() else { return };
        let worker = Arc::clone(&self.worker);
        self.importing = true; self.import_outcome = None;
        self.import_latest.request((request.kind, request.source), move |(kind, source)| data::import(worker, kind, source));
    }
    fn current_key(&self) -> Option<LedgerKey> {
        let account = match self.route { Route::Todas => None, Route::Cuenta(id) => Some(id), _ => return None };
        Some((account, self.search.text.clone(), self.version))
    }
    fn visible_entry(&self, id: Id) -> Option<&finanzas_core::store::EntryRow> {
        if self.ledger_key.as_ref() != self.current_key().as_ref() || self.current_key().is_none() { return None; }
        self.ledger.as_ref()?.rows.iter().find(|entry| entry.id == id)
    }
    fn proximos_key_now(&self) -> ProximosKey { (self.version, self.proximos_gen, self.today_date) }
    /// True only while the list on screen was read for the current data version and day.
    fn proximos_fresh(&self) -> bool { self.route == Route::Proximos && self.proximos.is_some() && self.proximos_key == Some(self.proximos_key_now()) }
    fn visible_schedule(&self, id: Id) -> Option<&finanzas_core::service::ScheduleView> {
        if !self.proximos_fresh() { return None; }
        self.proximos.as_ref()?.schedules.iter().find(|v| v.schedule.id == id)
    }
    fn open_schedule_sheet(&mut self, id: Option<Id>) {
        if self.mutation_active() || self.route != Route::Proximos { return; }
        if id.is_some_and(|id| self.visible_schedule(id).is_none()) { return; }
        self.schedule_sheet = Some(schedule_sheet::ScheduleSheet::new(self, id.and_then(|id| self.visible_schedule(id))));
        self.mutation_error = None;
    }
    fn submit_schedule(&mut self) {
        if self.mutation_active() { return; }
        let Some(sh) = self.schedule_sheet.as_ref() else { return };
        let Some(draft) = sh.draft() else { return };
        if sh.id.is_some_and(|id| self.visible_schedule(id).is_none()) { return; }
        let (id, worker) = (sh.id, Arc::clone(&self.worker));
        self.mutation_target = Some(MutationTarget::ScheduleSheet);
        self.mutation.request(data::Mutation::SaveSchedule(id, draft), move |m| data::mutate(worker, m));
    }
    /// Registrar (`post`) or Saltar a payment that is still the pending occurrence on screen.
    fn schedule_action(&mut self, id: Id, post: bool) {
        if self.mutation_active() { return; }
        if !self.visible_schedule(id).is_some_and(|v| !v.schedule.completed && v.next_date.is_some()) { return; }
        let (worker, today) = (Arc::clone(&self.worker), self.today_date);
        self.upcoming_error = None;
        self.mutation_target = Some(MutationTarget::ScheduleRow);
        let mutation = if post { data::Mutation::PostSchedule(id, today) } else { data::Mutation::SkipSchedule(id) };
        self.mutation.request(mutation, move |m| data::mutate(worker, m));
    }
    fn request_schedule_delete(&mut self, id: Id) {
        if self.mutation_active() || self.visible_schedule(id).is_none() { return; }
        self.schedule_selected = Some(id); self.schedule_confirm = Some(id); self.mutation_error = None;
    }
    fn delete_schedule_confirmed(&mut self) {
        if self.mutation_active() { return; }
        if let Some(id) = self.schedule_confirm.filter(|id| self.visible_schedule(*id).is_some()) {
            let worker = Arc::clone(&self.worker);
            self.mutation_target = Some(MutationTarget::ScheduleConfirm(id));
            self.mutation.request(data::Mutation::DeleteSchedule(id), move |m| data::mutate(worker, m));
        }
    }
    fn invalidate_selection(&mut self) {
        if self.schedule_selected.is_some_and(|id| self.visible_schedule(id).is_none()) { self.schedule_selected = None; }
        if self.schedule_confirm.is_some_and(|id| self.visible_schedule(id).is_none()) { self.schedule_confirm = None; }
        if self.selected.is_some_and(|id| self.visible_entry(id).is_none()) { self.selected = None; }
        if self.confirm.is_some_and(|id| self.visible_entry(id).is_none()) { self.confirm = None; }
    }
    fn mutation_active(&self) -> bool { self.mutation_target.is_some() || self.importing }
    fn open_sheet(&mut self, expense: bool, id: Option<Id>) {
        if self.mutation_active() { return; }
        if id.is_some_and(|id| self.visible_entry(id).is_none()) { return; }
        let entry = id.and_then(|id| self.visible_entry(id));
        self.sheet = Some(sheet::Sheet::new(self, expense, entry)); self.mutation_error = None;
        self.suggestions.clear(); self.suggestion_latest.cancel();
    }
    fn submit(&mut self) {
        if self.mutation_active() { return; }
        let Some(sh) = self.sheet.as_ref().filter(|sh| sh.valid()) else { return; };
        if sh.id.is_some_and(|id| self.visible_entry(id).is_none()) { return; }
        let (Some(date), Some(amount), Some(account)) = (entry_helpers::parse_date(&sh.date.text), money::cents_from(&sh.amount.text), sh.account) else { return; };
        let entry = finanzas_core::service::NewEntry { date, amount: if sh.expense { -amount } else { amount }, account_id: account, payee: Some(sh.payee.text.clone()), category_id: sh.category, notes: sh.notes.text.clone(), cleared: sh.cleared };
        let worker = Arc::clone(&self.worker);
        self.mutation_target = Some(MutationTarget::Sheet);
        self.mutation.request(data::Mutation::Save(sh.id, entry), move |m| data::mutate(worker, m));
    }
    fn delete_selected(&mut self) {
        if self.mutation_active() { return; }
        if let Some(id) = self.confirm.filter(|id| self.visible_entry(*id).is_some()) {
            let worker = Arc::clone(&self.worker);
            self.mutation_target = Some(MutationTarget::Confirmation(id));
            self.mutation.request(data::Mutation::Delete(id), move |m| data::mutate(worker, m));
        }
    }
    fn total(&self) -> i64 { self.accounts.iter().filter(|a| !a.account.closed).map(|a| a.balance).sum() }
    fn set_month(&mut self, month: i32) { self.month = month; self.refresh = true; self.budget_error = None; }
    fn budget_edit(&mut self, id: Id) -> &mut TextEdit {
        if self.mutation_active() || !self.budget_inputs.contains_key(&id) { return &mut self.blocked_edit; }
        &mut self.budget_inputs.get_mut(&id).expect("checked above").0
    }
    fn accept_budget(&mut self, view: finanzas_core::service::BudgetView) {
        if self.budget.as_ref().is_none_or(|b| b.month != view.month) { self.budget_inputs.clear(); }
        let mut visible = Vec::new();
        for c in view.groups.iter().filter(|g| !g.is_income).flat_map(|g| &g.categories).filter(|c| !c.hidden) {
            visible.push(c.id);
            let text = budget::input_text(c.budgeted);
            match self.budget_inputs.get_mut(&c.id) {
                // Text the user typed and has not committed survives a refresh.
                Some((edit, base)) if edit.text != *base => *base = text,
                _ => { self.budget_inputs.insert(c.id, (TextEdit::new(text.clone()), text)); }
            }
        }
        self.budget_inputs.retain(|id, _| visible.contains(id));
        self.budget = Some(view);
    }
    fn commit_budget(&mut self, id: Id) {
        if self.mutation_active() { return; }
        let Some(month) = self.budget.as_ref().map(|b| b.month).filter(|m| *m == self.month) else { return; };
        let Some((edit, base)) = self.budget_inputs.get(&id) else { return; };
        self.budget_error = None;
        if edit.text == *base { return; }
        let text = edit.text.clone();
        let worker = Arc::clone(&self.worker);
        self.mutation_target = Some(MutationTarget::Budget(id));
        self.mutation.request(data::Mutation::Budget(month, id, text), move |m| data::mutate(worker, m));
    }
    fn accept(&mut self, snapshot: data::Snapshot) {
        self.version = Some(snapshot.version); self.accounts = snapshot.accounts; self.overview = Some(snapshot.overview); self.groups = snapshot.groups; self.categories = snapshot.categories;
    }
}

fn scene(s: &mut State, seed: bool, dark: bool) {
    let result = std::thread::spawn(move || data::scene(seed)).join();
    *s = State::empty(); s.today_date = NaiveDate::from_ymd_opt(2026, 11, 10).expect("valid date"); s.month = 202611; s.today = 202611; s.dark = Some(dark); s.refresh = false;
    match result {
        Ok(Ok((worker, snapshot))) => { s.worker = worker; s.accept(snapshot); },
        Ok(Err(error)) => s.error = Some(error),
        Err(_) => s.error = Some("No se pudo preparar la escena".into()),
    }
}
fn demo_light(s: &mut State) { scene(s, true, false); }
fn demo_dark(s: &mut State) { scene(s, true, true); }
fn empty_scene(s: &mut State) { scene(s, false, false); }
fn movements_scene(s: &mut State, dark: bool) {
    scene(s, true, dark);
    s.route = Route::Todas;
    let worker = Arc::clone(&s.worker);
    match std::thread::spawn(move || data::ledger(worker, None, "")).join() {
        Ok(Ok(ledger)) => { s.ledger = Some(ledger); s.ledger_key = Some((None, String::new(), s.version)); }
        Ok(Err(error)) => s.error = Some(error),
        Err(_) => s.error = Some("No se pudo preparar la escena".into()),
    }
}
fn movements_light(s: &mut State) { movements_scene(s, false); }
fn movements_dark(s: &mut State) { movements_scene(s, true); }
fn budget_scene(s: &mut State, dark: bool) {
    scene(s, true, dark);
    s.route = Route::Presupuesto;
    let worker = Arc::clone(&s.worker);
    match std::thread::spawn(move || data::budget(worker, 202611)).join() {
        Ok(Ok(view)) => s.accept_budget(view),
        Ok(Err(error)) => s.error = Some(error),
        Err(_) => s.error = Some("No se pudo preparar la escena".into()),
    }
}
fn budget_light(s: &mut State) { budget_scene(s, false); }
fn budget_dark(s: &mut State) { budget_scene(s, true); }
fn entry_scene(s: &mut State, dark: bool) {
    movements_scene(s, dark); s.open_sheet(true, None);
    if let Some(sh) = &mut s.sheet { sh.amount.replace_text("125.50"); sh.payee.replace_text("Farmacia"); sh.notes.replace_text("Compra mensual"); }
}
fn proximos_scene(s: &mut State, dark: bool) {
    scene(s, true, dark);
    s.route = Route::Proximos;
    let (worker, today) = (Arc::clone(&s.worker), s.today_date);
    match std::thread::spawn(move || data::seed_schedules(&worker).and_then(|()| data::proximos(worker, today))).join() {
        Ok(Ok(view)) => { s.proximos = Some(view); s.proximos_key = Some(s.proximos_key_now()); }
        Ok(Err(error)) => s.error = Some(error),
        Err(_) => s.error = Some("No se pudo preparar la escena".into()),
    }
}
fn proximos_light(s: &mut State) { proximos_scene(s, false); }
fn proximos_dark(s: &mut State) { proximos_scene(s, true); }
fn schedule_sheet_scene(s: &mut State) {
    proximos_scene(s, false); s.open_schedule_sheet(None);
    if let Some(sh) = &mut s.schedule_sheet { sh.name.replace_text("Gimnasio"); sh.amount.replace_text("350"); sh.payee.replace_text("Smart Fit"); sh.start.replace_text("15/11/2026"); }
}
fn ajustes_scene(s: &mut State, dark: bool) { scene(s, true, dark); s.route = Route::Ajustes; }
fn ajustes_light(s: &mut State) { ajustes_scene(s, false); }
fn ajustes_dark(s: &mut State) { ajustes_scene(s, true); }
fn nueva_cuenta_scene(s: &mut State) {
    scene(s, true, false); s.open_account_sheet();
    if let Some(sh) = &mut s.account_sheet { sh.name.replace_text("Ahorros"); sh.opening.replace_text("1500"); }
}
/// Resumen with the payment reminders already planned (the real app plans them on the worker).
fn reminders_scene(s: &mut State) {
    scene(s, true, false);
    let (worker, now, settings) = (Arc::clone(&s.worker), s.now(), s.settings.clone());
    match std::thread::spawn(move || data::seed_schedules(&worker).and_then(|()| data::reminders(worker, now, settings.days, settings.hour))).join() {
        Ok(Ok(list)) => s.reminders = list,
        Ok(Err(error)) => s.error = Some(error),
        Err(_) => s.error = Some("No se pudo preparar la escena".into()),
    }
}
fn entry_light(s: &mut State) { entry_scene(s, false); }
fn entry_dark(s: &mut State) { entry_scene(s, true); }

fn setup(s: &mut State, waker: Waker) { s.latest.set_waker(waker.clone()); s.ledger_latest.set_waker(waker.clone()); s.mutation.set_waker(waker.clone()); s.budget_latest.set_waker(waker.clone()); s.proximos_latest.set_waker(waker.clone()); s.auto_latest.set_waker(waker.clone()); s.import_latest.set_waker(waker.clone()); s.reminders_latest.set_waker(waker.clone()); s.suggestion_latest.set_waker(waker); }
fn poll(s: &mut State, now: f64, tx: &mut Tx) -> bool {
    let mut changed = false;
    if let Some(result) = s.mutation.take() {
        let target = s.mutation_target.take();
        match result {
            Ok(()) => {
                match target {
                    Some(MutationTarget::Sheet) => s.sheet = None,
                    Some(MutationTarget::Account) => s.account_sheet = None,
                    Some(MutationTarget::Budget(id)) => { s.budget_inputs.remove(&id); s.budget_error = None; s.budget_gen += 1; }
                    Some(MutationTarget::Confirmation(id)) if s.confirm == Some(id) => { s.confirm = None; s.selected = None; }
                    Some(MutationTarget::ScheduleSheet) => s.schedule_sheet = None,
                    Some(MutationTarget::ScheduleConfirm(id)) if s.schedule_confirm == Some(id) => { s.schedule_confirm = None; s.schedule_selected = None; }
                    _ => {}
                }
                // Every write can move balances and projections, so the list is read again.
                s.proximos_gen += 1;
                s.refresh = true; s.ledger = None; s.ledger_key = None; s.ledger_latest.cancel(); tx.instant(); changed = true;
            }
            Err(e) => {
                match target { Some(MutationTarget::Budget(id)) => s.budget_error = Some((Some(id), e)), Some(MutationTarget::ScheduleRow) => s.upcoming_error = Some(e), _ => s.mutation_error = Some(e) }
                changed = true;
            }
        }
    }
    if let Some((query, result)) = s.suggestion_latest.take()
        && s.sheet.as_ref().is_some_and(|sh| sh.payee.text == query) {
        match result { Ok(items) => s.suggestions = items, Err(error) => { s.suggestions.clear(); s.mutation_error = Some(error); } }
        changed = true;
    }
    if let Some(sh) = &s.sheet {
        let key = sh.payee.text.clone(); let worker = Arc::clone(&s.worker);
        s.suggestion_latest.request_if_changed(&key, move |key| { let result = if key.is_empty() { Ok(Vec::new()) } else { data::suggestions(worker, key.clone()) }; (key, result) });
    }
    if let Some(result) = s.latest.take() {
        match result {
            Ok(Some(snapshot)) => { s.accept(snapshot); s.error = None; tx.instant(); changed = true; }
            Ok(None) => {}
            Err(error) => { s.error = Some(error); tx.instant(); changed = true; }
        }
    }
    if s.refresh || (now >= s.next_poll && !s.latest.is_pending()) {
        let force = s.refresh;
        s.refresh = false;
        s.next_poll = now + 2.0;
        let worker = Arc::clone(&s.worker);
        let input = (s.month, s.chart_months, s.version, force);
        let open_default = s.open_default;
        s.latest.request(input, move |(month, chart, version, force)| {
            if open_default { data::open_default(&worker)?; }
            data::fetch(worker, month, chart, version, force)
        });
    }
    if let Some((key, result)) = s.ledger_latest.take() {
        match result { Ok(ledger) => { s.ledger = Some(ledger); s.ledger_key = Some(key); tx.instant(); changed = true; }, Err(error) => { s.error = Some(error); changed = true; } }
    }
    if s.overview.is_some() && matches!(s.route, Route::Todas | Route::Cuenta(_)) {
        let account = match s.route { Route::Cuenta(id) => Some(id), _ => None };
        let key = (account, s.search.text.clone(), s.version);
        let worker = Arc::clone(&s.worker);
        if s.ledger_key.as_ref() != Some(&key) && s.ledger.is_some() { s.ledger = None; changed = true; }
        s.ledger_latest.request_if_changed(&key, move |key| { let result = data::ledger(worker, key.0, &key.1); (key, result) });
    }
    if let Some((key, result)) = s.budget_latest.take() && key.0 == s.month {
        match result { Ok(view) => s.accept_budget(view), Err(error) => s.budget_error = Some((None, error)) }
        tx.instant(); changed = true;
    }
    if s.route == Route::Presupuesto && s.version.is_some() {
        let key = (s.month, s.version, s.budget_gen);
        let worker = Arc::clone(&s.worker);
        s.budget_latest.request_if_changed(&key, move |key| { let result = data::budget(worker, key.0); (key, result) });
    }
    if let Some((key, result)) = s.proximos_latest.take() {
        match result { Ok(view) => { s.proximos = Some(view); s.proximos_key = Some(key); } Err(error) => s.upcoming_error = Some(error) }
        tx.instant(); changed = true;
    }
    if s.open_default && s.today_date != Local::now().date_naive() { s.today_date = Local::now().date_naive(); }
    if s.route == Route::Proximos && s.version.is_some() {
        let key = s.proximos_key_now();
        let worker = Arc::clone(&s.worker);
        s.proximos_latest.request_if_changed(&key, move |key| { let result = data::proximos(worker, key.2); (key, result) });
    }
    // Automatic payments run once after the data loads and then every hour, never on the UI thread.
    if let Some(result) = s.auto_latest.take() {
        match result {
            Ok(0) => {}
            Ok(_) => { s.proximos_gen += 1; s.refresh = true; s.ledger = None; s.ledger_key = None; s.ledger_latest.cancel(); tx.instant(); changed = true; }
            Err(error) => { s.upcoming_error = Some(error); changed = true; }
        }
    }
    if s.auto_post && s.version.is_some() && now >= s.next_auto && !s.auto_latest.is_pending() && !s.mutation_active() {
        s.next_auto = now + 3600.0;
        let (worker, today) = (Arc::clone(&s.worker), s.today_date);
        s.auto_latest.request((), move |()| data::auto_post(worker, today));
    }
    if let Some(result) = s.import_latest.take() {
        s.importing = false;
        if result.is_ok() {
            // Everything the screens show may have changed.
            s.proximos_gen += 1; s.budget_gen += 1; s.refresh = true; s.ledger = None; s.ledger_key = None; s.ledger_latest.cancel();
        }
        s.import_outcome = Some(result); tx.instant(); changed = true;
    }
    // Reminders are planned on the worker: at startup, then hourly (real app only), and whenever data or settings move.
    if s.hourly && now >= s.next_reminders { s.reminders_gen += 1; s.next_reminders = now + 3600.0; }
    if let Some((key, result)) = s.reminders_latest.take() && key == s.reminders_key() {
        match result { Ok(list) => s.reminders = list, Err(error) => { s.reminders.clear(); s.settings_error = Some(format!("No se pudieron calcular los recordatorios: {error}")); } }
        tx.instant(); changed = true;
    }
    if !s.settings.enabled {
        if !s.reminders.is_empty() { s.reminders.clear(); changed = true; }
        s.reminders_latest.cancel();
    } else if s.version.is_some() {
        let (key, worker, at) = (s.reminders_key(), Arc::clone(&s.worker), s.now());
        s.reminders_latest.request_if_changed(&key, move |key| { let result = data::reminders(worker, at, key.3, key.4); (key, result) });
    }
    s.invalidate_selection();
    tx.wake_at(if s.next_poll <= now { now + 0.25 } else { s.next_poll });
    changed
}
fn on_key(s: &mut State, ev: &KeyEvent, _: &mut Tx) -> bool {
    if s.mutation_active() { return true; }
    if s.sheet.is_some() { if ev.key == Key::Escape { if !s.mutation_active() { s.sheet = None; } return true; } return false; }
    if s.confirm.is_some() { if ev.key == Key::Escape { if !s.mutation_active() { s.confirm = None; } return true; } return false; }
    if s.schedule_sheet.is_some() { if ev.key == Key::Escape { s.schedule_sheet = None; return true; } return false; }
    if s.schedule_confirm.is_some() { if ev.key == Key::Escape { s.schedule_confirm = None; return true; } return false; }
    if s.account_sheet.is_some() { if ev.key == Key::Escape { s.account_sheet = None; return true; } return false; }
    if s.import_confirm.is_some() { if ev.key == Key::Escape { s.import_confirm = None; return true; } return false; }
    if (ev.mods.cmd || ev.mods.ctrl) && !ev.mods.shift && !ev.mods.alt && matches!(ev.key, Key::Command(',')) { s.route = Route::Ajustes; return true; }
    if s.route == Route::Proximos {
        if (ev.mods.cmd || ev.mods.ctrl) && matches!(ev.key, Key::Command('n')) { s.open_schedule_sheet(None); return true; }
        if matches!(ev.key, Key::Delete | Key::Backspace) && let Some(id) = s.schedule_selected { s.request_schedule_delete(id); return true; }
    }
    if matches!(s.route, Route::Todas | Route::Cuenta(_)) {
        if (ev.mods.cmd || ev.mods.ctrl) && matches!(ev.key, Key::Command('n')) { s.open_sheet(!ev.mods.shift, None); return true; }
        if matches!(ev.key, Key::Delete | Key::Backspace) && s.selected.is_some_and(|id| s.visible_entry(id).is_some()) { s.confirm = s.selected; return true; }
    }
    if !matches!(s.route, Route::Resumen | Route::Presupuesto) || !(ev.mods.cmd || ev.mods.ctrl) || ev.mods.alt || ev.mods.shift { return false; }
    match ev.key {
        Key::Left => { s.set_month(month::previous(s.month)); true }
        Key::Right => { s.set_month(month::next(s.month)); true }
        _ => false,
    }
}

// These are the adaptive colors in the original app's FinanceTheme.
#[derive(Clone, Copy)]
struct Palette { canvas: Color, card: Color, border: Color, petroleum: Color, emerald: Color, coral: Color, amber: Color, ink: Color, muted: Color, selected: Color }
fn color(hex: &str) -> Color { rune::color::hex(hex).unwrap_or(Color::BLACK) }
fn palette(dark: bool) -> Palette {
    if dark {
        Palette { canvas: color("#151D27"), card: color("#1E2936"), border: color("#354453"), petroleum: color("#67C6DF"), emerald: color("#6DDBAD"), coral: color("#FF9A8F"), amber: color("#F5C46B"), ink: color("#F3F7F9"), muted: color("#A9BAC6"), selected: color("#304657") }
    } else {
        Palette { canvas: color("#F4F7FA"), card: color("#FFFFFF"), border: color("#DEE7ED"), petroleum: color("#155E75"), emerald: color("#147D59"), coral: color("#BE493D"), amber: color("#986000"), ink: color("#202F3A"), muted: color("#6C7E8A"), selected: color("#E2F0F4") }
    }
}

fn nav(label: &str, key: String, route: Route, selected: bool, balance: Option<i64>, p: Palette) -> El {
    let mut content = row().w_full().h(38.0).px(12.0).center_y().justify_between().gap(8.0)
        .child(text(label.to_owned()).font(type_::BODY).color(if selected { p.petroleum } else { p.ink }));
    if let Some(amount) = balance {
        content = content.child(text(money::format(amount)).font(type_::BODY).color(if amount < 0 { p.coral } else { p.muted }));
    }
    content.key(key).role("button").label(label).focusable(true).radius(8.0)
        .fill(if selected { p.selected } else { Color::TRANSPARENT })
        .hover(p.selected).on_press(move |s: &mut State| s.route = route)
}

fn view(s: &State, cx: &mut Cx<State>) -> El {
    let dark = s.dark.unwrap_or_else(|| cx.system_dark());
    let p = palette(dark);
    if let Some(error) = &s.error {
        return col().w_full().h_full().center().gap(12.0).fill(p.canvas)
            .child(text("No se pudo abrir la base de datos").font(type_::TITLE).color(p.ink))
            .child(text(error.clone()).font(type_::BODY).color(p.muted));
    }
    let mut sidebar = col().w(280.0).h_full().padding(20.0).gap(3.0).fill(p.card)
        .child(row().gap(12.0).center_y().pb(24.0)
            .child(el().size(42.0, 42.0).radius(12.0).fill(color("#155E75")).center().child(text("F").font(type_::TITLE).color(Color::WHITE)))
            .child(col().gap(3.0).child(text("Finanzas").font(type_::TITLE).color(p.ink)).child(text("Tu dinero, en orden").font(type_::BODY).color(p.muted))))
        .child(nav("Resumen", "resumen".into(), Route::Resumen, s.route == Route::Resumen, None, p))
        .child(nav("Presupuesto", "presupuesto".into(), Route::Presupuesto, s.route == Route::Presupuesto, None, p))
        .child(nav("Próximos pagos", "proximos".into(), Route::Proximos, s.route == Route::Proximos, None, p))
        .child(text("CUENTAS").font(type_::SECTION).color(p.muted).pt(24.0).pb(7.0).px(12.0))
        .child(nav("Todas", "todas".into(), Route::Todas, s.route == Route::Todas, Some(s.total()), p));
    for a in s.accounts.iter().filter(|a| !a.account.closed) {
        let route = Route::Cuenta(a.account.id);
        sidebar = sidebar.child(nav(&a.account.name, format!("cuenta-{}", a.account.id), route, s.route == route, Some(a.balance), p));
    }
    sidebar = sidebar.child(row().w_full().h(38.0).px(12.0).center_y().gap(8.0).key("new-account").role("button").label("Nueva cuenta").focusable(true).radius(8.0).hover(p.selected)
        .on_press(|s: &mut State| s.open_account_sheet()).child(text("+  Nueva cuenta").font(type_::BODY).color(p.petroleum)));
    sidebar = sidebar.child(text("CERRADAS").font(type_::SECTION).color(p.muted).pt(24.0).pb(7.0).px(12.0));
    for a in s.accounts.iter().filter(|a| a.account.closed) {
        let route = Route::Cuenta(a.account.id);
        sidebar = sidebar.child(nav(&a.account.name, format!("cuenta-{}", a.account.id), route, s.route == route, Some(a.balance), p));
    }
    sidebar = sidebar.child(el().grow(1.0)).child(nav("Ajustes", "ajustes".into(), Route::Ajustes, s.route == Route::Ajustes, None, p));
    let title = match s.route {
        Route::Resumen => "Resumen", Route::Presupuesto => "Presupuesto", Route::Proximos => "Próximos pagos", Route::Ajustes => "Ajustes", Route::Todas => "Todas las cuentas",
        Route::Cuenta(id) => s.accounts.iter().find(|a| a.account.id == id).map_or("Cuenta", |a| a.account.name.as_str()),
    };
    let detail = if s.route == Route::Resumen { overview::render(s, p) } else if s.route == Route::Presupuesto { budget::render(s, p) } else if s.route == Route::Proximos { proximos::render(s, p, dark) } else if s.route == Route::Ajustes { ajustes::render(s, p) } else if matches!(s.route, Route::Todas | Route::Cuenta(_)) { ledger::render(s, p, title) } else {
        col().grow(1.0).h_full().padding(48.0).gap(12.0)
            .child(text(title.to_owned()).font(type_::TITLE).color(p.ink))
            .child(text("Tu espacio de finanzas").font(type_::BODY).color(p.muted))
    };
    let modal_open = s.sheet.is_some() || s.confirm.is_some() || s.schedule_sheet.is_some() || s.schedule_confirm.is_some() || s.account_sheet.is_some() || s.import_confirm.is_some();
    let sidebar = if modal_open { sidebar.inert() } else { sidebar };
    let detail = if modal_open { detail.inert() } else { detail };
    let mut root = row().w_full().h_full().fill(p.canvas)
        .child(sidebar).child(el().w(1.0).h_full().fill(p.border)).child(detail);
    if s.sheet.is_some() { root = root.child(sheet::render(s, p, dark)); }
    if s.confirm.is_some() { root = root.child(sheet::confirm(s, p)); }
    if s.schedule_sheet.is_some() { root = root.child(schedule_sheet::render(s, p, dark)); }
    if s.schedule_confirm.is_some() { root = root.child(schedule_sheet::confirm(s, p)); }
    if s.account_sheet.is_some() { root = root.child(account_sheet::render(s, p, dark)); }
    if s.import_confirm.is_some() { root = root.child(ajustes::confirm(s, p)); }
    root
}

fn main() {
    rune::run(AppSpec::<State> {
        scenes: &[("demo-light", demo_light), ("demo-dark", demo_dark), ("empty", empty_scene), ("movimientos-light", movements_light), ("movimientos-dark", movements_dark), ("entry-sheet", entry_light), ("entry-sheet-dark", entry_dark), ("presupuesto-light", budget_light), ("presupuesto-dark", budget_dark), ("proximos-light", proximos_light), ("proximos-dark", proximos_dark), ("schedule-sheet", schedule_sheet_scene), ("ajustes-light", ajustes_light), ("ajustes-dark", ajustes_dark), ("nueva-cuenta", nueva_cuenta_scene), ("recordatorios", reminders_scene)],
        setup: Some(setup), poll: Some(poll), on_key: Some(on_key),
        ..AppSpec::new("Finanzas", (1200.0, 800.0), State::load, view)
    });
}

fn chart_fraction(value: i64, maximum: i64) -> f32 {
    if maximum <= 0 { return 0.0; }
    (value as f32 / maximum as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    fn movement_runtime() -> Runtime<State> {
        let mut state = State::empty(); movements_light(&mut state);
        let mut rt = Runtime::new(state, view, (1200.0, 800.0), Env::new(false, 1.0));
        rt.on_key = Some(on_key); rt.poll = Some(poll); rt.frame(0.0); rt
    }
    fn command_n(rt: &mut Runtime<State>) {
        rt.key(KeyEvent { key: Key::Command('n'), mods: rune::Mods { cmd: true, ..Default::default() } });
        rt.frame(0.0);
    }

    fn budget_runtime() -> Runtime<State> {
        let mut state = State::empty(); budget_light(&mut state);
        let mut rt = Runtime::new(state, view, (1200.0, 800.0), Env::new(false, 1.0));
        rt.on_key = Some(on_key); rt.poll = Some(poll); rt.frame(0.0); rt
    }
    fn settle(rt: &mut Runtime<State>) { for _ in 0..150 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); } }
    fn category(rt: &Runtime<State>, name: &str) -> finanzas_core::service::BudgetCategory {
        let view = rt.state.budget.as_ref().unwrap();
        view.groups.iter().flat_map(|g| g.categories.iter()).find(|c| c.name == name).map(|c| finanzas_core::service::BudgetCategory { id: c.id, name: c.name.clone(), hidden: c.hidden, budgeted: c.budgeted, activity: c.activity, available: c.available, spent_share: c.spent_share }).unwrap()
    }
    fn text_of(rt: &Runtime<State>, key: &str) -> Option<String> { rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).and_then(|e| e.text) }
    fn type_into(rt: &mut Runtime<State>, key: &str, value: &str) {
        let (x, y) = rt.find(key).unwrap_or_else(|| panic!("missing {key}"));
        rt.pointer_down(x, y); rt.pointer_up(x, y); rt.frame(0.0);
        rt.key(KeyEvent { key: Key::Command('a'), mods: rune::Mods { cmd: true, ..Default::default() } });
        rt.key(KeyEvent { key: Key::Text(value.into()), mods: Default::default() });
        rt.key(KeyEvent { key: Key::Enter, mods: Default::default() });
        rt.frame(0.0);
    }
    #[test]
    fn editing_assigned_updates_the_category_and_available_to_assign() {
        let mut rt = budget_runtime();
        let vivienda = category(&rt, "Vivienda");
        let before = rt.state.budget.as_ref().unwrap().to_budget;
        assert_eq!(vivienda.budgeted, 80_000);
        type_into(&mut rt, &format!("budget-input-{}", vivienda.id), "150");
        settle(&mut rt);
        let after = category(&rt, "Vivienda");
        assert_eq!(after.budgeted, 15_000);
        assert_eq!(rt.state.budget.as_ref().unwrap().to_budget, before + 80_000 - 15_000);
        assert_eq!(text_of(&rt, "to-assign").unwrap(), money::format(before + 65_000));
        assert!(rt.state.budget_error.is_none());
    }
    #[test]
    fn invalid_assigned_keeps_text_and_shows_error() {
        let mut rt = budget_runtime();
        let vivienda = category(&rt, "Vivienda");
        type_into(&mut rt, &format!("budget-input-{}", vivienda.id), "abc");
        settle(&mut rt);
        assert_eq!(rt.state.budget_inputs[&vivienda.id].0.text, "abc");
        assert_eq!(text_of(&rt, "budget-error").as_deref(), Some("Importe inválido"));
        assert_eq!(category(&rt, "Vivienda").budgeted, 80_000);
    }
    #[test]
    fn month_navigation_changes_the_budget_month() {
        let mut rt = budget_runtime();
        for (key, month, title) in [("previous-month", 202610, "Octubre de 2026"), ("next-month", 202611, "Noviembre de 2026")] {
            let (x, y) = rt.find(key).unwrap();
            rt.pointer_down(x, y); rt.pointer_up(x, y);
            settle(&mut rt);
            assert_eq!(rt.state.budget.as_ref().unwrap().month, month);
            assert_eq!(text_of(&rt, "month-title").as_deref(), Some(title));
        }
    }
    #[test]
    fn pending_mutation_blocks_budget_edits() {
        let mut rt = budget_runtime();
        let vivienda = category(&rt, "Vivienda");
        let other = category(&rt, "Ocio");
        let key = format!("budget-input-{}", vivienda.id);
        let guard = Arc::clone(&rt.state.worker);
        let guard = guard.lock().unwrap();
        type_into(&mut rt, &key, "150");
        assert!(rt.state.mutation_active());
        let (x, y) = rt.find(&format!("budget-input-{}", other.id)).unwrap();
        rt.pointer_down(x, y); rt.pointer_up(x, y);
        rt.key(KeyEvent { key: Key::Text("999".into()), mods: Default::default() });
        rt.key(KeyEvent { key: Key::Enter, mods: Default::default() });
        rt.frame(0.0);
        assert_eq!(rt.state.budget_inputs[&other.id].0.text, "560");
        drop(guard);
        settle(&mut rt);
        assert_eq!(category(&rt, "Vivienda").budgeted, 15_000);
        assert_eq!(category(&rt, "Ocio").budgeted, other.budgeted);
    }
    #[test]
    fn hidden_categories_are_not_listed_and_group_totals_exclude_them() {
        let mut rt = budget_runtime();
        let id = { let guard = rt.state.worker.lock().unwrap(); let store = guard.as_ref().unwrap();
            let group = store.list_category_groups().unwrap().into_iter().find(|g| !g.is_income).unwrap().id;
            let id = store.insert_category(&finanzas_core::models::Category { id: 0, name: "Antigua".into(), hidden: true, sort_order: 99.0, group_id: Some(group) }).unwrap();
            store.set_budget(202611, id, 50_000).unwrap(); id };
        rt.state.refresh = true; rt.state.budget_gen += 1;
        settle(&mut rt);
        assert!(rt.state.budget.as_ref().unwrap().groups.iter().any(|g| g.categories.iter().any(|c| c.id == id)));
        assert!(rt.find(&format!("budget-input-{id}")).is_none());
        let group = rt.state.budget.as_ref().unwrap().groups.iter().find(|g| !g.is_income).unwrap();
        let visible: i64 = group.categories.iter().filter(|c| !c.hidden).map(|c| c.budgeted).sum();
        assert_eq!(text_of(&rt, &format!("group-budgeted-{}", group.id)).as_deref(), Some(money::format(visible).as_str()));
    }
    #[test]
    fn new_income_selects_a_visible_income_category() {
        let mut s = State::empty(); movements_light(&mut s);
        s.open_sheet(false, None);
        let id = s.sheet.as_ref().unwrap().category.expect("income category");
        assert!(s.categories.iter().any(|c| c.id == id && !c.hidden && s.groups.iter().any(|g| g.is_income && Some(g.id) == c.group_id)));
    }
    #[test]
    fn command_n_opens_entry_sheet() {
        let mut rt = movement_runtime(); command_n(&mut rt);
        assert!(rt.find("entry-sheet").is_some());
    }
    #[test]
    fn escape_cancels_entry_sheet() {
        let mut rt = movement_runtime(); command_n(&mut rt);
        rt.key(KeyEvent { key: Key::Escape, mods: Default::default() }); rt.frame(0.0);
        assert!(rt.state.sheet.is_none());
    }
    #[test]
    fn sheet_field_can_be_focused_by_pointer() {
        let mut rt = movement_runtime(); command_n(&mut rt);
        for _ in 0..60 { rt.frame(1.0 / 60.0); }
        let (x, y) = rt.find("entry-notes").unwrap(); rt.pointer_down(x, y); rt.pointer_up(x, y); rt.frame(0.0);
        assert!(rt.elements().into_iter().any(|e| e.key.as_deref() == Some("entry-notes") && e.focused));
    }
    #[test]
    fn entering_amount_and_return_adds_expense() {
        let mut rt = movement_runtime(); command_n(&mut rt);
        assert!(rt.find("entry-amount").is_some());
        rt.key(KeyEvent { key: Key::Text("125.50".into()), mods: Default::default() });
        rt.key(KeyEvent { key: Key::Enter, mods: Default::default() });
        for _ in 0..100 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); }
        assert!(rt.state.ledger.as_ref().unwrap().rows.iter().any(|r| r.amount == -12_550), "sheet amount {:?}, pending {}, error {:?}", rt.state.sheet.as_ref().map(|s| &s.amount.text), rt.state.mutation.is_pending(), rt.state.error);
    }
    #[test]
    fn failed_save_keeps_input_and_shows_error() {
        let mut rt = movement_runtime(); command_n(&mut rt);
        rt.state.sheet.as_mut().unwrap().amount.replace_text("125.50");
        rt.state.worker = Arc::new(Mutex::new(None));
        rt.state.submit();
        for _ in 0..100 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); }
        assert_eq!(rt.state.sheet.as_ref().map(|s| s.amount.text.as_str()), Some("125.50"));
        assert!(rt.state.mutation_error.is_some());
        assert!(rt.state.worker.lock().unwrap().is_none(), "failed test worker must not open the default store");
    }
    #[test]
    fn stale_selection_cannot_delete_after_context_change() {
        for mode in 0..3 {
            let mut rt = movement_runtime();
            let a = rt.state.accounts[0].account.id;
            let b = rt.state.accounts[1].account.id;
            let worker = Arc::clone(&rt.state.worker);
            let before = data::ledger(Arc::clone(&worker), None, "").unwrap();
            let balances = data::fetch(Arc::clone(&worker), 202611, 6, None, true).unwrap().unwrap().accounts.into_iter().map(|a| (a.account.id, a.balance)).collect::<Vec<_>>();
            rt.state.route = Route::Cuenta(a);
            rt.state.ledger = Some(data::ledger(Arc::clone(&worker), Some(a), "").unwrap());
            rt.state.ledger_key = Some((Some(a), String::new(), rt.state.version));
            let id = rt.state.ledger.as_ref().unwrap().rows[0].id;
            rt.state.selected = Some(id);
            if mode == 2 { rt.state.confirm = Some(id); }
            if mode == 1 { rt.state.search.replace_text("no-such-transaction"); } else { rt.state.route = Route::Cuenta(b); }
            if mode != 2 {
                rt.key(KeyEvent { key: Key::Delete, mods: Default::default() });
                assert!(rt.state.confirm.is_none(), "hidden selection requested deletion in mode {mode}");
                rt.state.confirm = Some(id); // A delayed menu or modal must also be rejected.
            }
            rt.state.delete_selected();
            for _ in 0..100 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); }
            let after = data::ledger(Arc::clone(&worker), None, "").unwrap();
            assert_eq!(after.rows.len(), before.rows.len(), "mode {mode}");
            assert!(after.rows.iter().any(|r| r.id == id), "mode {mode}");
            let after_balances = data::fetch(worker, 202611, 6, None, true).unwrap().unwrap().accounts.into_iter().map(|a| (a.account.id, a.balance)).collect::<Vec<_>>();
            assert_eq!(after_balances, balances, "mode {mode}");
        }
    }
    #[test]
    fn duplicate_pending_save_and_delete_mutate_once() {
        let mut rt = movement_runtime();
        let worker = Arc::clone(&rt.state.worker);
        let before = data::ledger(Arc::clone(&worker), None, "").unwrap();
        command_n(&mut rt);
        rt.state.sheet.as_mut().unwrap().amount.replace_text("123.45");
        let guard = worker.lock().unwrap();
        rt.state.submit(); rt.state.submit();
        assert!(rt.state.mutation_active());
        drop(guard);
        for _ in 0..100 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); }
        let saved = data::ledger(Arc::clone(&worker), None, "").unwrap();
        assert_eq!(saved.rows.len(), before.rows.len() + 1);
        assert_eq!(saved.total, before.total - 12_345);
        let id = saved.rows.iter().find(|r| r.amount == -12_345).unwrap().id;
        rt.state.confirm = Some(id);
        let guard = worker.lock().unwrap();
        rt.state.delete_selected(); rt.state.delete_selected();
        assert!(rt.state.mutation_active());
        drop(guard);
        for _ in 0..100 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); }
        let after = data::ledger(worker, None, "").unwrap();
        assert_eq!(after.rows.len(), before.rows.len());
        assert_eq!(after.total, before.total);
        assert!(!after.rows.iter().any(|r| r.id == id));
    }
    #[test]
    fn pending_save_blocks_draft_input_and_type_switch() {
        let mut rt = movement_runtime(); command_n(&mut rt);
        rt.state.sheet.as_mut().unwrap().amount.replace_text("125.50");
        let worker = Arc::clone(&rt.state.worker);
        let guard = worker.lock().unwrap();
        rt.key(KeyEvent { key: Key::Enter, mods: Default::default() });
        rt.frame(0.0);
        assert!(rt.state.mutation_active());
        for key in ["entry-amount", "entry-payee", "entry-date", "entry-notes", "account-picker", "category-picker", "Ingreso"] {
            let (x, y) = rt.find(key).unwrap();
            rt.pointer_down(x, y); rt.pointer_up(x, y);
            rt.key(KeyEvent { key: Key::Text("9".into()), mods: Default::default() });
            rt.frame(0.0);
        }
        let sh = rt.state.sheet.as_ref().unwrap();
        assert_eq!(sh.amount.text, "125.50");
        assert!(sh.payee.text.is_empty() && sh.notes.text.is_empty());
        assert_eq!(sh.date.text, "10/11/2026");
        assert_eq!(sh.category, None);
        assert!(sh.expense);
        drop(guard);
        for _ in 0..100 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); }
        assert!(rt.state.sheet.is_none());
        assert!(rt.state.ledger.as_ref().unwrap().rows.iter().any(|r| r.amount == -12_550 && r.notes.is_empty()));
    }
    #[test]
    fn pending_save_cannot_discard_form_before_failure() {
        let mut rt = movement_runtime(); command_n(&mut rt);
        rt.state.sheet.as_mut().unwrap().amount.replace_text("125.50");
        let worker = Arc::clone(&rt.state.worker);
        let guard = worker.lock().unwrap();
        rt.state.submit();
        for _ in 0..60 { rt.frame(1.0 / 60.0); }
        let (x, y) = rt.find("entry-cancel").unwrap();
        rt.pointer_down(x, y); rt.pointer_up(x, y); rt.frame(0.0);
        assert_eq!(rt.state.sheet.as_ref().map(|s| s.amount.text.as_str()), Some("125.50"));
        drop(guard);
    }
    #[test]
    fn selecting_payee_suggestion_prefills_category() {
        let mut rt = movement_runtime(); command_n(&mut rt);
        let expected = rt.state.ledger.as_ref().unwrap().rows.iter().find(|r| r.payee_name.as_deref() == Some("Farmacia")).unwrap().category_id;
        for _ in 0..60 { rt.frame(1.0 / 60.0); }
        let (x, y) = rt.find("entry-payee").unwrap();
        rt.pointer_down(x, y); rt.pointer_up(x, y);
        rt.key(KeyEvent { key: Key::Text("Farm".into()), mods: Default::default() });
        for _ in 0..100 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); }
        let (x, y) = rt.find("suggest-Farmacia").expect("payee suggestion");
        rt.pointer_down(x, y); rt.pointer_up(x, y); rt.frame(0.0);
        let sh = rt.state.sheet.as_ref().unwrap();
        assert_eq!(sh.payee.text, "Farmacia");
        assert_eq!(sh.category, expected);
        assert!(expected.is_some());
    }
    #[test]
    fn editing_row_keeps_id_and_untouched_fields() {
        let mut rt = movement_runtime();
        let original = &rt.state.ledger.as_ref().unwrap().rows[0];
        let (id, date, amount, account, category, payee, cleared) = (original.id, original.date, original.amount, original.account_id, original.category_id, original.payee_name.clone(), original.cleared);
        let (x, y) = rt.find(&format!("entry-{id}")).unwrap();
        rt.pointer_down(x, y); rt.pointer_up(x, y); rt.pointer_down(x, y); rt.pointer_up(x, y); rt.frame(0.0);
        assert_eq!(rt.state.sheet.as_ref().unwrap().id, Some(id));
        for _ in 0..60 { rt.frame(1.0 / 60.0); }
        let (x, y) = rt.find("entry-notes").unwrap();
        rt.pointer_down(x, y); rt.pointer_up(x, y);
        rt.key(KeyEvent { key: Key::Text("Nota nueva".into()), mods: Default::default() });
        rt.key(KeyEvent { key: Key::Enter, mods: Default::default() });
        for _ in 0..100 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); }
        let row = rt.state.ledger.as_ref().unwrap().rows.iter().find(|r| r.id == id).unwrap();
        assert_eq!((row.date, row.amount, row.account_id, row.category_id, row.payee_name.clone(), row.cleared), (date, amount, account, category, payee, cleared));
        assert_eq!(row.notes, "Nota nueva");
    }
    #[test]
    fn delete_requires_confirmation() {
        let mut rt = movement_runtime();
        let id = rt.state.ledger.as_ref().unwrap().rows[0].id;
        let (x, y) = rt.find(&format!("entry-{id}")).unwrap();
        rt.pointer_down(x, y); rt.pointer_up(x, y);
        rt.key(KeyEvent { key: Key::Delete, mods: Default::default() }); rt.frame(0.0);
        assert!(rt.find("confirm-delete").is_some());
        for _ in 0..60 { rt.frame(1.0 / 60.0); }
        let (x, y) = rt.find("delete-confirm").unwrap(); rt.pointer_down(x, y); rt.pointer_up(x, y);
        for _ in 0..100 { rt.frame(0.02); std::thread::sleep(std::time::Duration::from_millis(5)); }
        assert!(!rt.state.ledger.as_ref().unwrap().rows.iter().any(|r| r.id == id));
    }
    fn proximos_runtime() -> Runtime<State> {
        let mut state = State::empty(); proximos_scene(&mut state, false);
        let mut rt = Runtime::new(state, view, (1200.0, 1800.0), Env::new(false, 1.0));
        rt.on_key = Some(on_key); rt.poll = Some(poll); rt.frame(0.0); rt
    }
    fn click(rt: &mut Runtime<State>, key: &str) {
        let (x, y) = rt.find(key).unwrap_or_else(|| panic!("missing {key}"));
        rt.pointer_down(x, y); rt.pointer_up(x, y); rt.frame(0.0);
    }
    fn animate(rt: &mut Runtime<State>) { for _ in 0..60 { rt.frame(1.0 / 60.0); } }
    fn fill(rt: &mut Runtime<State>, key: &str, value: &str) {
        click(rt, key);
        rt.key(KeyEvent { key: Key::Command('a'), mods: rune::Mods { cmd: true, ..Default::default() } });
        rt.key(KeyEvent { key: Key::Text(value.into()), mods: Default::default() });
        rt.frame(0.0);
    }
    fn has_text(rt: &Runtime<State>, value: &str) -> bool { rt.elements().into_iter().any(|e| e.text.as_deref() == Some(value)) }
    fn schedule_id(rt: &Runtime<State>, name: &str) -> Id {
        rt.state.proximos.as_ref().unwrap().schedules.iter().find(|v| v.schedule.name == name).unwrap_or_else(|| panic!("no schedule {name}")).schedule.id
    }
    fn stored(rt: &Runtime<State>, id: Id) -> Option<finanzas_core::models::Schedule> { rt.state.worker.lock().unwrap().as_ref().unwrap().get_schedule(id).unwrap() }
    fn entries(rt: &Runtime<State>) -> data::Ledger { data::ledger(Arc::clone(&rt.state.worker), None, "").unwrap() }
    fn schedule_count(rt: &Runtime<State>) -> usize { data::proximos(Arc::clone(&rt.state.worker), rt.state.today_date).unwrap().schedules.len() }
    fn d(y: i32, m: u32, day: u32) -> NaiveDate { NaiveDate::from_ymd_opt(y, m, day).unwrap() }

    #[test]
    fn registrar_posts_one_entry_and_advances_the_next_date() {
        let mut rt = proximos_runtime();
        let id = schedule_id(&rt, "Renta");
        assert_eq!(text_of(&rt, "metric-overdue").as_deref(), Some("1"));
        let before = entries(&rt);
        click(&mut rt, &format!("post-{id}"));
        settle(&mut rt);
        let after = entries(&rt);
        assert_eq!(after.rows.len(), before.rows.len() + 1);
        let posted: Vec<_> = after.rows.iter().filter(|r| r.amount == -80_000).collect();
        assert_eq!(posted.len(), 1);
        assert_eq!((posted[0].date, posted[0].payee_name.as_deref(), posted[0].category_name.as_deref()), (d(2026, 11, 5), Some("Renta"), Some("Vivienda")));
        assert_eq!(stored(&rt, id).unwrap().next_date, Some(d(2026, 12, 5)));
        assert_eq!(text_of(&rt, "metric-overdue").as_deref(), Some("0"));
        assert!(rt.state.upcoming_error.is_none());
    }
    #[test]
    fn saltar_advances_without_creating_an_entry() {
        let mut rt = proximos_runtime();
        let id = schedule_id(&rt, "Renta");
        let before = entries(&rt);
        click(&mut rt, &format!("skip-{id}"));
        settle(&mut rt);
        let after = entries(&rt);
        assert_eq!((after.rows.len(), after.total), (before.rows.len(), before.total));
        assert_eq!(stored(&rt, id).unwrap().next_date, Some(d(2026, 12, 5)));
        assert_eq!(text_of(&rt, "metric-overdue").as_deref(), Some("0"));
    }
    #[test]
    fn duplicate_tap_while_pending_mutates_once() {
        let mut rt = proximos_runtime();
        let id = schedule_id(&rt, "Renta");
        let before = entries(&rt);
        let worker = Arc::clone(&rt.state.worker);
        let guard = worker.lock().unwrap();
        rt.state.schedule_action(id, true); rt.state.schedule_action(id, true); rt.state.schedule_action(id, false);
        assert!(rt.state.mutation_active());
        rt.frame(0.0);
        click(&mut rt, &format!("post-{id}"));
        drop(guard);
        settle(&mut rt);
        assert_eq!(entries(&rt).rows.len(), before.rows.len() + 1);
        assert_eq!(stored(&rt, id).unwrap().next_date, Some(d(2026, 12, 5)));
        assert!(rt.state.upcoming_error.is_none());
    }
    #[test]
    fn creating_a_monthly_schedule_shows_it() {
        let mut rt = proximos_runtime();
        rt.key(KeyEvent { key: Key::Command('n'), mods: rune::Mods { cmd: true, ..Default::default() } });
        rt.frame(0.0);
        assert!(rt.find("schedule-sheet").is_some());
        animate(&mut rt);
        fill(&mut rt, "schedule-name", "Gimnasio");
        fill(&mut rt, "schedule-amount", "350");
        fill(&mut rt, "schedule-date", "15/11/2026");
        click(&mut rt, "schedule-save");
        settle(&mut rt);
        assert!(rt.state.schedule_sheet.is_none(), "error {:?} sheet {:?}", rt.state.mutation_error, rt.state.schedule_sheet.as_ref().map(|s| (s.name.text.clone(), s.amount.text.clone(), s.start.text.clone(), s.account, s.valid())));
        let created = rt.state.proximos.as_ref().unwrap().schedules.iter().find(|v| v.schedule.name == "Gimnasio").expect("listed").schedule.clone();
        let rule = created.recurrence.unwrap();
        assert_eq!((created.amount, rule.frequency, rule.interval, rule.start, created.auto_post), (-35_000, finanzas_core::models::Frequency::Monthly, 1, d(2026, 11, 15), false));
        assert!(has_text(&rt, "Gimnasio"));
        assert_eq!(text_of(&rt, "metric-schedules").as_deref(), Some("5"));
    }
    #[test]
    fn editing_a_schedule_keeps_its_id_and_progress() {
        let mut rt = proximos_runtime();
        let id = schedule_id(&rt, "Renta");
        rt.state.open_schedule_sheet(Some(id));
        rt.frame(0.0); animate(&mut rt);
        assert_eq!(rt.state.schedule_sheet.as_ref().unwrap().id, Some(id));
        assert!(has_text(&rt, "Editar pago programado"));
        fill(&mut rt, "schedule-name", "Renta depa");
        click(&mut rt, "schedule-save");
        settle(&mut rt);
        let s = stored(&rt, id).unwrap();
        assert_eq!((s.name.as_str(), s.amount, s.next_date), ("Renta depa", -80_000, Some(d(2026, 11, 5))));
        assert_eq!(schedule_count(&rt), 4);
    }
    #[test]
    fn invalid_schedule_cannot_be_saved() {
        let mut rt = proximos_runtime();
        rt.state.open_schedule_sheet(None);
        let before = schedule_count(&rt);
        rt.state.submit_schedule();
        assert!(!rt.state.mutation_active());
        let sh = rt.state.schedule_sheet.as_mut().unwrap();
        sh.name.replace_text("Sin monto"); sh.amount.replace_text("abc");
        assert!(!sh.valid());
        sh.amount.replace_text("10"); sh.start.replace_text("31/02/2026");
        assert!(!sh.valid());
        rt.state.submit_schedule();
        assert!(!rt.state.mutation_active());
        assert_eq!(schedule_count(&rt), before);
    }
    #[test]
    fn delete_requires_confirmation_that_names_the_schedule() {
        let mut rt = proximos_runtime();
        let id = schedule_id(&rt, "Netflix");
        click(&mut rt, &format!("delete-{id}"));
        assert!(rt.find("confirm-schedule-delete").is_some());
        assert_eq!(schedule_count(&rt), 4, "nothing is deleted before confirming");
        assert!(rt.elements().into_iter().any(|e| e.text.as_deref().is_some_and(|t| t.contains("Netflix") && t.contains("Tarjeta") && t.contains(&money::format(-2_000)))));
        animate(&mut rt);
        click(&mut rt, "schedule-delete-cancel");
        assert!(rt.find("confirm-schedule-delete").is_none());
        assert_eq!(schedule_count(&rt), 4);
        click(&mut rt, &format!("delete-{id}"));
        animate(&mut rt);
        click(&mut rt, "schedule-delete-confirm");
        settle(&mut rt);
        assert_eq!(schedule_count(&rt), 3);
        assert!(stored(&rt, id).is_none());
        assert!(rt.state.schedule_confirm.is_none());
        assert!(!has_text(&rt, "Netflix"));
    }
    #[test]
    fn stale_target_cannot_delete() {
        for mode in 0..4 {
            let mut rt = proximos_runtime();
            let id = schedule_id(&rt, "Netflix");
            rt.state.request_schedule_delete(id);
            assert_eq!(rt.state.schedule_confirm, Some(id));
            match mode {
                0 => rt.state.route = Route::Resumen,
                1 => rt.state.proximos_gen += 1, // data changed; the list on screen is not current
                2 => rt.state.schedule_confirm = Some(9_999),
                _ => { rt.state.schedule_confirm = None; rt.state.route = Route::Todas; rt.key(KeyEvent { key: Key::Delete, mods: Default::default() }); assert!(rt.state.schedule_confirm.is_none(), "hidden selection asked to delete"); rt.state.schedule_confirm = Some(id); }
            }
            rt.state.delete_schedule_confirmed();
            assert!(!rt.state.mutation_active(), "mode {mode}");
            settle(&mut rt);
            assert_eq!(schedule_count(&rt), 4, "mode {mode}");
            assert!(stored(&rt, id).is_some(), "mode {mode}");
        }
    }
    #[test]
    fn duplicate_confirmed_delete_mutates_once() {
        let mut rt = proximos_runtime();
        let id = schedule_id(&rt, "Netflix");
        rt.state.request_schedule_delete(id);
        let worker = Arc::clone(&rt.state.worker);
        let guard = worker.lock().unwrap();
        rt.state.delete_schedule_confirmed(); rt.state.delete_schedule_confirmed();
        assert!(rt.state.mutation_active());
        drop(guard);
        settle(&mut rt);
        assert_eq!(schedule_count(&rt), 3);
        assert!(rt.state.mutation_error.is_none());
    }
    #[test]
    fn list_shows_overdue_auto_recurrence_account_and_category() {
        let rt = proximos_runtime();
        assert!(has_text(&rt, "Tus próximos compromisos"));
        for text in ["Vencido", "Automático", "Renta · Cuenta principal · Vivienda · Mensual", "Netflix · Tarjeta · Ocio · Mensual", "Saldo: ", "Registrar", "Saltar"] {
            assert!(rt.elements().into_iter().any(|e| e.text.as_deref().is_some_and(|t| t.starts_with(text))), "missing {text}");
        }
        assert!(has_text(&rt, &money::format(-80_000)) && has_text(&rt, &money::format(320_000)));
        assert!(rt.find("section-overdue").is_some() && rt.find("section-coming").is_some() && rt.find("section-schedules").is_some());
    }
    #[test]
    fn empty_list_says_nothing_is_scheduled() {
        let mut s = State::empty(); scene(&mut s, false, false); s.route = Route::Proximos;
        let loaded = data::proximos(Arc::clone(&s.worker), s.today_date).unwrap();
        s.proximos = Some(loaded); s.proximos_key = Some(s.proximos_key_now());
        let mut rt = Runtime::new(s, view, (1200.0, 800.0), Env::new(false, 1.0));
        rt.on_key = Some(on_key); rt.poll = Some(poll); rt.frame(0.0);
        assert!(has_text(&rt, "Nada programado"));
        assert_eq!(text_of(&rt, "metric-schedules").as_deref(), Some("0"));
        assert!(rt.find("section-overdue").is_none());
    }
    #[test]
    fn automatic_payments_post_once_off_the_ui_thread_only_when_enabled() {
        assert!(State::load().auto_post && !State::empty().auto_post);
        let mut rt = proximos_runtime();
        {
            let guard = rt.state.worker.lock().unwrap(); let store = guard.as_ref().unwrap();
            let account = store.list_accounts().unwrap()[0].id;
            let rule = finanzas_core::models::Recurrence { start: d(2026, 11, 8), frequency: finanzas_core::models::Frequency::Monthly, interval: 1, skip_weekend: false, weekend_mode: finanzas_core::models::WeekendMode::After, end_date: None };
            store.insert_schedule(&finanzas_core::models::Schedule { id: 0, name: "Spotify".into(), amount: -1_500, auto_post: true, completed: false, next_date: None, recurrence: Some(rule), account_id: Some(account), category_id: None, payee_id: None }).unwrap();
        }
        let before = entries(&rt).rows.len();
        settle(&mut rt);
        assert_eq!(entries(&rt).rows.len(), before, "scenes never auto-post");
        rt.state.auto_post = true;
        settle(&mut rt); settle(&mut rt);
        let after = entries(&rt);
        assert_eq!(after.rows.iter().filter(|r| r.amount == -1_500).count(), 1, "posted once, not on every frame");
        assert_eq!(after.rows.len(), before + 1);
        assert!(rt.state.proximos.as_ref().unwrap().schedules.iter().any(|v| v.schedule.name == "Spotify" && v.next_date == Some(d(2026, 12, 8))));
    }
    #[test]
    fn sidebar_navigation_changes_route() {
        let mut rt = Runtime::new(State::empty(), view, (1200.0, 800.0), Env::new(false, 1.0));
        rt.frame(0.0);
        let (x, y) = rt.find("presupuesto").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
        assert_eq!(rt.state.route, Route::Presupuesto);
    }
    #[test]
    fn previous_month_changes_the_displayed_header() {
        let mut state = State::empty();
        state.month = 202611;
        let mut rt = Runtime::new(state, view, (1200.0, 800.0), Env::new(false, 1.0));
        rt.frame(0.0);
        let title = |rt: &Runtime<State>| rt.elements().into_iter().find(|e| e.key.as_deref() == Some("month-title")).and_then(|e| e.text).unwrap();
        assert_eq!(title(&rt), "Noviembre de 2026");
        let (x, y) = rt.find("previous-month").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
        assert_eq!(title(&rt), "Octubre de 2026");
    }
    #[test]
    fn all_accounts_shows_the_movements_screen() {
        let mut rt = Runtime::new(State::empty(), view, (1200.0, 800.0), Env::new(false, 1.0));
        rt.frame(0.0);
        let (x, y) = rt.find("todas").unwrap();
        rt.pointer_down(x, y); rt.pointer_up(x, y); rt.frame(0.0);
        assert!(rt.find("ledger-title").is_some());
    }
    #[test]
    fn chart_range_marks_the_selected_segment() {
        let mut state = State::empty();
        demo_light(&mut state);
        let mut rt = Runtime::new(state, view, (1200.0, 800.0), Env::new(false, 1.0));
        rt.frame(0.0);
        let fill = |rt: &Runtime<State>, key: &str| rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap().fill;
        assert_ne!(fill(&rt, "chart-6"), fill(&rt, "chart-12"));
        let old = fill(&rt, "chart-12");
        let (x, y) = rt.find("chart-12").unwrap();
        rt.pointer_down(x, y); rt.pointer_up(x, y);
        for _ in 0..60 { rt.frame(1.0 / 60.0); }
        assert_ne!(fill(&rt, "chart-12"), old);
        assert_eq!(rt.state.chart_months, 12);
    }
    #[test]
    fn chart_scales_values_and_handles_empty_data() {
        assert_eq!(chart_fraction(50, 100), 0.5);
        assert_eq!(chart_fraction(0, 0), 0.0);
        assert_eq!(chart_fraction(-1, 100), 0.0);
        assert_eq!(chart_fraction(200, 100), 1.0);
    }
}
