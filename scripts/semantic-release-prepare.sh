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

    # Update mcp
    if [ -f "crates/mcp/Cargo.toml" ]; then
        echo "📦 Updating mcp version..."
        sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "crates/mcp/Cargo.toml"
        rm -f "crates/mcp/Cargo.toml.bak"
        echo "✅ Updated mcp to version $NEW_VERSION"
    fi
    
    # Update http-server
    if [ -f "crates/http-server/Cargo.toml" ]; then
        echo "📦 Updating http-server version..."
        sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "crates/http-server/Cargo.toml"
        rm -f "crates/http-server/Cargo.toml.bak"
        echo "✅ Updated http-server to version $NEW_VERSION"
    fi

    # Update workspace-manager
    if [ -f "crates/workspace-manager/Cargo.toml" ]; then
        echo "📦 Updating workspace-manager version..."
        sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "crates/workspace-manager/Cargo.toml"
        rm -f "crates/workspace-manager/Cargo.toml.bak"
        echo "✅ Updated workspace-manager to version $NEW_VERSION"
    fi

    # Update xtask
    if [ -f "crates/xtask/Cargo.toml" ]; then
        echo "📦 Updating xtask version..."
        sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "crates/xtask/Cargo.toml"
        rm -f "crates/xtask/Cargo.toml.bak"
        echo "✅ Updated xtask to version $NEW_VERSION"
    fi
    
    # Update logging
    if [ -f "crates/logging/Cargo.toml" ]; then
        echo "📦 Updating logging version..."
        sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "crates/logging/Cargo.toml"
        rm -f "crates/logging/Cargo.toml.bak"
        echo "✅ Updated logging to version $NEW_VERSION"
    fi

    # Update event-bus
    if [ -f "crates/event-bus/Cargo.toml" ]; then
        echo "📦 Updating event-bus version..."
        sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "crates/event-bus/Cargo.toml"
        rm -f "crates/event-bus/Cargo.toml.bak"
        echo "✅ Updated event-bus to version $NEW_VERSION"
    fi

    # Update database
    if [ -f "crates/database/Cargo.toml" ]; then
        echo "📦 Updating database version..."
        sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "crates/database/Cargo.toml"
        rm -f "crates/database/Cargo.toml.bak"
        echo "✅ Updated database to version $NEW_VERSION"
    fi

    # Update Cargo.lock with new workspace versions
    echo "🔄 Updating Cargo.lock..."
    cargo update --workspace
    echo "✅ Updated Cargo.lock"
else
    echo "⚠️  Cargo.toml not found, skipping Cargo updates"
fi

echo "🎉 All Cargo versions updated to $NEW_VERSION!" 
