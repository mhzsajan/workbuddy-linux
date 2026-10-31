import { useState } from "react";
import type { ToolResult } from "../lib/types";
import { DiffView } from "./DiffView";

interface Props {
  id: string;
  name: string;
  args: string;
  result?: ToolResult;
  running?: boolean;
}

const MUTATING = new Set(["run_shell", "write_file", "edit_file"]);

/** A short, human-readable one-liner describing what the tool is doing. */
function summarise(name: string, rawArgs: string): string {
  let parsed: Record<string, unknown> = {};
  try {
    parsed = rawArgs ? JSON.parse(rawArgs) : {};
  } catch {
    return rawArgs.slice(0, 90);
  }
  const pick = (key: string) =>
    typeof parsed[key] === "string" ? (parsed[key] as string) : undefined;

  switch (name) {
    case "run_shell":
      return pick("command") ?? "";
    case "read_file":
    case "write_file":
    case "edit_file":
      return pick("path") ?? "";
    case "list_dir":
      return pick("path") ?? "workspace root";
    case "search_files":
      return pick("pattern") ?? "";
    case "web_fetch":
      return pick("url") ?? "";
    default: {
      const first = Object.values(parsed)[0];
      return typeof first === "string" ? first : "";
    }
  }
}

export function ToolCallCard({ id, name, args, result, running }: Props) {
  const [open, setOpen] = useState(false);
  const ok = result?.ok ?? true;
  const status = running ? "running" : ok ? "done" : "failed";

  return (
    <div className={`tool-card ${status}`}>
      <button className="tool-head" onClick={() => setOpen((v) => !v)}>
        <span className={`tool-dot ${status}`} />
        <span className="tool-name">{name}</span>
        <span className="tool-summary">{summarise(name, args)}</span>
        {MUTATING.has(name) ? <span className="tool-badge">write</span> : null}
        <span className={`tool-status ${status}`}>
          {running ? "running" : ok ? "ok" : "failed"}
        </span>
        <span className={`chevron ${open ? "open" : ""}`}>›</span>
      </button>

      {open ? (
        <div className="tool-body">
          <div className="tool-section">
            <span className="tool-label">arguments</span>
            <pre className="tool-pre">{prettyArgs(args)}</pre>
          </div>
          {result ? (
            <div className="tool-section">
              <span className="tool-label">result</span>
              <pre className="tool-pre">{result.content}</pre>
            </div>
          ) : null}
          {result?.diff ? (
            <DiffView
              path={result.diff.path}
              before={result.diff.before}
              after={result.diff.after}
            />
          ) : null}
        </div>
      ) : result?.diff ? (
        <div className="tool-diff-hint">
          changed <code>{result.diff.path}</code> - expand to review
        </div>
      ) : null}
      <span hidden data-tool-id={id} />
    </div>
  );
}

function prettyArgs(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw || "(none)";
  }
}
