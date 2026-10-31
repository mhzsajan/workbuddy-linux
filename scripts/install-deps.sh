#!/bin/bash
set -Eeuo pipefail

MIN_NODE_MAJOR=20
NODEJS_MAJOR="${NODEJS_MAJOR:-22}"

info() {
    echo "[INFO] $*"
}

warn() {
    echo "[WARN] $*" >&2
}

error() {
    echo "[ERROR] $*" >&2
    exit 1
}

detect_distro() {
    if command -v apt-get >/dev/null 2>&1; then
        echo "apt"
    elif command -v dnf5 >/dev/null 2>&1; then
        echo "dnf5"
    elif command -v dnf >/dev/null 2>&1; then
        echo "dnf"
    elif command -v pacman >/dev/null 2>&1; then
        echo "pacman"
    elif command -v zypper >/dev/null 2>&1; then
        echo "zypper"
    else
        echo "unknown"
    fi
}

node_major() {
    local version major

    command -v node >/dev/null 2>&1 || return 1
    version="$(node -v 2>/dev/null || true)"
    major="${version#v}"
    major="${major%%.*}"

    case "$major" in
        ""|*[!0-9]*) return 1 ;;
        *) echo "$major" ;;
    esac
}

has_compatible_nodejs() {
    local major

    major="$(node_major 2>/dev/null || true)"
    [ -n "$major" ] \
        && [ "$major" -ge "$MIN_NODE_MAJOR" ] \
        && command -v npm >/dev/null 2>&1 \
        && command -v npx >/dev/null 2>&1
}

validate_nodejs_major() {
    case "$NODEJS_MAJOR" in
        ""|*[!0-9]*)
            error "NODEJS_MAJOR must be numeric, for example: NODEJS_MAJOR=22 bash scripts/install-deps.sh"
            ;;
    esac

    [ "$NODEJS_MAJOR" -ge "$MIN_NODE_MAJOR" ] || error "NODEJS_MAJOR must be >= $MIN_NODE_MAJOR"
}

apt_nodejs_candidate_major() {
    local candidate major=""

    candidate="$(apt-cache policy nodejs 2>/dev/null | awk '/Candidate:/ { print $2; exit }' || true)"
    [ -n "$candidate" ] && [ "$candidate" != "(none)" ] || return 1

    if [[ "$candidate" =~ ^[0-9]+:([0-9]+)\. ]]; then
        major="${BASH_REMATCH[1]}"
    elif [[ "$candidate" =~ ^([0-9]+)\. ]]; then
        major="${BASH_REMATCH[1]}"
    else
        return 1
    fi

    echo "$major"
}

install_nodesource_nodejs() {
    validate_nodejs_major

    local apt_arch keyring source_list tmp_key

    apt_arch="$(dpkg --print-architecture)"
    case "$apt_arch" in
        amd64|arm64|armhf) ;;
        *) error "NodeSource does not support apt architecture: $apt_arch" ;;
    esac

    keyring="/etc/apt/keyrings/nodesource.gpg"
    source_list="/etc/apt/sources.list.d/nodesource.list"
    tmp_key="$(mktemp)"

    info "Installing Node.js ${NODEJS_MAJOR}.x from NodeSource"
    sudo apt-get install -y ca-certificates gnupg
    sudo install -d -m 0755 /etc/apt/keyrings
    curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key -o "$tmp_key"
    gpg --dearmor < "$tmp_key" | sudo tee "$keyring" >/dev/null
    rm -f "$tmp_key"
    sudo chmod 0644 "$keyring"

    printf 'deb [arch=%s signed-by=%s] https://deb.nodesource.com/node_%s.x nodistro main\n' \
        "$apt_arch" "$keyring" "$NODEJS_MAJOR" \
        | sudo tee "$source_list" >/dev/null
    sudo apt-get update -qq
    sudo apt-get install -y nodejs
}

ensure_apt_nodejs() {
    local major

    if has_compatible_nodejs; then
        info "Node.js toolchain ready: node $(node -v), npm $(npm -v), npx $(npx -v)"
        return 0
    fi

    major="$(apt_nodejs_candidate_major 2>/dev/null || true)"
    if [ -n "$major" ] && [ "$major" -ge "$MIN_NODE_MAJOR" ]; then
        info "Installing distro Node.js/npm candidate"
        sudo apt-get install -y nodejs npm
    else
        install_nodesource_nodejs
    fi

    has_compatible_nodejs || error "Node.js $MIN_NODE_MAJOR+ with npm/npx is still unavailable"
    info "Node.js toolchain ready: node $(node -v), npm $(npm -v), npx $(npx -v)"
}

