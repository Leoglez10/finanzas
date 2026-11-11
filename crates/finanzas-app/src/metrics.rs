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

#[cfg(test)]
mod tests {
    use super::*;

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
