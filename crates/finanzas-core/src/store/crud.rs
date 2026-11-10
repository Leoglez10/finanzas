use super::Store;
use crate::models::{Account, Category, CategoryGroup, Id};
use rusqlite::{params, OptionalExtension, Result};

macro_rules! crud {
    ($get:ident, $list:ident, $update:ident, $delete:ident, $model:ident, $select:expr, $order:expr, $update_sql:expr, $delete_sql:expr, {$($field:ident: $index:expr),+}, [$($value:ident),+]) => {
        pub fn $get(&self, id: Id) -> Result<Option<$model>> {
            self.conn.query_row(concat!($select, " WHERE id = ?1"), [id], |row| Ok($model { $($field: row.get($index)?),+ })).optional()
        }
        pub fn $list(&self) -> Result<Vec<$model>> {
            let mut stmt = self.conn.prepare(concat!($select, $order))?;
            let rows = stmt.query_map([], |row| Ok($model { $($field: row.get($index)?),+ }))?;
            rows.collect()
        }
        pub fn $update(&self, value: &$model) -> Result<()> {
            self.conn.execute($update_sql, params![$(value.$value),+])?;
            Ok(())
        }
        pub fn $delete(&self, id: Id) -> Result<()> {
            self.conn.execute($delete_sql, [id])?;
            Ok(())
        }
    };
}
impl Store {
    crud!(get_account, list_accounts, update_account, delete_account, Account,
        "SELECT id, name, off_budget, closed, sort_order FROM account", " ORDER BY sort_order, name, id",
        "UPDATE account SET name=?1, off_budget=?2, closed=?3, sort_order=?4 WHERE id=?5", "DELETE FROM account WHERE id=?1",
        {id:0, name:1, off_budget:2, closed:3, sort_order:4}, [name, off_budget, closed, sort_order, id]);
    crud!(get_category_group, list_category_groups, update_category_group, delete_category_group, CategoryGroup,
        "SELECT id, name, is_income, sort_order FROM category_group", " ORDER BY sort_order, name, id",
        "UPDATE category_group SET name=?1, is_income=?2, sort_order=?3 WHERE id=?4", "DELETE FROM category_group WHERE id=?1",
        {id:0, name:1, is_income:2, sort_order:3}, [name, is_income, sort_order, id]);
    crud!(get_category, list_categories, update_category, delete_category, Category,
        "SELECT id, name, hidden, sort_order, group_id FROM category", " ORDER BY sort_order, name, id",
        "UPDATE category SET name=?1, hidden=?2, sort_order=?3, group_id=?4 WHERE id=?5", "DELETE FROM category WHERE id=?1",
        {id:0, name:1, hidden:2, sort_order:3, group_id:4}, [name, hidden, sort_order, group_id, id]);
}