ensure_generic_nodejs() {
    if has_compatible_nodejs; then
        info "Node.js toolchain ready: node $(node -v), npm $(npm -v), npx $(npx -v)"
        return 0
    fi

    error "Node.js $MIN_NODE_MAJOR+ with npm and npx is required. Install a supported Node.js package for this distro and re-run this script."
}

apt_has_7z() {
    command -v 7zz >/dev/null 2>&1 || command -v 7z >/dev/null 2>&1
}

# '7zip' only exists as a package name from Ubuntu 24.04 onwards. On 22.04 the
# old code did `apt-get install 7zip`, which fails outright and takes the whole
# `make deps` run with it. Try both names and explain the fallback honestly.
ensure_apt_7zip() {
    if apt_has_7z; then
        info "7-Zip already present: $(command -v 7zz 2>/dev/null || command -v 7z)"
        return 0
    fi

    if sudo apt-get install -y 7zip >/dev/null 2>&1 && apt_has_7z; then
        info "Installed 7zip"
        return 0
    fi

    warn "'7zip' is not installable on this release (the package appears in Ubuntu 24.04+)."
    if sudo apt-get install -y p7zip-full >/dev/null 2>&1 && apt_has_7z; then
        warn "Installed p7zip-full instead. p7zip is older than 21 and may fail to"
        warn "extract modern DMGs; if extraction fails, use the official static build."
        return 0
    fi

    error "No 7-Zip found. Install the official static build (version >= 21):
  curl -LO https://www.7-zip.org/a/7z2408-linux-x64.tar.xz
  sudo tar -C /usr/local/bin -xf 7z2408-linux-x64.tar.xz 7zz
Or upgrade to Ubuntu 24.04 or newer."
}

install_apt() {
    info "Detected Debian/Ubuntu (apt)"
    sudo apt-get update -qq
    sudo apt-get install -y \
        bash ca-certificates curl unzip python3 make g++ pkg-config \
        libx11-dev libxkbfile-dev libsecret-1-dev libkrb5-dev \
        dpkg-dev fakeroot desktop-file-utils icnsutils imagemagick
    ensure_apt_7zip
    ensure_apt_nodejs
}

install_dnf5() {
    info "Detected Fedora 41+ (dnf5)"
    sudo dnf install -y \
        bash curl unzip 7zip python3 nodejs npm make gcc-c++ pkgconf-pkg-config \
        libX11-devel libxkbfile-devel libsecret-devel krb5-devel \
        rpm-build desktop-file-utils ImageMagick
    ensure_generic_nodejs
}

install_dnf() {
    info "Detected Fedora/RHEL (dnf)"
    sudo dnf install -y \
        bash curl unzip 7zip python3 nodejs npm make gcc-c++ pkgconf-pkg-config \
        libX11-devel libxkbfile-devel libsecret-devel krb5-devel \
        rpm-build desktop-file-utils ImageMagick
    ensure_generic_nodejs
}

install_pacman() {
    info "Detected Arch Linux (pacman)"
    sudo pacman -S --needed --noconfirm \
        bash curl unzip 7zip python nodejs npm base-devel zstd fakeroot \
        libx11 libxkbfile libsecret krb5 desktop-file-utils imagemagick
    ensure_generic_nodejs
}

install_zypper() {
    info "Detected openSUSE (zypper)"
    sudo zypper --non-interactive install \
        bash curl unzip 7zip python3 nodejs-default npm-default make gcc-c++ pkg-config \
        libX11-devel libxkbfile-devel libsecret-devel krb5-devel \
        rpm-build desktop-file-utils ImageMagick
    ensure_generic_nodejs
}

main() {
    local distro

    distro="$(detect_distro)"
    case "$distro" in
        apt) install_apt ;;
        dnf5) install_dnf5 ;;
        dnf) install_dnf ;;
        pacman) install_pacman ;;
        zypper) install_zypper ;;
        *)
            error "Unsupported package manager. Install bash, curl, unzip, 7z/7zz, python3, Node.js $MIN_NODE_MAJOR+, npm, npx, make, g++, X11/libxkbfile/libsecret/krb5 development headers, then re-run make build-app."
            ;;
    esac

    info "Dependencies are ready. Next steps:"
    info "  put one official Intel/x64 WorkBuddy DMG in downloads/"
    info "  make build-app"
    info "  make package"
    info "  make install"
}

main "$@"
