# Security model

Kestrel executes model-generated actions on your computer. This document states
plainly what that means and where the boundaries are.

## What the approval gate does

Every tool call that can change your machine — `run_shell`, `write_file`,
`edit_file`, and every MCP tool — pauses and emits an approval request before it
runs. The request shows the exact tool and its raw arguments.

- The gate is enforced in the Rust process (`agent.rs`), not in the UI. A
  compromised or prompt-injected renderer cannot approve its own call.
- Approvals are per-call. There is no "remember this", no allowlist, and no
  session-wide grant.
- Requests time out after 10 minutes and are treated as denied.

## What the approval gate does not do

**It is not a sandbox.** Approved shell commands run with your full user
privileges. `rm -rf ~` will work if you approve it.

There is currently no OS-level isolation: no seccomp filter, no namespace, no
Landlock ruleset, no filesystem jail. The workspace setting scopes *relative*
paths, but absolute paths are deliberately allowed, so it is a convenience, not
a boundary.

## Prompt injection

The agent reads untrusted content: web pages via `web_fetch`, file contents via
`read_file`, and tool output. Any of it can contain instructions aimed at the
model.

Kestrel's defence is structural rather than clever: injected instructions can
make the model *ask* for something dangerous, but they cannot make it *happen*.
You still see the exact command and you still have to approve it. Read what you
approve — especially `run_shell` arguments that came from a fetched web page.

## Secrets

- API keys live in `settings.json` in your OS config directory, written with
  `0600` permissions on Unix.
- Keys are sent only to the provider you configured, in an auth header.
- Tool output is sent to your provider as part of the conversation. Do not point
  `read_file` at `~/.ssh` or `.env` unless you accept that.

## MCP servers

MCP servers are third-party programs that Kestrel spawns as child processes.
They run with your privileges and can do anything you can do — the approval gate
only governs *calls made through Kestrel*, not what the server does on its own
at startup. Only connect servers you trust, and read their source first.

## If you need real isolation today

Run Kestrel inside a container or a VM with only the project directory mounted.
That is the only reliable boundary available in v0.1.0.

## Reporting a vulnerability

Open a private security advisory on GitHub rather than a public issue. Please do
not include working exploit payloads in public discussion.
