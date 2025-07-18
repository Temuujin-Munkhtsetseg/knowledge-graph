#!/bin/bash
set -euo pipefail

# Script to sign and notarize macOS binaries
# Usage: ./scripts/macos-sign-notarize.sh <path-to-binary>

# Check if binary path is provided
if [ $# -eq 0 ]; then
    echo "Error: No binary path provided"
    echo "Usage: $0 <path-to-binary>"
    exit 1
fi

BINARY_PATH="$1"
BINARY_NAME=$(basename "$BINARY_PATH")

# Initialize variables for cleanup
KEYCHAIN_NAME="build.keychain"
P12_FILE=""
API_KEY_PATH=""
ZIP_FILE=""

# Cleanup function
cleanup() {
    local exit_code=$?
    echo "Performing cleanup..."
    
    # Delete temporary keychain if it exists
    echo "Deleting temporary keychain..."
    security delete-keychain "$KEYCHAIN_NAME" 2>/dev/null || true
    
    # Remove temporary files
    [ -n "$P12_FILE" ] && [ -f "$P12_FILE" ] && rm -f "$P12_FILE"
    [ -n "$API_KEY_PATH" ] && [ -f "$API_KEY_PATH" ] && rm -f "$API_KEY_PATH"
    [ -n "$ZIP_FILE" ] && [ -f "$ZIP_FILE" ] && rm -f "$ZIP_FILE"
    
    echo "Cleanup completed"
    exit $exit_code
}

# Set up cleanup trap
trap cleanup EXIT INT TERM

# Validate required environment variables and files
if [ -z "${APPLE_DEVELOPER_ID_CERTIFICATE_PASSWORD:-}" ]; then
    echo "Error: APPLE_DEVELOPER_ID_CERTIFICATE_PASSWORD environment variable is not set"
    exit 1
fi

if [ ! -f "${APPLE_DEVELOPER_ID_CERTIFICATE_PEM:-}" ]; then
    echo "Error: APPLE_DEVELOPER_ID_CERTIFICATE_PEM file not found at: ${APPLE_DEVELOPER_ID_CERTIFICATE_PEM:-}"
    exit 1
fi

if [ ! -f "${APPSTORE_CONNECT_API_KEY_FILE:-}" ]; then
    echo "Error: APPSTORE_CONNECT_API_KEY_FILE file not found at: ${APPSTORE_CONNECT_API_KEY_FILE:-}"
    exit 1
fi

if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at: $BINARY_PATH"
    exit 1
fi

echo "Starting code signing and notarization process for: $BINARY_NAME"

# Import certificate to keychain
echo "Importing certificate to keychain..."
KEYCHAIN_PASSWORD=$(openssl rand -base64 32)

# Create temporary keychain
security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
security list-keychains -s $(security list-keychains -d user | tr -d '"') "$KEYCHAIN_NAME"

# Download and import Apple intermediate certificate
echo "Downloading and importing Apple intermediate certificate..."
INTERMEDIATE_CERT_FILE="$(mktemp).cer"
curl -s -o "$INTERMEDIATE_CERT_FILE" "https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer"
security import "$INTERMEDIATE_CERT_FILE" -k "$KEYCHAIN_NAME" -T /usr/bin/codesign -T /usr/bin/security
rm -f "$INTERMEDIATE_CERT_FILE"

# Convert PEM to P12 format
P12_FILE="$(mktemp).p12"
P12_PASSWORD=$(openssl rand -base64 32)
openssl pkcs12 -export \
    -in "$APPLE_DEVELOPER_ID_CERTIFICATE_PEM" \
    -out "$P12_FILE" \
    -passin "pass:$APPLE_DEVELOPER_ID_CERTIFICATE_PASSWORD" \
    -passout "pass:$P12_PASSWORD" \
    -legacy

# Import certificate
security import "$P12_FILE" -k "$KEYCHAIN_NAME" -P "$P12_PASSWORD" -T /usr/bin/codesign -T /usr/bin/security
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME" >/dev/null

echo "Available identities:"
security find-identity "$KEYCHAIN_NAME"

echo "Reading Developer ID from keychain..."
DEVELOPER_ID=$(security find-identity -v build.keychain | grep "Developer ID Application" | awk '{print $2}')

# Sign the binary
echo "Signing binary..."
codesign --force --timestamp --options runtime --keychain "$KEYCHAIN_NAME" --sign "$DEVELOPER_ID" "$BINARY_PATH"

# Verify signature
echo "Verifying signature..."
codesign --verify --deep --strict --verbose=2 "$BINARY_PATH"

# Create ZIP for notarization
echo "Creating ZIP for notarization..."
ZIP_FILE="${BINARY_NAME}.zip"
ditto -c -k --keepParent "$BINARY_PATH" "$ZIP_FILE"

# Parse App Store Connect API key
API_KEY_ID=$(jq -r '.key_id' "$APPSTORE_CONNECT_API_KEY_FILE")
API_ISSUER_ID=$(jq -r '.issuer_id' "$APPSTORE_CONNECT_API_KEY_FILE")
API_KEY=$(jq -r '.private_key' "$APPSTORE_CONNECT_API_KEY_FILE")

# Create temporary API key file
API_KEY_PATH="$(mktemp)"
echo "$API_KEY" > "$API_KEY_PATH"

# Submit for notarization
echo "Submitting for notarization..."
set +e  # Temporarily disable exit on error
NOTARIZATION_OUTPUT=$(xcrun notarytool submit "$ZIP_FILE" \
    --key-id "$API_KEY_ID" \
    --issuer "$API_ISSUER_ID" \
    --key "$API_KEY_PATH" \
    --wait \
    --timeout 20m 2>&1)
NOTARIZATION_EXIT=$?
set -e  # Re-enable exit on error

# Always log the output
echo "$NOTARIZATION_OUTPUT"

# Extract submission ID for logging
SUBMISSION_ID=$(echo "$NOTARIZATION_OUTPUT" | grep -o 'id: [0-9a-f][0-9a-f]*-[0-9a-f][0-9a-f]*-[0-9a-f][0-9a-f]*-[0-9a-f][0-9a-f]*-[0-9a-f][0-9a-f]*' | sed 's/id: //' | head -1)

if [ $NOTARIZATION_EXIT -ne 0 ]; then
    echo "Notarization command exited with code $NOTARIZATION_EXIT"
    
    # Check if this is a timeout (notarization service continues processing) or actual failure
    if echo "$NOTARIZATION_OUTPUT" | grep -q "timeout"; then
        echo "Notarization timed out after 20 minutes, but service continues processing"
        if [ -n "$SUBMISSION_ID" ]; then
            echo "Submission ID: $SUBMISSION_ID"
        fi
        echo "Continuing with script execution..."
    else
        echo "Error: Notarization failed"
        if [ -n "$SUBMISSION_ID" ]; then
            echo "Submission ID: $SUBMISSION_ID"
            echo "Fetching notarization log..."
            xcrun notarytool log "$SUBMISSION_ID" \
                --key-id "$API_KEY_ID" \
                --issuer "$API_ISSUER_ID" \
                --key "$API_KEY_PATH" || true
        else
            echo "Could not extract submission ID for logging"
        fi
        exit $NOTARIZATION_EXIT
    fi
else
    echo "Notarization successful!"
    if [ -n "$SUBMISSION_ID" ]; then
        echo "Submission ID: $SUBMISSION_ID"
    fi
fi

if [ $NOTARIZATION_EXIT -eq 0 ]; then
    echo "Code signing and notarization completed successfully!"
else
    echo "Code signing completed, but notarization timed out"
fi
