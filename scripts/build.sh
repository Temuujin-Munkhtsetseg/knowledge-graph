#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
INC_DIR="$PROJECT_ROOT/include"

PLATFORM=${PLATFORM:-$(uname -s)}
ARCH=${ARCH:-$(uname -m)}
CARGO_PARAMS=${CARGO_PARAMS:-"--locked --release"}

case "$PLATFORM" in
    Darwin)
        case "$ARCH" in
            aarch64)
                LIB_DIR="$PROJECT_ROOT/lib/darwin_arm64"
                TARGET="aarch64-apple-darwin"
                ;;
            x86_64)
                LIB_DIR="$PROJECT_ROOT/lib/darwin_amd64"
                TARGET="x86_64-apple-darwin"
                ;;
        esac
        ;;
    Linux)
        case "$ARCH" in
            aarch64)
                LIB_DIR="$PROJECT_ROOT/lib/linux_arm64"
                TARGET="aarch64-unknown-linux-gnu"
                ;;
            x86_64)
                LIB_DIR="$PROJECT_ROOT/lib/linux_amd64"
                TARGET="x86_64-unknown-linux-gnu"
                ;;
        esac
        ;;
esac

if [ -z "LIB_DIR" ]; then
    echo "unknown arch '$ARCH' or platform '$PLATFORM'"
    exit 1
fi


build_bin() {
    cargo build $CARGO_PARAMS --bin gkg --target $TARGET

    # Sign and notarize the binary
    if [ "$PLATFORM" = "Darwin" ];then
        ./scripts/macos-sign-notarize.sh "target/${TARGET}/release/gkg"
    fi

    tar -czvf gkg-linux-${ARCH}.tar.gz -C target/${TARGET}/release gkg
    echo "created gkg-linux-${ARCH}.tar.gz"
}

build_lib() {
    mkdir -p $LIB_DIR

    cargo build $CARGO_PARAMS --target $TARGET -p indexer-c-bindings
    cp target/${TARGET}/release/libindexer_c_bindings.a "$LIB_DIR/"
    echo "created $LIB_DIR/libindexer_c_bindings.a"
}

TASK="${1:-all}"
[ "$TASK" = "bin" -o "$TASK" = "all" ] && build_bin
[ "$TASK" = "lib" -o "$TASK" = "all" ] && build_lib
