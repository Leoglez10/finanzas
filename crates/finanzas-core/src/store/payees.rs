use super::Store;
use crate::models::{Id, Payee};
use rusqlite::{OptionalExtension, Result};

pub(super) fn normalize(text: &str) -> String {
    text.trim().to_lowercase().chars().map(|c| match c {
        'á' => 'a', 'é' => 'e', 'í' => 'i', 'ó' => 'o', 'ú' | 'ü' => 'u', 'ñ' => 'n',
        _ => c,
    }).collect()
}

impl Store {
    pub fn insert_payee(&self, value: &Payee) -> Result<Id> {
        self.conn.execute("INSERT INTO payee(name) VALUES (?1)", [&value.name])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn payee_named(&self, name: &str) -> Result<Option<Payee>> {
        let name = normalize(name);
        if name.is_empty() { return Ok(None); }
        Ok(self.list_payees()?.into_iter().find(|p| normalize(&p.name) == name))
    }

    pub fn find_or_create_payee(&self, name: &str) -> Result<Option<Id>> {
        let name = name.trim();
        if name.is_empty() { return Ok(None); }
        if let Some(payee) = self.payee_named(name)? { return Ok(Some(payee.id)); }
        self.insert_payee(&Payee { id: 0, name: name.into() }).map(Some)
    }

    pub fn list_payees(&self) -> Result<Vec<Payee>> {
        let mut stmt = self.conn.prepare("SELECT id, name FROM payee ORDER BY name, id")?;
        let rows = stmt.query_map([], |row| Ok(Payee { id: row.get(0)?, name: row.get(1)? }))?;
        rows.collect()
    }

    pub fn payee_suggestions(&self, prefix: &str, limit: usize) -> Result<Vec<Payee>> {
        let prefix = normalize(prefix);
        Ok(self.list_payees()?.into_iter().filter(|p| normalize(&p.name).starts_with(&prefix)).take(limit).collect())
    }

    pub fn get_payee(&self, id: Id) -> Result<Option<Payee>> {
        self.conn.query_row("SELECT id, name FROM payee WHERE id = ?1", [id], |row| Ok(Payee { id: row.get(0)?, name: row.get(1)? })).optional()
    }
    pub fn update_payee(&self, value: &Payee) -> Result<()> {
        self.conn.execute("UPDATE payee SET name = ?1 WHERE id = ?2", rusqlite::params![value.name, value.id])?;
        Ok(())
    }
    pub fn delete_payee(&self, id: Id) -> Result<()> {
        self.conn.execute("DELETE FROM payee WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn suggested_category(&self, payee_id: Id) -> Result<Option<Id>> {
        self.conn.query_row("SELECT category_id FROM entry WHERE payee_id = ?1 AND category_id IS NOT NULL ORDER BY date DESC, id DESC LIMIT 1", [payee_id], |row| row.get(0)).optional()
    }
}
