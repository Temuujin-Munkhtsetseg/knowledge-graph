#!/bin/bash
set -euo pipefail

KUZU_INSTALL_DIR="$(pwd)/kuzu"

echo "--- Downloading and extracting Kuzu v${KUZU_VERSION} to ${KUZU_INSTALL_DIR}..."
mkdir -p "${KUZU_INSTALL_DIR}"
curl -L "https://github.com/kuzudb/kuzu/releases/download/v${KUZU_VERSION}/libkuzu-osx-universal.tar.gz" | tar xz -C "${KUZU_INSTALL_DIR}"

export KUZU_INCLUDE_DIR="${KUZU_INSTALL_DIR}"
export KUZU_LIBRARY_DIR="${KUZU_INSTALL_DIR}"
export KUZU_SHARED=1

export RUSTFLAGS="-C link-arg=-Wl,-rpath,${KUZU_INSTALL_DIR}"
