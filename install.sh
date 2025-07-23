#!/bin/bash
# GitLab Knowledge Graph (gkg) Installation Script
# Supports Mac (darwin) and Linux with x86_64 and aarch64 architectures
# Usage: curl -fsSL https://example.com/install.sh | bash
#    or: curl -fsSL https://example.com/install.sh | bash -s -- --version v0.6.0
#    or: GITLAB_TOKEN=your-token curl -fsSL https://example.com/install.sh | bash
# To run the already downloaded script:
# cat install.sh | bash

set -euo pipefail

# Configuration
INSTALL_DIR="${HOME}/.local/bin"
TEMP_DIR=$(mktemp -d)
VERSION=""
FORCE_INSTALL=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    if [ -d "$TEMP_DIR" ]; then
        rm -rf "$TEMP_DIR"
    fi

    # TODO: This may include clean up for any copied files or changed environment variables
}

trap cleanup EXIT

# Error handling
error() {
    echo -e "${RED}Error: $1${NC}" >&2
    exit 1
}

# Success message
success() {
    echo -e "${GREEN}$1${NC}"
}

# Warning message
warning() {
    echo -e "${YELLOW}$1${NC}"
}

# Print usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

OPTIONS:
    --version VERSION    Install specific version (e.g., v0.6.0)
    --force             Force installation even if gkg already exists
    --help              Show this help message

ENVIRONMENT VARIABLES:
    GITLAB_TOKEN        GitLab personal access token for authentication

EXAMPLES:
    # Install latest version
    curl -fsSL https://example.com/install-gkg.sh | bash

    # Install specific version
    curl -fsSL https://example.com/install-gkg.sh | bash -s -- --version v0.6.0

    # Install with GitLab authentication
    GITLAB_TOKEN=your-token curl -fsSL https://example.com/install-gkg.sh | bash

    # Force reinstall
    curl -fsSL https://example.com/install-gkg.sh | bash -s -- --force
EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --force)
            FORCE_INSTALL=true
            shift
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            ;;
    esac
done

# Detect OS
detect_os() {
    local os
    case "$(uname -s)" in
        Linux*)
            os="linux"
            ;;
        Darwin*)
            os="darwin"
            ;;
        *)
            error "Unsupported operating system: $(uname -s)"
            ;;
    esac
    echo "$os"
}

# Detect architecture
detect_arch() {
    local arch
    case "$(uname -m)" in
        x86_64|amd64)
            arch="x86_64"
            ;;
        arm64|aarch64)
            arch="aarch64"
            ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            ;;
    esac
    echo "$arch"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Download file with progress
