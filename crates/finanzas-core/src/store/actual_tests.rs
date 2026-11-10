use super::*;
use crate::models::{Account, Category, CategoryGroup, Frequency, WeekendMode};
use chrono::NaiveDate;
use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

const FIXTURE_SQL: &str = r####"CREATE TABLE accounts (id TEXT PRIMARY KEY, name TEXT, offbudget INTEGER DEFAULT 0, closed INTEGER DEFAULT 0, tombstone INTEGER DEFAULT 0, sort_order REAL);
CREATE TABLE category_groups (id TEXT PRIMARY KEY, name TEXT, is_income INTEGER DEFAULT 0, sort_order REAL, tombstone INTEGER DEFAULT 0, hidden BOOLEAN NOT NULL DEFAULT 0);
CREATE TABLE categories (id TEXT PRIMARY KEY, name TEXT, is_income INTEGER DEFAULT 0, cat_group TEXT, sort_order REAL, tombstone INTEGER DEFAULT 0, hidden BOOLEAN NOT NULL DEFAULT 0);
CREATE TABLE category_mapping (id TEXT PRIMARY KEY, transferId TEXT);
CREATE TABLE payees (id TEXT PRIMARY KEY, name TEXT, tombstone INTEGER DEFAULT 0, transfer_acct TEXT);
CREATE TABLE payee_mapping (id TEXT PRIMARY KEY, targetId TEXT);
CREATE TABLE transactions (id TEXT PRIMARY KEY, isParent INTEGER DEFAULT 0, isChild INTEGER DEFAULT 0, acct TEXT, category TEXT, amount INTEGER, description TEXT, notes TEXT, date INTEGER, tombstone INTEGER DEFAULT 0, cleared INTEGER DEFAULT 1, parent_id TEXT, schedule TEXT);
CREATE TABLE rules (id TEXT PRIMARY KEY, conditions TEXT, actions TEXT, tombstone INTEGER DEFAULT 0);
CREATE TABLE schedules (id TEXT PRIMARY KEY, rule TEXT, completed INTEGER DEFAULT 0, posts_transaction INTEGER DEFAULT 0, tombstone INTEGER DEFAULT 0, name TEXT);
CREATE TABLE schedules_next_date (id TEXT PRIMARY KEY, schedule_id TEXT, local_next_date INTEGER, local_next_date_ts INTEGER, base_next_date INTEGER, base_next_date_ts INTEGER, tombstone INTEGER DEFAULT 0);
CREATE TABLE zero_budgets (id TEXT PRIMARY KEY, month INTEGER, category TEXT, amount INTEGER DEFAULT 0, carryover INTEGER DEFAULT 0);

INSERT INTO accounts VALUES ('a1','Banco',0,0,0,1), ('a2','Inversión',1,0,0,2), ('a3','Borrada',0,0,1,3);
INSERT INTO category_groups VALUES ('g1','Gastos',0,1,0,0), ('g2','Ingresos',1,2,0,0);
INSERT INTO categories VALUES ('c1','Comida',0,'g1',1,0,0), ('c2','Sueldo',1,'g2',2,0,0), ('cOld','Súper',0,'g1',3,1,0);
INSERT INTO category_mapping VALUES ('c1','c1'), ('c2','c2'), ('cOld','c1');
INSERT INTO payees VALUES ('p1','Oxxo',0,NULL), ('p2','Gym',0,NULL), ('pOld','OXXO',1,NULL), ('pT','',0,'a2');
INSERT INTO payee_mapping VALUES ('p1','p1'), ('p2','p2'), ('pOld','p1'), ('pT','pT');
INSERT INTO transactions VALUES
  ('t1',0,0,'a1','c2',2500000,'p1','quincena',20260915,0,1,NULL,NULL),
  ('t2',0,0,'a1','cOld',-12550,'pOld','',20260920,0,0,NULL,NULL),
  ('t3',0,0,'a1','c1',-999,'p1','',20260921,1,1,NULL,NULL),
  ('tp',1,0,'a1',NULL,-30000,'p1','split',20260922,0,1,NULL,NULL),
  ('tc1',0,1,'a1','c1',-10000,'p1','',20260922,0,1,'tp',NULL),
  ('tc2',0,1,'a1','c1',-20000,'p1','',20260922,0,1,'tp',NULL),
  ('t4',0,0,'a1',NULL,-50000,'pT','',20260923,0,1,NULL,NULL),
  ('t5',0,0,'a1','c1',-2500,'p2','',20261001,0,1,NULL,'s1');
