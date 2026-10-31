#!/bin/bash
# Linux Electron runtime download. Sourced by install.sh.

electron_arch() {
    case "$ARCH" in
        x86_64) echo "x64" ;;
        aarch64) echo "arm64" ;;
        armv7l) echo "armv7l" ;;
        *) error "Unsupported architecture: $ARCH" ;;
    esac
}

download_electron_runtime() {
    local arch zip_name cache_dir cached_zip partial_zip
    local -a urls=()
    local url ok

    arch="$(electron_arch)"
    zip_name="electron-v${ELECTRON_VERSION}-linux-${arch}.zip"
    if [ -n "$ELECTRON_MIRROR" ]; then
        urls+=("${ELECTRON_MIRROR%/}/v${ELECTRON_VERSION}/${zip_name}")
    else
        urls+=("https://github.com/electron/electron/releases/download/v${ELECTRON_VERSION}/${zip_name}")
        # github.com is unreachable on plenty of networks (mainland China among
        # them) even when the rest of the internet is fine. npmmirror carries the
        # same official release artifacts, so fall through to it rather than
        # failing the build outright.
        urls+=("https://npmmirror.com/mirrors/electron/v${ELECTRON_VERSION}/${zip_name}")
    fi

    cache_dir="${WORKBUDDY_ELECTRON_CACHE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/workbuddy-linux/electron}"
    cached_zip="$cache_dir/$zip_name"
    partial_zip="$cached_zip.part"
    mkdir -p "$cache_dir"

    if [ ! -f "$cached_zip" ]; then
        ok=0
        for url in "${urls[@]}"; do
            info "Downloading $zip_name from $url"
            if curl -L --fail --continue-at - --progress-bar -o "$partial_zip" "$url"; then
                ok=1
                break
            fi
            warn "Download failed from that source; trying the next one"
            rm -f "$partial_zip"
        done
        [ "$ok" -eq 1 ] || error "Could not download $zip_name from any source.
Set ELECTRON_MIRROR to a reachable mirror, or pre-seed:
  $cached_zip"
        mv "$partial_zip" "$cached_zip"
    else
        info "Using cached Electron runtime: $cached_zip"
    fi

    unzip -qo "$cached_zip" -d "$INSTALL_DIR"
    [ -x "$INSTALL_DIR/electron" ] || error "Electron binary was not extracted"
}
