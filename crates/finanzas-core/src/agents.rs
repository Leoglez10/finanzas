//! MCP agents: which clients run `finanzas-mcp` right now and how to configure each one.
//! While connected, the server touches `mcp-clients/<agent>` beside the personal database
//! every [`BEAT`]; the app shows an agent as connected while that file is fresh.
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub const BEAT: Duration = Duration::from_secs(30);
/// Three missed beats: the client closed the server.
pub const STALE: Duration = Duration::from_secs(90);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Agent { Claude, Codex, OpenCode }

impl Agent {
    pub const ALL: [Agent; 3] = [Agent::Claude, Agent::Codex, Agent::OpenCode];
    pub fn name(self) -> &'static str { match self { Agent::Claude => "Claude", Agent::Codex => "Codex", Agent::OpenCode => "OpenCode" } }
    fn slug(self) -> &'static str { match self { Agent::Claude => "claude", Agent::Codex => "codex", Agent::OpenCode => "opencode" } }
    /// The MCP `clientInfo.name`: "claude-code", "codex-mcp-client", "opencode", …
    pub fn from_client(name: &str) -> Option<Agent> {
        let name = name.to_lowercase();
        Agent::ALL.into_iter().find(|a| name.contains(a.slug()))
    }
    pub fn instructions(self) -> &'static str {
        match self {
            Agent::Claude => "Ejecuta este comando en una terminal y reinicia Claude Code.",
            Agent::Codex => "Pega esto en ~/.codex/config.toml y reinicia Codex.",
            Agent::OpenCode => "Pega esto en ~/.config/opencode/opencode.json (o une la sección \"mcp\" con la tuya) y reinicia OpenCode.",
        }
    }
    /// The setup text for this agent, running the server at `command`.
    pub fn config(self, command: &Path) -> String {
        let command = command.display().to_string();
        match self {
            Agent::Claude => format!("claude mcp add --scope user finanzas -- \"{command}\""),
            // A TOML literal string: Windows backslashes need no escaping.
            Agent::Codex => format!("[mcp_servers.finanzas]\ncommand = '{command}'"),
            Agent::OpenCode => serde_json::to_string_pretty(&serde_json::json!({
                "$schema": "https://opencode.ai/config.json",
                "mcp": { "finanzas": { "type": "local", "command": [command], "enabled": true } }
            })).expect("static JSON"),
        }
    }
}

pub fn heartbeat_dir(home_db: &Path) -> PathBuf { home_db.with_file_name("mcp-clients") }

/// Marks `agent` as connected now.
pub fn beat(dir: &Path, agent: Agent) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(agent.slug()), std::process::id().to_string())
}

pub fn connected(dir: &Path, agent: Agent, now: SystemTime) -> bool {
    std::fs::metadata(dir.join(agent.slug())).and_then(|m| m.modified())
        .is_ok_and(|t| now.duration_since(t).unwrap_or_default() < STALE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clients_map_to_agents() {
        assert_eq!(Agent::from_client("claude-code"), Some(Agent::Claude));
        assert_eq!(Agent::from_client("Codex-MCP-Client"), Some(Agent::Codex));
        assert_eq!(Agent::from_client("opencode"), Some(Agent::OpenCode));
        assert_eq!(Agent::from_client("cursor"), None);
    }

    #[test]
    fn configs_point_at_the_command() {
        let cmd = Path::new("/Applications/Finanzas.app/Contents/MacOS/finanzas-mcp");
        assert_eq!(Agent::Claude.config(cmd), "claude mcp add --scope user finanzas -- \"/Applications/Finanzas.app/Contents/MacOS/finanzas-mcp\"");
        assert_eq!(Agent::Codex.config(cmd), "[mcp_servers.finanzas]\ncommand = '/Applications/Finanzas.app/Contents/MacOS/finanzas-mcp'");
        let json: serde_json::Value = serde_json::from_str(&Agent::OpenCode.config(Path::new(r"C:\Program Files\finanzas-mcp.exe"))).unwrap();
        assert_eq!(json["mcp"]["finanzas"]["command"][0], r"C:\Program Files\finanzas-mcp.exe");
        assert_eq!(json["mcp"]["finanzas"]["type"], "local");
    }

    #[test]
    fn beats_are_connected_until_stale() {
        let nanos = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("finanzas-agents-{}-{nanos}", std::process::id()));
        let now = SystemTime::now();
        assert!(!connected(&dir, Agent::Codex, now));
        beat(&dir, Agent::Codex).unwrap();
        assert!(connected(&dir, Agent::Codex, SystemTime::now()));
        assert!(!connected(&dir, Agent::Claude, SystemTime::now()));
        assert!(!connected(&dir, Agent::Codex, SystemTime::now() + STALE + Duration::from_secs(1)));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