download_file() {
    local url="$1"
    local output="$2"
    
    # TODO: Remove this when our repository becomes public
    # Prepare authentication headers if GITLAB_TOKEN is set
    local auth_header=""
    if [ -n "${GITLAB_TOKEN:-}" ]; then
        auth_header="Authorization: Bearer $GITLAB_TOKEN"
    fi
    
    if command_exists curl; then
        if [ -n "$auth_header" ]; then
            curl -fsSL --progress-bar -H "$auth_header" "$url" -o "$output" || return 1
        else
            curl -fsSL --progress-bar "$url" -o "$output" || return 1
        fi
    elif command_exists wget; then
        if [ -n "$auth_header" ]; then
            wget -q --show-progress --header="$auth_header" "$url" -O "$output" || return 1
        else
            wget -q --show-progress "$url" -O "$output" || return 1
        fi
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Verify SHA256 checksum
verify_checksum() {
    local file="$1"
    local checksum_url="$2"
    local checksum_file="${TEMP_DIR}/checksum.sha256"
    
    echo "Downloading checksum..."
    if ! download_file "$checksum_url" "$checksum_file"; then
        error "Checksum file not found. Installation aborted for security reasons."
    fi
    
    echo "Verifying checksum..."
    local expected_checksum=$(cat "$checksum_file" | awk '{print $1}')
    local actual_checksum
    
    if command_exists sha256sum; then
        actual_checksum=$(sha256sum "$file" | awk '{print $1}')
    elif command_exists shasum; then
        actual_checksum=$(shasum -a 256 "$file" | awk '{print $1}')
    else
        warning "No SHA256 tool found. Skipping checksum verification."
        return 0
    fi
    
    if [ "$expected_checksum" != "$actual_checksum" ]; then
        error "Checksum verification failed!\nExpected: $expected_checksum\nActual: $actual_checksum"
    fi
    
    success "Checksum verified successfully."
}

# Install gkg
install_gkg() {
    local platform="$1"
    local arch="$2"
    
    # Check if gkg already exists
    if [ "$FORCE_INSTALL" = false ] && [ -f "$INSTALL_DIR/gkg" ]; then
        warning "GitLab Knowledge Graph (gkg) is already installed at $INSTALL_DIR/gkg"
        echo "Use --force to reinstall."
        exit 0
    fi
    
    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"
    
    # Construct download URL using GitLab API v4 to avoid Cloudflare
    local project_path="69095239"
    local artifact_name="gkg-${platform}-${arch}.tar.gz"
    local download_url
    local checksum_url
    
    if [ -z "$VERSION" ]; then
        echo "Installing latest version of GitLab Knowledge Graph..."
        download_url="https://gitlab.com/api/v4/projects/${project_path}/releases/permalink/latest/downloads/${artifact_name}"
        checksum_url="https://gitlab.com/api/v4/projects/${project_path}/releases/permalink/latest/downloads/${artifact_name}.sha256"
    else
        echo "Installing GitLab Knowledge Graph version $VERSION..."
        # For specific versions, use the version tag directly
        download_url="https://gitlab.com/api/v4/projects/${project_path}/releases/${VERSION}/downloads/${artifact_name}"
        checksum_url="https://gitlab.com/api/v4/projects/${project_path}/releases/${VERSION}/downloads/${artifact_name}.sha256"
    fi
    
    # Check if authentication might be needed
    if [ -n "${GITLAB_TOKEN:-}" ]; then
        echo "Using GitLab authentication token..."
    fi
    
    # Download the tarball
    local tarball="${TEMP_DIR}/${artifact_name}"
    echo "Downloading GitLab Knowledge Graph for ${platform}-${arch}..."
    if ! download_file "$download_url" "$tarball"; then
        if [ -z "${GITLAB_TOKEN:-}" ]; then
            error "Failed to download GitLab Knowledge Graph. If the repository requires authentication, please set GITLAB_TOKEN environment variable."
        else
            error "Failed to download GitLab Knowledge Graph. Please check your internet connection, GitLab token permissions, and the version number."
        fi
    fi
    
    # Verify checksum
    verify_checksum "$tarball" "$checksum_url"
    
    # Extract the tarball
    echo "Extracting gkg..."
    if ! tar -xzf "$tarball" -C "$TEMP_DIR"; then
        error "Failed to extract the tarball."
    fi
    
    # Find and move the gkg binary, right now binary is at the root of the tarball
    # but this may get changed in the future and we need to find it in subdirectories like ./bin
    local gkg_binary="${TEMP_DIR}/gkg"
    if [ ! -f "$gkg_binary" ]; then
        # Try to find it in subdirectories
        gkg_binary=$(find "$TEMP_DIR" -name "gkg" -type f -executable | head -n 1)
        if [ -z "$gkg_binary" ]; then
            error "gkg binary not found in the extracted files."
        fi
    fi

    # TODO: This may include copying default config files or any runtime libraries in future
    
    # Make sure it's executable
    chmod +x "$gkg_binary"
    
    # Move to install directory
    echo "Installing gkg to $INSTALL_DIR..."
    mv "$gkg_binary" "$INSTALL_DIR/gkg"
    
    success "GitLab Knowledge Graph (gkg) has been successfully installed to $INSTALL_DIR/gkg"
}

# Update PATH
update_path() {
    local shell_rc=""
    local shell_name=""
    
    # Detect shell
    if [ -n "${BASH_VERSION:-}" ]; then
        shell_name="bash"
        shell_rc="${HOME}/.bashrc"
        # Also update .bash_profile on macOS
        if [ "$(uname -s)" = "Darwin" ]; then
            shell_rc="${HOME}/.bash_profile"
        fi
    elif [ -n "${ZSH_VERSION:-}" ]; then
        shell_name="zsh"
        shell_rc="${HOME}/.zshrc"
    else
        shell_name="sh"
        shell_rc="${HOME}/.profile"
    fi
    
    # Check if PATH already contains INSTALL_DIR
    if echo "$PATH" | grep -q "$INSTALL_DIR"; then
        echo "PATH already contains $INSTALL_DIR"
        return 0
    fi
    
    # Add to PATH in shell rc file
    echo "Updating PATH in $shell_rc..."
    
    # Create rc file if it doesn't exist
    touch "$shell_rc"
    
    # Check if the export line already exists
    if ! grep -q "export PATH=\"\$HOME/.local/bin:\$PATH\"" "$shell_rc"; then
        echo "" >> "$shell_rc"
        echo "# Added by GitLab Knowledge Graph installer" >> "$shell_rc"
        echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >> "$shell_rc"
        success "PATH has been updated in $shell_rc"
    else
        echo "PATH export already exists in $shell_rc"
    fi
    
    # Also update .profile for broader compatibility
    if [ "$shell_rc" != "${HOME}/.profile" ] && [ -f "${HOME}/.profile" ]; then
        if ! grep -q "export PATH=\"\$HOME/.local/bin:\$PATH\"" "${HOME}/.profile"; then
            echo "" >> "${HOME}/.profile"
            echo "# Added by GitLab Knowledge Graph installer" >> "${HOME}/.profile"
            echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >> "${HOME}/.profile"
        fi
    fi
    
    warning "Please run 'source $shell_rc' or restart your terminal for PATH changes to take effect."
}

# Main installation process
main() {
    echo "=== GitLab Knowledge Graph (gkg) Installation Script ==="
    echo
    
    # Detect system information
    local platform=$(detect_os)
    local arch=$(detect_arch)
    
    echo "Detected system: ${platform}-${arch}"
    echo
    
    # Install gkg
    install_gkg "$platform" "$arch"
    
    # Update PATH
    update_path
    
    echo
    success "Installation complete!"
    echo
    echo "To start using gkg, either:"
    echo "  1. Run: source ~/.bashrc (or ~/.zshrc, ~/.profile)"
    echo "  2. Open a new terminal"
    echo
    echo "Then verify the installation with: gkg --version"
}

# Run main function
main
