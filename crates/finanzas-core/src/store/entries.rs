use super::{payees::normalize, records::entry_row, Store};
use crate::models::{Cents, Id};
use chrono::NaiveDate;
use rusqlite::{params, Result};

#[derive(Debug, Default, Clone)]
pub struct EntryFilter {
    pub account_id: Option<Id>,
    pub category_id: Option<Id>,
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
    pub search: Option<String>,
    pub limit: Option<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryRow {
    pub id: Id,
    pub date: NaiveDate,
    pub amount: Cents,
    pub notes: String,
    pub cleared: bool,
    pub account_id: Option<Id>,
    pub category_id: Option<Id>,
    pub payee_id: Option<Id>,
    pub schedule_id: Option<Id>,
    pub account_name: Option<String>,
    pub payee_name: Option<String>,
    pub category_name: Option<String>,
}
fn joined_row(row: &rusqlite::Row<'_>) -> Result<EntryRow> {
    let entry = entry_row(row)?;
    Ok(EntryRow { id: entry.id, date: entry.date, amount: entry.amount, notes: entry.notes, cleared: entry.cleared, account_id: entry.account_id, category_id: entry.category_id, payee_id: entry.payee_id, schedule_id: entry.schedule_id, account_name: row.get(9)?, payee_name: row.get(10)?, category_name: row.get(11)? })
}

const FILTER: &str = " WHERE (?1 IS NULL OR e.account_id=?1) AND (?2 IS NULL OR e.category_id=?2) AND (?3 IS NULL OR e.date>=?3) AND (?4 IS NULL OR e.date<=?4)";
impl Store {
    pub fn spending_by_category(&self, start: NaiveDate, end: NaiveDate, include_off_budget: bool) -> Result<Vec<(Option<Id>, String, Cents)>> {
        let mut stmt = self.conn.prepare("SELECT e.category_id, COALESCE(c.name, 'Sin categoría'), -SUM(e.amount) AS spent FROM entry e LEFT JOIN account a ON a.id=e.account_id LEFT JOIN category c ON c.id=e.category_id WHERE e.date>=?1 AND e.date<=?2 AND e.amount<0 AND (?3 OR COALESCE(a.off_budget, 0)=0) GROUP BY e.category_id ORDER BY spent DESC, e.category_id")?;
        let rows = stmt.query_map(params![start.to_string(), end.to_string(), include_off_budget], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        rows.collect()
    }
    pub fn entry_row(&self, id: Id) -> Result<Option<EntryRow>> {
        use rusqlite::OptionalExtension;
        self.conn.query_row("SELECT e.id,e.date,e.amount,e.notes,e.cleared,e.account_id,e.category_id,e.payee_id,e.schedule_id,a.name,p.name,c.name FROM entry e LEFT JOIN account a ON a.id=e.account_id LEFT JOIN payee p ON p.id=e.payee_id LEFT JOIN category c ON c.id=e.category_id WHERE e.id=?1", [id], joined_row).optional()
    }
    pub fn entries(&self, filter: &EntryFilter) -> Result<(Vec<EntryRow>, usize)> {
        let start = filter.start.map(|d| d.to_string());
        let end = filter.end.map(|d| d.to_string());
        let search = filter.search.as_deref().map(normalize).filter(|s| !s.is_empty());
        let limit = filter.limit.unwrap_or(usize::MAX);
        let select = format!("SELECT e.id,e.date,e.amount,e.notes,e.cleared,e.account_id,e.category_id,e.payee_id,e.schedule_id,a.name,p.name,c.name FROM entry e LEFT JOIN account a ON a.id=e.account_id LEFT JOIN payee p ON p.id=e.payee_id LEFT JOIN category c ON c.id=e.category_id{FILTER} ORDER BY e.date DESC,e.id DESC");
        let mut output = Vec::new();
        let total;
        if search.is_none() {
            let count = format!("SELECT COUNT(*) FROM entry e{FILTER}");
            let count: i64 = self.conn.query_row(&count, params![filter.account_id, filter.category_id, start, end], |row| row.get(0))?;
            total = usize::try_from(count).unwrap_or(usize::MAX);
            let sql = format!("{select} LIMIT ?5");
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(params![filter.account_id, filter.category_id, start, end, i64::try_from(limit).unwrap_or(i64::MAX)], joined_row)?;
            for row in rows { output.push(row?); }
        } else {
            let mut count = 0;
            let mut stmt = self.conn.prepare(&select)?;
            let rows = stmt.query_map(params![filter.account_id, filter.category_id, start, end], joined_row)?;
            let needle = search.as_deref().unwrap_or_default();
            for row in rows {
                let row = row?;
                if [row.notes.as_str(), row.payee_name.as_deref().unwrap_or(""), row.category_name.as_deref().unwrap_or("")].iter().any(|s| normalize(s).contains(needle)) {
                    count += 1;
                    if output.len() < limit { output.push(row); }
                }
            }
            total = count;
        }
        Ok((output, total))
    }
    pub fn recent_entries(&self, limit: usize) -> Result<Vec<EntryRow>> {
        Ok(self.entries(&EntryFilter { limit: Some(limit), ..Default::default() })?.0)
    }
}
