#!/bin/bash
# Preflight checks. Run by install.sh before anything is downloaded, so a bad
# environment fails in two seconds instead of after a 2 GB Electron download.
#
# Can also be run standalone:
#   bash scripts/preflight.sh /path/to/WorkBuddy.dmg

set -Eeuo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$SCRIPT_DIR/scripts/lib/common.sh"

INPUT="${1:-}"
REQUIRED_FREE_KB=$((4 * 1024 * 1024)) # ~4 GB for Electron + extracted payload

check_arch() {
    local arch
    arch="$(uname -m)"
    case "$arch" in
        x86_64|amd64)
            info "Architecture: $arch"
            ;;
        aarch64|arm64)
            error "This toolchain downloads the x86_64 Linux Electron runtime, so it
cannot run natively on $arch. Use an x86_64 machine, or run it under emulation."
            ;;
        *)
            error "Unsupported architecture: $arch (x86_64 required)"
            ;;
    esac
}

check_disk() {
    local free_kb
    free_kb="$(df -Pk "$SCRIPT_DIR" 2>/dev/null | awk 'NR==2 {print $4}')"
    if [ -z "$free_kb" ]; then
        warn "Could not determine free disk space; skipping that check"
        return 0
    fi
    if [ "$free_kb" -lt "$REQUIRED_FREE_KB" ]; then
        error "Not enough free disk space: $((free_kb / 1024)) MB available,
$((REQUIRED_FREE_KB / 1024)) MB required. The Electron runtime plus the extracted
DMG payload needs roughly 4 GB."
    fi
    info "Free disk space: $((free_kb / 1024)) MB"
}

check_toolchain() {
    local seven_zip node_major
    # find_7z exits with a clear message when 7-Zip is missing or older than 21.
    seven_zip="$(find_7z)"
    info "7-Zip: $seven_zip"

    require_cmd node
    node_major="$(node -v | sed 's/^v//' | cut -d. -f1)"
    case "$node_major" in
        ''|*[!0-9]*) error "Could not parse the Node.js version from: $(node -v)" ;;
    esac
    if [ "$node_major" -lt 20 ]; then
        error "Node.js 20 or newer is required, found $(node -v). Run: make deps"
    fi
    info "Node.js: $(node -v)"
}

check_input() {
    [ -n "$INPUT" ] || return 0
    [ -e "$INPUT" ] || error "Input not found: $INPUT"

    if [ -d "$INPUT" ]; then
        info "Input: extracted bundle $INPUT"
        return 0
    fi

    local size_mb
    size_mb="$(du -m "$INPUT" | awk '{print $1}')"
    if [ "${size_mb:-0}" -lt 50 ]; then
        error "Input looks too small to be a WorkBuddy DMG: ${size_mb:-0} MB.
Did the download finish?"
    fi
    info "Input: $INPUT (${size_mb} MB)"

    # UDIF disk images end with a 512-byte 'koly' trailer.
    if [[ "$INPUT" == *.dmg ]]; then
        if ! tail -c 512 "$INPUT" 2>/dev/null | grep -qa "koly"; then
            warn "This file does not end with the expected UDIF trailer."
            warn "It may be a partial download or not a DMG at all."
        fi
    fi
}

check_distro_hint() {
    [ -r /etc/os-release ] || return 0
    local id version_id
    id="$(. /etc/os-release && printf '%s' "${ID:-}")"
    version_id="$(. /etc/os-release && printf '%s' "${VERSION_ID:-}")"

    if [ "$id" = "ubuntu" ]; then
        case "$version_id" in
            20.*|22.*|23.*)
                warn "Ubuntu $version_id does not ship the '7zip' package (it appears in"
                warn "24.04). The p7zip build it does ship is too old to extract modern"
                warn "DMGs. If 'make deps' fails, install the official static 7-Zip:"
                warn "  https://www.7-zip.org/a/7z2408-linux-x64.tar.xz"
                warn "  sudo tar -C /usr/local/bin -xf 7z2408-linux-x64.tar.xz 7zz"
                ;;
        esac
    fi
}

main() {
    info "=== Preflight ==="
    check_arch
    check_toolchain
    check_disk
    check_input
    check_distro_hint
    info "=== Preflight OK ==="
}

main "$@"
