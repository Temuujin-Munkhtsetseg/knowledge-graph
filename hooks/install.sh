#!/bin/bash

# Make sure we're in the project root
cd "$(dirname "$0")/.."

# Create hooks directory if it doesn't exist
mkdir -p .git/hooks

# Copy hook files from hooks directory to .git/hooks, excluding install.sh
for hook in hooks/*; do
    if [ "$(basename "$hook")" != "install.sh" ]; then
        cp "$hook" .git/hooks/
    fi
done

chmod +x .git/hooks/*

echo "✅ Git hooks installed successfully!"
