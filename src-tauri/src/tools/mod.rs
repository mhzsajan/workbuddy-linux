//! The tool layer. Every capability the agent has on the machine lives here,
//! and every mutating tool is flagged so the agent loop can gate it behind
//! an approval prompt.

use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use walkdir::WalkDir;

const MAX_OUTPUT_CHARS: usize = 20_000;
const DEFAULT_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Clone, Serialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub schema: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffPayload {
    pub path: String,
    pub before: String,
    pub after: String,
}

pub struct ToolOutcome {
    pub content: String,
    pub diff: Option<DiffPayload>,
}

/// Tools that change the machine. Used by the approval gate.
pub fn is_mutating(name: &str) -> bool {
    matches!(name, "run_shell" | "write_file" | "edit_file")
}

pub fn is_known(name: &str) -> bool {
    defs().iter().any(|d| d.name == name)
}

pub fn defs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "run_shell".into(),
            description: "Run a shell command on the user's machine and return its exit code, stdout and stderr. Use this for builds, tests, git, package managers and anything else a terminal would do. On Unix it runs via `sh -lc`, on Windows via `cmd /C`.".into(),
            schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "The command line to execute." },
                    "cwd": { "type": "string", "description": "Working directory. Defaults to the workspace root." },
                    "timeout_secs": { "type": "integer", "description": "Kill the command after this many seconds. Default 120." }
                },
                "required": ["command"]
            }),
        },
        ToolSpec {
            name: "read_file".into(),
            description: "Read a text file. Returns the content with 1-based line numbers so you can reference lines precisely. Always read a file before editing it.".into(),
            schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute path, or a path relative to the workspace root." },
                    "offset": { "type": "integer", "description": "First line to return (1-based)." },
                    "limit": { "type": "integer", "description": "Maximum number of lines to return." }
                },
                "required": ["path"]
            }),
        },
        ToolSpec {
            name: "write_file".into(),
            description: "Create a file or overwrite it entirely. Parent directories are created automatically. Prefer edit_file for changes to existing files.".into(),
            schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute path, or a path relative to the workspace root." },
                    "content": { "type": "string", "description": "The complete file content." }
                },
                "required": ["path", "content"]
            }),
        },
        ToolSpec {
            name: "edit_file".into(),
            description: "Replace an exact string in an existing file. The old_string must appear verbatim, including indentation. Include enough surrounding context to make it unique.".into(),
            schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute path, or a path relative to the workspace root." },
                    "old_string": { "type": "string", "description": "The exact text to replace." },
                    "new_string": { "type": "string", "description": "The replacement text." },
                    "replace_all": { "type": "boolean", "description": "Replace every occurrence instead of requiring exactly one." }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        },
        ToolSpec {
            name: "list_dir".into(),
            description: "List the entries of a directory, one level deep by default. Useful for orienting yourself in a project.".into(),
            schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory to list. Defaults to the workspace root." },
                    "depth": { "type": "integer", "description": "How many levels to descend. Default 1, max 3." }
                },
                "required": []
            }),
        },
        ToolSpec {
            name: "search_files".into(),
            description: "Search file contents with a regular expression, walking a directory tree. Returns matching lines as path:line: text. Skips .git, node_modules and target.".into(),
            schema: json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Rust/PCRE-style regular expression." },
                    "path": { "type": "string", "description": "Directory to search. Defaults to the workspace root." },
                    "name_filter": { "type": "string", "description": "Only search files whose name contains this substring, e.g. \".ts\"." },
                    "max_results": { "type": "integer", "description": "Stop after this many matches. Default 80." }
                },
                "required": ["pattern"]
            }),
        },
        ToolSpec {
            name: "web_fetch".into(),
            description: "Fetch a URL and return its text content with HTML tags stripped. Use this to read documentation or check something on the web.".into(),
            schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The http or https URL to fetch." }
                },
                "required": ["url"]
            }),
        },
    ]
}

pub async fn execute(name: &str, args: &Value, workspace: &Path) -> Result<ToolOutcome, String> {
    match name {
        "run_shell" => run_shell(args, workspace).await,
        "read_file" => read_file(args, workspace),
        "write_file" => write_file(args, workspace),
        "edit_file" => edit_file(args, workspace),
        "list_dir" => list_dir(args, workspace),
        "search_files" => search_files(args, workspace),
        "web_fetch" => web_fetch(args).await,
        other => Err(format!("unknown tool: {other}")),
    }
}

