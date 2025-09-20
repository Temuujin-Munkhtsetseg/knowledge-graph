#!/bin/bash
set -e

echo "🚀 Running semantic-release prepare script..."

# Get the new version from command line argument
NEW_VERSION="$1"

if [ -z "$NEW_VERSION" ]; then
    echo "❌ Error: NEW_VERSION is not provided as first argument. This script should be run by semantic-release."
    exit 1
fi

echo "📝 Updating version to: $NEW_VERSION"

# Update workspace packages using npm version with workspace flags
echo "📦 Updating npm workspace packages..."

echo "📦 Updating @gitlab-org/gkg-frontend..."
npm version "$NEW_VERSION" --workspace=@gitlab-org/gkg-frontend --git-tag-version=false
echo "✅ Updated @gitlab-org/gkg-frontend to version $NEW_VERSION"

# Update @gitlab-org/gkg
echo "📦 Updating @gitlab-org/gkg..."
npm version "$NEW_VERSION" --workspace=@gitlab-org/gkg --git-tag-version=false
echo "✅ Updated @gitlab-org/gkg to version $NEW_VERSION"

# Update docs
echo "📦 Updating docs..."
npm version "$NEW_VERSION" --workspace=docs --git-tag-version=false
echo "✅ Updated docs to version $NEW_VERSION"

echo "🎉 All packages updated to $NEW_VERSION!" 
