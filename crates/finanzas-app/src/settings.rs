//! Reminder preferences: a small JSON file next to the database.
use std::path::Path;
use serde::{Deserialize, Serialize};

pub const MAX_DAYS: i64 = 14;
pub const MAX_HOUR: u32 = 23;

/// Unknown fields (the Tauri app also kept a `sent` list) are ignored; missing ones take the defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings { pub enabled: bool, pub days: i64, pub hour: u32, pub hidden_profiles: Vec<String>, pub dark: Option<bool> }
impl Default for Settings { fn default() -> Self { Self { enabled: true, days: 2, hour: 9, hidden_profiles: Vec::new(), dark: None } } }
impl Settings {
    fn clamped(mut self) -> Self { self.days = self.days.clamp(0, MAX_DAYS); self.hour = self.hour.min(MAX_HOUR); self }
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
