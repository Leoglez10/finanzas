use super::Store;
use crate::service::{Wish, WishContribution, WishItem, WishItemDraft, WishKind};
use crate::Id;
use chrono::NaiveDate;
use rusqlite::{params, Result};

impl Store {
    pub(crate) fn wishes(&self) -> Result<Vec<Wish>> {
        let mut stmt = self.conn.prepare("SELECT id,name,price,url,notes,category_id,reserved,archived,kind,target_date,\n            (SELECT COALESCE(SUM(amount),0) FROM wish_item i WHERE i.wish_id = wish.id AND i.paid = 1) FROM wish ORDER BY id")?;
        let rows = stmt.query_map([], |r| Ok(Wish { id: r.get(0)?, name: r.get(1)?, price: r.get(2)?,
            url: r.get(3)?, notes: r.get(4)?, category_id: r.get(5)?, reserved: r.get(6)?, archived: r.get(7)?,
            kind: WishKind::parse(&r.get::<_, String>(8)?), target_date: r.get::<_, Option<String>>(9)?.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()),
            paid: r.get(10)? }))?;
        rows.collect()
    }

    pub(crate) fn put_wish(&self, wish: &Wish) -> Result<i64> {
        if wish.id == 0 {
            self.conn.execute("INSERT INTO wish(name,price,url,notes,category_id,reserved,archived,kind,target_date) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![wish.name,wish.price,wish.url,wish.notes,wish.category_id,wish.reserved,wish.archived,wish.kind.as_str(),wish.target_date.map(|d| d.to_string())])?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute("UPDATE wish SET name=?1,price=?2,url=?3,notes=?4,category_id=?5,reserved=?6,archived=?7,kind=?8,target_date=?9 WHERE id=?10",
                params![wish.name,wish.price,wish.url,wish.notes,wish.category_id,wish.reserved,wish.archived,wish.kind.as_str(),wish.target_date.map(|d| d.to_string()),wish.id])?;
            Ok(wish.id)
        }
    }

    pub(crate) fn wish_items(&self) -> Result<Vec<WishItem>> {
        let mut stmt = self.conn.prepare("SELECT id,wish_id,name,amount,paid FROM wish_item ORDER BY wish_id, sort_order, id")?;
        let rows = stmt.query_map([], |r| Ok(WishItem { id: r.get(0)?, wish_id: r.get(1)?, name: r.get(2)?, amount: r.get(3)?, paid: r.get(4)? }))?;
        rows.collect()
    }

    /// Makes `items` the goal's line items in that order: listed ids are updated, new ones inserted, the rest deleted.
    pub(crate) fn replace_wish_items(&self, wish_id: Id, items: &[WishItemDraft]) -> Result<()> {
        let kept: Vec<String> = items.iter().filter_map(|i| i.id).map(|id| id.to_string()).collect();
        self.conn.execute(&format!("DELETE FROM wish_item WHERE wish_id = ?1 AND id NOT IN ({})", kept.join(",")), params![wish_id])?;
        for (order, item) in items.iter().enumerate() {
            match item.id {
                Some(id) => self.conn.execute("UPDATE wish_item SET name=?1, amount=?2, paid=?3, sort_order=?4 WHERE id=?5 AND wish_id=?6",
                    params![item.name.trim(), item.amount, item.paid, order as i64, id, wish_id])?,
                None => self.conn.execute("INSERT INTO wish_item(wish_id,name,amount,paid,sort_order) VALUES (?1,?2,?3,?4,?5)",
                    params![wish_id, item.name.trim(), item.amount, item.paid, order as i64])?,
            };
        }
        Ok(())
    }

    /// Returns the goal the item belongs to, or None when there is no such item.
    pub(crate) fn set_wish_item_paid(&self, id: Id, paid: bool) -> Result<Option<Id>> {
        use rusqlite::OptionalExtension;
        self.conn.query_row("UPDATE wish_item SET paid = ?1 WHERE id = ?2 RETURNING wish_id", params![paid, id], |r| r.get(0)).optional()
    }

    pub(crate) fn add_wish_contribution(&self, wish_id: Id, date: NaiveDate, amount: i64) -> Result<()> {
        if amount != 0 { self.conn.execute("INSERT INTO wish_contribution(wish_id,date,amount) VALUES (?1,?2,?3)", params![wish_id, date.to_string(), amount])?; }
        Ok(())
    }

    pub(crate) fn wish_contributions(&self) -> Result<Vec<WishContribution>> {
        let mut stmt = self.conn.prepare("SELECT wish_id,date,amount FROM wish_contribution ORDER BY wish_id, date, id")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, Id>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?)))?;
        rows.filter_map(|row| match row {
            Ok((wish_id, date, amount)) => NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok().map(|date| Ok(WishContribution { wish_id, date, amount })),
            Err(e) => Some(Err(e)),
        }).collect()
    }
}
