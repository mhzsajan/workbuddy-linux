# WorkBuddy for Linux (unofficial)

Turns an **official WorkBuddy macOS build that you supply** — a China-channel
`.dmg` or an international-channel `.zip` — into a native Linux application, and
builds a `.deb` / `.rpm` / `.pkg.tar.zst` from it.

A maintained fork of [`JipZeonGit/workbuddy-linux`](https://github.com/JipZeonGit/workbuddy-linux)
(MIT). The approach is unchanged — this fork fixes the ways it used to fail
silently. See [IMPROVEMENTS.md](IMPROVEMENTS.md) for the full list.

> **This repository contains no Tencent software.** It is a converter, not a
> distribution channel. You download the DMG from Tencent yourself; nothing is
> redistributed here. Generated artifacts are gitignored and stay local.

---

## How it works

WorkBuddy is an Electron/VS Code application, and its `app.asar` JavaScript is
platform-neutral. So only the binary layer needs replacing:

1. Extract the DMG with 7-Zip.
2. Read the Electron version out of the bundle's `Info.plist`.
3. Download the matching **Linux x86_64** Electron runtime.
4. Copy `app.asar` + `app.asar.unpacked` into the new app directory.
5. Recompile the native modules (`node-pty`, `better-sqlite3`, …) against Linux
   Electron headers — the macOS `.node` binaries cannot load on Linux.
6. Patch `app.asar` for the Linux-specific failures below.
7. Generate a launcher, desktop entry and icon, then package.

### What the patches fix

| Patch | Symptom it fixes |
| --- | --- |
| `process.env` shim | A ~260 KB config string in an env var exceeds Linux's 128 KB `MAX_ARG_STRLEN`, so every `execve()` fails with `E2BIG` and **no window ever appears** |
| Tray context menu | libayatana-appindicator never emits `click`, so the tray menu is empty unless `setContextMenu()` is called explicitly |
| Tray icon path | The AppIndicator backend cannot read an in-memory `NativeImage`, so the tray shows a broken-image placeholder; an on-disk PNG is used instead |
| Sidecar spawn | Large env entries are spilled to temp files before spawning |

## Requirements

| Requirement | Notes |
| --- | --- |
| Architecture | **x86_64 only** — the toolchain downloads an x86_64 Electron runtime |
| 7-Zip | **Version ≥ 21**, DMG input only. ZIP input uses `unzip` and needs no 7-Zip |
| Node.js | ≥ 20 (installed by `make deps` if your distro's is older) |
| Disk | ~4 GB free |
| Input | An official WorkBuddy **`.dmg`** (China channel) or **`.zip`** (international channel), or an extracted `.app` |

Tested package managers: `apt`, `dnf5`/`dnf`, `pacman`, `zypper`.

> **Ubuntu 22.04 note:** the `7zip` package only exists from 24.04 onwards, and
> 22.04's `p7zip` is too old. `make deps` now detects this and tells you how to
> install the official static build. 24.04+ is the supported path.

## Quick start

```bash
git clone https://github.com/mhzsajan/workbuddy-linux
cd workbuddy-linux

make deps                      # system packages + Node 20+

mkdir -p downloads             # download the macOS DMG from workbuddy.ai
                               # put exactly ONE .dmg in here

make preflight                 # arch, disk, 7-Zip, Node, input sanity
make build-app                 # extract, swap runtime, rebuild natives, patch
make run-app                   # test before packaging anything

make package                   # .deb / .rpm / .pkg.tar.zst into dist/
make install                   # dpkg -i + dependency repair
```

Prefer to point at a file directly?

```bash
make build-app DMG=/home/you/Downloads/WorkBuddy.dmg
```

## Environment variables

| Variable | Default | Effect |
| --- | --- | --- |
| `WB_ALLOW_UNPATCHED` | `0` | `1` lets the build continue when patching fails. The result will almost certainly show no window. |
| `WB_NO_SANDBOX` | `1` | `0` re-enables Electron's Chromium sandbox (needs unprivileged user namespaces). |
| `WORKBUDDY_INSTALL_DIR` | `./workbuddy-app` | Where the converted app is assembled. |
| `ELECTRON_VERSION` | auto-detected | Force a runtime version if detection falls back. |
| `ELECTRON_MIRROR` | — | Mirror for the Electron runtime download. |
| `ELECTRON_HEADERS_URL` | — | Headers dist URL used for native rebuilds. |

## Troubleshooting

| Symptom | Cause | Fix |
| --- | --- | --- |
| Build says patches failed, exit 4 or 5 | The DMG is newer than the patch anchors support | Use an older DMG, or open an issue with the version and exit code |
| App starts but no window | `app.asar` was not patched | Check `asarPatched` in `.workbuddy-linux/build-info.json` |
| `make deps` fails on 7-Zip | Ubuntu 22.04 has no `7zip` package | Install the static build, see above |
| `dpkg` leaves the package unconfigured | Missing dependencies | `sudo apt-get install -f` (now run automatically by `make install`) |
| Blank window on Wayland | GPU/compositing issue | `./workbuddy-app/start.sh --disable-gpu` |
| Tray icon is a broken image | Icon PNG not written | Verify `workbuddy-app/.workbuddy-linux/workbuddy.png` exists |

## Accounts and regions

WorkBuddy has two editions with separate account systems, and **the edition you
install decides whether you can log in at all**:

| | China edition | International edition |
| --- | --- | --- |
| Login | phone (SMS) / WeChat | Google, GitHub OAuth |
| Needs a +86 number | **yes** | no |
| Official desktop build for Linux | published (`.deb`) | **none** |
| Convertible with this toolchain | yes (`.dmg`) | **yes (`.zip`)** |

If you are outside mainland China and have no +86 number, the China `.deb` is
unusable — and Tencent publishes no international Linux desktop build. Its
international channel still advertises a Linux entry, but the artifact behind
it is a stale `5.5.2` that returns 404; macOS and Windows are on `5.9.4`.

You can still get the **international desktop app** on Linux by converting
Tencent's international macOS `.zip` with this toolchain:

```bash
./install.sh --fresh downloads/WorkBuddy-darwin-x64-5.9.4.42593613.zip
```

Verified working against 5.9.4 (Electron 37.10.3): the app fetches its config
from `www.workbuddy.ai` and emits an international login URL that you can
complete in a browser on any machine. Note that at this version the UI patch
anchors no longer match, so the build runs the env shim only — the window opens,
but the tray menu/icon are unpatched.

Alternatively the **CLI** offers the same international login:

```bash
npm install -g @tencent-ai/codebuddy-code
codebuddy --serve --host 127.0.0.1 --port 7788   # prints a password-protected URL
```

Do not try to repoint the China build's auth endpoint at `www.workbuddy.ai` —
tokens are region-bound (`authStateReplication: "regional"`) and the trusted
origins are a fixed `.cn` allowlist, so it cannot work.

Full evidence, API responses and the SSH port-forward recipe:
[docs/account-and-region.md](docs/account-and-region.md).

## Known limitations

These come from upstream and are **not** fixed by this fork:

- **Tencent Docs engine removed** — `@tencent/docs-engine` ships only a macOS
  arm64 `.dylib`, which would `dlopen`-crash on Linux.
- **AI code sandbox degraded** — Tencent's proprietary `vendor/sandbox` has no
  Linux build, so code execution falls back to an unsandboxed terminal or is
  refused.
- **No auto-update** — upstream relies on macOS ShipIt / Windows Squirrel.
  Re-run the pipeline with a newer DMG to update.
- **Patch anchors are exact strings.** The patcher matches minified upstream
  code. It now fails loudly when an anchor is missing instead of producing a
  broken app, but it still cannot patch a structurally different version.

## Security

The generated launcher runs Electron with `--no-sandbox` by default, because
upstream does not ship a SUID sandbox helper. That disables the Chromium
renderer sandbox, while the application can execute AI-generated commands.
Combined with the missing code sandbox above, treat this as **running untrusted
code with your user privileges**. Use a VM or container if that matters.

## Legal

- Not affiliated with, endorsed by, or supported by Tencent.
- No Tencent binaries are stored or redistributed by this repository. You supply
  the DMG; all generated artifacts stay on your machine.
- Converting a macOS-licensed application to run on another OS may conflict with
  WorkBuddy's EULA. That decision is yours.
- **Do not report problems from this conversion to Tencent support.**

## History

This repository previously held an unrelated desktop agent application. That
code is still reachable in the git history; the current tree is the Linux
conversion toolchain only.

---

## 简体中文

本项目是 [`JipZeonGit/workbuddy-linux`](https://github.com/JipZeonGit/workbuddy-linux)
的维护分支（MIT），原理不变：把**你自行下载的官方 macOS DMG** 转换成可在 Linux
运行的 Electron 应用，并打包为 `.deb` / `.rpm` / `.pkg.tar.zst`。

与上游相比，主要修复的是**失败被静默吞掉**的问题：补丁失败以前只打印一行警告
然后继续，最终得到一个「启动后没有窗口」的包；现在会直接中止构建并说明原因。
完整改动见 [IMPROVEMENTS.md](IMPROVEMENTS.md)。

```bash
make deps
mkdir -p downloads      # 放入唯一一份官方 macOS DMG
make preflight
make build-app
make run-app
make package && make install
```

本仓库**不分发任何腾讯二进制文件**，仅作为转换工具。由此产生的任何问题请勿反馈给
官方客服。

### 关于账号与地区（重要）

WorkBuddy 分国内版与国际版，**两个账号体系互不通用**：国内版用手机号/微信登录，
国际版用 Google / GitHub 登录。国内版 `.deb` 只认 +86 手机号；而国际版**没有发布
Linux 桌面版**（官方更新接口里 Linux 条目停留在 5.5.2，实际文件已 404，macOS /
Windows 已到 5.9.4）。

在 Linux 上要使用国际版账号，请改用 CLI：

```bash
npm install -g @tencent-ai/codebuddy-code
codebuddy --serve --host 127.0.0.1 --port 7788
```

首次启动的登录方式选择界面里选 **Log in via International Site** 即可。详细证据与
远程端口转发方法见 [docs/account-and-region.md](docs/account-and-region.md)。
