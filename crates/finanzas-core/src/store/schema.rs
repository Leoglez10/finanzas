use rusqlite::{Connection, Result};

pub(super) const CURRENT_VERSION: i64 = 1;

pub(super) fn migrate(conn: &mut Connection) -> Result<()> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > CURRENT_VERSION {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if version == 0 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "CREATE TABLE account (id INTEGER PRIMARY KEY, name TEXT NOT NULL, off_budget INTEGER NOT NULL, closed INTEGER NOT NULL, sort_order REAL NOT NULL);
             CREATE TABLE category_group (id INTEGER PRIMARY KEY, name TEXT NOT NULL, is_income INTEGER NOT NULL, sort_order REAL NOT NULL);
             CREATE TABLE category (id INTEGER PRIMARY KEY, name TEXT NOT NULL, hidden INTEGER NOT NULL, sort_order REAL NOT NULL, group_id INTEGER REFERENCES category_group(id) ON DELETE CASCADE);
             CREATE TABLE payee (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
             CREATE TABLE schedule (id INTEGER PRIMARY KEY, name TEXT NOT NULL, amount INTEGER NOT NULL, auto_post INTEGER NOT NULL, completed INTEGER NOT NULL, next_date TEXT, recurrence TEXT, account_id INTEGER REFERENCES account(id) ON DELETE SET NULL, category_id INTEGER REFERENCES category(id) ON DELETE SET NULL, payee_id INTEGER REFERENCES payee(id) ON DELETE SET NULL);
             CREATE TABLE entry (id INTEGER PRIMARY KEY, date TEXT NOT NULL, amount INTEGER NOT NULL, notes TEXT NOT NULL, cleared INTEGER NOT NULL, account_id INTEGER REFERENCES account(id) ON DELETE CASCADE, category_id INTEGER REFERENCES category(id) ON DELETE SET NULL, payee_id INTEGER REFERENCES payee(id) ON DELETE SET NULL, schedule_id INTEGER REFERENCES schedule(id) ON DELETE SET NULL);
             CREATE TABLE budget_amount (id INTEGER PRIMARY KEY, month INTEGER NOT NULL, amount INTEGER NOT NULL, carryover INTEGER NOT NULL, category_id INTEGER REFERENCES category(id) ON DELETE CASCADE, UNIQUE(month, category_id));
             CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE INDEX entry_account_idx ON entry(account_id);
             CREATE INDEX entry_date_idx ON entry(date);
             CREATE INDEX entry_category_date_idx ON entry(category_id, date);
             CREATE INDEX entry_payee_date_idx ON entry(payee_id, date);",

        )?;
        tx.pragma_update(None, "user_version", CURRENT_VERSION)?;
        tx.commit()?;
    }
    Ok(())
}
