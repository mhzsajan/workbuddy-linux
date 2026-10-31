# Contributing to Kestrel

Thanks for taking a look. This project is early, so the highest-value
contributions right now are bug reports from real Linux desktops and small,
focused pull requests.

## Development setup

Prerequisites: Node 20+, Rust 1.77+, and the WebKitGTK headers listed in the
README.

```bash
npm install
npm run tauri dev
```

`npm run tauri dev` builds the Rust side (slow the first time) and serves the
frontend with hot reload.

### Frontend only

If you do not want to install Rust:

```bash
npm run dev
```

This serves the UI in a browser with a simulated backend (`src/lib/demo.ts`), so
you can work on layout, streaming and diff rendering without a model or a
toolchain. It is clearly marked as a preview in the sidebar.

### Checks before opening a PR

```bash
npm run build                     # tsc --noEmit + vite build
cd src-tauri && cargo fmt --all && cargo clippy --all-targets
```

CI builds the Linux bundles on every push, so a broken Rust change shows up
there quickly.

## Code layout

- `src-tauri/src/agent.rs` — the loop. If you are changing behaviour, this is
  usually where it goes.
- `src-tauri/src/tools/mod.rs` — add a tool here. Register it in `defs()` and
  `execute()`, and mark it mutating in `is_mutating()` if it can change the disk
  or run a command.
- `src-tauri/src/providers/` — one module per wire format. A new provider means
  a new module plus a variant in `ProviderKind`.
- `src/lib/store.ts` — all frontend state. Events from Rust land in `App.tsx`
  and are dispatched here.

## Guidelines

**Keep the approval gate honest.** Any tool that writes, deletes or executes must
return `true` from `is_mutating()`. A PR that quietly bypasses the gate will be
rejected, regardless of how convenient it is.

**Cap your output.** Tool results go straight into the model's context. Truncate
generously and say that you truncated.

**Errors are model input.** Return `Err(String)` with a message the model can act
on ("old_string not found in foo.rs — read the file again") rather than a bare
error code. The agent loop feeds it straight back.

**Match the existing style.** Rust: `cargo fmt` defaults. TypeScript: strict mode
is on, and `npm run build` must pass. No new runtime dependencies without a
reason in the PR description.

## Commit messages

Short imperative subject, blank line, then *why* rather than *what*:

```
Add timeout to run_shell

A hung command blocked the agent loop forever. Default 120s, overridable
per call.
```

## Reporting bugs

Include your distro and desktop environment, whether you are on Wayland or X11,
the Kestrel version, and the relevant lines from the terminal if you launched it
from one. Model/provider issues should say which provider and model you used —
not your API key.
