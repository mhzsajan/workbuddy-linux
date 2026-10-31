#!/bin/bash
# Linux-specific post-copy patches applied to the packed app.asar.
#
# Fix 1 — "no window appears on launch" (E2BIG):
#   The main process stores a ~260KB product configuration JSON into the
#   environment variable ACC_PRODUCT_CONFIG_V3. Linux's MAX_ARG_STRLEN cap
#   (128KB per env string) then causes every execve() to fail with E2BIG,
#   including Chromium's internal /proc/self/exe spawn for the network
#   service and utility processes. Without those child processes the
#   renderer cannot start and the main window never shows.
#
#   We install a tiny shim at the top of main/index.js that intercepts
#   writes to ACC_PRODUCT_CONFIG_V3 / ACC_PRODUCT_CONFIG_V2 via
#   Object.defineProperty on process.env. The value is kept in a JS slot
#   only — libc setenv is never called, so execve() stays well under the
#   per-string limit while JS code still observes the same values.
#
# Fix 2 — "tray icon menu is empty":
#   On Linux, Electron's Tray is backed by libayatana-appindicator. The
#   indicator never emits the `click` / `right-click` events the upstream
#   code relies on, and only renders a menu that has been attached via
#   tray.setContextMenu(...). We inject that call right after the Tray is
#   constructed so the "显示窗口 / 退出" menu actually appears.
#
# Fix 3 — "tray icon is a missing-image placeholder" (exclamation mark):
#   Upstream hands the Tray a resized in-memory NativeImage. The
#   AppIndicator backend can't re-read those bytes through GTK so the
#   indicator shows its "broken image" fallback. We patch the Tray
#   construction on Linux to use the on-disk PNG at
#   <install-dir>/.workbuddy-linux/workbuddy.png (written by install.sh
#   and shipped inside the generated .deb/.rpm/.pkg.tar.zst under
#   /opt/<app>/.workbuddy-linux/).

LINUX_PATCHES_SHIM_MARKER="__WB_LINUX_PATCHES_V5__"

# Returns 0 when the patch set was applied, 1 when it was skipped because
# WB_ALLOW_UNPATCHED=1 was requested, and aborts the build otherwise.
#
# Note: apply-linux-patches.js exits non-zero (codes 2, 3, 6) when it cannot
# read, unpack or repack the archive, and the previous version of this wrapper
# discarded that signal — so a build whose patches failed still reported success
# and then produced an application that launched with no window. Do not turn
# these back into warnings.
#
# A payload newer than the patch set is different: the patcher then applies the
# env shim only and returns 0, because that shim is what makes a window appear
# while the tray/frame patches are cosmetic. That path logs its own warnings.
apply_linux_runtime_patches() {
    local app_dir="$1"
    local asar_path="$app_dir/resources/app.asar"

    if [ ! -f "$asar_path" ]; then
        error "Linux patches: app.asar not found at $asar_path.
The DMG payload did not contain the expected app.asar - check that you used the
official WorkBuddy macOS build."
    fi

    info "=== Applying Linux runtime patches to app.asar ==="

    # The Node helper needs @electron/asar available. Install it into the
    # per-build WORK_DIR so we don't pollute the project with a persistent
    # node_modules tree.
    local asar_tool_dir="$WORK_DIR/asar-tool"
    if [ ! -x "$asar_tool_dir/node_modules/.bin/asar" ]; then
        info "  Installing @electron/asar for patcher"
        mkdir -p "$asar_tool_dir"
        if ! (
            cd "$asar_tool_dir"
            npm init -y >/dev/null 2>&1
            npm install @electron/asar --no-audit --no-fund --silent 2>&1
        ); then
            error "Could not install @electron/asar, so app.asar cannot be patched.
Check your network or npm registry, then re-run. To build an unpatched (and
unusable) app anyway: WB_ALLOW_UNPATCHED=1 make build-app"
        fi
    fi

    local patch_rc=0
    NODE_PATH="$asar_tool_dir/node_modules" \
        node "$SCRIPT_DIR/scripts/lib/apply-linux-patches.js" \
             "$asar_path" \
             "$LINUX_PATCHES_SHIM_MARKER" \
        || patch_rc=$?

    if [ "$patch_rc" -ne 0 ]; then
        if [ "${WB_ALLOW_UNPATCHED:-0}" = "1" ]; then
            warn "  Linux patches failed (exit $patch_rc); continuing because WB_ALLOW_UNPATCHED=1."
            warn "  The resulting app will most likely show no window."
            return 1
        fi
        error "Linux runtime patches failed (exit $patch_rc).
This almost always means the DMG is newer than the patches support: the minified
upstream code no longer contains the anchor strings the patcher looks for.
Options:
  1. Use an older official macOS DMG (the patches were verified against 4.22.10).
  2. Open an issue with the DMG version and the exit code above.
  3. Build anyway (unusable): WB_ALLOW_UNPATCHED=1 make build-app"
    fi

    info "  Linux runtime patches applied successfully"
    return 0
}
