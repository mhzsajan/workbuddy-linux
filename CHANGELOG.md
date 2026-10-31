# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

This project is a converter, not a distribution channel: releases contain the
build toolchain only. You supply the official WorkBuddy macOS archive.

## [Unreleased]

## [1.2.2] - 2026-10-31

Found while reading a sibling project,
[`JipZeonGit/codebuddy-ide-cn-linux`](https://github.com/JipZeonGit/codebuddy-ide-cn-linux).
Two of its observations transferred directly; the rest of it did not, because
this repository already handles native modules more thoroughly.

### Fixed

- **296 MB of non-Linux payload was being shipped.** `native-modules.sh` deleted
  the `darwin-*` / `win32-*` trees in Phase 1 and then reinstalled `node-pty`
  from npm in Phase 3, which copies the whole published module back — prebuilds
  and all. The purge ran before the step that undid it. A final sweep (Phase 5)
  now runs after everything else: 918 MB → 621 MB, 15 trees, 303,592 KB.
  `WB_KEEP_FOREIGN_PLATFORM=1` opts out.
- **Package dependencies were incomplete.** All three formats declared only
  GTK/NSS/XSS/asound/secret/xkbfile, silently omitting `libnotify4`
  (notifications), `libxtst6` (synthetic input), `libatspi2.0-0` (accessibility),
  `libuuid1` and `xdg-utils`. They now match what Tencent's own Linux build
  declares, with `t64` and `ayatana` alternatives so one stanza works from
  Ubuntu 22.04 through 26.04. `libayatana-appindicator3-1` is now recommended —
  without AppIndicator a GTK tray icon cannot appear at all.

### Added

- `docs/upstream-linux-builds.md` — Tencent now publishes **official Linux
  packages for the China edition** (`5.6.2.40046327`: x86_64 `.deb`, x86_64
  `.rpm`, arm64 `.deb`), with the verified URLs, sizes and control metadata.
  The international channel still has none, which is why this project exists.

### Changed

- The README no longer claims "WorkBuddy does not ship a Linux desktop build."
  That is true only of the international edition. Visitors with a Chinese
  account are now sent to the official packages instead.

## [1.2.1] - 2026-10-31

A documentation release. No converter code changed; this records the first
end-to-end run through packaging and installation, which 1.2.0 only claimed.

### Added

- `docs/verification.md` — the full evidence trail for one real run: the input
  archive and its checksum, the 918 MB converted tree, the 279 MB `.deb`, the
  `dpkg -i` output, and the runtime log proving the app talks to
  `www.workbuddy.ai`. Also states plainly what was *not* verified (tray icon,
  `.rpm` / `.pkg.tar.zst`, ARM).

### Fixed

- README no longer implies the tray icon works on 5.9.4. It is a known
  limitation, not a silent failure.

## [1.2.0] - 2026-10-31

Adds the international edition, which is the only way to get a
desktop client that authenticates outside mainland China.

### Added

- **ZIP input.** The international channel publishes `.zip`, not `.dmg`.
  `resolve_input_path`, `resolve_app_bundle` and `preflight.sh` now accept
  either, with a `PK` magic check so a truncated download fails immediately
  instead of inside `unzip`.
- **Electron mirror fallback.** `github.com` is unreachable on some networks,
  which aborted the runtime download outright. The downloader now tries the
  official release URL and then npmmirror's copy of the same artifacts.
- `CHANGELOG.md`, release automation, and a self-contained bootstrap installer.

### Changed

- **7-Zip is now only required for DMG input.** ZIP input goes through `unzip`,
  so a machine without 7-Zip can complete a full build. Previously `check_deps`
  rejected such a setup before it ever looked at the input type.
- **Linux patches degrade instead of aborting.** The env shim and the cosmetic
  tray/window patches used to be all-or-nothing: one missing anchor killed the
  build. The shim (the patch that actually makes a window appear) stays
  mandatory; the tray menu, tray icon and window-frame patches now emit a loud
  warning and are skipped when the payload is newer than the anchor set.
- The launcher picks the Ozone platform from `WAYLAND_DISPLAY` / `DISPLAY`
  instead of `--ozone-platform-hint=auto`.

### Fixed

- **`pick_app_bundle` could select an AppleDouble shell.** ZIP archives carry a
  `__MACOSX/` tree whose entries can be named `*.app`; that shell has no
  `Contents/Info.plist` and conversion failed much later with a poor message.
  It is now pruned, bundles without a manifest are rejected, and the shallowest
  bundle wins so Electron's own `Contents/Frameworks/* Helper.app` entries can
  never be mistaken for the application.
- **Every build failed with `APP_DIR: unbound variable`.** An unescaped
  `$APP_DIR` inside the generated-launcher heredoc was expanded by `set -u`
  while writing `start.sh`, aborting after the expensive steps had already run.
- **`Missing X server or $DISPLAY` on Wayland-only sessions.**
  `--ozone-platform-hint=auto` resolves to X11 whenever `DISPLAY` happens to be
  set, and then aborts on a session that has no X server (typical over SSH).

### Verified

- Converted `WorkBuddy-darwin-x64-5.9.4.42593613` (504,460,931 bytes) on Ubuntu
  26.04 x86_64: Electron 37.10.3, bundle id `com.workbuddy.workbuddy-ai`,
  174 MB `app.asar`. The result fetches `https://www.workbuddy.ai/v3/config`
  (200) and emits an international login URL, confirming it is the
  international client rather than a China build repointed at a new host.

## [1.1.0] - 2026-10-31

Turns silent failures into loud ones. The upstream toolchain reported success
for builds that produced an application with no window.

### Added

- `scripts/preflight.sh` — architecture, disk, toolchain and input sanity,
  run before the multi-gigabyte Electron download rather than after it.
- `IMPROVEMENTS.md` documenting each defect with line references.

### Changed

- Patch failures now abort the build. The wrapper previously discarded the
  patcher's non-zero exit and continued, which is how "launches with no window"
  builds were shipped as successful.
- `app.asar` is backed up before patching and restored by an `EXIT` trap if the
  build does not finish cleanly. `build-info.json` records `asarPatched`.
- `scripts/install-deps.sh` splits `7zip` out of the apt list (the package only
  exists from Ubuntu 24.04) and falls back to `p7zip-full` with explicit
  static-build instructions.

### Fixed

- `dpkg -i` leaving the package unconfigured — `apt-get install -f -y` is now
  run automatically.
- The launcher honours `WB_NO_SANDBOX=0` for users who can enable the sandbox.

## [1.0.0] - 2026-10-31

Initial maintained fork of [`JipZeonGit/workbuddy-linux`](https://github.com/JipZeonGit/workbuddy-linux).

### Added

- Conversion of an official macOS WorkBuddy DMG into a runnable Linux Electron
  application: extract, detect the Electron version, fetch the matching Linux
  runtime, copy `app.asar` + `app.asar.unpacked`, rebuild native modules, patch,
  and package.
- Native module rebuild for `node-pty`, `better-sqlite3` and friends against
  Linux Electron headers, plus Linux platform packages for `@lydell/node-pty`
  and `@vscode/ripgrep`.
- Packaging as `.deb`, `.rpm` and `.pkg.tar.zst`.

### Fixed

- **`E2BIG` / no window.** A ~260 KB `ACC_PRODUCT_CONFIG_V3` string exceeds
  Linux's 128 KB `MAX_ARG_STRLEN`, so every `execve()` fails and the renderer
  never starts. An env shim keeps the value in a JS slot and spills oversized
  entries to temp files.
- **Empty tray menu.** libayatana-appindicator never emits `click`, so a
  `setContextMenu()` call is injected after the `Tray` is constructed.
- **Broken tray icon.** The AppIndicator backend cannot read an in-memory
  `NativeImage`; the on-disk PNG is used instead.

[Unreleased]: https://github.com/mhzsajan/workbuddy-linux/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/mhzsajan/workbuddy-linux/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/mhzsajan/workbuddy-linux/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/mhzsajan/workbuddy-linux/releases/tag/v1.0.0
