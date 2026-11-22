use super::Store;
use crate::service::{Wish, WishKind};
use chrono::NaiveDate;
use rusqlite::{params, Result};

impl Store {
    pub(crate) fn wishes(&self) -> Result<Vec<Wish>> {
        let mut stmt = self.conn.prepare("SELECT id,name,price,url,notes,category_id,reserved,archived,kind,target_date FROM wish ORDER BY id")?;
        let rows = stmt.query_map([], |r| Ok(Wish { id: r.get(0)?, name: r.get(1)?, price: r.get(2)?,
            url: r.get(3)?, notes: r.get(4)?, category_id: r.get(5)?, reserved: r.get(6)?, archived: r.get(7)?,
            kind: WishKind::parse(&r.get::<_, String>(8)?), target_date: r.get::<_, Option<String>>(9)?.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()) }))?;
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
}
