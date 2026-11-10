use std::path::{Path, PathBuf};
use super::{ImportError, ImportSummary, Store};

use crate::models::{Frequency, Id, Recurrence, WeekendMode};
use chrono::{Duration, NaiveDate};
use rusqlite::{backup::Backup, params, Connection, OpenFlags, TransactionBehavior};
use serde_json::Value;
use std::collections::HashMap;

type IdMap = HashMap<String, Id>;
const NEXT_DATE: &str = "CASE WHEN nd.local_next_date_ts = nd.base_next_date_ts THEN nd.local_next_date ELSE nd.base_next_date END";

fn snapshot(path: &Path) -> Result<Connection, ImportError> {
    let source = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    let mut copy = Connection::open_in_memory()?;
    {
        let backup = Backup::new(&source, &mut copy)?;
        backup.run_to_completion(128, std::time::Duration::from_millis(10), None)?;
    }
    Ok(copy)
}

fn date_components(year: i64, month: i64, day: i64) -> Option<NaiveDate> {
    let months = year.checked_mul(12)?.checked_add(month.checked_sub(1)?)?;
    let year = months.div_euclid(12).try_into().ok()?;
    let month = (months.rem_euclid(12) + 1) as u32;
    NaiveDate::from_ymd_opt(year, month, 1)?.checked_add_signed(Duration::try_days(day.checked_sub(1)?)?)
}
fn date_int(value: i64) -> Option<NaiveDate> {
    date_components(value / 10000, value / 100 % 100, value % 100)
}
fn date_iso(value: &str) -> Option<NaiveDate> {
    let parts: Vec<i64> = value.split('-').filter_map(|v| v.parse().ok()).collect();
    if parts.len() != 3 { return None; }
    date_components(parts[0], parts[1], parts[2])
}

#[derive(Default)]
struct Rule {
    payee: Option<String>,
    account: Option<String>,
    amount: i64,
    recurrence: Option<Recurrence>,
}
fn parse_rule(json: &str) -> Rule {
    let mut rule = Rule::default();
    let Ok(Value::Array(conditions)) = serde_json::from_str(json) else { return rule };
    for condition in conditions {
        let value = &condition["value"];
        match condition["field"].as_str() {
            Some("description") => rule.payee = value.as_str().map(str::to_owned),
            Some("acct") => rule.account = value.as_str().map(str::to_owned),
            Some("amount") => {
                if let Some(n) = value.as_i64() { rule.amount = n; }
                else if let (Some(a), Some(b)) = (value["num1"].as_i64(), value["num2"].as_i64()) {
                    rule.amount = ((i128::from(a) + i128::from(b)) / 2) as i64;
                }
            }
            Some("date") => {
                if let Some(date) = value.as_str().and_then(date_iso) {
                    rule.recurrence = Some(Recurrence { start: date, frequency: Frequency::Monthly, interval: 1, skip_weekend: false, weekend_mode: WeekendMode::After, end_date: Some(date) });
                } else if value.is_object() { rule.recurrence = parse_recurrence(value); }
            }
            _ => {}
        }
    }
    rule
}
fn parse_recurrence(value: &Value) -> Option<Recurrence> {
    let start = date_iso(value["start"].as_str()?)?;
    let frequency = match value["frequency"].as_str()? {
        "daily" => Frequency::Daily, "weekly" => Frequency::Weekly,
        "monthly" => Frequency::Monthly, "yearly" => Frequency::Yearly, _ => return None,
    };
    let interval = value["interval"].as_i64().unwrap_or(1).max(1).try_into().unwrap_or(u32::MAX);
    let mut recurrence = Recurrence { start, frequency, interval, skip_weekend: value["skipWeekend"].as_bool().unwrap_or(false), weekend_mode: if value["weekendSolveMode"].as_str() == Some("before") { WeekendMode::Before } else { WeekendMode::After }, end_date: None };
    match value["endMode"].as_str() {
        Some("on_date") => recurrence.end_date = value["endDate"].as_str().and_then(date_iso),
        Some("after_n_occurrences") => {
            let mut raw = recurrence.clone(); raw.skip_weekend = false;
            let n = value["endOccurrences"].as_i64().unwrap_or(1).max(1);
            let mut last = start;
            for _ in 1..n {
                let Some(next) = last.checked_add_signed(Duration::days(1)).and_then(|d| raw.next_on_or_after(d)) else { break };
                last = next;
            }
            recurrence.end_date = Some(last);
        }
        _ => {}
    }
    Some(recurrence)
}
fn mapped(ids: &IdMap, id: Option<String>) -> Option<Id> { id.and_then(|id| ids.get(&id).copied()) }

