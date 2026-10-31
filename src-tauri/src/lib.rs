//! Kestrel - an open-source, Linux-first agent workbench.
//!
//! The Rust side owns everything privileged: the model calls, the tool
//! execution, and the approval gate. The frontend is a thin, honest view of it.

mod agent;
mod config;
mod mcp;
mod providers;
mod state;
mod tools;

use crate::mcp::McpClient;
use crate::state::AppState;
use crate::tools::ToolSpec;
use serde_json::json;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex as AsyncMutex;

#[tauri::command]
fn load_settings(state: State<AppState>) -> config::Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: config::Settings) -> Result<(), String> {
    settings.save()?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
fn get_history(state: State<AppState>, session_id: String) -> Vec<providers::Msg> {
    state
        .sessions
        .lock()
        .unwrap()
        .get(&session_id)
        .cloned()
        .unwrap_or_default()
}

#[tauri::command]
fn reset_session(state: State<AppState>, session_id: String) {
    state.sessions.lock().unwrap().remove(&session_id);
}

#[tauri::command]
async fn send_message(
    app: tauri::AppHandle,
    session_id: String,
    text: String,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("message is empty".into());
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        agent::run_turn(handle, session_id, text).await;
    });
    Ok(())
}

#[tauri::command]
fn respond_approval(state: State<AppState>, id: String, approved: bool) -> Result<(), String> {
    let sender = state.approvals.lock().unwrap().remove(&id);
    match sender {
        Some(tx) => {
            let _ = tx.send(approved);
            Ok(())
        }
        None => Err("this approval request has expired".into()),
    }
}

/// Connect to an MCP server, discover its tools, and register them so the agent
/// can call them as `mcp__<server>__<tool>`.
#[tauri::command]
async fn mcp_connect(
    state: State<'_, AppState>,
    name: String,
    command: String,
    args: Vec<String>,
) -> Result<Vec<String>, String> {
    if name.trim().is_empty() {
        return Err("server name is required".into());
    }

    let mut client = McpClient::spawn(&command, &args).await?;
    client.initialize().await?;
    let tools = client.list_tools().await?;

    let mut specs: Vec<ToolSpec> = Vec::new();
    let mut registered: Vec<String> = Vec::new();

    for tool in &tools {
        let tool_name = tool
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or_default()
            .to_string();
        if tool_name.is_empty() {
            continue;
        }
        let description = tool
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        let schema = tool
            .get("inputSchema")
            .cloned()
            .unwrap_or_else(|| json!({ "type": "object", "properties": {} }));

        let full_name = format!("mcp__{name}__{tool_name}");
        registered.push(full_name.clone());
        specs.push(ToolSpec {
            name: full_name,
            description,
            schema,
        });
    }

    state
        .mcp
        .lock()
        .unwrap()
        .insert(name.clone(), Arc::new(AsyncMutex::new(client)));

    let prefix = format!("mcp__{name}__");
    {
        let mut registry = state.mcp_tools.lock().unwrap();
        registry.retain(|s| !s.name.starts_with(&prefix));
        registry.extend(specs);
    }

    {
        let mut settings = state.settings.lock().unwrap();
        if !settings.mcp_servers.iter().any(|s| s.name == name) {
            settings.mcp_servers.push(config::McpServerConfig {
                name: name.clone(),
                command,
                args,
                enabled: true,
            });
            settings.save()?;
        }
    }

    Ok(registered)
}

#[tauri::command]
fn mcp_disconnect(state: State<AppState>, name: String) -> Result<(), String> {
    state.mcp.lock().unwrap().remove(&name);
    let prefix = format!("mcp__{name}__");
    state
        .mcp_tools
        .lock()
        .unwrap()
        .retain(|s| !s.name.starts_with(&prefix));
    Ok(())
}

#[tauri::command]
fn mcp_connected(state: State<AppState>) -> Vec<String> {
    state.mcp.lock().unwrap().keys().cloned().collect()
}

pub fn run() {
    let settings = config::Settings::load();

    tauri::Builder::default()
        .manage(AppState::new(settings))
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            get_history,
            reset_session,
            send_message,
            respond_approval,
            mcp_connect,
            mcp_disconnect,
            mcp_connected,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Kestrel");
}
