//! Update notices: asks GitHub for the latest release at launch and remembers the last version
//! that ran, so the app can say "there is a new version" and "you were updated".
use std::path::Path;

pub const CURRENT: &str = env!("CARGO_PKG_VERSION");
pub const RELEASES: &str = "https://github.com/Leoglez10/finanzas/releases";
const API: &str = "https://api.github.com/repos/Leoglez10/finanzas/releases/latest";

/// `1.10.0` > `1.9.3`; a leading `v` and missing parts are accepted, anything unparsable is never newer.
pub fn newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Option<Vec<u64>> { v.trim().trim_start_matches('v').split('.').map(|n| n.parse().ok()).collect() };
    match (parse(latest), parse(current)) { (Some(l), Some(c)) => l > c, _ => false }
}

/// The release tag without its `v`, from the GitHub API's JSON.
pub fn tag(json: &[u8]) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    value["tag_name"].as_str().map(|t| t.trim_start_matches('v').to_owned()).ok_or_else(|| "respuesta sin tag_name".into())
}

/// Latest published version. Runs on a worker thread.
// ponytail: shells out to curl (bundled with macOS, Windows 10+ and most Linux) instead of adding an HTTP+TLS stack.
pub fn fetch_latest() -> Result<String, String> {
    let mut cmd = std::process::Command::new("curl");
    cmd.args(["-fsSL", "--max-time", "10", "-H", "Accept: application/vnd.github+json", API]);
    #[cfg(windows)]
    { use std::os::windows::process::CommandExt; cmd.creation_flags(0x0800_0000); } // CREATE_NO_WINDOW: no console flash
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() { return Err(String::from_utf8_lossy(&out.stderr).trim().to_owned()); }
    tag(&out.stdout)
}

/// Records `current` as the version that last ran in `dir` and returns the previous one when it
/// differs. The first launch (no record yet) returns `None`, so it is not announced as an update.
pub fn record_version(dir: &Path, current: &str) -> Option<String> {
    let path = dir.join("last-version");
    let previous = std::fs::read_to_string(&path).ok().map(|v| v.trim().to_owned());
    if previous.as_deref() == Some(current) { return None; }
    let _ = std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&path, current));
    previous
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_numerically() {
        assert!(newer("0.2.0", "0.1.0"));
        assert!(newer("v1.10.0", "1.9.3"));
        assert!(newer("1.0", "0.9.9"));
        assert!(!newer("0.1.0", "0.1.0"));
        assert!(!newer("0.1.0", "0.2.0"));
        assert!(!newer("nightly", "0.1.0"));
    }

    #[test]
    fn reads_the_tag_from_the_api_response() {
        assert_eq!(tag(br#"{"tag_name":"v0.3.1","name":"Finanzas 0.3.1"}"#).unwrap(), "0.3.1");
        assert!(tag(br#"{"message":"Not Found"}"#).is_err());
        assert!(tag(b"<html>").is_err());
    }

    #[test]
    fn announces_only_a_changed_version() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/update-tests").join(std::process::id().to_string());
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(record_version(&dir, "0.1.0"), None, "first launch");
        assert_eq!(record_version(&dir, "0.1.0"), None, "same version");
        assert_eq!(record_version(&dir, "0.2.0"), Some("0.1.0".into()), "updated");
        assert_eq!(record_version(&dir, "0.2.0"), None, "already announced");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
