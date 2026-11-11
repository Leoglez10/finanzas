//! Envelope budget calculations.

use std::collections::{HashMap, HashSet};
use crate::models::{Cents, Id, MonthKey};
use super::month as calendar;

#[derive(Debug, Clone, Default)]
pub struct EnvelopeInputs {
    pub categories: Vec<(Id, bool)>,
    pub budgeted: HashMap<(MonthKey, Id), Cents>,
    pub carryover: HashSet<(MonthKey, Id)>,
    pub activity: HashMap<(MonthKey, Id), Cents>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row { pub category_id: Id, pub is_income: bool, pub budgeted: Cents, pub activity: Cents, pub available: Cents }
impl Row {
    pub fn spent_share(&self) -> f64 {
        if self.available < 0 { return 1.0; }
        let spent = self.activity.saturating_neg().max(0);
        let funds = spent as f64 + self.available as f64;
        if funds > 0.0 { spent as f64 / funds } else { 0.0 }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetMonth { pub month: MonthKey, pub to_budget: Cents, pub income: Cents, pub budgeted: Cents, pub rows: Vec<Row> }

pub fn month(inputs: &EnvelopeInputs, target: MonthKey) -> BudgetMonth {
    months(inputs, target, target).pop().expect("one requested month")
}
pub fn months(inputs: &EnvelopeInputs, from: MonthKey, through: MonthKey) -> Vec<BudgetMonth> {
    if from > through { return Vec::new(); }
    let earliest = inputs.budgeted.keys().chain(inputs.activity.keys()).map(|(m, _)| *m).min().unwrap_or(from);
    let mut current = earliest.min(from);
    let mut carry: HashMap<Id, Cents> = HashMap::new();
    let (mut previous_to_budget, mut previous_overspent) = (0, 0);
    let mut result = Vec::new();
    loop {
        let (mut income, mut assigned, mut overspent) = (0, 0, 0);
        let mut rows = Vec::with_capacity(inputs.categories.len());
        for &(id, is_income) in &inputs.categories {
            let key = (current, id);
            let activity = *inputs.activity.get(&key).unwrap_or(&0);
            let (budgeted, available) = if is_income { income += activity; (0, 0) } else {
                let budgeted = *inputs.budgeted.get(&key).unwrap_or(&0);
                assigned += budgeted;
                let available = carry.get(&id).copied().unwrap_or(0) + budgeted + activity;
                if available >= 0 || inputs.carryover.contains(&key) { carry.insert(id, available); } else { carry.insert(id, 0); overspent -= available; }
                (budgeted, available)
            };
            rows.push(Row { category_id: id, is_income, budgeted, activity, available });
        }
        let to_budget = income + previous_to_budget - previous_overspent - assigned;
        previous_to_budget = to_budget; previous_overspent = overspent;
        if current >= from { result.push(BudgetMonth { month: current, to_budget, income, budgeted: assigned, rows }); }
        if current == through { break; }
        current = calendar::next(current);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> EnvelopeInputs { EnvelopeInputs { categories: vec![(1,true),(2,false),(3,false)], ..Default::default() } }
    fn row(m: &BudgetMonth,id: Id) -> &Row { m.rows.iter().find(|r|r.category_id==id).unwrap() }
    #[test] fn assigned_minus_spending_is_available() { let mut b=inputs(); b.budgeted.insert((202609,2),30000); b.activity.extend([((202609,1),100000),((202609,2),-10000)]); let m=month(&b,202609); assert_eq!((row(&m,2).budgeted,row(&m,2).activity,row(&m,2).available),(30000,-10000,20000)); assert_eq!((m.income,m.to_budget),(100000,70000)); assert_eq!((row(&m,1).budgeted,row(&m,1).available),(0,0)); }
    #[test] fn leftover_and_unassigned_money_carry_forward() { let mut b=inputs(); b.budgeted.insert((202609,2),30000); b.activity.extend([((202609,1),100000),((202609,2),-10000)]); let m=month(&b,202610); assert_eq!((row(&m,2).available,m.to_budget),(20000,70000)); let all=months(&b,202609,202610); assert_eq!(all.len(),2); assert_eq!(all[1],m); }
    #[test] fn overspending_resets_category_and_reduces_next_month_to_budget() { let mut b=inputs(); b.budgeted.insert((202609,3),10000); b.activity.extend([((202609,1),50000),((202609,3),-15000)]); assert_eq!(row(&month(&b,202609),3).available,-5000); let m=month(&b,202610); assert_eq!((row(&m,3).available,m.to_budget),(0,35000)); }
    #[test] fn carryover_keeps_overspending_in_the_category() { let mut b=inputs(); b.budgeted.insert((202609,3),10000); b.activity.extend([((202609,1),50000),((202609,3),-15000)]); b.carryover.insert((202609,3)); let m=month(&b,202610); assert_eq!((row(&m,3).available,m.to_budget),(-5000,40000)); }
    #[test] fn over_assigning_makes_to_budget_negative_and_it_carries() { let mut b=inputs(); b.budgeted.extend([((202609,2),80000),((202610,2),10000)]); b.activity.insert((202609,1),50000); assert_eq!(month(&b,202609).to_budget,-30000); let m=month(&b,202610); assert_eq!((m.to_budget,row(&m,2).available),(-40000,90000)); }
    #[test] fn empty_month_before_any_data_is_zero() { let mut b=inputs(); b.activity.insert((202609,1),1000); let m=month(&b,202601); assert_eq!((m.to_budget,row(&m,2).available),(0,0)); }
    #[test] fn spent_share_is_spending_over_available_funds() { for (budgeted,activity,available,share) in [(50000,-45000,5000,0.9),(50000,-60000,-10000,1.0),(0,0,0,0.0),(0,0,20000,0.0),(30000,10000,40000,0.0)] { assert_eq!(Row { category_id:2,is_income:false,budgeted,activity,available }.spent_share(),share); } }
}
