---
title: gkg server
description: Start the GitLab Knowledge Graph HTTP server with web interface and API
---

# gkg server

Start the GitLab Knowledge Graph HTTP server, providing a web interface, HTTP API, and optional file watching for automatic re-indexing.

## Synopsis

```bash
gkg server [OPTIONS]
```

## Description

The `gkg server` command starts a long-running HTTP server that provides:

- **Web Interface**: Browser-based UI for exploring the knowledge graph
- **HTTP API**: RESTful endpoints for programmatic access
- **Real-time Events**: WebSocket-based progress updates
- **File Watching**: Automatic re-indexing when files change (optional)
- **Background Jobs**: Queue-based processing for concurrent operations
- **MCP Integration**: Model Context Protocol support for AI tools

The server is designed for ongoing development workflows where you want continuous access to the knowledge graph and automatic updates as your code changes.

## Options

### `--register-mcp <FILE>`

Register the server with MCP (Model Context Protocol) configuration.

- **Type**: File path
- **Default**: None
- **Example**: `~/.gitlab/duo/mcp.json`

This option automatically registers the GitLab Knowledge Graph server with your MCP configuration file, enabling AI tools to discover and use the knowledge graph API.

**Example:**

```bash
gkg server --register-mcp ~/.gitlab/duo/mcp.json
```

### `--enable-reindexing`

Enable automatic file watching and re-indexing.

- **Type**: Flag
- **Default**: `false`

When enabled, the server monitors registered workspaces for file changes and automatically queues re-indexing jobs. This keeps the knowledge graph up-to-date as you develop.

**Example:**

```bash
gkg server --enable-reindexing
```
