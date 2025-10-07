#!/bin/bash
set -e

# This script retags the locally built images and pushes them to Docker Hub.
# It assumes you have already run './scripts/docker/build.sh'
# and are logged into Docker with 'docker login'.

# Define your Docker Hub username
USERNAME="temuujinmunkhtsetseg"
REPO="gkg"

echo "Tagging and pushing images to $USERNAME/$REPO..."

# Tag and push the CLI image
docker tag gkg:cli $USERNAME/$REPO:cli
docker push $USERNAME/$REPO:cli

# Tag and push the Desktop UI image
docker tag gkg:desktop $USERNAME/$REPO:desktop
docker push $USERNAME/$REPO:desktop

# Tag and push the Webserver image
docker tag gkg:webserver $USERNAME/$REPO:webserver
docker push $USERNAME/$REPO:webserver

# Tag and push the Indexer image
docker tag gkg:indexer $USERNAME/$REPO:indexer
docker push $USERNAME/$REPO:indexer

echo "All images pushed successfully to https://hub.docker.com/r/$USERNAME/$REPO"