// ---------------------------------------------------------------- helpers

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing required argument `{key}`"))
}

fn resolve(path: &str, workspace: &Path) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        workspace.join(p)
    }
}

fn truncate(s: &str) -> String {
    if s.chars().count() <= MAX_OUTPUT_CHARS {
        return s.to_string();
    }
    let kept: String = s.chars().take(MAX_OUTPUT_CHARS).collect();
    format!("{kept}\n\n[truncated at {MAX_OUTPUT_CHARS} characters]")
}

fn is_ignored_dir(name: &str) -> bool {
    matches!(
        name,
        ".git" | "node_modules" | "target" | "dist" | ".venv" | "__pycache__" | ".next"
    )
}

// ---------------------------------------------------------------- shell

async fn run_shell(args: &Value, workspace: &Path) -> Result<ToolOutcome, String> {
    let command = str_arg(args, "command")?;
    let cwd = match args.get("cwd").and_then(|v| v.as_str()) {
        Some(c) => resolve(c, workspace),
        None => workspace.to_path_buf(),
    };
    let timeout = args
        .get("timeout_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TIMEOUT_SECS);

    if !cwd.exists() {
        return Err(format!("working directory does not exist: {}", cwd.display()));
    }

    let mut cmd = if cfg!(windows) {
        let mut c = tokio::process::Command::new("cmd");
        c.arg("/C").arg(command);
        c
    } else {
        let mut c = tokio::process::Command::new("sh");
        c.arg("-lc").arg(command);
        c
    };
    cmd.current_dir(&cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    let output = match tokio::time::timeout(Duration::from_secs(timeout), cmd.output()).await {
        Err(_) => return Err(format!("command timed out after {timeout}s: {command}")),
        Ok(Err(e)) => return Err(format!("failed to spawn command: {e}")),
        Ok(Ok(out)) => out,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output
        .status
        .code()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "terminated by signal".to_string());

    let mut content = format!("exit_code: {code}\n");
    if !stdout.trim().is_empty() {
        content.push_str("\n--- stdout ---\n");
        content.push_str(&stdout);
    }
    if !stderr.trim().is_empty() {
        content.push_str("\n--- stderr ---\n");
        content.push_str(&stderr);
    }
    if stdout.trim().is_empty() && stderr.trim().is_empty() {
        content.push_str("(no output)");
    }

    Ok(ToolOutcome {
        content: truncate(&content),
        diff: None,
    })
}

// ---------------------------------------------------------------- files

fn read_file(args: &Value, workspace: &Path) -> Result<ToolOutcome, String> {
    let path = resolve(str_arg(args, "path")?, workspace);
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;

    let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(1).max(1) as usize;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(2_000) as usize;

    let mut out = String::new();
    let mut shown = 0usize;
    for (i, line) in raw.lines().enumerate() {
        let n = i + 1;
        if n < offset {
            continue;
        }
        if shown >= limit {
            out.push_str("\n[more lines available; increase `limit` to see them]");
            break;
        }
        out.push_str(&format!("{n:>6}\t{line}\n"));
        shown += 1;
    }

    if out.is_empty() {
        out.push_str("(file is empty)");
    }

    Ok(ToolOutcome {
        content: truncate(&format!("{}\n\n{out}", path.display())),
        diff: None,
    })
}

fn write_file(args: &Value, workspace: &Path) -> Result<ToolOutcome, String> {
    let path = resolve(str_arg(args, "path")?, workspace);
    let content = str_arg(args, "content")?;

    let before = std::fs::read_to_string(&path).unwrap_or_default();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    std::fs::write(&path, content)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;

    let verb = if before.is_empty() { "created" } else { "overwrote" };
    let lines = content.lines().count();

    Ok(ToolOutcome {
        content: format!("{verb} {} ({lines} lines)", path.display()),
        diff: Some(DiffPayload {
            path: path.display().to_string(),
            before,
            after: content.to_string(),
        }),
    })
}

fn edit_file(args: &Value, workspace: &Path) -> Result<ToolOutcome, String> {
    let path = resolve(str_arg(args, "path")?, workspace);
    let old = str_arg(args, "old_string")?;
    let new = str_arg(args, "new_string")?;
    let replace_all = args
        .get("replace_all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let before = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;

    let occurrences = before.matches(old).count();
    if occurrences == 0 {
        return Err(format!(
            "old_string not found in {} - read the file again and copy the text verbatim",
            path.display()
        ));
    }
    if occurrences > 1 && !replace_all {
        return Err(format!(
            "old_string appears {occurrences} times in {}; add surrounding context or set replace_all",
            path.display()
        ));
    }

    let after = if replace_all {
        before.replace(old, new)
    } else {
        before.replacen(old, new, 1)
    };

    std::fs::write(&path, &after)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;

    Ok(ToolOutcome {
        content: format!(
            "edited {} ({occurrences} replacement{})",
            path.display(),
            if occurrences == 1 { "" } else { "s" }
        ),
        diff: Some(DiffPayload {
            path: path.display().to_string(),
            before,
            after,
        }),
    })
}

fn list_dir(args: &Value, workspace: &Path) -> Result<ToolOutcome, String> {
    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => resolve(p, workspace),
        None => workspace.to_path_buf(),
    };
    let depth = args
        .get("depth")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .clamp(1, 3) as usize;

    if !path.is_dir() {
        return Err(format!("not a directory: {}", path.display()));
    }

    let mut out = String::new();
    let walker = WalkDir::new(&path).max_depth(depth).sort_by_file_name();
    for entry in walker.into_iter().flatten() {
        if entry.path() == path {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if entry.file_type().is_dir() && is_ignored_dir(&name) {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(&path)
            .unwrap_or(entry.path())
            .display()
            .to_string();
        if entry.file_type().is_dir() {
            out.push_str(&format!("{rel}/\n"));
        } else {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            out.push_str(&format!("{rel}  ({size} bytes)\n"));
        }
    }

    if out.is_empty() {
        out.push_str("(empty directory)");
    }

    Ok(ToolOutcome {
        content: truncate(&format!("{}\n\n{out}", path.display())),
        diff: None,
    })
}

fn search_files(args: &Value, workspace: &Path) -> Result<ToolOutcome, String> {
    let pattern = str_arg(args, "pattern")?;
    let re = Regex::new(pattern).map_err(|e| format!("invalid regex: {e}"))?;
    let root = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => resolve(p, workspace),
        None => workspace.to_path_buf(),
    };
    let name_filter = args.get("name_filter").and_then(|v| v.as_str());
    let max_results = args
        .get("max_results")
        .and_then(|v| v.as_u64())
        .unwrap_or(80) as usize;

    let mut out = String::new();
    let mut hits = 0usize;

    'outer: for entry in WalkDir::new(&root).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if let Some(filter) = name_filter {
            if !name.contains(filter) {
                continue;
            }
        }
        if entry
            .path()
            .components()
            .any(|c| is_ignored_dir(&c.as_os_str().to_string_lossy()))
        {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        for (i, line) in raw.lines().enumerate() {
            if re.is_match(line) {
                out.push_str(&format!(
                    "{}:{}: {}\n",
                    entry.path().display(),
                    i + 1,
                    line.trim_end()
                ));
                hits += 1;
                if hits >= max_results {
                    out.push_str(&format!("\n[stopped at {max_results} matches]"));
                    break 'outer;
                }
            }
        }
    }

    if out.is_empty() {
        out.push_str("no matches");
    }

    Ok(ToolOutcome {
        content: truncate(&out),
        diff: None,
    })
}

// ---------------------------------------------------------------- web

async fn web_fetch(args: &Value) -> Result<ToolOutcome, String> {
    let url = str_arg(args, "url")?;
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("only http and https URLs are supported".into());
    }

    let client = reqwest::Client::builder()
        .user_agent("kestrel/0.1 (+https://github.com/mhzsajan/kestrel)")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("{} returned {}", url, resp.status()));
    }
    let body = resp.text().await.map_err(|e| e.to_string())?;

    // Crude but dependency-free readability pass: drop script/style blocks,
    // strip tags, collapse whitespace.
    let script = Regex::new(r"(?is)<(script|style)[^>]*>.*?</(script|style)>").unwrap();
    let tags = Regex::new(r"(?s)<[^>]+>").unwrap();
    let spaces = Regex::new(r"[ \t]+").unwrap();
    let blanks = Regex::new(r"\n{3,}").unwrap();

    let stripped = script.replace_all(&body, " ");
    let stripped = tags.replace_all(&stripped, " ");
    let stripped = spaces.replace_all(&stripped, " ");
    let stripped = blanks.replace_all(&stripped, "\n\n");
    let text = stripped.trim().to_string();

    if text.is_empty() {
        return Ok(ToolOutcome {
            content: format!("{url} returned no readable text ({} bytes of markup)", body.len()),
            diff: None,
        });
    }

    Ok(ToolOutcome {
        content: truncate(&text),
        diff: None,
    })
}
