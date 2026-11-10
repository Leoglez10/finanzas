//! Month summary calculations.

use std::collections::HashSet;
use crate::models::{Cents, Entry, Id, MonthKey};
use super::{envelope::BudgetMonth, money, month};

#[derive(Debug, Clone, PartialEq)]
pub struct MonthSummary { pub income: Cents, pub expenses: Cents, pub budgeted: Cents, pub savings_rate: Option<f64>, pub spending: Vec<(Id, Cents)> }
impl From<&BudgetMonth> for MonthSummary {
    fn from(budget: &BudgetMonth) -> Self {
        let spending: Vec<_> = budget.rows.iter().filter(|r| !r.is_income && r.activity < 0).map(|r| (r.category_id, -r.activity)).collect();
        let expenses: Cents = spending.iter().map(|(_, amount)| amount).sum();
        let savings_rate = (budget.income > 0).then(|| (budget.income as f64 - expenses as f64) / budget.income as f64);
        Self { income: budget.income, expenses, budgeted: budget.budgeted, savings_rate, spending }
    }
}
pub fn relative_change(current: Cents, previous: Cents) -> Option<f64> {
    (previous != 0).then(|| (current as f64 - previous as f64) / (previous as f64).abs())
}
pub fn to_budget_hint(amount: Cents) -> String {
    if amount < 0 { format!("Asignaste {} más de lo que tienes. Registra un ingreso o baja alguna categoría.", money::format(amount.saturating_neg())) }
    else if amount == 0 { "Todo tu dinero tiene destino.".to_owned() }
    else { format!("Tienes {} sin destino. Asígnalo a alguna categoría.", money::format(amount)) }
}
pub fn uncategorized_income(entries: &[Entry], off_budget_accounts: &HashSet<Id>, target: MonthKey) -> Cents {
    entries.iter().filter(|e| e.amount > 0 && e.category_id.is_none() && e.account_id.is_some_and(|id| !off_budget_accounts.contains(&id)) && month::month_of(e.date) == target).map(|e| e.amount).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::envelope::{self, EnvelopeInputs};
    use chrono::NaiveDate;
    #[test] fn month_summary_takes_income_and_spending_from_the_budget_rows() { let mut b=EnvelopeInputs { categories:vec![(1,true),(2,false),(3,false)],..Default::default() }; b.budgeted.insert((202610,2),30000); b.activity.extend([((202610,1),100000),((202610,2),-25000),((202610,3),-15000)]); let s=MonthSummary::from(&envelope::month(&b,202610)); assert_eq!((s.income,s.expenses,s.budgeted,s.savings_rate),(100000,40000,30000,Some(0.6))); assert!(s.spending.contains(&(2,25000))); assert!(!s.spending.iter().any(|(id,_)| *id==1)); }
    #[test] fn savings_rate_is_nil_without_income() { let mut b=EnvelopeInputs { categories:vec![(2,false)],..Default::default() }; b.activity.insert((202610,2),-5000); assert_eq!(MonthSummary::from(&envelope::month(&b,202610)).savings_rate,None); }
    #[test] fn change_is_relative_to_the_previous_month() { for (a,b,c) in [(110,100,Some(0.1)),(90,100,Some(-0.1)),(5,0,None)] { assert_eq!(relative_change(a,b),c); } }
    #[test] fn a_refunded_category_does_not_make_spending_negative() { let mut b=EnvelopeInputs { categories:vec![(1,false),(2,false)],..Default::default() }; b.activity.extend([((202609,1),9800),((202609,2),-4900)]); assert_eq!(MonthSummary::from(&envelope::month(&b,202609)).expenses,4900); }
    #[test] fn uncategorized_income_counts_only_this_months_on_budget_deposits_without_category() { let entry=|date,amount,account_id,category_id| Entry { id:1,date,amount,notes:String::new(),cleared:false,account_id,category_id,payee_id:None,schedule_id:None }; let oct=NaiveDate::from_ymd_opt(2026,10,1).unwrap(); let sep=NaiveDate::from_ymd_opt(2026,9,1).unwrap(); let entries=[entry(oct,250000,Some(1),None),entry(oct,10000,Some(1),Some(3)),entry(oct,-5000,Some(1),None),entry(oct,40000,Some(2),None),entry(sep,30000,Some(1),None),entry(oct,999,None,None)]; assert_eq!(uncategorized_income(&entries,&HashSet::from([2]),202610),250000); }
    #[test] fn to_budget_hint_tells_what_to_do_next() { assert_eq!(to_budget_hint(-50000),"Asignaste $500.00 más de lo que tienes. Registra un ingreso o baja alguna categoría."); assert_eq!(to_budget_hint(12000),"Tienes $120.00 sin destino. Asígnalo a alguna categoría."); assert_eq!(to_budget_hint(0),"Todo tu dinero tiene destino."); }
}
