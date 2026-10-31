//! Provider abstraction. Two wire formats cover almost everything in practice:
//! the OpenAI chat-completions schema (OpenAI, Ollama, LM Studio, vLLM, and most
//! hosted providers that copy it) and Anthropic's messages API.

pub mod anthropic;
pub mod openai;

use crate::config::{ProviderConfig, ProviderKind};
use crate::tools::ToolSpec;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// Raw JSON string of arguments, exactly as the model emitted it.
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Msg {
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
}

impl Msg {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
            tool_calls,
            tool_call_id: None,
        }
    }

    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// Incremental events pushed to the UI while a response streams in.
#[derive(Debug, Clone)]
pub enum Delta {
    Text(String),
    ToolStart { id: String, name: String },
    ToolArgs(String),
}

pub struct Reply {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
}

pub async fn stream_chat(
    cfg: &ProviderConfig,
    system: &str,
    history: &[Msg],
    tool_defs: &[ToolSpec],
    tx: UnboundedSender<Delta>,
) -> Result<Reply, String> {
    match cfg.kind {
        ProviderKind::Openai => openai::stream_chat(cfg, system, history, tool_defs, tx).await,
        ProviderKind::Anthropic => anthropic::stream_chat(cfg, system, history, tool_defs, tx).await,
    }
}

pub fn to_openai_tools(defs: &[ToolSpec]) -> Vec<Value> {
    defs.iter()
        .map(|d| {
            json!({
                "type": "function",
                "function": {
                    "name": d.name,
                    "description": d.description,
                    "parameters": d.schema,
                }
            })
        })
        .collect()
}

pub fn to_anthropic_tools(defs: &[ToolSpec]) -> Vec<Value> {
    defs.iter()
        .map(|d| {
            json!({
                "name": d.name,
                "description": d.description,
                "input_schema": d.schema,
            })
        })
        .collect()
}

/// Parse a tool-call argument string defensively. Models sometimes emit
/// empty or malformed JSON for zero-argument calls.
pub fn parse_args(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return json!({});
    }
    serde_json::from_str(trimmed).unwrap_or_else(|_| json!({}))
}
