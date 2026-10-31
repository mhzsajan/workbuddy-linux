//! Settings persistence. Stored as JSON in the OS config directory with
//! owner-only permissions on Unix, because it holds API keys.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_SYSTEM_PROMPT: &str = "\
You are Kestrel, an agent running on the user's own computer. You complete tasks by \
calling tools, not by describing what the user could do themselves.

Rules:
- Inspect before you change anything. Read the file before editing it.
- Prefer the smallest change that solves the problem.
- Mutating tools (run_shell, write_file, edit_file) require the user's approval; the \
application asks on your behalf, so just call them.
- When a tool fails, read the error and adjust. Do not repeat an identical failing call.
- Report what you actually did, with file paths. Never claim a step succeeded if the \
tool returned an error.";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Openai,
    Anthropic,
}

impl Default for ProviderKind {
    fn default() -> Self {
        ProviderKind::Openai
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub kind: ProviderKind,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: String,
}

impl ProviderConfig {
    fn new(id: &str, label: &str, kind: ProviderKind, base_url: &str, model: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            kind,
            base_url: base_url.to_string(),
            model: model.to_string(),
            api_key: String::new(),
        }
    }
}

/// When the agent must ask before touching the machine.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalMode {
    /// Ask before every tool call, including reads.
    Always,
    /// Ask before mutating calls only. The default.
    Writes,
    /// Never ask. Only for sandboxed environments.
    Never,
}

impl Default for ApprovalMode {
    fn default() -> Self {
        ApprovalMode::Writes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub providers: Vec<ProviderConfig>,
    pub active_provider: String,
    pub workspace: String,
    pub system_prompt: String,
    pub max_iterations: u32,
    pub approval_mode: ApprovalMode,
    pub mcp_servers: Vec<McpServerConfig>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            providers: vec![
                ProviderConfig::new(
                    "openai",
                    "OpenAI",
                    ProviderKind::Openai,
                    "https://api.openai.com/v1",
                    "gpt-4o",
                ),
                ProviderConfig::new(
                    "anthropic",
                    "Anthropic",
                    ProviderKind::Anthropic,
                    "https://api.anthropic.com",
                    "claude-sonnet-4-5",
                ),
                ProviderConfig::new(
                    "ollama",
                    "Ollama (local)",
                    ProviderKind::Openai,
                    "http://localhost:11434/v1",
                    "qwen2.5-coder",
                ),
            ],
            active_provider: "openai".to_string(),
            workspace: dirs::home_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| ".".to_string()),
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            max_iterations: 25,
            approval_mode: ApprovalMode::Writes,
            mcp_servers: Vec::new(),
        }
    }
}

impl Settings {
    pub fn active(&self) -> Option<&ProviderConfig> {
        self.providers
            .iter()
            .find(|p| p.id == self.active_provider)
            .or_else(|| self.providers.first())
    }

    pub fn load() -> Self {
        let path = settings_path();
        match fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|err| {
                eprintln!("kestrel: settings at {} are invalid ({err}), using defaults", path.display());
                Settings::default()
            }),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = config_dir();
        fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let path = settings_path();
        let raw = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, raw).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        restrict_permissions(&path);
        Ok(())
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kestrel")
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

/// The settings file contains API keys, so make it owner-read/write only.
#[cfg(unix)]
fn restrict_permissions(path: &PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o600);
        let _ = fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &PathBuf) {}
