import { useMemo } from "react";
import { diffLines, diffStats } from "../lib/diff";

interface Props {
  path: string;
  before: string;
  after: string;
}

export function DiffView({ path, before, after }: Props) {
  const lines = useMemo(() => diffLines(before, after), [before, after]);
  const stats = useMemo(() => diffStats(lines), [lines]);

  return (
    <div className="diff">
      <div className="diff-head">
        <span className="diff-path">{path}</span>
        <span className="diff-stats">
          <span className="plus">+{stats.added}</span>
          <span className="minus">-{stats.removed}</span>
        </span>
      </div>
      <pre className="diff-body">
        {lines.map((line, i) => (
          <div className={`diff-line ${line.type}`} key={i}>
            <span className="diff-gutter">
              {line.type === "add" ? "+" : line.type === "del" ? "-" : " "}
            </span>
            <span className="diff-text">{line.text || " "}</span>
          </div>
        ))}
      </pre>
    </div>
  );
}
