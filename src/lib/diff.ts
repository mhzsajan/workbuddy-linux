export type DiffLineType = "add" | "del" | "ctx";

export interface DiffLine {
  type: DiffLineType;
  text: string;
}

const MAX_LINES = 3000;

/**
 * Line-level diff via longest common subsequence. Files are small here (the
 * backend caps payloads), so the quadratic table is fine; oversized inputs
 * fall back to a whole-file replace rather than stalling the UI.
 */
export function diffLines(before: string, after: string): DiffLine[] {
  const a = before.length ? before.split("\n") : [];
  const b = after.length ? after.split("\n") : [];

  if (a.length > MAX_LINES || b.length > MAX_LINES) {
    return [
      ...a.map<DiffLine>((text) => ({ type: "del", text })),
      ...b.map<DiffLine>((text) => ({ type: "add", text })),
    ];
  }

  const n = a.length;
  const m = b.length;
  const table: Uint32Array[] = Array.from(
    { length: n + 1 },
    () => new Uint32Array(m + 1),
  );

  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      table[i][j] =
        a[i] === b[j]
          ? table[i + 1][j + 1] + 1
          : Math.max(table[i + 1][j], table[i][j + 1]);
    }
  }

  const out: DiffLine[] = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      out.push({ type: "ctx", text: a[i] });
      i++;
      j++;
    } else if (table[i + 1][j] >= table[i][j + 1]) {
      out.push({ type: "del", text: a[i] });
      i++;
    } else {
      out.push({ type: "add", text: b[j] });
      j++;
    }
  }
  while (i < n) {
    out.push({ type: "del", text: a[i] });
    i++;
  }
  while (j < m) {
    out.push({ type: "add", text: b[j] });
    j++;
  }

  return out;
}

export function diffStats(lines: DiffLine[]): { added: number; removed: number } {
  return lines.reduce(
    (acc, line) => {
      if (line.type === "add") acc.added++;
      if (line.type === "del") acc.removed++;
      return acc;
    },
    { added: 0, removed: 0 },
  );
}
