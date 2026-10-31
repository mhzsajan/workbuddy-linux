import type { PendingApproval } from "../lib/types";

interface Props {
  approval: PendingApproval;
  onRespond: (approved: boolean) => void;
}

const RISK: Record<string, string> = {
  run_shell: "This runs a command on your machine.",
  write_file: "This writes a file on your machine.",
  edit_file: "This modifies a file on your machine.",
};

export function ApprovalDialog({ approval, onRespond }: Props) {
  const isMcp = approval.tool.startsWith("mcp__");
  const note =
    RISK[approval.tool] ??
    (isMcp
      ? "This calls a third-party MCP server, which may act outside this app."
      : "The agent wants to use a tool.");

  return (
    <div className="overlay" role="dialog" aria-modal="true">
      <div className="approval">
        <div className="approval-head">
          <span className="approval-kicker">Approval required</span>
          <h2>{approval.tool}</h2>
          <p>{note}</p>
        </div>

        <pre className="approval-args">{pretty(approval.arguments)}</pre>

        <div className="approval-actions">
          <button className="btn ghost" onClick={() => onRespond(false)}>
            Deny
          </button>
          <button className="btn primary" onClick={() => onRespond(true)} autoFocus>
            Allow once
          </button>
        </div>
        <p className="approval-foot">
          Kestrel never remembers an approval. Every mutating call asks again.
        </p>
      </div>
    </div>
  );
}

function pretty(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}
