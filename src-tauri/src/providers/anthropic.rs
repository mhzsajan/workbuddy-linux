//! Anthropic Messages API streaming client.
//!
//! The wire format differs from OpenAI in two ways that matter here: tool calls
//! arrive as `tool_use` content blocks, and tool results are returned as
//! `tool_result` blocks inside a *user* message. Anthropic also rejects
//! consecutive messages with the same role, so adjacent blocks are merged.

use super::{parse_args, Delta, Msg, Reply, ToolCall};
use crate::config::ProviderConfig;
use crate::tools::ToolSpec;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use tokio::sync::mpsc::UnboundedSender;

const MAX_TOKENS: u32 = 8192;

pub async fn stream_chat(
    cfg: &ProviderConfig,
    system: &str,
    history: &[Msg],
    tool_defs: &[ToolSpec],
    tx: UnboundedSender<Delta>,
) -> Result<Reply, String> {
    let url = format!("{}/v1/messages", cfg.base_url.trim_end_matches('/'));

    let mut body = json!({
        "model": cfg.model,
        "max_tokens": MAX_TOKENS,
        "system": system,
        "messages": to_wire(history),
        "stream": true,
    });
    if !tool_defs.is_empty() {
        body["tools"] = json!(super::to_anthropic_tools(tool_defs));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(&url)
        .header("x-api-key", &cfg.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("request to {url} failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        return Err(format!(
            "{url} returned {status}: {}",
            detail.chars().take(600).collect::<String>()
        ));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut text = String::new();
    let mut calls: BTreeMap<usize, (String, String, String)> = BTreeMap::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("stream error: {e}"))?;
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            let line = line.trim();
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data.is_empty() {
                continue;
            }
            let Ok(event) = serde_json::from_str::<Value>(data) else {
                continue;
            };

            let index = event.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;

            match event.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                "content_block_start" => {
                    if let Some(block) = event.get("content_block") {
                        if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                            let id = block
                                .get("id")
                                .and_then(|i| i.as_str())
                                .unwrap_or_default()
                                .to_string();
                            let name = block
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or_default()
                                .to_string();
                            let _ = tx.send(Delta::ToolStart {
                                id: id.clone(),
                                name: name.clone(),
                            });
                            calls.insert(index, (id, name, String::new()));
                        }
                    }
                }
                "content_block_delta" => {
                    if let Some(delta) = event.get("delta") {
                        match delta.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                            "text_delta" => {
                                if let Some(piece) = delta.get("text").and_then(|t| t.as_str()) {
                                    if !piece.is_empty() {
                                        text.push_str(piece);
                                        let _ = tx.send(Delta::Text(piece.to_string()));
                                    }
                                }
                            }
                            "input_json_delta" => {
                                if let Some(piece) =
                                    delta.get("partial_json").and_then(|p| p.as_str())
                                {
                                    if !piece.is_empty() {
                                        calls
                                            .entry(index)
                                            .or_insert_with(|| {
                                                (String::new(), String::new(), String::new())
                                            })
                                            .2
                                            .push_str(piece);
                                        let _ = tx.send(Delta::ToolArgs(piece.to_string()));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                "error" => {
                    let detail = event
                        .get("error")
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "unknown error".to_string());
                    return Err(format!("provider error: {detail}"));
                }
                _ => {}
            }
        }
    }

    let tool_calls = calls
        .into_iter()
        .filter(|(_, (_, name, _))| !name.is_empty())
        .map(|(index, (id, name, arguments))| ToolCall {
            id: if id.is_empty() {
                format!("toolu_{index}")
            } else {
                id
            },
            name,
            arguments,
        })
        .collect();

    Ok(Reply { text, tool_calls })
}

fn to_wire(history: &[Msg]) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();

    for m in history {
        let (role, blocks) = convert(m);
        if blocks.is_empty() {
            continue;
        }

        let same_role = out
            .last()
            .and_then(|l| l.get("role"))
            .and_then(|r| r.as_str())
            .map(|r| r == role)
            .unwrap_or(false);

        if same_role {
            let last = out.last_mut().unwrap();
            if let Some(arr) = last.get_mut("content").and_then(|c| c.as_array_mut()) {
                arr.extend(blocks);
                continue;
            }
        }

        out.push(json!({ "role": role, "content": blocks }));
    }

    out
}

fn convert(m: &Msg) -> (&'static str, Vec<Value>) {
    match m.role.as_str() {
        "tool" => (
            "user",
            vec![json!({
                "type": "tool_result",
                "tool_use_id": m.tool_call_id.clone().unwrap_or_default(),
                "content": m.content,
            })],
        ),
        "assistant" => {
            let mut blocks = Vec::new();
            if !m.content.trim().is_empty() {
                blocks.push(json!({ "type": "text", "text": m.content }));
            }
            for c in &m.tool_calls {
                blocks.push(json!({
                    "type": "tool_use",
                    "id": c.id,
                    "name": c.name,
                    "input": parse_args(&c.arguments),
                }));
            }
            ("assistant", blocks)
        }
        _ => ("user", vec![json!({ "type": "text", "text": m.content })]),
    }
}