INSERT INTO rules VALUES
  ('r1','[{"op":"is","field":"description","value":"p2"},{"op":"is","field":"acct","value":"a1"},{"op":"isapprox","field":"date","value":{"start":"2026-05-11","interval":1,"frequency":"monthly","patterns":[],"skipWeekend":true,"weekendSolveMode":"before","endMode":"never","endOccurrences":1,"endDate":"2026-04-12"}},{"op":"is","field":"amount","value":-2500}]','[]',0),
  ('r2','[{"op":"is","field":"acct","value":"a1"},{"op":"is","field":"date","value":{"start":"2026-01-10","interval":2,"frequency":"weekly","patterns":[],"skipWeekend":false,"weekendSolveMode":"after","endMode":"after_n_occurrences","endOccurrences":3,"endDate":"2026-01-01"}},{"op":"isbetween","field":"amount","value":{"num1":-1000,"num2":-2000}}]','[]',0),
  ('r3','[{"op":"is","field":"date","value":"2026-12-24"},{"op":"is","field":"amount","value":-80000}]','[]',0);
INSERT INTO schedules_next_date VALUES
  ('n1','s1',20261011,5,20261011,5,0),
  ('n2','s2',20260101,1,20260207,2,0);
INSERT INTO zero_budgets VALUES
  ('b1',202609,'c1',30000,0), ('b2',202609,'cOld',5000,0), ('b3',202610,'c1',20000,1),
  ('b4',202609,'c2',0,0), ('b5',202609,'cGone',100,0);
INSERT INTO schedules VALUES ('s1','r1',0,1,0,'Gym'), ('s2','r2',1,0,0,'Quincenal'), ('s3','r3',0,0,0,'Regalo'), ('sDead','r1',0,0,1,'Viejo');"####;