fn import_budgets(source: &Connection, target: &Connection, categories: &IdMap) -> Result<usize, ImportError> {
    let mut rows_by_key: HashMap<(i64, Id), (i64, bool)> = HashMap::new();
    let mut stmt = source.prepare("SELECT b.month, cm.transferId, IFNULL(b.amount,0), IFNULL(b.carryover,0) FROM zero_budgets b LEFT JOIN category_mapping cm ON cm.id=b.category WHERE IFNULL(b.amount,0) != 0 OR IFNULL(b.carryover,0) != 0")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let Some(category) = mapped(categories, row.get(1)?) else { continue };
        let entry = rows_by_key.entry((row.get::<_, Option<i64>>(0)?.unwrap_or(0), category)).or_default();
        entry.0 += row.get::<_, i64>(2)?;
        entry.1 |= row.get::<_, i64>(3)? != 0;
    }
    for ((month, category), (amount, carryover)) in &rows_by_key {
        target.execute("INSERT INTO budget_amount(month, category_id, amount, carryover) VALUES (?1,?2,?3,?4)", params![month, category, amount, carryover])?;
    }
    Ok(rows_by_key.len())
}

impl Store {
    pub fn import_actual_budgets(&mut self, db: &Path) -> Result<usize, ImportError> {
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.query_row("SELECT EXISTS(SELECT 1 FROM budget_amount)", [], |r| r.get::<_, bool>(0))? { return Err(ImportError::BudgetsNotEmpty); }
        let source = snapshot(db)?;
        let mut by_name = IdMap::new();
        {
            let mut stmt = tx.prepare("SELECT c.id, IFNULL(g.name,'') || '/' || c.name FROM category c LEFT JOIN category_group g ON g.id=c.group_id ORDER BY c.id")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? { by_name.entry(r.get(1)?).or_insert(r.get(0)?); }
        }
        let mut categories = IdMap::new();
        {
            let mut stmt = source.prepare("SELECT c.id, IFNULL(g.name,'') || '/' || c.name FROM categories c LEFT JOIN category_groups g ON g.id=c.cat_group WHERE IFNULL(c.tombstone,0)=0")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                let name: Option<String> = r.get(1)?;
                if let Some(id) = name.and_then(|n| by_name.get(&n).copied()) { categories.insert(r.get::<_, Option<String>>(0)?.unwrap_or_default(), id); }
            }
        }
        let count = import_budgets(&source, &tx, &categories)?;
        tx.commit()?;
        Ok(count)
    }
    pub fn import_actual_schedule_dates(&mut self, db: &Path) -> Result<usize, ImportError> {
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let source = snapshot(db)?;
        let mut by_name = IdMap::new();
        {
            let mut stmt = tx.prepare("SELECT id,name FROM schedule ORDER BY id")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? { by_name.entry(r.get(1)?).or_insert(r.get(0)?); }
        }
        let mut count = 0;
        {
            let sql = format!("SELECT s.name, {NEXT_DATE} FROM schedules s JOIN schedules_next_date nd ON nd.schedule_id=s.id AND IFNULL(nd.tombstone,0)=0 WHERE IFNULL(s.tombstone,0)=0 AND s.name IS NOT NULL");
            let mut stmt = source.prepare(&sql)?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                let name: String = r.get(0)?;
                let date = r.get::<_, Option<i64>>(1)?.and_then(date_int);
                if let (Some(id), Some(date)) = (by_name.get(&name), date) {
                    tx.execute("UPDATE schedule SET next_date=?1 WHERE id=?2", params![date.to_string(), id])?;
                    count += 1;
                }
            }
        }
        tx.commit()?;
        Ok(count)
    }

    pub fn import_actual(&mut self, db: &Path) -> Result<ImportSummary, ImportError> {
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let nonempty: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM account) OR EXISTS(SELECT 1 FROM entry)", [], |r| r.get(0))?;
        if nonempty { return Err(ImportError::StoreNotEmpty); }
        let source = snapshot(db)?;
        let mut summary = ImportSummary::default();
        let (mut accounts, mut groups, mut categories, mut payees, mut schedules) = (IdMap::new(), IdMap::new(), IdMap::new(), IdMap::new(), IdMap::new());
        {
            let mut stmt = source.prepare("SELECT id, IFNULL(name,''), IFNULL(offbudget,0), IFNULL(closed,0), IFNULL(sort_order,0) FROM accounts WHERE IFNULL(tombstone,0)=0")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                tx.execute("INSERT INTO account(name,off_budget,closed,sort_order) VALUES (?1,?2,?3,?4)", params![r.get::<_, String>(1)?, r.get::<_, i64>(2)? != 0, r.get::<_, i64>(3)? != 0, r.get::<_, f64>(4)?])?;
                accounts.insert(r.get::<_, Option<String>>(0)?.unwrap_or_default(), tx.last_insert_rowid());
            }
        }
        {
            let mut stmt = source.prepare("SELECT id, IFNULL(name,''), IFNULL(is_income,0), IFNULL(sort_order,0) FROM category_groups WHERE IFNULL(tombstone,0)=0")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                tx.execute("INSERT INTO category_group(name,is_income,sort_order) VALUES (?1,?2,?3)", params![r.get::<_, String>(1)?, r.get::<_, i64>(2)? != 0, r.get::<_, f64>(3)?])?;
                groups.insert(r.get::<_, Option<String>>(0)?.unwrap_or_default(), tx.last_insert_rowid());
            }
        }
        {
            let mut stmt = source.prepare("SELECT id, IFNULL(name,''), IFNULL(hidden,0), cat_group, IFNULL(sort_order,0) FROM categories WHERE IFNULL(tombstone,0)=0")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                tx.execute("INSERT INTO category(name,hidden,group_id,sort_order) VALUES (?1,?2,?3,?4)", params![r.get::<_, String>(1)?, r.get::<_, i64>(2)? != 0, mapped(&groups, r.get(3)?), r.get::<_, f64>(4)?])?;
                categories.insert(r.get::<_, Option<String>>(0)?.unwrap_or_default(), tx.last_insert_rowid());
            }
        }
        {
            let mut stmt = source.prepare("SELECT id, IFNULL(name,'') FROM payees WHERE IFNULL(tombstone,0)=0 AND transfer_acct IS NULL")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                tx.execute("INSERT INTO payee(name) VALUES (?1)", [r.get::<_, String>(1)?])?;
                payees.insert(r.get::<_, Option<String>>(0)?.unwrap_or_default(), tx.last_insert_rowid());
            }
        }
        let mut payee_mapping = HashMap::<String, String>::new();
        {
            let mut stmt = source.prepare("SELECT id, targetId FROM payee_mapping")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? { payee_mapping.insert(r.get::<_, Option<String>>(0)?.unwrap_or_default(), r.get::<_, Option<String>>(1)?.unwrap_or_default()); }
        }
        {
            let sql = format!("SELECT s.id, s.name, IFNULL(s.completed,0), IFNULL(s.posts_transaction,0), IFNULL(r.conditions,''), {NEXT_DATE} FROM schedules s JOIN rules r ON r.id=s.rule LEFT JOIN schedules_next_date nd ON nd.schedule_id=s.id AND IFNULL(nd.tombstone,0)=0 WHERE IFNULL(s.tombstone,0)=0");
            let mut stmt = source.prepare(&sql)?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                let rule = parse_rule(&r.get::<_, String>(4)?);
                let Some(recurrence) = rule.recurrence else { continue };
                let payee = rule.payee.as_ref().and_then(|id| payees.get(payee_mapping.get(id).unwrap_or(id))).copied();
                let name = match r.get::<_, Option<String>>(1)? {
                    Some(name) => name,
                    None => match payee { Some(id) => tx.query_row("SELECT name FROM payee WHERE id=?1", [id], |row| row.get(0))?, None => "Sin nombre".to_owned() },
                };
                let recurrence = serde_json::to_string(&recurrence).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let date = r.get::<_, Option<i64>>(5)?.and_then(date_int).map(|d| d.to_string());
                tx.execute("INSERT INTO schedule(name,amount,auto_post,completed,next_date,recurrence,account_id,payee_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)", params![name, rule.amount, r.get::<_, i64>(3)? != 0, r.get::<_, i64>(2)? != 0, date, recurrence, mapped(&accounts, rule.account), payee])?;
                schedules.insert(r.get::<_, Option<String>>(0)?.unwrap_or_default(), tx.last_insert_rowid());
            }
        }
        {
            let mut stmt = source.prepare("SELECT t.acct,cm.transferId,IFNULL(t.amount,0),pm.targetId,IFNULL(t.notes,''),t.date,IFNULL(t.cleared,0),t.schedule FROM transactions t LEFT JOIN category_mapping cm ON cm.id=t.category LEFT JOIN payee_mapping pm ON pm.id=t.description LEFT JOIN transactions parent ON t.isChild=1 AND parent.id=t.parent_id WHERE IFNULL(t.tombstone,0)=0 AND IFNULL(t.isParent,0)=0 AND t.date IS NOT NULL AND t.acct IS NOT NULL AND (IFNULL(t.isChild,0)=0 OR IFNULL(parent.tombstone,1)=0)")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                let Some(account) = mapped(&accounts, r.get(0)?) else { continue };
                let Some(date) = date_int(r.get(5)?) else { continue };
                tx.execute("INSERT INTO entry(account_id,category_id,amount,payee_id,notes,date,cleared,schedule_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)", params![account, mapped(&categories, r.get(1)?), r.get::<_, i64>(2)?, mapped(&payees, r.get(3)?), r.get::<_, String>(4)?, date.to_string(), r.get::<_, i64>(6)? != 0, mapped(&schedules, r.get(7)?)])?;
                summary.entries += 1;
            }
        }
        summary.budgets = import_budgets(&source, &tx, &categories)?;
        summary.accounts = accounts.len(); summary.categories = categories.len(); summary.payees = payees.len(); summary.schedules = schedules.len();
        tx.commit()?;
        Ok(summary)
    }
}

