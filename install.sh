#!/bin/bash
# GitLab Knowledge Graph (gkg) Installation Script
# Supports Mac (darwin) and Linux with x86_64 and aarch64 architectures
# To run the already downloaded script:
#   bash install.sh

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
    --version VERSION    Install specific version (e.g., v0.11.0)
    --force             Force installation even if gkg already exists
    --help              Show this help message

EXAMPLES:
    # Install latest version
    bash install.sh

    # Install specific version
    bash install.sh --version v0.11.0

    # Force reinstall
    bash install.sh --force
EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            if [[ -z "${2:-}" || "$2" == --* ]]; then
                error "Missing value for --version"
            fi
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

# Normalize version to include leading 'v' when provided
if [ -n "$VERSION" ]; then
    case "$VERSION" in
        v*) : ;;
        *) VERSION="v$VERSION" ;;
    esac
fi

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

    if command_exists curl; then
        curl -fsSL --progress-bar "$url" -o "$output" || return 1
    elif command_exists wget; then
        wget -q --show-progress "$url" -O "$output" || return 1
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
        error "Checksum file not found at $checksum_url. Installation aborted for security reasons."
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
        error "Checksum verification failed for $file using $checksum_url\nExpected: $expected_checksum\nActual:   $actual_checksum"
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
        echo "To upgrade or reinstall:"
        echo "  - Reinstall same/latest: run with --force"
        echo "  - Install specific: run with --version vX.Y.Z [and optionally --force]"
        exit 0
    fi
    
    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"
    
    # Construct download URL using GitLab API v4
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
    
    # Download the tarball
    local tarball="${TEMP_DIR}/${artifact_name}"
    echo "Downloading GitLab Knowledge Graph for ${platform}-${arch}..."
    if ! download_file "$download_url" "$tarball"; then
        error "Failed to download GitLab Knowledge Graph from $download_url. Please check your internet connection and the version number."
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

    # Install to install directory
    echo "Installing gkg to $INSTALL_DIR..."
    if command_exists install; then
        install -m 0755 "$gkg_binary" "$INSTALL_DIR/gkg"
    else
        chmod +x "$gkg_binary"
        mv "$gkg_binary" "$INSTALL_DIR/gkg"
    fi
    
    success "GitLab Knowledge Graph (gkg) has been successfully installed to $INSTALL_DIR/gkg"
}

# Update PATH for all detected shells
update_path() {
    local targets=()
    local os_name="$(uname -s)"

    # Detect and prepare rc files for zsh and bash, regardless of current shell
    if command_exists zsh || [ -x "/bin/zsh" ]; then
        targets+=("${HOME}/.zshrc")
        if [ "$os_name" = "Darwin" ]; then
            targets+=("${HOME}/.zprofile")
        fi
    fi

    if command_exists bash || [ -x "/bin/bash" ]; then
        if [ "$os_name" = "Darwin" ]; then
            targets+=("${HOME}/.bash_profile")
        else
            targets+=("${HOME}/.bashrc")
        fi
    fi

    # Always include .profile for broader compatibility
    targets+=("${HOME}/.profile")

    # Ensure PATH export exists in each target file
    for shell_rc in "${targets[@]}"; do
        echo "Updating PATH in $shell_rc..."
        touch "$shell_rc"

        # Consider it present if any PATH assignment mentions .local/bin via $HOME, ${HOME}, ~, or expanded absolute path
        local home_path="${HOME}/.local/bin"

        if grep -Fq "# Added by GitLab Knowledge Graph installer" "$shell_rc"; then
            echo "PATH export already exists in $shell_rc"
            continue
        fi

        if grep -Eq '^[[:space:]]*(export[[:space:]]+)?PATH=.*((\$HOME|\${HOME}|~)/\.local/bin)' "$shell_rc" || \
           grep -Fq "$home_path" "$shell_rc"; then
            echo "PATH export already exists in $shell_rc"
        else
            echo "" >> "$shell_rc"
            echo "# Added by GitLab Knowledge Graph installer" >> "$shell_rc"
            echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >> "$shell_rc"
            success "PATH has been updated in $shell_rc"
        fi
    done
}

# Ensure required dependencies are available
ensure_dependencies() {
    if ! command_exists tar; then
        error "Required dependency 'tar' not found. Please install it and re-run the installer."
    fi
    if ! command_exists curl && ! command_exists wget; then
        error "Neither 'curl' nor 'wget' found. Please install one of them and re-run the installer."
    fi
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
    
    # Check dependencies
    ensure_dependencies

    # Install gkg
    install_gkg "$platform" "$arch"
    
    # Update PATH
    update_path
    
    echo
    success "Installation complete!"
    echo
    echo "To start using gkg in your terminal run:"
    if [ "$platform" = "darwin" ]; then
        echo "  - zsh:  'source ~/.zshrc' (login shells: 'source ~/.zprofile')"
        echo "  - bash: 'source ~/.bash_profile'"
    else
        echo "  - zsh:  'source ~/.zshrc'"
        echo "  - bash: 'source ~/.bashrc'"
    fi
    echo "  - Or open a new terminal"
    echo
    echo "If you use other shells or terminals, add \$HOME/.local/bin to PATH manually."
    echo
    echo "Then verify the installation with: gkg --version"
}

# Run main function
main
