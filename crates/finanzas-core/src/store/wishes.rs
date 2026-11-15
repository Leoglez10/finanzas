use super::Store;
use crate::service::Wish;
use rusqlite::{params, Result};

impl Store {
    pub(crate) fn wishes(&self) -> Result<Vec<Wish>> {
        let mut stmt = self.conn.prepare("SELECT id,name,price,url,notes,category_id,reserved,archived FROM wish ORDER BY id")?;
        let rows = stmt.query_map([], |r| Ok(Wish { id: r.get(0)?, name: r.get(1)?, price: r.get(2)?,
            url: r.get(3)?, notes: r.get(4)?, category_id: r.get(5)?, reserved: r.get(6)?, archived: r.get(7)? }))?;
        rows.collect()
    }

    pub(crate) fn put_wish(&self, wish: &Wish) -> Result<i64> {
        if wish.id == 0 {
            self.conn.execute("INSERT INTO wish(name,price,url,notes,category_id,reserved,archived) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![wish.name,wish.price,wish.url,wish.notes,wish.category_id,wish.reserved,wish.archived])?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute("UPDATE wish SET name=?1,price=?2,url=?3,notes=?4,category_id=?5,reserved=?6,archived=?7 WHERE id=?8",
                params![wish.name,wish.price,wish.url,wish.notes,wish.category_id,wish.reserved,wish.archived,wish.id])?;
            Ok(wish.id)
        }
    }
}
