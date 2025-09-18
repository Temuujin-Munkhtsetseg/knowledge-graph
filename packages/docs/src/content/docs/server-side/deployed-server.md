---
title: Deployed Server
description: A reference for running Knowledge Graph server on server-side
---

# Knowldge Graph Server-side Deployment

Knowledge Graph can be deployed also on server-side using `http-server-deployed` service.

## Overview

The `http-server-deployed` provides a HTTP server built with Axum that can operate in different modes - `indexer` or `webserver`. `indexer` is used for running indexing service and `webserver` is used for running querying service. It communicates via Unix domain sockets.

## Usage

```bash
# Start server in indexing mode
cargo run --bin http-server-deployed -m indexer -s /tmp/gkg-indexer-http.sock

# Start server in webserver mode
cargo run --bin http-server-deployed -m webserver -s /tmp/gkg-webserver-http.sock
```
