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

# Update the VERSION file
echo "$NEW_VERSION" > VERSION
echo "✅ Updated VERSION file"

# Update Cargo workspace packages
echo "🦀 Updating Cargo workspace packages..."

# Update root Cargo.toml workspace package version
if [ -f "Cargo.toml" ]; then
    echo "📦 Updating root Cargo.toml workspace package version..."
    sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
    rm -f Cargo.toml.bak
    echo "✅ Updated root Cargo.toml workspace package to version $NEW_VERSION"
fi

# Update workspace members manually
if [ -f "Cargo.toml" ]; then
    echo "📦 Updating Cargo workspace members..."
    
    # Update indexer
    if [ -f "crates/indexer/Cargo.toml" ]; then
        echo "📦 Updating indexer version..."
        sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "crates/indexer/Cargo.toml"
        rm -f "crates/indexer/Cargo.toml.bak"
        echo "✅ Updated indexer to version $NEW_VERSION"
    fi
    
    # Update cli
    if [ -f "crates/gkg/Cargo.toml" ]; then
        echo "📦 Updating cli version..."
        sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "crates/gkg/Cargo.toml"
        rm -f "crates/gkg/Cargo.toml.bak"
        echo "✅ Updated cli to version $NEW_VERSION"
    fi
    
    # Update Cargo.lock with new workspace versions
    echo "🔄 Updating Cargo.lock..."
    cargo update --workspace
    echo "✅ Updated Cargo.lock"
else
    echo "⚠️  Cargo.toml not found, skipping Cargo updates"
fi

echo "🎉 All Cargo versions updated to $NEW_VERSION!" 
