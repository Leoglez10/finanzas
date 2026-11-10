use super::Store;
use crate::models::{Id, Recurrence};
use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
use rusqlite::{params, backup::Backup, Connection, OpenFlags, Row};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub accounts: usize,
    pub categories: usize,
    pub payees: usize,
    pub entries: usize,
    pub schedules: usize,
    pub budgets: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("target store is not empty")]
    StoreNotEmpty,
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
}

// Core Data dates are seconds relative to 2001-01-01 UTC, including fractions.
fn date_in<T: TimeZone>(seconds: f64, timezone: &T) -> Option<NaiveDate> {
    if !seconds.is_finite() { return None; }
    let unix = seconds + 978_307_200.0;
    let whole = unix.floor();
    if whole < i64::MIN as f64 || whole >= i64::MAX as f64 { return None; }
    let nanos = ((unix - whole) * 1_000_000_000.0).round() as u32;
    let seconds = (whole as i64).checked_add(i64::from(nanos / 1_000_000_000))?;
    let instant = DateTime::<Utc>::from_timestamp(seconds, nanos % 1_000_000_000)?;
    Some(instant.with_timezone(timezone).date_naive())
}

fn recurrence<T: TimeZone>(blob: Option<Vec<u8>>, timezone: &T) -> Option<Recurrence> {
    let mut value: serde_json::Value = serde_json::from_slice(&blob?).ok()?;
    for key in ["start", "endDate"] {
        if let Some(date) = value.get(key) {
            if date.is_null() && key == "endDate" { continue; }
            let converted = date_in(date.as_f64()?, timezone)?;
            value[key] = serde_json::Value::String(converted.to_string());
        } else if key == "start" { return None; }
    }
    serde_json::from_value(value).ok()
}

