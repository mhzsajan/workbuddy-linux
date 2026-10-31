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
| Converted app | `du -sh workbuddy-app` | 918 MB, 3,666 files |
| Package build | `scripts/build-deb.sh` | 279 MB `.deb` in 2m 12s |
| Package install | `dpkg -i` | no dependency errors |
| Launch from `/opt` | `/usr/bin/workbuddy` | 8 processes, window attaches |
| International endpoint | NetLog | `www.workbuddy.ai/v3/config` → **200** |
| Login flow | emitted URL | `www.workbuddy.ai/login?platform=workbuddy-ai` |

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
