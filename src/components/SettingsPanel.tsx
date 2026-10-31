import { useState } from "react";
import type { ProviderConfig, Settings } from "../lib/types";

interface Props {
  settings: Settings;
  onClose: () => void;
  onSave: (settings: Settings) => Promise<void>;
}

export function SettingsPanel({ settings, onClose, onSave }: Props) {
  const [draft, setDraft] = useState<Settings>(settings);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const active =
    draft.providers.find((p) => p.id === draft.active_provider) ??
    draft.providers[0];

  const patchProvider = (patch: Partial<ProviderConfig>) => {
    setDraft((d) => ({
      ...d,
      providers: d.providers.map((p) =>
        p.id === d.active_provider ? { ...p, ...patch } : p,
      ),
    }));
  };

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await onSave(draft);
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="overlay" role="dialog" aria-modal="true">
      <div className="sheet">
        <header className="sheet-head">
          <h2>Settings</h2>
          <button className="icon-btn" onClick={onClose} aria-label="Close">
            ×
          </button>
        </header>

        <div className="sheet-body">
          <section className="field-group">
            <h3>Model</h3>
            <label className="field">
              <span>Provider</span>
              <select
                value={draft.active_provider}
                onChange={(e) =>
                  setDraft((d) => ({ ...d, active_provider: e.target.value }))
                }
              >
                {draft.providers.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.label}
                  </option>
                ))}
              </select>
            </label>

            <label className="field">
              <span>Base URL</span>
              <input
                value={active?.base_url ?? ""}
                onChange={(e) => patchProvider({ base_url: e.target.value })}
                placeholder="https://api.openai.com/v1"
              />
            </label>

            <label className="field">
              <span>Model</span>
              <input
                value={active?.model ?? ""}
                onChange={(e) => patchProvider({ model: e.target.value })}
                placeholder="gpt-4o"
              />
            </label>

            <label className="field">
              <span>API key</span>
              <input
                type="password"
                value={active?.api_key ?? ""}
                onChange={(e) => patchProvider({ api_key: e.target.value })}
                placeholder="sk-..."
              />
              <small>
                Stored in <code>settings.json</code> with owner-only permissions.
                Leave empty for a local server such as Ollama.
              </small>
            </label>
          </section>

          <section className="field-group">
            <h3>Workspace</h3>
            <label className="field">
              <span>Root directory</span>
              <input
                value={draft.workspace}
                onChange={(e) =>
                  setDraft((d) => ({ ...d, workspace: e.target.value }))
                }
                placeholder="/home/you/projects"
              />
              <small>
                Relative tool paths resolve against this directory. Absolute
                paths are still allowed.
              </small>
            </label>
          </section>

          <section className="field-group">
            <h3>Behaviour</h3>
            <label className="field">
              <span>Ask before</span>
              <select
                value={draft.approval_mode}
                onChange={(e) =>
                  setDraft((d) => ({
                    ...d,
                    approval_mode: e.target.value as Settings["approval_mode"],
                  }))
                }
              >
                <option value="writes">writes and commands (recommended)</option>
                <option value="always">every tool call</option>
                <option value="never">never - unsafe, sandboxes only</option>
              </select>
            </label>

            <label className="field">
              <span>Max tool iterations per turn</span>
              <input
                type="number"
                min={1}
                max={200}
                value={draft.max_iterations}
                onChange={(e) =>
                  setDraft((d) => ({
                    ...d,
                    max_iterations: Number(e.target.value) || 1,
                  }))
                }
              />
            </label>

            <label className="field">
              <span>System prompt</span>
              <textarea
                rows={6}
                value={draft.system_prompt}
                onChange={(e) =>
                  setDraft((d) => ({ ...d, system_prompt: e.target.value }))
                }
              />
            </label>
          </section>

          {error ? <div className="error-box">{error}</div> : null}
        </div>

        <footer className="sheet-foot">
          <button className="btn ghost" onClick={onClose}>
            Cancel
          </button>
          <button className="btn primary" onClick={save} disabled={saving}>
            {saving ? "Saving" : "Save"}
          </button>
        </footer>
      </div>
    </div>
  );
}
