//! The agent loop: call the model, run the tools it asks for, feed the results
//! back, repeat until it stops asking for tools.

use crate::config::ApprovalMode;
use crate::providers::{self, Delta, Msg, ToolCall};
use crate::state::AppState;
use crate::tools::{self, DiffPayload, ToolSpec};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::oneshot;

pub const EV_DELTA: &str = "kestrel://delta";
pub const EV_TOOL_START: &str = "kestrel://tool_start";
pub const EV_TOOL_ARGS: &str = "kestrel://tool_args";
pub const EV_ASSISTANT: &str = "kestrel://assistant";
pub const EV_TOOL_RESULT: &str = "kestrel://tool_result";
pub const EV_APPROVAL: &str = "kestrel://approval";
pub const EV_TURN_DONE: &str = "kestrel://turn_done";
pub const EV_ERROR: &str = "kestrel://error";

const APPROVAL_TIMEOUT_SECS: u64 = 600;
const UI_TEXT_CAP: usize = 4_000;
const UI_DIFF_LINES: usize = 400;

/// Entry point used by the `send_message` command. Always terminates the turn
/// cleanly so the UI never gets stuck in a "thinking" state.
pub async fn run_turn(app: AppHandle, session_id: String, text: String) {
    if let Err(err) = run_turn_inner(app.clone(), &session_id, text).await {
        let _ = app.emit(
            EV_ERROR,
            json!({ "sessionId": session_id, "message": err }),
        );
    }
    let _ = app.emit(EV_TURN_DONE, json!({ "sessionId": session_id }));
}

async fn run_turn_inner(
    app: AppHandle,
    session_id: &str,
    text: String,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let settings = { state.settings.lock().unwrap().clone() };

    let provider = settings
        .active()
        .cloned()
        .ok_or_else(|| "no model provider configured - open Settings and add one".to_string())?;

    let workspace = PathBuf::from(&settings.workspace);

    let mut history = {
        let sessions = state.sessions.lock().unwrap();
        sessions.get(session_id).cloned().unwrap_or_default()
    };
    history.push(Msg::user(text));

    let mut specs: Vec<ToolSpec> = tools::defs();
    specs.extend(state.mcp_tools.lock().unwrap().iter().cloned());

    let mut iterations: u32 = 0;

    loop {
        if iterations >= settings.max_iterations {
            persist(&state, session_id, &history);
            return Err(format!(
                "stopped after {} iterations - raise the limit in Settings if the task genuinely needs more",
                settings.max_iterations
            ));
        }
        iterations += 1;

        // Pump provider deltas to the UI while the response streams in.
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Delta>();
        let pump_app = app.clone();
        let pump_sid = session_id.to_string();
        let pump = tokio::spawn(async move {
            while let Some(delta) = rx.recv().await {
                match delta {
                    Delta::Text(t) => {
                        let _ = pump_app.emit(
                            EV_DELTA,
                            json!({ "sessionId": pump_sid, "text": t }),
                        );
                    }
                    Delta::ToolStart { id, name } => {
                        let _ = pump_app.emit(
                            EV_TOOL_START,
                            json!({ "sessionId": pump_sid, "id": id, "name": name }),
                        );
                    }
                    Delta::ToolArgs(a) => {
                        let _ = pump_app.emit(
                            EV_TOOL_ARGS,
                            json!({ "sessionId": pump_sid, "text": a }),
                        );
                    }
                }
            }
        });

        let result = providers::stream_chat(
            &provider,
            &settings.system_prompt,
            &history,
            &specs,
            tx,
        )
        .await;

        // The sender is dropped when stream_chat returns, so the pump drains.
        let _ = pump.await;
        let reply = result?;

        let calls = reply.tool_calls.clone();
        history.push(Msg::assistant(reply.text.clone(), calls.clone()));
        persist(&state, session_id, &history);

        let _ = app.emit(
            EV_ASSISTANT,
            json!({
                "sessionId": session_id,
                "text": reply.text,
                "toolCalls": calls,
            }),
        );

        if calls.is_empty() {
            return Ok(());
        }

        for call in &calls {
            let approved = request_approval(&app, &state, settings.approval_mode, session_id, call).await;

            if !approved {
                let note = "The user denied this tool call. Do not retry the same call. \
                            Explain what you were trying to do and ask how to proceed.";
                history.push(Msg::tool(call.id.clone(), note.to_string()));
                persist(&state, session_id, &history);
                let _ = app.emit(
                    EV_TOOL_RESULT,
                    json!({
                        "sessionId": session_id,
                        "id": call.id,
                        "name": call.name,
                        "ok": false,
                        "content": "Denied by the user.",
                    }),
                );
                continue;
            }

            let args = providers::parse_args(&call.arguments);

            let (content, diff): (String, Option<DiffPayload>) = if call
                .name
                .starts_with("mcp__")
            {
                match call_mcp(&state, &call.name, args).await {
                    Ok(text) => (text, None),
                    Err(err) => (format!("ERROR: {err}"), None),
                }
            } else {
                match tools::execute(&call.name, &args, &workspace).await {
                    Ok(outcome) => (outcome.content, outcome.diff),
                    Err(err) => (format!("ERROR: {err}"), None),
                }
            };

            history.push(Msg::tool(call.id.clone(), content.clone()));
            persist(&state, session_id, &history);

            let mut payload = json!({
                "sessionId": session_id,
                "id": call.id,
                "name": call.name,
                "ok": !content.starts_with("ERROR:"),
                "content": cap(&content, UI_TEXT_CAP),
            });
            if let Some(d) = &diff {
                payload["diff"] = json!({
                    "path": d.path,
                    "before": cap_lines(&d.before, UI_DIFF_LINES),
                    "after": cap_lines(&d.after, UI_DIFF_LINES),
                });
            }
            let _ = app.emit(EV_TOOL_RESULT, payload);
        }
    }
}

