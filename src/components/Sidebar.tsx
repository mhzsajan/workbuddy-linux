import type { Settings } from "../lib/types";

interface Props {
  settings: Settings | null;
  onNewSession: () => void;
  onOpenSettings: () => void;
  onOpenMcp: () => void;
  mcpCount: number;
  demoTauri: boolean;
}

export function Sidebar({
  settings,
  onNewSession,
  onOpenSettings,
  onOpenMcp,
  mcpCount,
  demoTauri,
}: Props) {
  const active = settings?.providers.find(
    (p) => p.id === settings.active_provider,
  );

  return (
    <aside className="sidebar">
      <div className="brand">
        <span className="brand-mark" aria-hidden="true" />
        <div>
          <div className="brand-name">Kestrel</div>
          <div className="brand-tag">agent workbench</div>
        </div>
      </div>

      <button className="btn new-session" onClick={onNewSession}>
        <span>+</span> New session
      </button>

      <div className="side-group">
        <div className="side-label">Workspace</div>
        <div className="side-value mono" title={settings?.workspace}>
          {settings?.workspace ?? "-"}
        </div>
      </div>

      <div className="side-group">
        <div className="side-label">Model</div>
        <div className="side-value">
          {active ? `${active.label} · ${active.model}` : "not configured"}
        </div>
      </div>

      <div className="side-group">
        <div className="side-label">Approvals</div>
        <div className="side-value">
          {settings?.approval_mode === "always"
            ? "every tool call"
            : settings?.approval_mode === "never"
              ? "never (unsafe)"
              : "writes only"}
        </div>
      </div>

      <button className="side-link" onClick={onOpenMcp}>
        MCP servers
        <span className="side-count">{mcpCount}</span>
      </button>

      <button className="side-link" onClick={onOpenSettings}>
        Settings
      </button>

      <div className="sidebar-foot">
        {demoTauri ? (
          <span className="pill warn">browser preview</span>
        ) : (
          <span className="pill">v0.1.0 · MIT</span>
        )}
      </div>
    </aside>
  );
}
