#!/bin/bash
set -e

# termflix installer
# Usage: curl -fsSL https://raw.githubusercontent.com/paulrobello/termflix/main/install.sh | bash

REPO="paulrobello/termflix"
BINARY="termflix"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}ℹ${NC}  $1"; }
ok()    { echo -e "${GREEN}✓${NC}  $1"; }
warn()  { echo -e "${YELLOW}⚠${NC}  $1"; }
error() { echo -e "${RED}✗${NC}  $1"; exit 1; }

# Detect OS and architecture
detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="macos" ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *) error "Unsupported OS: $(uname -s)" ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) error "Unsupported architecture: $(uname -m)" ;;
    esac

    echo "${os}-${arch}"
}

# Map platform to artifact name
get_artifact_name() {
    local platform="$1"
    case "$platform" in
        linux-x86_64)   echo "${BINARY}-linux-x86_64" ;;
        linux-aarch64)  echo "${BINARY}-linux-aarch64" ;;
        macos-x86_64)   echo "${BINARY}-macos-x86_64" ;;
        macos-aarch64)  echo "${BINARY}-macos-aarch64" ;;
        windows-x86_64) echo "${BINARY}-windows-x86_64.exe" ;;
        *) error "No binary available for: $platform" ;;
    esac
}

# Get latest release tag
get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    if command -v curl &>/dev/null; then
        curl -fsSL "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
    elif command -v wget &>/dev/null; then
        wget -qO- "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
    else
        error "Neither curl nor wget found. Please install one."
    fi
}

# Download file
download() {
    local url="$1" dest="$2"
    if command -v curl &>/dev/null; then
        curl -fsSL -o "$dest" "$url"
    elif command -v wget &>/dev/null; then
        wget -qO "$dest" "$url"
    fi
}

main() {
    echo ""
    echo -e "${CYAN}🎬 termflix installer${NC}"
    echo ""

    # Detect platform
    local platform
    platform=$(detect_platform)
    info "Detected platform: ${platform}"

    # Get artifact name
    local artifact
    artifact=$(get_artifact_name "$platform")

    # Get latest version
    info "Fetching latest release..."
    local version
    version=$(get_latest_version)
    if [ -z "$version" ]; then
        error "Could not determine latest version. Check https://github.com/${REPO}/releases"
    fi
    if [[ ! "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        error "Refusing unexpected version tag from GitHub API: '${version}' (expected vX.Y.Z)"
    fi
    ok "Latest version: ${version}"

    # Download
    local download_url="https://github.com/${REPO}/releases/download/${version}/${artifact}"
    local tmp_file
    tmp_file=$(mktemp)

    info "Downloading ${artifact}..."
    download "$download_url" "$tmp_file" || {
        rm -f "$tmp_file"
        error "Download failed. Check https://github.com/${REPO}/releases"
    }

    if [ ! -s "$tmp_file" ]; then
        rm -f "$tmp_file"
        error "Download failed (empty file). Check https://github.com/${REPO}/releases"
    fi
    ok "Downloaded successfully"

    # Verify checksum against the release's SHA256SUMS before installing
    local sums_file
    sums_file=$(mktemp)
    info "Downloading SHA256SUMS..."
    download "https://github.com/${REPO}/releases/download/${version}/SHA256SUMS" "$sums_file" || {
        rm -f "$tmp_file" "$sums_file"
        error "Could not download SHA256SUMS for ${version}. Refusing to install an unverified binary."
    }

    local expected
    expected=$(awk -v f="$artifact" '$2 == f {print $1}' "$sums_file")
    if [ -z "$expected" ]; then
        rm -f "$tmp_file" "$sums_file"
        error "No checksum found for ${artifact} in SHA256SUMS. Refusing to install."
    fi

    local actual
    if command -v sha256sum &>/dev/null; then
        actual=$(sha256sum "$tmp_file" | awk '{print $1}')
    elif command -v shasum &>/dev/null; then
        actual=$(shasum -a 256 "$tmp_file" | awk '{print $1}')
    else
        rm -f "$tmp_file" "$sums_file"
        error "Neither sha256sum nor shasum is available. Cannot verify the binary."
    fi

    if [ "$actual" != "$expected" ]; then
        rm -f "$tmp_file" "$sums_file"
        error "Checksum mismatch for ${artifact}: expected ${expected}, got ${actual}."
    fi
    rm -f "$sums_file"
    ok "Checksum verified"

    # Install
    local dest="${INSTALL_DIR}/${BINARY}"
    if [[ "$platform" == windows-* ]]; then
        dest="${INSTALL_DIR}/${BINARY}.exe"
    fi

    # Check if we need sudo
    local use_sudo=""
    if [ ! -w "$INSTALL_DIR" ]; then
        if command -v sudo &>/dev/null; then
            use_sudo="sudo"
            warn "Need sudo to install to ${INSTALL_DIR}"
        else
            error "Cannot write to ${INSTALL_DIR} and sudo is not available. Set INSTALL_DIR to a writable location:\n  INSTALL_DIR=~/.local/bin curl -fsSL ... | bash"
        fi
    fi

    # Create install dir if needed
    $use_sudo mkdir -p "$INSTALL_DIR"

    # Move binary
    $use_sudo mv "$tmp_file" "$dest"
    $use_sudo chmod +x "$dest"

    # Verify
    if command -v "$BINARY" &>/dev/null; then
        ok "Installed ${BINARY} ${version} to ${dest}"
    else
        ok "Installed to ${dest}"
        if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
            warn "${INSTALL_DIR} is not in your PATH. Add it:\n  export PATH=\"${INSTALL_DIR}:\$PATH\""
        fi
    fi

    echo ""
    echo -e "${GREEN}🎬 Run '${BINARY}' to start!${NC}"
    echo -e "   ${BINARY} --list        # List all animations"
    echo -e "   ${BINARY} -a fire       # Run fire animation"
    echo -e "   ${BINARY} --help        # Show all options"
    echo ""
}

main "$@"
