# Accounts, regions, and what actually works on Linux

WorkBuddy ships as **two separate editions** that share a codebase but not an
account system. Getting the right edition matters more than getting the right
`.deb`, because the wrong one cannot be logged into from outside mainland China.

Everything below was verified against a live Ubuntu 26.04 x86_64 machine on
2026-10-31 by querying Tencent's own update API and reading the shipped
`app.asar` — not from documentation.

---

## The two editions

| | China edition | International edition |
| --- | --- | --- |
| Web | `www.workbuddy.cn` | `www.workbuddy.ai` |
| API region | `cn-saas` | (international) |
| Login | phone (SMS) / WeChat | Google, GitHub OAuth |
| Needs a +86 number | **yes** | no |
| Desktop on Linux | published | **not published** |

The desktop client is one binary that picks its backend from a fetched product
config. On a China-edition install that config is pinned:

```
productFeaturesConfig.NetworkFailover.region = "cn-saas"
productFeatures.InternationalLogin           = true
authentication.attributes.externalDomain     = ["www.codebuddy.ai", ...]
authentication.attributes.internalDomain     = ["www.workbuddy.cn", ...]
endpoint                                     = "https://www.workbuddy.cn"
```

`InternationalLogin: true` is only a *capability declaration* — the string
appears in the bundle solely inside the `ProductFeature` enum, with no consumer
that reaches the login flow. It does **not** give the China build an
international login button.

The login URL the China build emits is:

```
https://www.workbuddy.cn/login?platform=workbuddy&state=<uuid>&version=<ver>&loginSessionId=<uuid>
```

That page offers Chinese phone and WeChat only.

## Linux desktop availability

Queried from the public update API (`GET /v2/update?platform=<key>`):

| Channel | Platform key | Version | Artifact |
| --- | --- | --- | --- |
| China | `workbuddy-linux-x64-deb` | `5.6.2.40046327` | **HTTP 206 — live** |
| International | `workbuddy-linux-x64-deb` | `5.5.2.37849279` | **HTTP 404 — removed** |
| International | `workbuddy-darwin-arm64` | `5.9.4.42593613` | HTTP 206 — live |
| International | `workbuddy-win32-x64-user` | `5.9.4.42593613` | HTTP 206 — live |

The international channel is four minor versions ahead on macOS/Windows and
still advertises a Linux entry — but the object behind it is gone. Probe keys
`workbuddy-linux-x64`, `-appimage`, `-tar-gz`, `ide-linux-x64` all return no
version at all.

**Conclusion: there is no maintained international desktop build for Linux.**
The China `.deb` is the only desktop option, and it requires a +86 number.

---

## What does work on Linux: the CLI

`CodeBuddy Code` — the same product family, terminal form — is published to npm
and **does** expose international login on Linux. This is the supported route
for an account outside mainland China.

```bash
npm install -g @tencent-ai/codebuddy-code
codebuddy
```

First run presents a selector:

```
Select login method:
❯ Log in via Chinese Site
  Log in via International Site        ← Google / GitHub, no +86 number
  Log in via Enterprise Domain
  Log in via iOA (Tencent only)
```

Internally these map to auth types `internal`, `external`, `selfhosted`, `iOA`.
`external` authenticates against `codebuddy.ai`.

### Over SSH, use the Web UI instead of arrow keys

The TUI selector is raw-mode Ink and is awkward over a remote terminal. The CLI
can serve a browser login page instead:

```bash
# on the Ubuntu box
codebuddy --serve --host 127.0.0.1 --port 7788
```

It prints a password-protected URL:

```
Web UI      http://127.0.0.1:7788/?password=<generated>
```

Forward the port from your own machine and open that URL locally:

```bash
ssh -L 7788:127.0.0.1:7788 switchgod@192.168.88.252
```

The page renders the same four login methods as buttons. Choosing
**International Site** opens OAuth in your own browser, so the Google/GitHub
session happens on the machine you are sitting at, not the remote one.

Keep the host at `127.0.0.1`. Binding the Web UI to a LAN address exposes an
agent that can run shell commands, guarded only by the printed password.

---

## Do not patch the auth domain

It is tempting to point the China build's `endpoint` at `www.workbuddy.ai` and
reuse the international login. It does not work, and it is not safe to try:

- `authStateReplication` is `"regional"` — the token minted by one region is
  rejected by the other region's API.
- `NetworkFailover.routes.core-api.trustedOrigins` is a fixed allowlist of three
  `.cn` hosts; requests to other origins fail closed rather than falling back.
- The `state` / `loginSessionId` handshake is issued by the origin that will
  verify it. A handshake started at `.cn` cannot be completed at `.ai`.
- Editing `app.asar` to bypass this is a licence violation and breaks on every
  update.

If you need the desktop GUI with an international account, run it on macOS or
Windows, where Tencent publishes it. On Linux, use the CLI.
