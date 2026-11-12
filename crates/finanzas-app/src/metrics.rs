pub fn change_badge(change: Option<f64>) -> String {
    match change {
        Some(value) => format!("{:+.0}% vs mes anterior", (value * 100.0).round()),
        None => "— vs mes anterior".into(),
    }
}
pub fn net_average(points: &[(i64, i64)]) -> i64 {
    if points.is_empty() { return 0; }
    let sum: i128 = points.iter().map(|(income, expense)| i128::from(*income) - i128::from(*expense)).sum();
    (sum / points.len() as i128).clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

/// "$5,304.28" → ("$5,304", ".28"), so the cents can be drawn smaller.
pub fn split_money(cents: i64) -> (String, String) {
    let full = finanzas_core::domain::money::format(cents);
    let at = full.rfind('.').unwrap_or(full.len());
    (full[..at].to_owned(), full[at..].to_owned())
}

/// Where this month's income goes: already spent, still owed to scheduled payments, and what is left.
#[derive(Debug, PartialEq, Eq)]
pub struct Forecast { pub income: i64, pub spent: i64, pub pending: i64, pub left: i64 }
/// `pending` holds the signed amounts of scheduled items still due this month; only outflows count.
pub fn forecast(income: i64, spent: i64, pending: impl IntoIterator<Item = i64>) -> Forecast {
    let pending: i64 = pending.into_iter().filter(|v| *v < 0).map(|v| -v).sum();
    Forecast { income, spent, pending, left: income - spent - pending }
}
impl Forecast {
    /// Share of the bar for each part: of the income, or of all outflows once they exceed it.
    pub fn shares(&self) -> (f32, f32, f32) {
        let base = self.income.max(self.spent + self.pending);
        if base <= 0 { return (0.0, 0.0, 0.0); }
        let f = |v: i64| (v.max(0) as f64 / base as f64) as f32;
        (f(self.spent), f(self.pending), f(self.left))
    }
    /// Whole-percent share of the income, `None` without income.
    pub fn percent_of_income(&self, value: i64) -> Option<i64> {
        (self.income > 0).then(|| (value as f64 / self.income as f64 * 100.0).round() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_money_separates_cents_for_positive_and_negative_amounts() {
        assert_eq!(split_money(530_428), ("$5,304".into(), ".28".into()));
        assert_eq!(split_money(-1_999), ("-$19".into(), ".99".into()));
        assert_eq!(split_money(0), ("$0".into(), ".00".into()));
    }

    #[test]
    fn forecast_subtracts_spending_and_pending_outflows_only() {
        let f = forecast(2_800_000, 766_622, [-1_452_950, -50_000, 20_000]);
        assert_eq!(f, Forecast { income: 2_800_000, spent: 766_622, pending: 1_502_950, left: 530_428 });
        assert_eq!(f.percent_of_income(f.left), Some(19));
        let (spent, pending, left) = f.shares();
        assert!((spent + pending + left - 1.0).abs() < 1e-4);
    }

    #[test]
    fn forecast_without_income_or_with_overspending_stays_bounded() {
        let none = forecast(0, 0, []);
        assert_eq!(none.shares(), (0.0, 0.0, 0.0));
        assert_eq!(none.percent_of_income(0), None);
        let over = forecast(1_000, 1_500, [-500]);
        assert_eq!(over.left, -1_000);
        assert_eq!(over.shares(), (0.75, 0.25, 0.0));
    }

    #[test]
    fn badge_formats_positive_negative_and_missing_changes() {
        assert_eq!(change_badge(Some(0.125)), "+13% vs mes anterior");
        assert_eq!(change_badge(Some(-0.2)), "-20% vs mes anterior");
        assert_eq!(change_badge(None), "— vs mes anterior");
    }

    #[test]
    fn average_net_flow_handles_empty_and_negative_months() {
        assert_eq!(net_average(&[]), 0);
        assert_eq!(net_average(&[(1000, 300), (200, 400)]), 250);
        assert_eq!(net_average(&[(0, 100)]), -100);
    }
}
