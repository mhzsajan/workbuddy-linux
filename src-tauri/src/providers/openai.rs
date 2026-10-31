//! OpenAI-compatible streaming client. Works against OpenAI, Ollama, LM Studio,
//! vLLM and any other endpoint that speaks `/chat/completions`.

use super::{parse_args, Delta, Msg, Reply, ToolCall};
use crate::config::ProviderConfig;
use crate::tools::ToolSpec;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use tokio::sync::mpsc::UnboundedSender;

pub async fn stream_chat(
    cfg: &ProviderConfig,
    system: &str,
    history: &[Msg],
    tool_defs: &[ToolSpec],
    tx: UnboundedSender<Delta>,
) -> Result<Reply, String> {
    let url = format!(
        "{}/chat/completions",
        cfg.base_url.trim_end_matches('/')
    );

    let mut messages: Vec<Value> = vec![json!({ "role": "system", "content": system })];
    for m in history {
        messages.push(to_wire(m));
    }

    let mut body = json!({
        "model": cfg.model,
        "messages": messages,
        "stream": true,
    });
    if !tool_defs.is_empty() {
        body["tools"] = json!(super::to_openai_tools(tool_defs));
        body["tool_choice"] = json!("auto");
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let mut req = client.post(&url).json(&body);
    if !cfg.api_key.is_empty() {
        req = req.bearer_auth(&cfg.api_key);
    }

    let resp = req.send().await.map_err(|e| format!("request to {url} failed: {e}"))?;
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
    // index -> (id, name, arguments)
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
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            let Ok(event) = serde_json::from_str::<Value>(data) else {
                continue;
            };

            if let Some(err) = event.get("error") {
                return Err(format!("provider error: {err}"));
            }

            let Some(choices) = event.get("choices").and_then(|c| c.as_array()) else {
                continue;
            };
            for choice in choices {
                let Some(delta) = choice.get("delta") else {
                    continue;
                };

                if let Some(piece) = delta.get("content").and_then(|c| c.as_str()) {
                    if !piece.is_empty() {
                        text.push_str(piece);
                        let _ = tx.send(Delta::Text(piece.to_string()));
                    }
                }

                if let Some(tcs) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                    for tc in tcs {
                        let index = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                        let entry = calls
                            .entry(index)
                            .or_insert_with(|| (String::new(), String::new(), String::new()));

                        if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                            if !id.is_empty() {
                                entry.0 = id.to_string();
                            }
                        }
                        if let Some(func) = tc.get("function") {
                            if let Some(name) = func.get("name").and_then(|n| n.as_str()) {
                                if !name.is_empty() && entry.1.is_empty() {
                                    entry.1 = name.to_string();
                                    let id = if entry.0.is_empty() {
                                        format!("call_{index}")
                                    } else {
                                        entry.0.clone()
                                    };
                                    let _ = tx.send(Delta::ToolStart {
                                        id,
                                        name: name.to_string(),
                                    });
                                }
                            }
                            if let Some(args) = func.get("arguments").and_then(|a| a.as_str()) {
                                if !args.is_empty() {
                                    entry.2.push_str(args);
                                    let _ = tx.send(Delta::ToolArgs(args.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let tool_calls = calls
        .into_iter()
        .filter(|(_, (_, name, _))| !name.is_empty())
        .map(|(index, (id, name, arguments))| ToolCall {
            id: if id.is_empty() { format!("call_{index}") } else { id },
            name,
            arguments,
        })
        .collect();

    Ok(Reply { text, tool_calls })
}

fn to_wire(m: &Msg) -> Value {
    match m.role.as_str() {
        "tool" => json!({
            "role": "tool",
            "tool_call_id": m.tool_call_id.clone().unwrap_or_default(),
            "content": m.content,
        }),
        "assistant" if !m.tool_calls.is_empty() => json!({
            "role": "assistant",
            "content": m.content,
            "tool_calls": m.tool_calls.iter().map(|c| json!({
                "id": c.id,
                "type": "function",
                "function": { "name": c.name, "arguments": c.arguments },
            })).collect::<Vec<_>>(),
        }),
        other => json!({ "role": other, "content": m.content }),
    }
}

// Kept for symmetry with the Anthropic adapter; the OpenAI format carries the
// arguments as a string, so parsing happens at the call site.
#[allow(dead_code)]
fn args_of(call: &ToolCall) -> Value {
    parse_args(&call.arguments)
}
