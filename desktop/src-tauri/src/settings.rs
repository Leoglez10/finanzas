use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub enabled: bool,
    pub days: i64,
    pub sent: HashSet<String>,
}

impl Default for Settings {
    fn default() -> Self { Self { enabled: true, days: 2, sent: HashSet::new() } }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_apply_to_missing_json_fields() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.days, 2);
        assert!(settings.sent.is_empty());
    }
    #[test]
    fn saved_sent_ids_and_preferences_round_trip() {
        let mut settings = Settings { enabled: false, days: 14, ..Settings::default() };
        settings.sent.insert("pago-1-20260301".into());
        let json = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();
        assert!(!restored.enabled);
        assert_eq!(restored.days, 14);
        assert!(restored.sent.contains("pago-1-20260301"));
        assert!(serde_json::from_str::<Settings>("not json").is_err());
    }
}
