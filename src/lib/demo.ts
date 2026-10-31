import type { Settings } from "./types";

/**
 * Browser-only demo mode. Lets contributors work on the UI without a Rust
 * toolchain, and gives the marketing screenshots something to show. Nothing
 * here runs in the packaged app.
 */
export const demoSettings: Settings = {
  providers: [
    {
      id: "openai",
      label: "OpenAI",
      kind: "openai",
      base_url: "https://api.openai.com/v1",
      model: "gpt-4o",
      api_key: "",
    },
  ],
  active_provider: "openai",
  workspace: "~/projects/kestrel",
  system_prompt: "You are Kestrel, an agent running on the user's own computer.",
  max_iterations: 25,
  approval_mode: "writes",
  mcp_servers: [],
};

type Emit = (event: string, payload: Record<string, unknown>) => void;

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function runDemo(emit: Emit, prompt: string, sessionId: string) {
  const say = async (text: string) => {
    for (const chunk of text.match(/.{1,14}/gs) ?? []) {
      emit("kestrel://delta", { sessionId, text: chunk });
      await wait(18);
    }
  };

  await say("Let me look at the project before changing anything.");
  await wait(300);

  emit("kestrel://tool_start", { sessionId, id: "demo-1", name: "list_dir" });
  await wait(500);
  emit("kestrel://assistant", {
    sessionId,
    text: "Let me look at the project before changing anything.",
    toolCalls: [
      { id: "demo-1", name: "list_dir", arguments: '{"path": "~/projects/kestrel"}' },
    ],
  });
  emit("kestrel://tool_result", {
    sessionId,
    id: "demo-1",
    name: "list_dir",
    ok: true,
    content:
      "src/\nsrc-tauri/\npackage.json\nREADME.md\nvite.config.ts\n.github/\n(6 entries)",
  });
  await wait(400);

  emit("kestrel://approval", {
    id: "demo-approval",
    sessionId,
    tool: "edit_file",
    arguments: JSON.stringify({
      path: "~/projects/kestrel/package.json",
      old_string: '"version": "0.1.0"',
      new_string: '"version": "0.1.1"',
    }),
    mutating: true,
  });
  await wait(1200);

  emit("kestrel://assistant", {
    sessionId,
    text: `Here's what I found. The repository is a Tauri v2 + React project, so the version bump belongs in \`package.json\`.

This is demo mode — no model was called and no file was touched. Run \`npm run tauri:dev\` with a Rust toolchain to talk to a real model.`,
    toolCalls: [
      {
        id: "demo-2",
        name: "edit_file",
        arguments: '{"path": "package.json", "old_string": "\\"version\\": \\"0.1.0\\"", "new_string": "\\"version\\": \\"0.1.1\\""}',
      },
    ],
  });
  emit("kestrel://tool_result", {
    sessionId,
    id: "demo-2",
    name: "edit_file",
    ok: true,
    content: "edited package.json (1 replacement)",
    diff: {
      path: "package.json",
      before: '{\n  "name": "kestrel",\n  "version": "0.1.0",\n  "private": true\n}',
      after: '{\n  "name": "kestrel",\n  "version": "0.1.1",\n  "private": true\n}',
    },
  });
  emit("kestrel://turn_done", { sessionId });
}
