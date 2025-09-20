#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
RELEASE_DIR="$PROJECT_ROOT/release"

PLATFORM=${PLATFORM:-$(uname -s)}
PLATFORM=$(echo "$PLATFORM" | tr '[:upper:]' '[:lower:]')

ARCH=${ARCH:-$(uname -m)}
if [ "$ARCH" = "arm64" ]; then
    ARCH="aarch64"
fi
CARGO_PARAMS=${CARGO_PARAMS:-"--locked --release"}

echo "Building for $PLATFORM $ARCH"

case "$PLATFORM" in
    darwin)
        case "$ARCH" in
            aarch64|arm64)
                TARGET="aarch64-apple-darwin"
                ;;
            x86_64)
                TARGET="x86_64-apple-darwin"
                ;;
        esac
        ;;
    linux)
        case "$ARCH" in
            aarch64)
                TARGET="aarch64-unknown-linux-gnu"
                ;;
            x86_64)
                TARGET="x86_64-unknown-linux-gnu"
                ;;
        esac
        ;;
esac

if [ -z "$TARGET" ]; then
    echo "unknown arch '$ARCH' or platform '$PLATFORM'"
    exit 1
fi

build_bin() {
    cargo build $CARGO_PARAMS --bin gkg --target $TARGET
    tar -czvf gkg-${PLATFORM}-${ARCH}.tar.gz -C target/${TARGET}/release gkg
    echo "created gkg-${PLATFORM}-${ARCH}.tar.gz"
}

build_bin
