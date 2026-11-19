//! App preferences: `settings.json` next to the database, shared with the Rune app.
//! Field names stay snake_case so both apps read the same file; missing fields take the defaults
//! and the Rune app ignores the ones it does not know.
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub const MAX_DAYS: i64 = 14;
pub const MAX_HOUR: u32 = 23;
pub const PALETTES: [&str; 6] = ["petroleo", "bosque", "ciruela", "grafito", "arena", "sistema"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub days: i64,
    pub hour: u32,
    pub hidden_profiles: Vec<String>,
    /// `None` follows the system.
    pub dark: Option<bool>,
    pub palette: String,
    pub tray: bool,
    pub autostart: bool,
    pub auto_update: bool,
    pub profile_colors: HashMap<String, String>,
    /// Reminder ids already shown, so a restart never repeats one.
    pub sent: HashSet<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { enabled: true, days: 2, hour: 9, hidden_profiles: Vec::new(), dark: None, palette: "petroleo".into(),
            tray: true, autostart: false, auto_update: true, profile_colors: HashMap::new(), sent: HashSet::new() }
    }
}

impl Settings {
    pub fn clamped(mut self) -> Self {
        self.days = self.days.clamp(0, MAX_DAYS);
        self.hour = self.hour.min(MAX_HOUR);
        if !PALETTES.contains(&self.palette.as_str()) { self.palette = "petroleo".into(); }
        self
    }
}

/// A missing file means "never configured": the defaults.
pub fn load(path: &Path) -> Result<Settings, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice::<Settings>(&bytes).map(Settings::clamped).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(e) => Err(e.to_string()),
    }
}

/// Writes a sibling file and renames it over the target, so a crash never leaves half a file.
pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    let temp = path.with_extension("json.tmp");
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    std::fs::write(&temp, json).and_then(|()| std::fs::rename(&temp, path)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("finanzas-desktop-settings-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reads_the_rune_app_file_and_fills_new_fields_with_defaults() {
        let dir = scratch();
        let path = dir.join("settings.json");
        std::fs::write(&path, r#"{"enabled":false,"days":5,"hour":7,"hidden_profiles":["negocio"],"dark":true}"#).unwrap();
        let s = load(&path).unwrap();
        assert_eq!((s.enabled, s.days, s.hour, s.dark), (false, 5, 7, Some(true)));
        assert_eq!(s.hidden_profiles, vec!["negocio".to_string()]);
        assert_eq!((s.palette.as_str(), s.tray, s.auto_update), ("petroleo", true, true));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_file_is_defaults_and_out_of_range_values_are_clamped() {
        let dir = scratch();
        assert_eq!(load(&dir.join("settings.json")).unwrap(), Settings::default());
        std::fs::write(dir.join("settings.json"), r#"{"days":99,"hour":40,"palette":"neon"}"#).unwrap();
        let s = load(&dir.join("settings.json")).unwrap();
        assert_eq!((s.days, s.hour, s.palette.as_str()), (MAX_DAYS, MAX_HOUR, "petroleo"));
        std::fs::write(dir.join("settings.json"), "{{").unwrap();
        assert!(load(&dir.join("settings.json")).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn save_round_trips_and_keeps_snake_case_for_the_rune_app() {
        let dir = scratch();
        let path = dir.join("settings.json");
        let mut s = Settings { palette: "arena".into(), hour: 8, ..Settings::default() };
        s.sent.insert("pago-1-20260301".into());
        save(&path, &s).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"hidden_profiles\"") && raw.contains("\"auto_update\""));
        assert_eq!(load(&path).unwrap(), s);
        assert!(!dir.join("settings.json.tmp").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
