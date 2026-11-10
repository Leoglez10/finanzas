//! SQLite storage.

mod crud;
mod records;
mod entries;
pub use entries::{EntryFilter, EntryRow};
mod payees;
mod schema;
mod swiftdata;
mod actual;
pub use actual::extract_actual;
pub use swiftdata::{ImportError, ImportSummary};

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::models::{Account, Category, CategoryGroup, Cents, Entry, Id, MonthKey};
use rusqlite::{params, Connection, OpenFlags, Result};

#[derive(Debug)]
pub struct ForeignDatabase(pub PathBuf);

impl std::fmt::Display for ForeignDatabase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not a Finanzas database: {}", self.0.display())
    }
}

impl std::error::Error for ForeignDatabase {}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn transaction<T, E: From<rusqlite::Error>>(&mut self, operation: impl FnOnce(&Store) -> std::result::Result<T, E>) -> std::result::Result<T, E> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match operation(self) {
            Ok(value) => { self.conn.execute_batch("COMMIT")?; Ok(value) }
            Err(error) => { self.conn.execute_batch("ROLLBACK")?; Err(error) }
        }
    }
    pub fn default_path() -> Option<std::path::PathBuf> {
        std::env::var_os("FINANZAS_DB").map(std::path::PathBuf::from)
            .or_else(|| dirs::data_dir().map(|p| p.join("Finanzas").join("finanzas.db")))
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        }
        match std::fs::metadata(path) {
            Ok(metadata) if metadata.len() > 0 => {
                let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
                let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
                let (has_tables, has_core_data): (bool, bool) = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table'), EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND upper(name) IN ('Z_METADATA', 'Z_PRIMARYKEY'))",
                    [], |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                if has_core_data || (version == 0 && has_tables) {
                    return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(ForeignDatabase(path.to_owned()))));
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(error))),
        }
        Self::configure(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::configure(Connection::open_in_memory()?)
    }

    fn configure(mut conn: Connection) -> Result<Self> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        schema::migrate(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn envelope_inputs(&self) -> Result<crate::domain::envelope::EnvelopeInputs> {
        let mut stmt = self.conn.prepare("SELECT c.id, COALESCE(g.is_income, 0) FROM category c LEFT JOIN category_group g ON g.id = c.group_id ORDER BY c.sort_order, c.name, c.id")?;
        let categories = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<Vec<_>>>()?;
        let (budgeted, carryover) = self.budget_rows()?;
        Ok(crate::domain::envelope::EnvelopeInputs { categories, budgeted, carryover, activity: self.activity_by_month_category()? })
    }

    pub fn first_run_checks(&self) -> Result<(bool, bool, bool)> {
        self.conn.query_row("SELECT EXISTS(SELECT 1 FROM account WHERE closed=0), EXISTS(SELECT 1 FROM entry e JOIN category c ON c.id=e.category_id JOIN category_group g ON g.id=c.group_id WHERE e.amount>0 AND g.is_income=1), EXISTS(SELECT 1 FROM budget_amount WHERE amount>0)", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
    }

    pub fn data_version(&self) -> Result<i64> {
        self.conn.pragma_query_value(None, "data_version", |row| row.get(0))
    }

    pub fn insert_account(&self, value: &Account) -> Result<Id> {
        self.conn.execute("INSERT INTO account(name, off_budget, closed, sort_order) VALUES (?1, ?2, ?3, ?4)", params![value.name, value.off_budget, value.closed, value.sort_order])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_category_group(&self, value: &CategoryGroup) -> Result<Id> {
        self.conn.execute("INSERT INTO category_group(name, is_income, sort_order) VALUES (?1, ?2, ?3)", params![value.name, value.is_income, value.sort_order])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_category(&self, value: &Category) -> Result<Id> {
        self.conn.execute("INSERT INTO category(name, hidden, sort_order, group_id) VALUES (?1, ?2, ?3, ?4)", params![value.name, value.hidden, value.sort_order, value.group_id])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_entry(&self, value: &Entry) -> Result<Id> {
        self.conn.execute("INSERT INTO entry(date, amount, notes, cleared, account_id, category_id, payee_id, schedule_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", params![value.date.to_string(), value.amount, value.notes, value.cleared, value.account_id, value.category_id, value.payee_id, value.schedule_id])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn uncategorized_income(&self, month: MonthKey) -> Result<Cents> {
        self.conn.query_row("SELECT COALESCE(SUM(e.amount), 0) FROM entry e JOIN account a ON a.id = e.account_id WHERE a.off_budget = 0 AND e.category_id IS NULL AND e.amount > 0 AND CAST(strftime('%Y%m', e.date) AS INTEGER) = ?1", [month], |row| row.get(0))
    }

    pub fn account_balances(&self) -> Result<HashMap<Id, Cents>> {
        let mut stmt = self.conn.prepare("SELECT a.id, COALESCE(SUM(e.amount), 0) FROM account a LEFT JOIN entry e ON e.account_id = a.id GROUP BY a.id")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect()
    }

    pub fn activity_by_month_category(&self) -> Result<HashMap<(MonthKey, Id), Cents>> {
        let mut stmt = self.conn.prepare("SELECT CAST(strftime('%Y%m', e.date) AS INTEGER), e.category_id, SUM(e.amount) FROM entry e JOIN account a ON a.id = e.account_id WHERE e.category_id IS NOT NULL AND a.off_budget = 0 GROUP BY 1, 2")?;
        let rows = stmt.query_map([], |r| Ok(((r.get(0)?, r.get(1)?), r.get(2)?)))?;
        rows.collect()
    }

    pub fn set_budget(&self, month: MonthKey, category: Id, amount: Cents) -> Result<()> {
        self.conn.execute("INSERT INTO budget_amount(month, category_id, amount, carryover) VALUES (?1, ?2, ?3, 0) ON CONFLICT(month, category_id) DO UPDATE SET amount = excluded.amount", params![month, category, amount])?;
        Ok(())
    }

    pub fn budget_rows(&self) -> Result<BudgetRows> {
        let mut stmt = self.conn.prepare("SELECT month, category_id, amount, carryover FROM budget_amount WHERE category_id IS NOT NULL")?;
        let mut amounts = HashMap::new();
        let mut carryover = HashSet::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let key = (row.get(0)?, row.get(1)?);
            amounts.insert(key, row.get(2)?);
            if row.get(3)? { carryover.insert(key); }
        }
        Ok((amounts, carryover))
    }
}

pub type BudgetRows = (HashMap<(MonthKey, Id), Cents>, HashSet<(MonthKey, Id)>);

#[cfg(test)]
mod tests;
