use super::Store;
use crate::models::{BudgetAmount, Entry, Id, Schedule};
use chrono::NaiveDate;
use rusqlite::{params, types::Type, OptionalExtension, Result, Row};

fn date(row: &Row<'_>, index: usize) -> Result<Option<NaiveDate>> {
    let text: Option<String> = row.get(index)?;
    text.map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(|e| rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(e)))).transpose()
}

pub(super) fn entry_row(row: &Row<'_>) -> Result<Entry> {
    Ok(Entry { id: row.get(0)?, date: date(row, 1)?.ok_or(rusqlite::Error::InvalidQuery)?, amount: row.get(2)?, notes: row.get(3)?, cleared: row.get(4)?, account_id: row.get(5)?, category_id: row.get(6)?, payee_id: row.get(7)?, schedule_id: row.get(8)? })
}
fn schedule_row(row: &Row<'_>) -> Result<Schedule> {
    let json: Option<String> = row.get(6)?;
    let recurrence = json.map(|s| serde_json::from_str(&s).map_err(|e| rusqlite::Error::FromSqlConversionFailure(6, Type::Text, Box::new(e)))).transpose()?;
    Ok(Schedule { id: row.get(0)?, name: row.get(1)?, amount: row.get(2)?, auto_post: row.get(3)?, completed: row.get(4)?, next_date: date(row, 5)?, recurrence, account_id: row.get(7)?, category_id: row.get(8)?, payee_id: row.get(9)? })
}
fn budget_row(row: &Row<'_>) -> Result<BudgetAmount> {
    Ok(BudgetAmount { id: row.get(0)?, month: row.get(1)?, amount: row.get(2)?, carryover: row.get(3)?, category_id: row.get(4)? })
}
fn recurrence(value: &Schedule) -> Result<Option<String>> {
    value.recurrence.as_ref().map(|r| serde_json::to_string(r).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))).transpose()
}

impl Store {
    pub fn get_entry(&self, id: Id) -> Result<Option<Entry>> {
        self.conn.query_row("SELECT id,date,amount,notes,cleared,account_id,category_id,payee_id,schedule_id FROM entry WHERE id=?1", [id], entry_row).optional()
    }
    pub fn list_entries(&self) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare("SELECT id,date,amount,notes,cleared,account_id,category_id,payee_id,schedule_id FROM entry ORDER BY date DESC,id DESC")?;
        let rows = stmt.query_map([], entry_row)?; rows.collect()
    }
    pub fn update_entry(&self, v: &Entry) -> Result<()> {
        self.conn.execute("UPDATE entry SET date=?1,amount=?2,notes=?3,cleared=?4,account_id=?5,category_id=?6,payee_id=?7,schedule_id=?8 WHERE id=?9", params![v.date.to_string(),v.amount,v.notes,v.cleared,v.account_id,v.category_id,v.payee_id,v.schedule_id,v.id])?; Ok(())
    }
    pub fn delete_entry(&self, id: Id) -> Result<()> { self.conn.execute("DELETE FROM entry WHERE id=?1", [id])?; Ok(()) }
    pub fn insert_schedule(&self, v: &Schedule) -> Result<Id> {
        self.conn.execute("INSERT INTO schedule(name,amount,auto_post,completed,next_date,recurrence,account_id,category_id,payee_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![v.name,v.amount,v.auto_post,v.completed,v.next_date.map(|d| d.to_string()),recurrence(v)?,v.account_id,v.category_id,v.payee_id])?; Ok(self.conn.last_insert_rowid())
    }
    pub fn get_schedule(&self, id: Id) -> Result<Option<Schedule>> {
        self.conn.query_row("SELECT id,name,amount,auto_post,completed,next_date,recurrence,account_id,category_id,payee_id FROM schedule WHERE id=?1", [id], schedule_row).optional()
    }
    pub fn list_schedules(&self) -> Result<Vec<Schedule>> {
        let mut stmt = self.conn.prepare("SELECT id,name,amount,auto_post,completed,next_date,recurrence,account_id,category_id,payee_id FROM schedule ORDER BY name,id")?;
        let rows = stmt.query_map([], schedule_row)?; rows.collect()
    }
    pub fn update_schedule(&self, v: &Schedule) -> Result<()> {
        self.conn.execute("UPDATE schedule SET name=?1,amount=?2,auto_post=?3,completed=?4,next_date=?5,recurrence=?6,account_id=?7,category_id=?8,payee_id=?9 WHERE id=?10", params![v.name,v.amount,v.auto_post,v.completed,v.next_date.map(|d| d.to_string()),recurrence(v)?,v.account_id,v.category_id,v.payee_id,v.id])?; Ok(())
    }
    pub fn delete_schedule(&self, id: Id) -> Result<()> { self.conn.execute("DELETE FROM schedule WHERE id=?1", [id])?; Ok(()) }
    pub fn insert_budget_amount(&self, v: &BudgetAmount) -> Result<Id> {
        self.conn.execute("INSERT INTO budget_amount(month,amount,carryover,category_id) VALUES (?1,?2,?3,?4)", params![v.month,v.amount,v.carryover,v.category_id])?; Ok(self.conn.last_insert_rowid())
    }
    pub fn get_budget_amount(&self, id: Id) -> Result<Option<BudgetAmount>> {
        self.conn.query_row("SELECT id,month,amount,carryover,category_id FROM budget_amount WHERE id=?1", [id], budget_row).optional()
    }
    pub fn list_budget_amounts(&self) -> Result<Vec<BudgetAmount>> {
        let mut stmt = self.conn.prepare("SELECT id,month,amount,carryover,category_id FROM budget_amount ORDER BY month, id")?;
        let rows = stmt.query_map([], budget_row)?; rows.collect()
    }
    pub fn update_budget_amount(&self, v: &BudgetAmount) -> Result<()> {
        self.conn.execute("UPDATE budget_amount SET month=?1,amount=?2,carryover=?3,category_id=?4 WHERE id=?5", params![v.month,v.amount,v.carryover,v.category_id,v.id])?; Ok(())
    }
    pub fn delete_budget_amount(&self, id: Id) -> Result<()> { self.conn.execute("DELETE FROM budget_amount WHERE id=?1", [id])?; Ok(()) }
}
