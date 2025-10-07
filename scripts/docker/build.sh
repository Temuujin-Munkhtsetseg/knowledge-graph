#!/bin/bash
set -e

# This script builds all the service images from the root of the project.
# It assumes the Dockerfile is located at ./docker/Dockerfile

echo "Building GKG images..."

# Build the CLI image
docker build --target cli -f docker/Dockerfile -t gkg:cli .

# Build the Desktop UI image
docker build --target desktop -f docker/Dockerfile -t gkg:desktop .

# Build the Webserver image
docker build --target webserver -f docker/Dockerfile -t gkg:webserver .

# Build the Indexer image
docker build --target indexer -f docker/Dockerfile -t gkg:indexer .

echo "All images built successfully."
echo "You can find them with 'docker images | grep gkg'"
