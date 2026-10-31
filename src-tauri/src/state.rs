//! Shared application state, managed by Tauri.

use crate::config::Settings;
use crate::mcp::McpClient;
use crate::providers::Msg;
use crate::tools::ToolSpec;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{oneshot, Mutex as AsyncMutex};

pub struct AppState {
    /// Current settings. Replaced wholesale on save.
    pub settings: Mutex<Settings>,
    /// In-memory conversation history, keyed by session id.
    pub sessions: Mutex<HashMap<String, Vec<Msg>>>,
    /// Pending approval requests: id -> channel the UI answers on.
    pub approvals: Mutex<HashMap<String, oneshot::Sender<bool>>>,
    /// Live MCP server connections, keyed by server name.
    pub mcp: Mutex<HashMap<String, Arc<AsyncMutex<McpClient>>>>,
    /// Tool specs discovered from MCP servers, already namespaced.
    pub mcp_tools: Mutex<Vec<ToolSpec>>,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings: Mutex::new(settings),
            sessions: Mutex::new(HashMap::new()),
            approvals: Mutex::new(HashMap::new()),
            mcp: Mutex::new(HashMap::new()),
            mcp_tools: Mutex::new(Vec::new()),
        }
    }
}
