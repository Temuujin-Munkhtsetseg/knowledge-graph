---
title: Deployed Server
description: A reference for running Knowledge Graph server on server-side
---

# Knowldge Graph Server-side Deployment

Knowledge Graph can be deployed also on server-side using `http-server-deployed` service.

## Overview

The `http-server-deployed` provides a HTTP server built with Axum that can operate in different modes - `indexer` or `webserver`. `indexer` is used for running indexing service and `webserver` is used for running querying service. It communicates via TCP or Unix domain sockets.

## Usage

All server modes require JWT authentication via a secret file and a data directory for persistent storage:

```bash
# Start server in indexing mode on Unix socket
cargo run --bin http-server-deployed -- -m indexer --socket /tmp/gkg-indexer-http.sock --secret-path /path/to/jwt-secret --data-dir /data/gkg

# Start server in indexing mode on TCP socket
cargo run --bin http-server-deployed -- -m indexer --bind 0.0.0.0:3333 --secret-path /path/to/jwt-secret --data-dir /data/gkg

# Start server in webserver mode on Unix socket
cargo run --bin http-server-deployed -- -m webserver --socket /tmp/gkg-webserver-http.sock --secret-path /path/to/jwt-secret --data-dir /data/gkg

# Start server in webserver mode on TCP socket
cargo run --bin http-server-deployed -- -m webserver --bind 0.0.0.0:3334 --secret-path /path/to/jwt-secret --data-dir /data/gkg
```

## Command Line Options

- `--mode, -m`: Server mode - either `indexer` or `webserver` (default: `indexer`)
- `--socket, -s`: Unix socket file path (default: `/tmp/gkg-indexer-http.sock`)
- `--bind, -b`: TCP bind address (conflicts with `--socket`)
- `--secret-path`: Path to JWT secret file (required)
- `--data-dir`: Data directory for persistent storage (required)

## Data Directory

The `--data-dir` argument specifies where the server stores persistent data including index files, databases, and metadata. This is particularly important for containerized deployments where data needs to be stored in mounted volumes.

The server will:

- Create the directory if it doesn't exist
- Validate write permissions on startup
- Exit with an error if the path exists but is not a directory

Example for Kubernetes deployments:

```bash
./http-server-deployed -m indexer --bind 0.0.0.0:3333 --data-dir /data/gkg --secret-path /secrets/jwt
```

## JWT Authentication

The server uses JWT authentication for all endpoints except `/health` and `/metrics`. Requests must include a valid JWT token in the `Authorization` header:

```bash
# Example API request with JWT authentication
curl -H "Authorization: Bearer <jwt-token>" http://localhost:3334/webserver/v1/tool
```

### Public Endpoints (No Authentication Required)

- `/health` - Health check endpoint
- `/metrics` - Prometheus metrics endpoint

### Protected Endpoints

- `/webserver/v1/*` - Webserver API endpoints (when in webserver mode)
- `/indexer/v1/*` - Indexer API endpoints (when in indexer mode)