type IdMap = HashMap<Id, Id>;
fn mapped(row: &Row<'_>, index: usize, ids: &IdMap) -> rusqlite::Result<Option<Id>> {
    Ok(row.get::<_, Option<Id>>(index)?.and_then(|id| ids.get(&id).copied()))
}
fn remember(row: &Row<'_>, ids: &mut IdMap, conn: &Connection) -> rusqlite::Result<()> {
    if let Some(id) = row.get::<_, Option<Id>>(0)? { ids.insert(id, conn.last_insert_rowid()); }
    Ok(())
}
fn source_date(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<String>> {
    row.get::<_, Option<f64>>(index)?.map(|s| date_in(s, &Local).map(|d| d.to_string()).ok_or(rusqlite::Error::InvalidQuery)).transpose()
}

impl Store {
    pub fn import_swiftdata(&mut self, path: &Path) -> Result<ImportSummary, ImportError> {
        // Check under the same write transaction as the import to avoid racing a writer.
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let nonempty: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM account) OR EXISTS(SELECT 1 FROM entry)", [], |r| r.get(0))?;
        if nonempty { return Err(ImportError::StoreNotEmpty); }
        let source = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        let mut snapshot = Connection::open_in_memory()?;
        {
            let backup = Backup::new(&source, &mut snapshot)?;
            backup.run_to_completion(128, std::time::Duration::from_millis(10), None)?;
        }
        drop(source);
        let mut summary = ImportSummary::default();
        let (mut accounts, mut groups, mut categories, mut payees, mut schedules) = (IdMap::new(), IdMap::new(), IdMap::new(), IdMap::new(), IdMap::new());
        {
            let mut stmt = snapshot.prepare("SELECT Z_PK,ZNAME,ZOFFBUDGET,ZCLOSED,ZSORTORDER FROM ZACCOUNT")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                tx.execute("INSERT INTO account(name,off_budget,closed,sort_order) VALUES (?1,?2,?3,?4)", params![r.get::<_, Option<String>>(1)?.unwrap_or_default(), r.get::<_, Option<bool>>(2)?.unwrap_or_default(), r.get::<_, Option<bool>>(3)?.unwrap_or_default(), r.get::<_, Option<f64>>(4)?.unwrap_or_default()])?;
                remember(r, &mut accounts, &tx)?; summary.accounts += 1;
            }
        }
        {
            let mut stmt = snapshot.prepare("SELECT Z_PK,ZNAME,ZISINCOME,ZSORTORDER FROM ZCATEGORYGROUP")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                tx.execute("INSERT INTO category_group(name,is_income,sort_order) VALUES (?1,?2,?3)", params![r.get::<_, Option<String>>(1)?.unwrap_or_default(),r.get::<_, Option<bool>>(2)?.unwrap_or_default(),r.get::<_, Option<f64>>(3)?.unwrap_or_default()])?;
                remember(r, &mut groups, &tx)?;
            }
        }
        {
            let mut stmt = snapshot.prepare("SELECT Z_PK,ZNAME,ZHIDDEN,ZSORTORDER,ZGROUP FROM ZCATEGORY")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                tx.execute("INSERT INTO category(name,hidden,sort_order,group_id) VALUES (?1,?2,?3,?4)", params![r.get::<_, Option<String>>(1)?.unwrap_or_default(),r.get::<_, Option<bool>>(2)?.unwrap_or_default(),r.get::<_, Option<f64>>(3)?.unwrap_or_default(),mapped(r,4,&groups)?])?;
                remember(r, &mut categories, &tx)?; summary.categories += 1;
            }
        }
        {
            let mut stmt = snapshot.prepare("SELECT Z_PK,ZNAME FROM ZPAYEE")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                tx.execute("INSERT INTO payee(name) VALUES (?1)", [r.get::<_, Option<String>>(1)?.unwrap_or_default()])?;
                remember(r, &mut payees, &tx)?; summary.payees += 1;
            }
        }
        {
            let mut stmt = snapshot.prepare("SELECT Z_PK,ZNAME,ZAMOUNT,ZAUTOPOST,ZCOMPLETED,ZACCOUNT,ZCATEGORY,ZPAYEE,ZRECURRENCEDATA,ZNEXTDATE FROM ZSCHEDULE")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                let rule = recurrence(r.get(8)?, &Local).map(|v| serde_json::to_string(&v).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))).transpose()?;
                tx.execute("INSERT INTO schedule(name,amount,auto_post,completed,account_id,category_id,payee_id,recurrence,next_date) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![r.get::<_, Option<String>>(1)?.unwrap_or_default(),r.get::<_, Option<i64>>(2)?.unwrap_or_default(),r.get::<_, Option<bool>>(3)?.unwrap_or_default(),r.get::<_, Option<bool>>(4)?.unwrap_or_default(),mapped(r,5,&accounts)?,mapped(r,6,&categories)?,mapped(r,7,&payees)?,rule,source_date(r,9)?])?;
                remember(r, &mut schedules, &tx)?; summary.schedules += 1;
            }
        }
        {
            let mut stmt = snapshot.prepare("SELECT Z_PK,ZAMOUNT,ZCLEARED,ZACCOUNT,ZCATEGORY,ZPAYEE,ZSCHEDULE,ZDATE,ZNOTES FROM ZLEDGERENTRY")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                // The destination requires a date; a missing source date uses the Core Data epoch.
                let date = source_date(r,7)?.unwrap_or_else(|| date_in(0.0, &Local).expect("valid epoch").to_string());
                tx.execute("INSERT INTO entry(amount,cleared,account_id,category_id,payee_id,schedule_id,date,notes) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)", params![r.get::<_, Option<i64>>(1)?.unwrap_or_default(),r.get::<_, Option<bool>>(2)?.unwrap_or_default(),mapped(r,3,&accounts)?,mapped(r,4,&categories)?,mapped(r,5,&payees)?,mapped(r,6,&schedules)?,date,r.get::<_, Option<String>>(8)?.unwrap_or_default()])?;
                summary.entries += 1;
            }
        }
        {
            let mut stmt = snapshot.prepare("SELECT Z_PK,ZAMOUNT,ZCARRYOVER,ZMONTH,ZCATEGORY FROM ZBUDGETAMOUNT")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                tx.execute("INSERT INTO budget_amount(amount,carryover,month,category_id) VALUES (?1,?2,?3,?4)", params![r.get::<_, Option<i64>>(1)?.unwrap_or_default(),r.get::<_, Option<bool>>(2)?.unwrap_or_default(),r.get::<_, Option<i32>>(3)?.unwrap_or_default(),mapped(r,4,&categories)?])?;
                summary.budgets += 1;
            }
        }
        tx.commit()?;
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, NaiveDate};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("finanzas-test-{}-{}.store", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        let source = Connection::open(&path).unwrap();
        source.execute_batch("CREATE TABLE ZACCOUNT(Z_PK, ZNAME, ZOFFBUDGET, ZCLOSED, ZSORTORDER);
            CREATE TABLE ZCATEGORYGROUP(Z_PK,ZNAME,ZISINCOME,ZSORTORDER);
            CREATE TABLE ZCATEGORY(Z_PK,ZNAME,ZHIDDEN,ZSORTORDER,ZGROUP);
            CREATE TABLE ZPAYEE(Z_PK,ZNAME);
            CREATE TABLE ZLEDGERENTRY(Z_PK,ZAMOUNT,ZCLEARED,ZACCOUNT,ZCATEGORY,ZPAYEE,ZSCHEDULE,ZDATE,ZNOTES);
            CREATE TABLE ZSCHEDULE(Z_PK,ZNAME,ZAMOUNT,ZAUTOPOST,ZCOMPLETED,ZACCOUNT,ZCATEGORY,ZPAYEE,ZRECURRENCEDATA,ZNEXTDATE);
            CREATE TABLE ZBUDGETAMOUNT(Z_PK,ZAMOUNT,ZCARRYOVER,ZMONTH,ZCATEGORY);
            INSERT INTO ZACCOUNT VALUES (20,'Bank',NULL,NULL,1);
            INSERT INTO ZCATEGORYGROUP VALUES (30,'Food',NULL,1);
            INSERT INTO ZCATEGORY VALUES (40,'Groceries',NULL,1,30), (41,NULL,NULL,NULL,999);
            INSERT INTO ZPAYEE VALUES (50,'Shop');
            INSERT INTO ZSCHEDULE VALUES (60,'Rent',-100,1,NULL,20,40,50,CAST('{\"start\":801036000,\"frequency\":\"monthly\",\"interval\":1,\"skipWeekend\":false,\"weekendMode\":\"after\",\"endDate\":801036000}' AS BLOB),801036000);
            INSERT INTO ZSCHEDULE VALUES (61,NULL,NULL,NULL,NULL,999,NULL,NULL,X'00',NULL);
            INSERT INTO ZLEDGERENTRY VALUES (70,-100,1,20,40,50,60,801036000.5,NULL), (71,NULL,NULL,999,999,999,999,NULL,NULL);
            INSERT INTO ZBUDGETAMOUNT VALUES (80,500,1,202605,40);").unwrap();
        drop(source);
        path
    }

    #[test]
    fn imports_synthetic_store_without_changing_source() {
        let path = fixture();
        let before = std::fs::read(&path).unwrap();
        let mut store = Store::open_in_memory().unwrap();
        let summary = store.import_swiftdata(&path).unwrap();
        assert_eq!(summary, ImportSummary { accounts: 1, categories: 2, payees: 1, entries: 2, schedules: 2, budgets: 1 });
        assert_eq!(serde_json::to_value(&summary).unwrap()["accounts"], 1);
        let account = &store.list_accounts().unwrap()[0];
        assert_eq!(account.name, "Bank"); assert!(!account.off_budget);
        let group = &store.list_category_groups().unwrap()[0];
        let categories = store.list_categories().unwrap();
        let category = categories.iter().find(|c| c.name == "Groceries").unwrap();
        assert_eq!(category.group_id, Some(group.id));
        assert_eq!(categories.iter().find(|c| c.name.is_empty()).unwrap().group_id, None);
        let payee = &store.list_payees().unwrap()[0];
        let schedules = store.list_schedules().unwrap();
        let schedule = schedules.iter().find(|s| s.name == "Rent").unwrap();
        assert_eq!((schedule.account_id, schedule.category_id, schedule.payee_id), (Some(account.id),Some(category.id),Some(payee.id)));
        assert_eq!(schedule.next_date, date_in(801036000.0, &Local));
        let rule = schedule.recurrence.as_ref().unwrap();
        assert_eq!(rule.start, date_in(801036000.0, &Local).unwrap());
        assert_eq!(rule.end_date, Some(rule.start));
        assert_eq!(rule.frequency, crate::models::Frequency::Monthly);
        assert!(schedules.iter().find(|s| s.name.is_empty()).unwrap().recurrence.is_none());
        let entries = store.list_entries().unwrap();
        let entry = entries.iter().find(|e| e.amount == -100).unwrap();
        assert_eq!((entry.account_id,entry.category_id,entry.payee_id,entry.schedule_id), (Some(account.id),Some(category.id),Some(payee.id),Some(schedule.id)));
        assert_eq!(entry.date, date_in(801036000.5, &Local).unwrap()); assert_eq!(entry.notes, "");
        let dangling = entries.iter().find(|e| e.amount == 0).unwrap();
        assert_eq!((dangling.account_id,dangling.category_id,dangling.payee_id,dangling.schedule_id), (None,None,None,None));
        assert_eq!(store.list_budget_amounts().unwrap()[0].category_id, Some(category.id));
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn fixed_offset_conversion_and_invalid_recurrence() {
        let tz = FixedOffset::west_opt(6*3600).unwrap();
        assert_eq!(date_in(0.0, &tz), NaiveDate::from_ymd_opt(2000,12,31));
        assert_eq!(date_in(21600.5, &tz), NaiveDate::from_ymd_opt(2001,1,1));
        assert!(date_in(f64::NAN, &tz).is_none());
        assert!(recurrence(Some(b"{}".to_vec()), &tz).is_none());
        let rule = recurrence(Some(br#"{"start":21600,"frequency":"monthly","interval":1}"#.to_vec()), &tz).unwrap();
        assert_eq!(rule.start, NaiveDate::from_ymd_opt(2001,1,1).unwrap());
    }

    #[test]
    fn refuses_nonempty_and_rolls_back_on_error() {
        let path = fixture();
        let mut store = Store::open_in_memory().unwrap();
        store.insert_account(&crate::models::Account { id: 0, name: "existing".into(), off_budget: false, closed: false, sort_order: 0.0 }).unwrap();
        assert!(matches!(store.import_swiftdata(&path), Err(ImportError::StoreNotEmpty)));
        assert_eq!(store.list_accounts().unwrap().len(), 1);
        assert!(store.list_categories().unwrap().is_empty());
        let bad = std::env::temp_dir().join(format!("finanzas-bad-{}-{}.store", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        let source = Connection::open(&bad).unwrap();
        source.execute_batch("CREATE TABLE ZACCOUNT(Z_PK,ZNAME,ZOFFBUDGET,ZCLOSED,ZSORTORDER); INSERT INTO ZACCOUNT VALUES(1,'partial',0,0,0)").unwrap();
        drop(source);
        let mut empty = Store::open_in_memory().unwrap();
        assert!(empty.import_swiftdata(&bad).is_err());
        assert!(empty.list_accounts().unwrap().is_empty());
        let entry = crate::models::Entry { id: 0, date: NaiveDate::from_ymd_opt(2020,1,1).unwrap(), amount: 1, notes: String::new(), cleared: false, account_id: None, category_id: None, payee_id: None, schedule_id: None };
        empty.insert_entry(&entry).unwrap();
        assert!(matches!(empty.import_swiftdata(&path), Err(ImportError::StoreNotEmpty)));
        assert_eq!(empty.list_entries().unwrap().len(), 1);
        assert!(empty.list_accounts().unwrap().is_empty());
    }
}
