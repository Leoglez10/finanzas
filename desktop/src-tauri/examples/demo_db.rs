//! Writes a demo database for looking at the app without touching real data:
//!   cargo run --example demo_db -- /tmp/finanzas-demo/finanzas.db
//!   FINANZAS_DB=/tmp/finanzas-demo/finanzas.db pnpm --dir desktop tauri dev
//! Same data as the Rune demo scene, dated around the current month, plus schedules and wishes.
use chrono::{Datelike, Local, NaiveDate};
use finanzas_core::{domain::month, models::{Account, Category, CategoryGroup, Entry, Frequency, Recurrence, Schedule, WeekendMode}, service, store::Store};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("uso: demo_db <ruta/al/archivo.db>")?;
    if std::path::Path::new(&path).exists() { return Err(format!("{path} ya existe; usa una ruta nueva").into()); }
    let mut store = Store::open(&path)?;
    let today = Local::now().date_naive();
    let this_month = today.year() * 100 + today.month() as i32;
    let day = |m: i32, d: u32| NaiveDate::from_ymd_opt(m / 100, (m % 100) as u32, d).ok_or("fecha inválida");

    let account = store.insert_account(&Account { id: 0, name: "Cuenta principal".into(), off_budget: false, closed: false, sort_order: 0.0 })?;
    let card = store.insert_account(&Account { id: 0, name: "Tarjeta".into(), off_budget: false, closed: false, sort_order: 1.0 })?;
    let income_group = store.insert_category_group(&CategoryGroup { id: 0, name: "Ingresos".into(), is_income: true, sort_order: 0.0 })?;
    let expense_group = store.insert_category_group(&CategoryGroup { id: 0, name: "Gastos".into(), is_income: false, sort_order: 1.0 })?;
    let salary = store.insert_category(&Category { id: 0, name: "Sueldo".into(), hidden: false, sort_order: 0.0, group_id: Some(income_group) })?;
    let budgets = [80_000, 47_000, 60_000, 30_000, 48_000, 40_000];
    let mut categories = Vec::new();
    for (i, name) in ["Vivienda", "Alimentación", "Transporte", "Ocio", "Servicios", "Salud"].iter().enumerate() {
        let id = store.insert_category(&Category { id: 0, name: (*name).into(), hidden: false, sort_order: i as f64, group_id: Some(expense_group) })?;
        store.set_budget(this_month, id, budgets[i])?;
        categories.push(id);
    }
    let employer = store.find_or_create_payee("Empresa SA")?.ok_or("comercio inválido")?;
    let mut payees = Vec::new();
    for name in ["Renta", "Supermercado La Esquina", "Uber", "Netflix", "CFE", "Farmacia"] { payees.push(store.find_or_create_payee(name)?.ok_or("comercio inválido")?); }

    let mut m = this_month;
    for index in 0..7 {
        let date = day(m, 5)?;
        store.insert_entry(&Entry { id: 0, date, amount: 320_000 + index * 5_000, notes: "Ingreso mensual".into(), cleared: true, account_id: Some(account), category_id: Some(salary), payee_id: Some(employer), schedule_id: None })?;
        for (i, category) in categories.iter().enumerate() {
            let date = day(m, 5 + (i as u32 % 3) * 2)?;
            store.insert_entry(&Entry { id: 0, date, amount: -(50_000 - i as i64 * 5_000 + index * 500), notes: if i == 1 { "Despensa".into() } else { String::new() }, cleared: true,
                account_id: Some(if i == 4 { card } else { account }), category_id: Some(*category), payee_id: Some(payees[i]), schedule_id: None })?;
        }
        m = month::previous(m);
    }

    let rule = |start: NaiveDate| Recurrence { start, frequency: Frequency::Monthly, interval: 1, skip_weekend: false, weekend_mode: WeekendMode::After, end_date: None };
    let schedule = |name: &str, amount: i64, start: NaiveDate, auto_post: bool, acc: i64, category: i64, payee: &str| -> Result<(), Box<dyn std::error::Error>> {
        let payee_id = store.find_or_create_payee(payee)?;
        store.insert_schedule(&Schedule { id: 0, name: name.into(), amount, auto_post, completed: false, next_date: Some(start), recurrence: Some(rule(start)), account_id: Some(acc), category_id: Some(category), payee_id })?;
        Ok(())
    };
    let soon = |days: i64| today + chrono::Duration::days(days);
    schedule("Renta", -80_000, soon(-5), false, account, categories[0], "Renta")?;
    schedule("Netflix", -2_000, soon(4), true, card, categories[3], "Netflix")?;
    schedule("Pago mensual", -10_000, soon(10), false, account, categories[0], "Renta")?;
    schedule("Nómina", 320_000, soon(12), false, account, salary, "Empresa SA")?;

    for (name, price, reserve, category) in [("PlayStation 5", 50_000, 0, categories[3]), ("Audífonos inalámbricos", 15_000, 15_000, categories[2]), ("Silla ergonómica", 32_000, 12_000, categories[0])] {
        let wish = service::save_wish(&mut store, None, service::WishDraft { name: name.into(), price, url: String::new(), notes: String::new(), category_id: Some(category), kind: service::WishKind::Compra, target_date: None, items: None })?;
        if reserve > 0 { service::reserve_wish(&mut store, wish.id, reserve, today)?; }
    }
    println!("Base demo creada en {path}");
    Ok(())
}
