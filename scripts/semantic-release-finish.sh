#!/bin/bash
set -e

NEW_VERSION="$1"

if [ -z "$NEW_VERSION" ]; then
    echo "❌ Error: NEW_VERSION is not provided as first argument. This script should be run by semantic-release."
    exit 1
fi

URL="https://gitlab.com/api/v4/projects/69095239/repository/tags?tag_name=bindings/go/${NEW_VERSION}&ref=${NEW_VERSION}"
if ! curl --request POST --header "PRIVATE-TOKEN: $GL_TOKEN" --url "$URL"; then
    echo "❌ Error: failed to create bindings tag."
    exit 1
fi
