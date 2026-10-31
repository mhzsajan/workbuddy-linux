import { useState } from "react";
import type { McpServerConfig } from "../lib/types";

interface Props {
  servers: McpServerConfig[];
  connected: string[];
  onConnect: (name: string, command: string, args: string[]) => Promise<string[]>;
  onDisconnect: (name: string) => Promise<void>;
  onClose: () => void;
}

export function McpPanel({ servers, connected, onConnect, onDisconnect, onClose }: Props) {
  const [name, setName] = useState("");
  const [command, setCommand] = useState("");
  const [args, setArgs] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tools, setTools] = useState<string[]>([]);

  const connect = async () => {
    if (!name.trim() || !command.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const parsed = args.trim().length ? args.trim().split(/\s+/) : [];
      const discovered = await onConnect(name.trim(), command.trim(), parsed);
      setTools(discovered);
      setName("");
      setCommand("");
      setArgs("");
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="overlay" role="dialog" aria-modal="true">
      <div className="sheet">
        <header className="sheet-head">
          <h2>MCP servers</h2>
          <button className="icon-btn" onClick={onClose} aria-label="Close">
            ×
          </button>
        </header>

        <div className="sheet-body">
          <p className="sheet-note">
            Kestrel speaks the Model Context Protocol over stdio. Each tool a
            server exposes becomes <code>mcp__&lt;server&gt;__&lt;tool&gt;</code> and is
            still gated behind an approval prompt.
          </p>

          <section className="field-group">
            <h3>Add a server</h3>
            <label className="field">
              <span>Name</span>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="filesystem"
              />
            </label>
            <label className="field">
              <span>Command</span>
              <input
                value={command}
                onChange={(e) => setCommand(e.target.value)}
                placeholder="npx"
              />
            </label>
            <label className="field">
              <span>Arguments</span>
              <input
                value={args}
                onChange={(e) => setArgs(e.target.value)}
                placeholder="-y @modelcontextprotocol/server-filesystem /home/you"
              />
            </label>
            <button className="btn primary" onClick={connect} disabled={busy}>
              {busy ? "Connecting" : "Connect"}
            </button>
            {error ? <div className="error-box">{error}</div> : null}
            {tools.length > 0 ? (
              <div className="tools-found">
                <span className="tool-label">discovered</span>
                <ul>
                  {tools.map((t) => (
                    <li key={t}>
                      <code>{t}</code>
                    </li>
                  ))}
                </ul>
              </div>
            ) : null}
          </section>

          <section className="field-group">
            <h3>Configured</h3>
            {servers.length === 0 ? (
              <p className="muted">No servers yet.</p>
            ) : (
              <ul className="server-list">
                {servers.map((s) => {
                  const live = connected.includes(s.name);
                  return (
                    <li key={s.name}>
                      <div>
                        <strong>{s.name}</strong>
                        <div className="mono muted small">
                          {s.command} {s.args.join(" ")}
                        </div>
                      </div>
                      <span className={`pill ${live ? "ok" : ""}`}>
                        {live ? "connected" : "saved"}
                      </span>
                      {live ? (
                        <button
                          className="btn ghost small"
                          onClick={() => onDisconnect(s.name)}
                        >
                          Disconnect
                        </button>
                      ) : null}
                    </li>
                  );
                })}
              </ul>
            )}
          </section>
        </div>

        <footer className="sheet-foot">
          <button className="btn ghost" onClick={onClose}>
            Done
          </button>
        </footer>
      </div>
    </div>
  );
}
