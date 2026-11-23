use super::Store;
use crate::service::{CardTerms, Installment};
use crate::{Cents, Id};
use chrono::NaiveDate;
use rusqlite::{params, OptionalExtension, Result};
use std::collections::HashMap;

impl Store {
    pub(crate) fn card_terms(&self) -> Result<HashMap<Id, CardTerms>> {
        let mut stmt = self.conn.prepare("SELECT account_id, credit_limit, cut_day, due_day, annual_rate, min_payment FROM credit_card")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, CardTerms { credit_limit: r.get(1)?, cut_day: r.get(2)?, due_day: r.get(3)?, annual_rate: r.get(4)?, min_payment: r.get(5)? })))?;
        rows.collect()
    }

    /// `None` stops treating the account as a credit card.
    pub(crate) fn put_card_terms(&self, account_id: Id, terms: Option<&CardTerms>) -> Result<()> {
        match terms {
            None => self.conn.execute("DELETE FROM credit_card WHERE account_id = ?1", [account_id])?,
            Some(t) => self.conn.execute("INSERT INTO credit_card(account_id, credit_limit, cut_day, due_day, annual_rate, min_payment) VALUES (?1,?2,?3,?4,?5,?6)
                ON CONFLICT(account_id) DO UPDATE SET credit_limit=excluded.credit_limit, cut_day=excluded.cut_day, due_day=excluded.due_day, annual_rate=excluded.annual_rate, min_payment=excluded.min_payment",
                params![account_id, t.credit_limit, t.cut_day, t.due_day, t.annual_rate, t.min_payment])?,
        };
        Ok(())
    }

    pub(crate) fn installments(&self) -> Result<Vec<Installment>> {
        let mut stmt = self.conn.prepare("SELECT id, account_id, name, total, months, start_date, annual_rate FROM installment ORDER BY start_date, id")?;
        let rows = stmt.query_map([], |r| Ok(Installment { id: r.get(0)?, account_id: r.get(1)?, name: r.get(2)?, total: r.get(3)?, months: r.get(4)?,
            start_date: NaiveDate::parse_from_str(&r.get::<_, String>(5)?, "%Y-%m-%d").map_err(|e| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e)))?,
            annual_rate: r.get(6)? }))?;
        rows.collect()
    }

    pub(crate) fn put_installment(&self, i: &Installment) -> Result<Id> {
        if i.id == 0 {
            self.conn.execute("INSERT INTO installment(account_id, name, total, months, start_date, annual_rate) VALUES (?1,?2,?3,?4,?5,?6)",
                params![i.account_id, i.name, i.total, i.months, i.start_date.to_string(), i.annual_rate])?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute("UPDATE installment SET account_id=?1, name=?2, total=?3, months=?4, start_date=?5, annual_rate=?6 WHERE id=?7",
                params![i.account_id, i.name, i.total, i.months, i.start_date.to_string(), i.annual_rate, i.id])?;
            Ok(i.id)
        }
    }

    pub(crate) fn remove_installment(&self, id: Id) -> Result<Option<Installment>> {
        let found = self.installments()?.into_iter().find(|i| i.id == id);
        if found.is_some() { self.conn.execute("DELETE FROM installment WHERE id = ?1", [id])?; }
        Ok(found)
    }

    /// Account balance counting only movements on or before `date`.
    pub(crate) fn balance_through(&self, account_id: Id, date: NaiveDate) -> Result<Cents> {
        self.conn.query_row("SELECT COALESCE(SUM(amount), 0) FROM entry WHERE account_id = ?1 AND date <= ?2", params![account_id, date.to_string()], |r| r.get(0))
    }

    /// Money that came into the account after `after` and up to `through` (card payments, refunds).
    pub(crate) fn inflows_between(&self, account_id: Id, after: NaiveDate, through: NaiveDate) -> Result<Cents> {
        self.conn.query_row("SELECT COALESCE(SUM(amount), 0) FROM entry WHERE account_id = ?1 AND amount > 0 AND date > ?2 AND date <= ?3",
            params![account_id, after.to_string(), through.to_string()], |r| r.get(0))
    }

    pub(crate) fn account_exists(&self, id: Id) -> Result<bool> {
        Ok(self.conn.query_row("SELECT 1 FROM account WHERE id = ?1", [id], |_| Ok(())).optional()?.is_some())
    }
}
