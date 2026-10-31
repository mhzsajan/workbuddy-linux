# End-to-end verification log

A record of what this toolchain was actually run against, so you can judge the
claims in the README instead of trusting them. Nothing here is inferred from
documentation.

**Host:** Ubuntu 26.04, x86_64, kernel 7.0.0-42-generic, GNOME on Wayland
**Date:** 2026-10-31
**Input:** `WorkBuddy-darwin-x64-5.9.4.42593613-fa0a0901.zip` (international
channel), 504,460,931 bytes, `unzip -tq` clean

---

## Summary

| Stage | Command | Result |
| --- | --- | --- |
| Archive integrity | `unzip -tq` | clean |
| Bundle selection | `pick_app_bundle` | correct bundle, `__MACOSX` pruned |
| Electron runtime | mirror download | Electron 37.10.3 |
| `app.asar` patch | `apply-linux-patches.js` | env shim applied, UI patches skipped (anchors gone) |
| Converted app | `du -sh workbuddy-app` | 918 MB → **621 MB** after the Phase 5 prune |
| Foreign payload | `purge_foreign_platform_trees` | 15 trees, 303,592 KB reclaimed |
| Package build | `scripts/build-deb.sh` | **159 MB** `.deb` (was 279 MB before the prune) |
| Package install | `dpkg -i` | no dependency errors |
| Launch from `/opt` | `/usr/bin/workbuddy` | 9 processes, window attaches |
| International endpoint | NetLog | `www.workbuddy.ai/v3/config` → **200** |
| Runtime errors | log scan | 0 `MODULE_NOT_FOUND` / `dlopen` failures after pruning |

---

## The `E2BIG` fix is what makes the window appear

The single most important patch. WorkBuddy 5.9.4 exports a
**367,232-byte** `ACC_PRODUCT_CONFIG_V3`. Linux caps a single environment string
at `MAX_ARG_STRLEN` (128 KB), so every `execve()` in the Electron process tree
fails with `E2BIG` and the app dies before drawing anything.

Observed in the runtime log after conversion:

```
[WorkbuddyProductConfig] Spilled oversized ACC_PRODUCT_CONFIG_V3 (367232 bytes)
to ~/.workbuddy-ai/cache/acc-product-config-v3-52687-f2f3a612-...json
to avoid execve E2BIG.
```

If you see a blank window or no window at all, this patch is why it exists —
and if it did not apply, that is the reason.

## The purge ran before the step that undid it

The first packaged build was 279 MB and installed 918 MB. Most of that was
payload that can never execute on Linux — a macOS build carries prebuilt
binaries for every platform it supports, and they were being copied verbatim
into the package.

`native-modules.sh` did delete them in Phase 1. But Phase 3 rebuilds `node-pty`
by running `npm install node-pty@<version>` and copying the whole published
module back over the pruned one — and the npm tarball ships `prebuilds/darwin-*`
and `prebuilds/win32-*` inside it. So the purge was always overwritten.

A Phase 5 sweep now runs after everything else:

```
[INFO]   Removed 15 non-Linux platform tree(s), 303592 KB reclaimed
918M  workbuddy-app   ->   621M
279M  .deb            ->   159M
```

The largest single item was `@tencent/tencent-docs-ai-engine/bin/darwin-x64` at
233 MB — the Docs engine has no Linux build at all, which is why it is listed as
a known limitation. Removing its macOS payload costs nothing that was working.

Verified after the prune: 9 processes, `www.workbuddy.ai/v3/config` still 200,
`[VendorExtract] ensureAll finished: 0 extracted, 7 skipped, 0 failed`, and zero
`MODULE_NOT_FOUND` or `dlopen` errors in the runtime log. The Linux `node-pty`
binary (`bin/linux-x64-136/`) is untouched.

## Dependency list

The first `.deb` declared six libraries. Tencent's own Linux package declares
nine, and the difference is not cosmetic: without `libnotify4` notifications do
nothing, without `libatspi2.0-0` accessibility is broken, `libxtst6` breaks
synthetic input. See [upstream-linux-builds.md](upstream-linux-builds.md) for
the full control file and the `t64` / `ayatana` alternatives now used so one
stanza covers Ubuntu 22.04 through 26.04.


## Packaging and relocation

`build-deb.sh` installs the tree at `/opt/workbuddy` and a thin wrapper at
`/usr/bin/workbuddy`. The launcher derives its own directory, so it does not
care where it was built:

```bash
APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
```

Verified that the installed copy at `/opt/workbuddy/start.sh` runs
`/opt/workbuddy/electron` — not the build directory it came from.

The package also preserves the SUID bit on the Chromium sandbox helper:

```
-rwsr-xr-x root/root 15328 ./opt/workbuddy/chrome-sandbox
```

Note that the generated launcher still passes `--no-sandbox` by default, because
unprivileged user namespaces are unavailable on many distros. Setting
`WB_NO_SANDBOX=0` uses the SUID helper instead.

## Desktop integration

```
$ desktop-file-validate /usr/share/applications/workbuddy.desktop
(no output — valid)

$ dpkg -s workbuddy
Status: install ok installed
Version: 5.9.4.42593613
Architecture: amd64
```

Installing triggered `gnome-menus`, `desktop-file-utils` and
`hicolor-icon-theme` updates, so the entry and icon appear in the application
grid without a re-login.

---

## What was **not** verified

Be aware of the edges:

- **The tray icon and tray menu are unpatched on 5.9.4.** All three UI anchors
  the patcher looked for are gone from this build. The env shim (the patch that
  matters) is mandatory and applied; the cosmetic patches warn and skip. Expect
  no tray icon.
- **`.rpm` and `.pkg.tar.zst` were not built here** — no `rpmbuild` or `makepkg`
  on the test host. `build-deb.sh` is the only path with an observed success.
- **x86_64 only.** ARM/aarch64 is unsupported: Tencent publishes no arm64 macOS
  build to convert.
- **No Tencent support.** This is an unofficial local conversion.
