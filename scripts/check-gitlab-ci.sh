#!/usr/bin/env bash
set -euo pipefail

# Script to validate .gitlab-ci.yml using glab CLI tool

CI_FILE=".gitlab-ci.yml"

if [ ! -f "$CI_FILE" ]; then
  echo "Error: $CI_FILE not found"
  exit 1
fi

echo "Validating GitLab CI configuration..."

# Check if glab is installed and available
if command -v glab > /dev/null 2>&1; then
  echo "Using glab to validate $CI_FILE"

  # Run glab ci lint command
  if glab ci lint "$CI_FILE"; then
    echo "✅ GitLab CI configuration is valid"
    exit 0
  else
    echo "❌ GitLab CI configuration validation failed"
    exit 1
  fi
else
  echo "⚠️  Warning: glab is not installed. Cannot validate $CI_FILE"
  echo "   Install glab for GitLab CI validation: https://gitlab.com/gitlab-org/cli"
  echo "   Continuing without validation - your CI configuration might have errors!"
  exit 0
fi