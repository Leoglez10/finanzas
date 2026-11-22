use super::{budget_month, upcoming, Result, ServiceError};
use crate::{store::Store, Cents, Id, domain::month};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// What a goal is for. Stored as its lowercase Spanish name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WishKind { #[default] Compra, Viaje, Experiencia, Fondo }
impl WishKind {
    pub fn as_str(self) -> &'static str { match self { Self::Compra => "compra", Self::Viaje => "viaje", Self::Experiencia => "experiencia", Self::Fondo => "fondo" } }
    /// Unknown text falls back to a purchase; the column's CHECK keeps it from happening.
    pub fn parse(s: &str) -> Self { match s { "viaje" => Self::Viaje, "experiencia" => Self::Experiencia, "fondo" => Self::Fondo, _ => Self::Compra } }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WishDraft {
    pub name: String, pub price: Cents, pub url: String, pub notes: String, pub category_id: Option<Id>,
    #[serde(default)] pub kind: WishKind, #[serde(default)] pub target_date: Option<NaiveDate>,
    /// Some: the goal's line items become exactly these (their sum is the price when not empty). None: they stay as they are.
    #[serde(default)] pub items: Option<Vec<WishItemDraft>>,
}
/// A line item to save; `id` is set for an item the goal already has.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WishItemDraft { #[serde(default)] pub id: Option<Id>, pub name: String, pub amount: Cents, #[serde(default)] pub paid: bool }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WishItem { pub id: Id, pub wish_id: Id, pub name: String, pub amount: Cents, pub paid: bool }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wish {
    pub id: Id, pub name: String, pub price: Cents, pub url: String, pub notes: String,
    pub category_id: Option<Id>, pub reserved: Cents, pub archived: bool,
    #[serde(default)] pub kind: WishKind, #[serde(default)] pub target_date: Option<NaiveDate>,
    /// Sum of the paid line items: money already spent toward the goal. Read-only, derived from the items.
    #[serde(default)] pub paid: Cents,
}
/// How to reach a goal with a target date: set aside `monthly` on each of `months_left` monthly dates
/// (today's day of the month, up to the target). Overdue goals ask for everything now; archived goals have no plan.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WishPlan { pub target_date: NaiveDate, pub remaining: Cents, pub months_left: i64, pub monthly: Cents, pub overdue: bool }
pub fn plan(wish: &Wish, today: NaiveDate) -> Option<WishPlan> {
    use chrono::Datelike;
    if wish.archived { return None; }
    let target = wish.target_date?;
    let remaining = (wish.price - wish.paid - wish.reserved).max(0);
    let overdue = target < today && remaining > 0;
    let months = (target.year() - today.year()) as i64 * 12 + target.month() as i64 - today.month() as i64;
    let months_left = if target < today { 0 } else { months - i64::from(target.day() < today.day()) + 1 };
    let monthly = if months_left == 0 { remaining } else { (remaining + months_left - 1) / months_left };
    Some(WishPlan { target_date: target, remaining, months_left, monthly, overdue })
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WishEvaluation {
    pub status: String, pub as_of: NaiveDate, pub horizon_days: i64,
    pub category_available: Option<Cents>, pub available_for_wish: Option<Cents>,
    pub budget_shortfall: Option<Cents>, pub reservation_deficit: Cents,
    pub cash_balance: Option<Cents>, pub upcoming_outflows: Cents,
    pub cash_shortfall: Option<Cents>, pub reasons: Vec<String>,
}
/// One change to the set-aside amount: positive when money was set aside, negative when released.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WishContribution { pub wish_id: Id, pub date: NaiveDate, pub amount: Cents }
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WishView { pub wish: Wish, pub evaluation: WishEvaluation, pub plan: Option<WishPlan>, pub items: Vec<WishItem>, pub contributions: Vec<WishContribution> }

fn invalid(message: &'static str) -> ServiceError { ServiceError::InvalidWish(message) }
fn find(store: &Store, id: Id) -> Result<Wish> {
    store.wishes()?.into_iter().find(|w| w.id == id).ok_or(ServiceError::NotFound)
}
fn total_reserved(wishes: &[Wish], predicate: impl Fn(&Wish) -> bool) -> Result<Cents> {
    wishes.iter().filter(|w| !w.archived && w.category_id.is_some() && predicate(w))
        .try_fold(0i64, |sum, w| sum.checked_add(w.reserved).ok_or_else(|| invalid("Reservation total is too large")))
}
fn validate(store: &Store, draft: &WishDraft) -> Result<()> {
    if draft.name.trim().is_empty() { return Err(invalid("Wish name must not be empty")); }
    if draft.price <= 0 { return Err(invalid("Wish price must be greater than zero")); }
    let url = draft.url.trim();
    if !url.is_empty() && !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(invalid("Wish link must use http:// or https://"));
    }
    if let Some(id) = draft.category_id {
        let category = store.get_category(id)?.ok_or(ServiceError::NotFound)?;
        if let Some(group) = category.group_id.and_then(|id| store.get_category_group(id).transpose()).transpose()? {
            if group.is_income { return Err(ServiceError::IncomeCategory); }
        }
    }
    Ok(())
}

pub fn save_wish(store: &mut Store, id: Option<Id>, mut draft: WishDraft) -> Result<Wish> {
    store.transaction(|store| {
        if let Some(items) = draft.items.as_ref().filter(|items| !items.is_empty()) {
            if items.iter().any(|i| i.name.trim().is_empty()) { return Err(invalid("Line item name must not be empty")); }
            if items.iter().any(|i| i.amount <= 0) { return Err(invalid("Line item amount must be greater than zero")); }
            draft.price = items.iter().try_fold(0i64, |sum, i| sum.checked_add(i.amount)).ok_or_else(|| invalid("Line item total is too large"))?;
        }
        if draft.items.is_none() {
            // Items kept as they are still decide the price.
            let kept: Vec<_> = store.wish_items()?.into_iter().filter(|i| Some(i.wish_id) == id).collect();
            if !kept.is_empty() { draft.price = kept.iter().map(|i| i.amount).sum(); }
        }
        validate(store, &draft)?;
        let old = id.map(|id| find(store, id)).transpose()?;
        if old.as_ref().is_some_and(|w| w.archived) { return Err(invalid("Archived wishes cannot be edited")); }
        let paid = match &draft.items {
            Some(items) => {
                let own: Vec<Id> = store.wish_items()?.into_iter().filter(|i| Some(i.wish_id) == id).map(|i| i.id).collect();
                if items.iter().filter_map(|i| i.id).any(|item| !own.contains(&item)) { return Err(ServiceError::NotFound); }
                items.iter().filter(|i| i.paid).map(|i| i.amount).sum()
            }
            None => old.as_ref().map_or(0, |w| w.paid),
        };
        let reserved = old.as_ref().filter(|w| w.category_id == draft.category_id).map_or(0, |w| w.reserved);
        if reserved > draft.price - paid { return Err(invalid("Release reserved funds before reducing the price")); }
        let mut wish = Wish { id: id.unwrap_or(0), name: draft.name.trim().into(), price: draft.price,
            url: draft.url.trim().into(), notes: draft.notes, category_id: draft.category_id, reserved, archived: false,
            kind: draft.kind, target_date: draft.target_date, paid };
        wish.id = store.put_wish(&wish)?;
        if let Some(items) = &draft.items { store.replace_wish_items(wish.id, items)?; }
        Ok(wish)
    })
}

/// Marks a line item as paid (or not). Money set aside beyond what is still unpaid is released: the payment used it.
pub fn set_wish_item_paid(store: &mut Store, item: Id, paid: bool, today: NaiveDate) -> Result<Wish> {
    store.transaction(|store| {
        let wish_id = store.set_wish_item_paid(item, paid)?.ok_or(ServiceError::NotFound)?;
        let mut wish = find(store, wish_id)?;
        if wish.archived { return Err(invalid("Archived wishes cannot be edited")); }
        let unpaid = (wish.price - wish.paid).max(0);
        if wish.reserved > unpaid {
            store.add_wish_contribution(wish.id, today, unpaid - wish.reserved)?;
            wish.reserved = unpaid;
            store.put_wish(&wish)?;
        }
        Ok(wish)
    })
}

/// Absolute reserved amount, not a delta. The write lock spans validation and persistence.
/// Reductions remain possible even if later spending made existing claims underfunded.
pub fn reserve_wish(store: &mut Store, id: Id, amount: Cents, today: NaiveDate) -> Result<Wish> {
    store.transaction(|store| {
        let mut wish = find(store, id)?;
        if wish.archived { return Err(invalid("Archived wishes cannot reserve funds")); }
        if amount < 0 || amount > wish.price - wish.paid { return Err(invalid("Reservation must be between zero and the wish price")); }
        if amount > wish.reserved {
            let category = wish.category_id.ok_or_else(|| invalid("Choose an expense category before reserving"))?;
            let budget = budget_month(store, month::month_of(today))?;
            let available = budget.groups.iter().filter(|g| !g.is_income).flat_map(|g| &g.categories)
                .find(|c| c.id == category).ok_or_else(|| invalid("Expense category is not in the budget"))?.available;
            let others = total_reserved(&store.wishes()?, |w| w.id != id && w.category_id == Some(category))?;
            if amount > available.saturating_sub(others).max(0) { return Err(invalid("Not enough unreserved category funds")); }
        }
        store.add_wish_contribution(wish.id, today, amount - wish.reserved)?;
        wish.reserved = amount;
        store.put_wish(&wish)?;
        Ok(wish)
    })
}
/// Undo support: puts back a snapshot of an existing wish (archived flag and reservation included).
pub fn restore_wish(store: &mut Store, snapshot: &Wish) -> Result<Wish> {
    store.transaction(|store| {
        find(store, snapshot.id)?;
        store.put_wish(snapshot)?;
        Ok(snapshot.clone())
    })
}
pub fn archive_wish(store: &mut Store, id: Id) -> Result<Wish> {
    store.transaction(|store| {
        let mut wish = find(store, id)?;
        wish.archived = true;
        wish.reserved = 0;
        store.put_wish(&wish)?;
        Ok(wish)
    })
}

fn evaluate(store: &Store, wish: &Wish, today: NaiveDate) -> Result<WishEvaluation> {
    let wishes = store.wishes()?;
    let budget = budget_month(store, month::month_of(today))?;
    let category_available = wish.category_id.and_then(|id| budget.groups.iter().filter(|g| !g.is_income)
        .flat_map(|g| &g.categories).find(|c| c.id == id).map(|c| c.available));
    let other_category_claims = total_reserved(&wishes, |w| w.id != wish.id && w.category_id == wish.category_id)?;
    let category_claims = total_reserved(&wishes, |w| w.category_id == wish.category_id)?;
    let available_for_wish = category_available.map(|a| a.saturating_sub(other_category_claims).max(0));
    // Paid line items are already spent; only the unpaid part has to fit the budget and the cash.
    let due = (wish.price - wish.paid).max(0);
    let budget_shortfall = available_for_wish.map(|a| due.saturating_sub(a).max(0));
    let reservation_deficit = category_available.map_or(0, |a| category_claims.saturating_sub(a.max(0)).max(0));
    let accounts: Vec<_> = store.list_accounts()?.into_iter().filter(|a| !a.closed && !a.off_budget).map(|a| a.id).collect();
    let balances = store.account_balances()?;
    let cash_balance = if accounts.is_empty() { None } else { Some(accounts.iter().try_fold(0i64, |sum, id|
        sum.checked_add(*balances.get(id).unwrap_or(&0)).ok_or_else(|| invalid("Cash total is too large")))?) };
    let payments = upcoming(store, today, 30)?;
    let mut unknown_payments = false;
    let mut upcoming_outflows = 0i64;
    for payment in payments.iter().filter(|p| p.amount < 0) {
        if payment.account_id.is_none() { unknown_payments = true; }
        if payment.account_id.is_some_and(|id| accounts.contains(&id)) {
            upcoming_outflows = upcoming_outflows.checked_add(payment.amount.checked_neg().ok_or_else(|| invalid("Payment amount is too large"))?)
                .ok_or_else(|| invalid("Payment total is too large"))?;
        }
    }
    let other_claims = total_reserved(&wishes, |w| w.id != wish.id)?;
    let cash_shortfall = cash_balance.map(|cash| due.saturating_sub(cash.saturating_sub(upcoming_outflows).saturating_sub(other_claims).max(0)).max(0));
    let status = if wish.archived { "archived" }
        else if category_available.is_none() || cash_balance.is_none() || unknown_payments { "unknown" }
        else if reservation_deficit > 0 { "reservation_deficit" }
        else if budget_shortfall.is_some_and(|v| v > 0) { "budget_shortfall" }
        else if budget.to_budget < 0 { "budget_overassigned" }
        else if cash_shortfall.is_some_and(|v| v > 0) { "cash_shortfall" }
        else { "affordable" };
    let mut reasons = vec!["Evaluación del mes actual y pagos registrados de los próximos 30 días, incluidos vencidos; no incluye gastos no registrados ni ingresos futuros.".into()];
    if category_available.is_none() { reasons.push("Elige una categoría de gasto incluida en el presupuesto para evaluar la compra.".into()); }
    if cash_balance.is_none() { reasons.push("No hay cuentas abiertas dentro del presupuesto para evaluar el saldo.".into()); }
    if unknown_payments { reasons.push("Hay pagos sin cuenta: no se puede confirmar la liquidez.".into()); }
    if reservation_deficit > 0 { reasons.push("Los gastos o cambios de presupuesto dejaron reservas sin respaldo en esta categoría.".into()); }
    if budget.to_budget < 0 { reasons.push("El presupuesto tiene más dinero asignado del disponible.".into()); }
    Ok(WishEvaluation { status: status.into(), as_of: today, horizon_days: 30, category_available, available_for_wish,
        budget_shortfall, reservation_deficit, cash_balance, upcoming_outflows, cash_shortfall, reasons })
}
pub fn evaluate_wish(store: &mut Store, id: Id, today: NaiveDate) -> Result<WishEvaluation> {
    store.transaction(|store| evaluate(store, &find(store, id)?, today))
}
pub fn list_wishes(store: &mut Store, today: NaiveDate, include_archived: bool) -> Result<Vec<WishView>> {
    store.transaction(|store| {
        let items = store.wish_items()?;
        let contributions = store.wish_contributions()?;
        store.wishes()?.into_iter().filter(|w| include_archived || !w.archived).map(|wish| {
            let evaluation = evaluate(store, &wish, today)?;
            let plan = plan(&wish, today);
            let items = items.iter().filter(|i| i.wish_id == wish.id).cloned().collect();
            let contributions = contributions.iter().filter(|c| c.wish_id == wish.id).cloned().collect();
            Ok(WishView { wish, evaluation, plan, items, contributions })
        }).collect()
    })
}
