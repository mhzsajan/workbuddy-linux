#!/bin/bash
# Build the files that get attached to a GitHub release.
#
# Releases ship the *toolchain*, never a converted application: the payload of a
# converted build is Tencent's proprietary app.asar, and this repository does not
# redistribute it. What is published here is everything needed to reproduce the
# build locally from an archive you obtained yourself.
#
#   dist/release/workbuddy-linux-<version>.tar.gz   full toolchain source
#   dist/release/install-workbuddy.sh               self-contained bootstrap
#   dist/release/SHA256SUMS                         checksums for both
#   dist/release/CHANGELOG.md                       copy, for the release body
set -Eeuo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
. "$REPO_DIR/scripts/lib/common.sh"

VERSION="${1:-${VERSION:-}}"
if [ -z "$VERSION" ]; then
    VERSION="$(git -C "$REPO_DIR" describe --tags --abbrev=0 2>/dev/null | sed 's/^v//' || true)"
fi
[ -n "$VERSION" ] || error "No version. Pass one: make release-bundle VERSION=1.2.0"

REPO_SLUG="${REPO_SLUG:-mhzsajan/workbuddy-linux}"
OUT_DIR="$REPO_DIR/dist/release"
ARCHIVE="workbuddy-linux-$VERSION.tar.gz"

info "Building release assets for v$VERSION"
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

# --- source bundle -------------------------------------------------------
# Excluded: .git, installed payloads, downloads, and any build output. What is
# left is small enough to read and contains no third-party binaries.
info "  packaging $ARCHIVE"
tar czf "$OUT_DIR/$ARCHIVE" \
    --exclude='.git' \
    --exclude='node_modules' \
    --exclude='downloads' \
    --exclude='build' \
    --exclude='dist' \
    --exclude='workbuddy-app' \
    --exclude='workbuddy-app-next' \
    --transform "s,^\.,workbuddy-linux-$VERSION," \
    -C "$REPO_DIR" .

# --- bootstrap installer -------------------------------------------------
# Single file a user can curl. It fetches the source bundle from the release,
# verifies it against SHA256SUMS, unpacks it and hands over to install.sh.
info "  generating install-workbuddy.sh"
BOOTSTRAP="$OUT_DIR/install-workbuddy.sh"
cat > "$BOOTSTRAP" <<'BOOTSTRAP_EOF'
#!/bin/bash
# install-workbuddy.sh - bootstrap the WorkBuddy-for-Linux build toolchain.
#
# This does NOT download WorkBuddy. It fetches the converter, then asks you for
# an official macOS WorkBuddy archive (a China-channel .dmg or an
# international-channel .zip) which you obtain from Tencent yourself.
#
#   ./install-workbuddy.sh                      # interactive
#   ./install-workbuddy.sh ~/WorkBuddy.dmg      # pass the archive directly
#   ./install-workbuddy.sh --toolchain-only     # just unpack, do not build
set -Eeuo pipefail

VERSION="__VERSION__"
REPO_SLUG="__REPO_SLUG__"
ARCHIVE="workbuddy-linux-__VERSION__.tar.gz"
BASE_URL="https://github.com/$REPO_SLUG/releases/download/v$VERSION"
DEST="${WORKBUDDY_TOOLCHAIN_DIR:-$HOME/.local/share/workbuddy-linux}"

die() { echo "error: $*" >&2; exit 1; }
say() { echo "[bootstrap] $*" >&2; }

for cmd in curl tar sha256sum; do
    command -v "$cmd" >/dev/null 2>&1 || die "missing required command: $cmd"
done

say "toolchain v$VERSION -> $DEST"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

say "downloading $ARCHIVE"
curl -fsSL --retry 3 --retry-all-errors -o "$WORK/$ARCHIVE" "$BASE_URL/$ARCHIVE" \
    || die "download failed: $BASE_URL/$ARCHIVE"

# Verify against the published checksum list when it is reachable. A mismatch is
# fatal; an unreachable list is a warning, so an offline mirror still works.
if curl -fsSL --retry 2 -o "$WORK/SHA256SUMS" "$BASE_URL/SHA256SUMS" 2>/dev/null; then
    expected="$(awk -v f="$ARCHIVE" '$2 == f || $2 == "*"f {print $1}' "$WORK/SHA256SUMS")"
    if [ -n "$expected" ]; then
        actual="$(sha256sum "$WORK/$ARCHIVE" | awk '{print $1}')"
        [ "$expected" = "$actual" ] \
            || die "checksum mismatch for $ARCHIVE (expected $expected, got $actual)"
        say "checksum verified"
    else
        say "warning: $ARCHIVE not listed in SHA256SUMS"
    fi
else
    say "warning: could not fetch SHA256SUMS; skipping verification"
fi

say "unpacking"
mkdir -p "$DEST"
tar xzf "$WORK/$ARCHIVE" -C "$DEST" --strip-components=1
chmod +x "$DEST/install.sh" "$DEST"/scripts/*.sh "$DEST"/scripts/lib/*.sh 2>/dev/null || true

if [ "${1:-}" = "--toolchain-only" ]; then
    say "toolchain unpacked. Build later with:"
    say "  $DEST/install.sh /path/to/WorkBuddy.dmg-or-.zip"
    exit 0
fi

say "starting the converter"
exec bash "$DEST/install.sh" "$@"
BOOTSTRAP_EOF

# Bake the version/repo in. Done with a placeholder replace rather than an
# unquoted heredoc so nothing else in the script is expanded.
sed -i.bak \
    -e "s/__VERSION__/$VERSION/g" \
    -e "s|__REPO_SLUG__|$REPO_SLUG|g" \
    "$BOOTSTRAP"
rm -f "$BOOTSTRAP.bak"
chmod +x "$BOOTSTRAP"

# --- changelog copy ------------------------------------------------------
if [ -f "$REPO_DIR/CHANGELOG.md" ]; then
    cp "$REPO_DIR/CHANGELOG.md" "$OUT_DIR/CHANGELOG.md"
fi

# --- checksums -----------------------------------------------------------
info "  writing SHA256SUMS"
(
    cd "$OUT_DIR"
    # CHANGELOG.md is excluded: it is release-body material, not an artifact.
    sha256sum "$ARCHIVE" install-workbuddy.sh > SHA256SUMS
)

info "Release assets in $OUT_DIR:"
ls -lh "$OUT_DIR" | sed 's/^/  /'
