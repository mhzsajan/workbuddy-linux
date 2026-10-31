# Kestrel

**An open-source agent workbench for the desktop — built Linux-first.**

[![build](https://github.com/mhzsajan/kestrel/actions/workflows/build.yml/badge.svg)](https://github.com/mhzsajan/kestrel/actions/workflows/build.yml)
[![license: MIT](https://img.shields.io/badge/license-MIT-b0531f.svg)](LICENSE)

Kestrel runs a tool-using AI agent against your own machine. It reads files,
runs commands, edits code and searches the web — and it asks before it changes
anything.

Most desktop agent workbenches ship for macOS and Windows only, leaving Linux
users with a browser tab. Kestrel targets Linux as a first-class platform:
`.deb`, `.rpm` and AppImage builds are produced on every push.

> **Status: v0.1.0, pre-release.** The interfaces are still moving and there are
> no automated tests yet. Treat it as a foundation to build on, not a finished
> product.

---

## Why another agent app?

Because the interesting part is not the chat box — it is the permission model.
Kestrel is built around one rule:

> **Every action that can change your machine must be approved, one call at a time.**

Nothing is remembered, nothing is whitelisted, and the approval gate lives in
the Rust process, not in the UI. A prompt-injected model cannot click its own
"Allow" button.

## Features

- **Real tool use** — shell, read, write, edit, list, search, fetch.
- **Approval gate** — mutating tools pause and show you the exact arguments
  before they run. Configurable: every call, writes only (default), or never.
- **Diff preview** — every write or edit is rendered as a line diff, so you see
  what changed before it lands.
- **Provider agnostic** — OpenAI-compatible endpoints (OpenAI, Ollama, LM Studio,
  vLLM, DeepSeek, Qwen, Moonshot, …) and the Anthropic Messages API.
- **MCP support** — connect Model Context Protocol servers over stdio; each tool
  becomes `mcp__<server>__<tool>`, still behind the approval gate.
- **Small footprint** — Tauri rather than Electron: a native webview, ~10 MB
  binaries, no bundled Chromium.
- **Local settings** — API keys stay in a `0600` JSON file on your disk. No
  account, no telemetry, no cloud round-trip.

## Install

### Linux (recommended)

Download the latest `.deb`, `.rpm` or AppImage from
[Releases](https://github.com/mhzsajan/kestrel/releases).

```bash
# Debian / Ubuntu
sudo apt install ./kestrel_0.1.0_amd64.deb

# Fedora / RHEL
sudo dnf install ./kestrel-0.1.0-1.x86_64.rpm

# Any distro — AppImage
chmod +x kestrel_0.1.0_amd64.AppImage
./kestrel_0.1.0_amd64.AppImage
```

### Build from source

Requires Node 20+, Rust 1.77+, and the WebKitGTK development headers.

```bash
# Debian / Ubuntu
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

# Fedora
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel

# Arch
sudo pacman -S webkit2gtk-4.1 base-devel curl wget file openssl \
  appmenu-gtk-module gtk3 libappindicator-gtk3 librsvg
```

```bash
git clone https://github.com/mhzsajan/kestrel.git
cd kestrel
npm install
npm run tauri dev      # development, hot reload
npm run tauri build    # bundles land in src-tauri/target/release/bundle/
```

To work on the UI alone, `npm run dev` serves the frontend in a browser with a
simulated backend — no Rust toolchain needed.

## Configure

Open **Settings** in the sidebar and set:

| Field | Notes |
| --- | --- |
| Provider | OpenAI, Anthropic, or Ollama are preconfigured; edit any of them |
| Base URL | `https://api.openai.com/v1`, `http://localhost:11434/v1`, … |
| Model | e.g. `gpt-4o`, `claude-sonnet-4-5`, `qwen2.5-coder` |
| API key | Leave blank for a local server |
| Workspace | Relative tool paths resolve here; absolute paths still work |

Settings are stored at:

- Linux — `~/.config/kestrel/settings.json`
- macOS — `~/Library/Application Support/kestrel/settings.json`
- Windows — `%APPDATA%\kestrel\settings.json`

The file is written with owner-only permissions because it holds your API key.

## Tools

| Tool | Mutating | What it does |
| --- | :---: | --- |
| `run_shell` | yes | Runs a command via `sh -lc` (Unix) or `cmd /C` (Windows), with a timeout |
| `read_file` | no | Reads a file with line numbers |
| `write_file` | yes | Creates or overwrites a file |
| `edit_file` | yes | Exact-string replacement, one occurrence by default |
| `list_dir` | no | Lists a directory, 1–3 levels deep |
| `search_files` | no | Regex search across a tree, skipping `.git`, `node_modules`, `target` |
| `web_fetch` | no | Fetches a URL and strips it to readable text |

Tool output is capped at 20,000 characters before it goes back to the model, so
a runaway `ls -R` cannot blow up your context window.

## MCP servers

In **MCP servers**, add a name, a command and its arguments:

```
name:    filesystem
command: npx
args:    -y @modelcontextprotocol/server-filesystem /home/you/projects
```

Kestrel spawns the server, completes the `initialize` handshake, lists its tools
and registers them. MCP calls are always confirmed in the default approval mode,
because a third-party server can act outside this app.

## Security model

Be clear-eyed about what this is: **a program that runs model-generated commands
on your computer.** The approval gate is a seatbelt, not a sandbox.

- Approvals are per-call and never cached. The agent cannot approve itself.
- The gate is enforced in Rust before execution, not in the React layer.
- API keys are stored locally with `0600` permissions and are never sent
  anywhere except your chosen provider.
- Shell commands inherit your user's privileges. There is **no** OS-level
  sandbox yet — `run_shell` can do anything you could do in a terminal.

If you need real isolation today, run Kestrel in a container or a VM. An
optional bubblewrap/Landlock sandbox is on the roadmap; see [SECURITY.md](SECURITY.md).

## Architecture

```
src/                     React + TypeScript frontend
  components/            chat, tool cards, diff view, approval dialog, settings
  lib/store.ts           zustand state
  lib/api.ts             typed wrappers over Tauri commands
  lib/diff.ts            line-level LCS diff
  lib/demo.ts            browser-only simulated backend

src-tauri/src/
  lib.rs                 Tauri commands and app wiring
  agent.rs               the agent loop and approval gate
  providers/             OpenAI-compatible and Anthropic streaming clients
  tools/                 tool registry and implementations
  mcp/                   MCP stdio client (JSON-RPC 2.0)
  config.rs              settings load/save
  state.rs               shared state
```

A turn flows like this:

1. `send_message` spawns the agent loop.
2. The loop streams the model's reply to the UI via events.
3. If the model asks for tools, each mutating call emits an approval request and
   **awaits your answer on a oneshot channel**.
4. The tool runs, its output goes back into the conversation, and the loop
   repeats — up to `max_iterations` (default 25).

## Roadmap

- [ ] Cancel a running turn (currently you can only wait it out)
- [ ] Persist sessions to disk
- [ ] Optional sandbox for `run_shell` (bubblewrap / Landlock)
- [ ] Git-aware diffs and hunk-level accept/reject
- [ ] Provider registry with per-provider defaults
- [ ] Integration tests for the agent loop and diff engine

## Contributing

Issues and pull requests are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md).
The most useful contributions right now are bug reports from real Linux desktops
(GNOME, KDE, Wayland) and the sandbox work.

## License

MIT. See [LICENSE](LICENSE).

Kestrel is an independent project. It is not affiliated with, endorsed by, or
derived from any commercial agent product.