fn persist(state: &AppState, session_id: &str, history: &[Msg]) {
    state
        .sessions
        .lock()
        .unwrap()
        .insert(session_id.to_string(), history.to_vec());
}

async fn request_approval(
    app: &AppHandle,
    state: &AppState,
    mode: ApprovalMode,
    session_id: &str,
    call: &ToolCall,
) -> bool {
    let needs_prompt = match mode {
        ApprovalMode::Never => false,
        ApprovalMode::Always => true,
        // MCP tools are third-party code, so always confirm them in the default mode.
        ApprovalMode::Writes => tools::is_mutating(&call.name) || call.name.starts_with("mcp__"),
    };
    if !needs_prompt {
        return true;
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let id = format!("ap-{stamp}");

    let (tx, rx) = oneshot::channel();
    state.approvals.lock().unwrap().insert(id.clone(), tx);

    let _ = app.emit(
        EV_APPROVAL,
        json!({
            "id": id,
            "sessionId": session_id,
            "tool": call.name,
            "arguments": call.arguments,
            "mutating": tools::is_mutating(&call.name),
        }),
    );

    match tokio::time::timeout(Duration::from_secs(APPROVAL_TIMEOUT_SECS), rx).await {
        Ok(Ok(approved)) => approved,
        _ => {
            state.approvals.lock().unwrap().remove(&id);
            false
        }
    }
}

async fn call_mcp(state: &AppState, full_name: &str, args: Value) -> Result<String, String> {
    let rest = full_name.strip_prefix("mcp__").unwrap_or(full_name);
    let (server, tool) = rest
        .split_once("__")
        .ok_or_else(|| format!("malformed MCP tool name: {full_name}"))?;

    let client = {
        let map = state.mcp.lock().unwrap();
        map.get(server).cloned()
    }
    .ok_or_else(|| format!("MCP server `{server}` is not connected"))?;

    let mut guard = client.lock().await;
    let result = guard.call_tool(tool, args).await?;
    Ok(summarize_mcp(&result))
}

fn summarize_mcp(value: &Value) -> String {
    if let Some(blocks) = value.get("content").and_then(|c| c.as_array()) {
        let mut out = String::new();
        for block in blocks {
            match block.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                "text" => {
                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                        out.push_str(text);
                        out.push('\n');
                    }
                }
                other => out.push_str(&format!("[{other} content omitted]\n")),
            }
        }
        if !out.trim().is_empty() {
            return out;
        }
    }
    value.to_string()
}

fn cap(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}\n[...]")
}

fn cap_lines(s: &str, max: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= max {
        return s.to_string();
    }
    let mut out = lines[..max].join("\n");
    out.push_str(&format!("\n[... {} more lines]", lines.len() - max));
    out
}