fn fixture() -> PathBuf {
    let path = std::env::temp_dir().join(format!("actual-fixture-{}-{}.sqlite", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    Connection::open(&path).unwrap().execute_batch(FIXTURE_SQL).unwrap();
    path
}
fn day(y: i32, m: u32, d: u32) -> NaiveDate { NaiveDate::from_ymd_opt(y, m, d).unwrap() }

#[test]
fn imports_live_rows_with_mappings_resolved() {
    let path = fixture();
    let before = std::fs::read(&path).unwrap();
    let mut store = Store::open_in_memory().unwrap();
    let summary = store.import_actual(&path).unwrap();
    assert_eq!(summary, ImportSummary { accounts: 2, categories: 2, payees: 2, entries: 6, schedules: 3, budgets: 2 });
    let accounts = store.list_accounts().unwrap();
    assert_eq!(accounts.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), vec!["Banco", "Inversión"]);
    assert!(accounts[1].off_budget);
    assert_eq!(store.account_balances().unwrap()[&accounts[0].id], 2_500_000 - 12_550 - 10_000 - 20_000 - 50_000 - 2_500);
    let groups = store.list_category_groups().unwrap();
    assert_eq!(groups.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), vec!["Gastos", "Ingresos"]);
    assert!(groups[1].is_income);
    let categories = store.list_categories().unwrap();
    assert_eq!(categories.iter().filter(|c| c.group_id == Some(groups[0].id)).map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["Comida"]);
    let payees = store.list_payees().unwrap();
    let entries = store.list_entries().unwrap();
    let merged = entries.iter().find(|e| e.amount == -12_550).unwrap();
    assert_eq!(merged.category_id, Some(categories.iter().find(|c| c.name == "Comida").unwrap().id));
    assert_eq!(merged.payee_id, Some(payees.iter().find(|p| p.name == "Oxxo").unwrap().id));
    assert!(!merged.cleared);
    assert_eq!(merged.date, day(2026, 9, 20));
    let transfer = entries.iter().find(|e| e.amount == -50_000).unwrap();
    assert_eq!((transfer.payee_id, transfer.category_id), (None, None));
    let gym = entries.iter().find(|e| e.amount == -2_500).unwrap();
    assert_eq!(gym.schedule_id, Some(store.list_schedules().unwrap().iter().find(|s| s.name == "Gym").unwrap().id));
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn imports_schedules_with_their_recurrence() {
    let mut store = Store::open_in_memory().unwrap();
    store.import_actual(&fixture()).unwrap();
    let schedules = store.list_schedules().unwrap();
    let gym = schedules.iter().find(|s| s.name == "Gym").unwrap();
    assert_eq!((gym.amount, gym.auto_post, gym.completed), (-2500, true, false));
    assert_eq!(gym.payee_id, Some(store.list_payees().unwrap().iter().find(|p| p.name == "Gym").unwrap().id));
    assert_eq!(gym.account_id, Some(store.list_accounts().unwrap()[0].id));
    let r = gym.recurrence.as_ref().unwrap();
    assert_eq!((r.start, r.frequency, r.interval, r.skip_weekend, r.weekend_mode, r.end_date), (day(2026, 5, 11), Frequency::Monthly, 1, true, WeekendMode::Before, None));
    assert_eq!(gym.next_date, Some(day(2026, 10, 11)));
    let biweekly = schedules.iter().find(|s| s.name == "Quincenal").unwrap();
    assert_eq!(biweekly.next_date, Some(day(2026, 2, 7)));
    assert_eq!(biweekly.amount, -1500);
    assert!(biweekly.completed);
    assert_eq!(biweekly.recurrence.as_ref().unwrap().end_date, Some(day(2026, 2, 7)));
    let one_off = schedules.iter().find(|s| s.name == "Regalo").unwrap().recurrence.as_ref().unwrap();
    assert_eq!(one_off.occurrences(day(2026, 1, 1), day(2027, 12, 31)), vec![day(2026, 12, 24)]);
}

#[test]
fn refuses_to_import_into_a_non_empty_store() {
    let mut store = Store::open_in_memory().unwrap();
    store.insert_account(&Account { id: 0, name: "Existente".into(), off_budget: false, closed: false, sort_order: 0.0 }).unwrap();
    assert!(matches!(store.import_actual(&fixture()), Err(ImportError::StoreNotEmpty)));
    assert_eq!(store.list_accounts().unwrap().len(), 1);
}

#[test]
fn imports_budget_amounts_merging_mapped_categories() {
    let mut store = Store::open_in_memory().unwrap();
    store.import_actual(&fixture()).unwrap();
    let mut amounts = store.list_budget_amounts().unwrap();
    amounts.sort_by_key(|b| b.month);
    assert_eq!(amounts.iter().map(|b| b.month).collect::<Vec<_>>(), vec![202609, 202610]);
    assert_eq!(amounts.iter().map(|b| b.amount).collect::<Vec<_>>(), vec![35000, 20000]);
    assert_eq!(amounts.iter().map(|b| b.carryover).collect::<Vec<_>>(), vec![false, true]);
    let comida = store.list_categories().unwrap().into_iter().find(|c| c.name == "Comida").unwrap();
    assert!(amounts.iter().all(|b| b.category_id == Some(comida.id)));
}

#[test]
fn imports_budgets_alone_into_an_existing_store_by_category_name() {
    let mut store = Store::open_in_memory().unwrap();
    let group = store.insert_category_group(&CategoryGroup { id: 0, name: "Gastos".into(), is_income: false, sort_order: 1.0 }).unwrap();
    store.insert_category(&Category { id: 0, name: "Comida".into(), hidden: false, sort_order: 1.0, group_id: Some(group) }).unwrap();
    let path = fixture();
    assert_eq!(store.import_actual_budgets(&path).unwrap(), 2);
    let mut amounts: Vec<_> = store.list_budget_amounts().unwrap().iter().map(|b| b.amount).collect();
    amounts.sort();
    assert_eq!(amounts, vec![20_000, 35_000]);
    assert!(matches!(store.import_actual_budgets(&path), Err(ImportError::BudgetsNotEmpty)));
    assert_eq!(store.list_budget_amounts().unwrap().len(), 2);
}

#[test]
fn imports_schedule_dates_into_an_existing_store_by_name() {
    let mut store = Store::open_in_memory().unwrap();
    let path = fixture();
    store.import_actual(&path).unwrap();
    store.conn.execute("UPDATE schedule SET next_date=NULL", []).unwrap();
    store.conn.execute("INSERT INTO schedule(name,amount,auto_post,completed) VALUES ('Solo aquí',-1,0,0)", []).unwrap();
    assert_eq!(store.import_actual_schedule_dates(&path).unwrap(), 2);
    let schedules = store.list_schedules().unwrap();
    assert_eq!(schedules.iter().find(|s| s.name == "Gym").unwrap().next_date, Some(day(2026, 10, 11)));
    assert_eq!(schedules.iter().find(|s| s.name == "Quincenal").unwrap().next_date, Some(day(2026, 2, 7)));
    assert_eq!(schedules.iter().find(|s| s.name == "Solo aquí").unwrap().next_date, None);
}

#[test]
fn rollback_and_filter_dangling_children_and_fallback_names() {
    let path = fixture();
    let source = Connection::open(&path).unwrap();
    source.execute_batch("INSERT INTO transactions VALUES ('dead-parent',1,0,'a1','c1',-1,'p1','',20260920,1,1,NULL,NULL), ('orphan-child',0,1,'a1','c1',-7,'p1','',20260920,0,1,'dead-parent',NULL), ('missing-parent',0,1,'a1','c1',-8,'p1','',20260920,0,1,'absent',NULL); UPDATE schedules SET name=NULL WHERE id IN ('s1','s2');").unwrap();
    drop(source);
    let mut store = Store::open_in_memory().unwrap();
    let result = store.import_actual(&path).unwrap();
    assert_eq!(result.entries, 6);
    let schedules = store.list_schedules().unwrap();
    assert!(schedules.iter().any(|s| s.name == "Gym"));
    assert!(schedules.iter().any(|s| s.name == "Sin nombre"));
    let broken = fixture();
    Connection::open(&broken).unwrap().execute_batch("ALTER TABLE zero_budgets RENAME TO missing_budgets").unwrap();
    let mut empty = Store::open_in_memory().unwrap();
    assert!(empty.import_actual(&broken).is_err());
    assert!(empty.list_accounts().unwrap().is_empty());
    assert!(empty.list_categories().unwrap().is_empty());
    assert!(empty.list_entries().unwrap().is_empty());
}

#[test]
fn normalizes_out_of_range_calendar_components_like_swift() {
    assert_eq!(date_int(20260231), Some(day(2026, 3, 3)));
    assert_eq!(date_iso("2026-02-31"), Some(day(2026, 3, 3)));
}

#[test]
fn schedule_dates_match_first_duplicate_and_budgets_refuse_any_existing_row() {
    let path = fixture();
    let mut store = Store::open_in_memory().unwrap();
    store.conn.execute("INSERT INTO schedule(name,amount,auto_post,completed) VALUES ('Gym',0,0,0),('Gym',0,0,0)", []).unwrap();
    assert_eq!(store.import_actual_schedule_dates(&path).unwrap(), 1);
    let schedules = store.list_schedules().unwrap();
    assert_eq!(schedules[0].next_date, Some(day(2026, 10, 11)));
    assert_eq!(schedules[1].next_date, None);
    store.conn.execute("INSERT INTO budget_amount(month,amount,carryover,category_id) VALUES (202601,0,0,NULL)", []).unwrap();
    assert!(matches!(store.import_actual_budgets(&path), Err(ImportError::BudgetsNotEmpty)));
}
