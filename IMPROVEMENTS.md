# Changes from upstream

Forked from [`JipZeonGit/workbuddy-linux`](https://github.com/JipZeonGit/workbuddy-linux)
at commit `4df73ac` (archived June 2026). Every change below is in the conversion
toolchain only — no patch behaviour was altered, so a DMG that converted before
converts identically now.

The theme: **stop hiding failures.** The original toolchain's most common outcome
was a build that reported success and then produced an application that launched
with no window.

---

## 1. Patch failures now abort the build

**`scripts/lib/linux-patches.sh`**

`apply-linux-patches.js` already exits with codes 4 and 5 when an anchor it needs
is missing. The wrapper threw that signal away:

```bash
# before
node .../apply-linux-patches.js "$asar_path" "$marker" || {
    warn "  Failed to apply Linux patches; leaving app.asar untouched"
    return 0        # <- build continues and reports success
}
```

Now it propagates the exit code, explains what it means, and stops:

```bash
# after
|| patch_rc=$?
if [ "$patch_rc" -ne 0 ]; then
    [ "${WB_ALLOW_UNPATCHED:-0}" = "1" ] && { warn ...; return 1; }
    error "Linux runtime patches failed (exit $patch_rc). ..."
fi
```

The same treatment applies to a failed `@electron/asar` install, which previously
also degraded to a warning. `WB_ALLOW_UNPATCHED=1` remains as an explicit escape
hatch for debugging.

## 2. `app.asar` is backed up and restored on failure

**`install.sh`**

The patcher rewrites `app.asar` in place. A crash or a partially-applied patch
set left a corrupt archive with no way back. Now:

- a pristine copy is taken to `app.asar.orig` before patching;
- an `EXIT` trap restores it if the build did not complete;
- the backup is deleted on success so it never ships inside the package.

## 3. `build-info.json` records whether the build is actually usable

**`install.sh`**

Adds `"asarPatched": true|false`. Previously the only way to tell a good build
from a broken one was to launch it and see whether a window appeared.

## 4. New preflight stage

**`scripts/preflight.sh`** (new), called by `install.sh` and exposed as
`make preflight`.

Fails in about a second instead of after a multi-gigabyte Electron download:

- rejects non-x86_64 hosts (the runtime fetched is x86_64 only);
- requires ≥ 4 GB free disk;
- requires 7-Zip ≥ 21 and Node ≥ 20;
- sanity-checks the input (size, and the `koly` UDIF trailer on `.dmg`);
- warns on Ubuntu ≤ 23.x, where the `7zip` package does not exist.

## 5. 7-Zip install no longer hard-fails on Ubuntu 22.04

**`scripts/install-deps.sh`**

`apt-get install 7zip` fails outright before 24.04, taking `make deps` with it —
and `common.sh` rejects the `p7zip` it *does* have as too old (< 21), so 22.04
satisfied neither path. The package name is now tried separately, with a fallback
and an explicit message pointing at the official static build.

## 6. `dpkg -i` failures are repaired

**`scripts/install-package.sh`**

`sudo dpkg -i` leaves a package unconfigured when a dependency is missing, and
the original script stopped there. It now runs `sudo apt-get install -f -y`
afterwards.

## 7. Correct `.app` selection, and a loud Electron fallback

**`scripts/lib/dmg.sh`**

Bundle selection was `find … | sort | head -1` — alphabetical, therefore
arbitrary. That was tolerable when the DMG contained one `.app`; the current
official macOS download is a **universal** build that can contain several. New
`pick_app_bundle()` prefers a bundle named `*WorkBuddy*`, otherwise the largest,
and says which one it chose.

Electron version detection silently fell back to a hardcoded `41.1.1`. A runtime
mismatch is exactly the kind of thing that produces a meaningless crash, so the
fallback now warns.

## 8. Chromium sandbox is configurable

**`install.sh`**

The launcher hardcoded `--no-sandbox --disable-gpu-sandbox`. Behaviour is
unchanged by default (it is still required on most systems), but `WB_NO_SANDBOX=0`
now re-enables the sandbox on systems that support unprivileged user namespaces,
and the trade-off is documented in the README rather than buried in a heredoc.

---

## Not changed

- **Patch logic.** `apply-linux-patches.js` is byte-identical to upstream. Its
  exact-string anchors are the real reason newer DMGs fail, and rewriting them
  without a set of real DMGs to test against would be guesswork. Change 1 makes
  the failure visible instead of silent; that is the honest half of the fix.
- **Packaging metadata**, `docs/`, and the `workbuddy.sh`/launcher layout.
- **Upstream limitations**: no Tencent Docs engine, degraded AI code sandbox, no
  auto-update. These need Linux builds of proprietary components and cannot be
  fixed here.

## Verification

```
bash -n   install.sh scripts/*.sh scripts/lib/*.sh    all pass
node --check scripts/lib/apply-linux-patches.js        passes
pick_app_bundle                                       4 cases tested:
                                                      single / named-wins /
                                                      largest-fallback / empty
```

Not verified: a full end-to-end conversion. That requires a real WorkBuddy DMG
and an x86_64 Linux host, neither of which was available when these changes were
written.
