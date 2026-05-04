#!/bin/sh
# rustkyll installer for Linux and macOS.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/alexeygrigorev/rustkyll/main/scripts/install.sh | sh
#
# Environment variables:
#   RUSTKYLL_VERSION       - release tag to install (default: latest)
#   RUSTKYLL_INSTALL_DIR   - directory to install into (default: $HOME/.local/bin)

set -eu

REPO="alexeygrigorev/rustkyll"
VERSION="${RUSTKYLL_VERSION:-latest}"
INSTALL_DIR="${RUSTKYLL_INSTALL_DIR:-$HOME/.local/bin}"

err() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

info() {
    printf '%s\n' "$1"
}

detect_target() {
    os=$(uname -s)
    arch=$(uname -m)

    case "$os" in
        Linux)  os_part="linux" ;;
        Darwin) os_part="darwin" ;;
        *) err "unsupported OS: $os (Windows users: install via 'pip install rustkyll' or download the .exe from https://github.com/$REPO/releases)" ;;
    esac

    case "$arch" in
        x86_64|amd64)        arch_part="amd64" ;;
        aarch64|arm64)       arch_part="arm64" ;;
        *) err "unsupported architecture: $arch" ;;
    esac

    echo "${os_part}-${arch_part}"
}

pick_downloader() {
    if command -v curl >/dev/null 2>&1; then
        echo "curl"
    elif command -v wget >/dev/null 2>&1; then
        echo "wget"
    else
        err "need curl or wget to download the release"
    fi
}

download() {
    url="$1"
    out="$2"
    case "$DOWNLOADER" in
        curl) curl -fsSL "$url" -o "$out" ;;
        wget) wget -q "$url" -O "$out" ;;
    esac
}

main() {
    target=$(detect_target)
    asset="rustkyll-${target}"
    DOWNLOADER=$(pick_downloader)

    if [ "$VERSION" = "latest" ]; then
        url="https://github.com/${REPO}/releases/latest/download/${asset}"
        info "Installing latest rustkyll for ${target}..."
    else
        url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
        info "Installing rustkyll ${VERSION} for ${target}..."
    fi

    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    info "Downloading ${url}"
    if ! download "$url" "$tmpdir/rustkyll"; then
        err "download failed - check that ${asset} exists at the requested release"
    fi

    chmod +x "$tmpdir/rustkyll"

    mkdir -p "$INSTALL_DIR"
    mv "$tmpdir/rustkyll" "$INSTALL_DIR/rustkyll"

    info "Installed to ${INSTALL_DIR}/rustkyll"

    installed_version=$("$INSTALL_DIR/rustkyll" --version 2>/dev/null || echo "(version check failed)")
    info "Version: ${installed_version}"

    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            info "Run 'rustkyll --help' to get started."
            ;;
        *)
            info ""
            info "${INSTALL_DIR} is not in your PATH."
            info "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
            info ""
            info "    export PATH=\"${INSTALL_DIR}:\$PATH\""
            ;;
    esac
}

main "$@"