#[cfg(test)]
#[path = "actual_tests.rs"]
mod importer_tests;

pub fn extract_actual(path: &Path, scratch: &Path) -> Result<PathBuf, ImportError> {
    use std::io::{Read, Seek};
    let mut file = std::fs::File::open(path)?;
    let mut header = [0; 16];
    if file.read_exact(&mut header).is_ok() && &header == b"SQLite format 3\0" {
        return Ok(path.to_owned());
    }
    file.rewind()?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut entry = archive.by_name("db.sqlite")?;
    let output = scratch.join("db.sqlite");
    let mut destination = std::fs::OpenOptions::new().write(true).create_new(true).open(&output)?;
    std::io::copy(&mut entry, &mut destination)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, time::{SystemTime, UNIX_EPOCH}};

    fn scratch() -> PathBuf {
        let path = std::env::temp_dir().join(format!("actual-extract-{}-{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn extract_actual_raw_sqlite() {
        let dir = scratch();
        let path = dir.join("source.sqlite");
        std::fs::write(&path, b"SQLite format 3\0test").unwrap();
        assert_eq!(extract_actual(&path, &dir).unwrap(), path);
    }

    #[test]
    fn extract_actual_zip() {
        let dir = scratch();
        let path = dir.join("source.zip");
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("db.sqlite", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"SQLite format 3\0test").unwrap();
        zip.finish().unwrap();
        let extracted = extract_actual(&path, &dir).unwrap();
        assert_eq!(extracted, dir.join("db.sqlite"));
        assert_eq!(std::fs::read(extracted).unwrap(), b"SQLite format 3\0test");
    }
}
