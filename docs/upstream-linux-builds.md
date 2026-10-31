# Upstream Linux builds: what Tencent actually publishes

This page exists because the project's premise changed. It was written after
noticing that [`JipZeonGit/codebuddy-ide-cn-linux`](https://github.com/JipZeonGit/codebuddy-ide-cn-linux)
opened with a warning that Tencent now ships an official Linux `.deb` for
CodeBuddy IDE. The same turned out to be true for WorkBuddy — for one of the two
editions.

Verified 2026-10-31 by querying Tencent's own update API and ranged-reading the
published artifacts.

---

## The short version

| | China edition | International edition |
| --- | --- | --- |
| Update channel | `www.workbuddy.cn` | `www.workbuddy.ai` |
| x86_64 `.deb` | **yes** — `5.6.2.40046327` | advertised, **URL 404s** |
| x86_64 `.rpm` | **yes** — `5.6.2.40046327` | none |
| arm64 `.deb` | **yes** — `5.6.2.40046327` | none |
| AppImage / tar | none | none |

So: **if you have a Chinese account, use Tencent's package and ignore this
repository.** If you need the international edition, there is still no Linux
build and this converter remains the only route.

## The artifacts

`GET https://www.workbuddy.cn/v2/update?platform=<key>`:

```json
// workbuddy-linux-x64-deb
{"version":"5.6.2.40046327",
 "url":"https://download.codebuddy.cn/workbuddy/saas/linux-x64-deb/WorkBuddy-linux-x64-deb-5.6.2.40046327-c7b525f3.deb",
 "sha256hash":"12149ad8c0cd748fd5062d8f62c7827d688d8095427ada99b1ae9e7019897523",
 "supportsFastUpdate":false}

// workbuddy-linux-arm64-deb  -> .../linux-arm64-deb/...   sha256 561822578435d382...
// workbuddy-linux-x64-rpm    -> .../linux-x64-rpm/...     sha256 2e49fac166708e97...
```

Confirmed by ranged `Range: bytes 0-0` requests — `content-range` gives the real
size, and a 1 KB fetch starts with the `!<arch>` magic of a Debian package:

| Artifact | Bytes |
| --- | --- |
| x86_64 `.deb` | 429,829,872 |
| arm64 `.deb` | 400,068,468 |
| x86_64 `.rpm` | 378,230,056 |

The international channel is a different story. `www.workbuddy.ai` still answers
`workbuddy-linux-x64-deb` with `5.5.2.37849279`, and **both** that URL and the
5.6.2 filename pattern return `404 Not Found` from `download.codebuddy.ai`. It is
a stale catalogue entry, not a downloadable build. Do not trust the version
number it reports.

## What the official package declares

Read straight out of `control.tar.xz` (offset 132 in the ar archive, 245,080
bytes — no need to download the 410 MB payload):

```
Package: workbuddy
Version: 5.6.2
Vendor: Tencent Technology (Shenzhen) Company Limited <workbuddy@tencent.com>
Architecture: amd64
Installed-Size: 1673597
Depends: libgtk-3-0, libnotify4, libnss3, libxss1, libxtst6, xdg-utils,
         libatspi2.0-0, libuuid1, libsecret-1-0
Recommends: libappindicator3-1, imagemagick
Section: default
Homepage: https://www.workbuddy.cn
```

Two things worth copying:

1. **`Recommends: libappindicator3-1`.** That is the tray dependency. On GNOME a
   tray needs AppIndicator, not the raw `GtkStatusIcon` path. Note the ordering:
   the library is *necessary but not sufficient*. On the test host
   `libayatana-appindicator3-1` was already installed and the tray still does not
   appear, because the tray patches themselves do not apply to 5.9.4. Declaring
   the dependency stops the tray from being doubly broken; it does not create it.
2. **The `Depends` list is longer than a naive Electron guess.** This project
   used to declare only GTK/NSS/XSS/asound/secret/xkbfile, and silently skipped
   `libnotify4` (notifications), `libxtst6` (synthetic input), `libatspi2.0-0`
   (accessibility) and `libuuid1`. `packaging/linux/control` now mirrors the
   official list, with `t64` and `ayatana` alternatives so the same stanza works
   on Ubuntu 22.04 and 24.04+/26.04:

   ```
   Depends: libgtk-3-0 | libgtk-3-0t64 | libgtk-4-1, libnotify4, libnss3,
            libxss1, libxtst6, xdg-utils, libatspi2.0-0 | libatspi2.0-0t64,
            libuuid1, libsecret-1-0, libasound2 | libasound2t64, libxkbfile1
   Recommends: libayatana-appindicator3-1 | libappindicator3-1, imagemagick
   ```

`Installed-Size` of 1,673,597 KB (~1.6 GB) is also a useful reference point: an
official Linux build is roughly twice the size of a converted macOS payload,
because it ships the Linux native modules and both architecture-independent and
architecture-specific resources.

## Why this does not replace the converter

The official package is the **China edition**. Its product config is pinned to
`cn-saas` and its login is phone/WeChat — the same blocker documented in
[account-and-region.md](account-and-region.md). Getting a `.deb` does not change
which account system the binary talks to.

What the official package *is* good for:

- **A dependency and packaging reference** — this is where the `control` list
  above came from.
- **A future runtime donor.** If Tencent ever ships an international Linux build,
  or if the international macOS `app.asar` can be paired with an officially built
  Linux runtime, the conversion step disappears entirely. That is not done here;
  the two builds are different versions (5.6.2 vs 5.9.4), so the native modules
  would not match.
- **A signal to stop.** If the international channel starts publishing
  `workbuddy-linux-x64-deb` with a URL that resolves, this project should be
  archived rather than maintained.
