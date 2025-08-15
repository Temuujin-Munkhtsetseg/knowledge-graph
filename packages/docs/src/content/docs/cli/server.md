---
title: gkg server
description: Manage the GitLab Knowledge Graph HTTP server with web interface and API
sidebar:
  order: 2
---

Manage the GitLab Knowledge Graph HTTP server, providing a web interface, HTTP API, and optional file watching for automatic re-indexing.

## Synopsis

```bash
gkg server start [OPTIONS]
```

## Description

The `gkg server` command starts a long-running HTTP server that provides:

- **Web Interface**: Browser-based UI for exploring the knowledge graph
- **[HTTP API](/api/server)**: RESTful endpoints for programmatic access to workspace and graph data
- **[Real-time Events](/api/server#server-sent-events-sse)**: Server-Sent Events (SSE) for live progress updates during indexing
- **File Watching**: Automatic re-indexing when files change (optional)
- **Background Jobs**: Queue-based processing for concurrent operations
- **[MCP Integration](/mcp/endpoints)**: Model Context Protocol support for AI tools with [dedicated tools](/mcp/tools) for code analysis

The server is designed for ongoing development workflows, providing continuous access to the knowledge graph and automatic updates as your code changes.

## Options

### `--register-mcp <FILE>`

Register the server with MCP (Model Context Protocol) configuration.

- **Type**: File path
- **Default**: None
- **Example**: `~/.gitlab/duo/mcp.json`

This option automatically registers the GitLab Knowledge Graph server with your MCP configuration file, enabling AI tools to discover and use the knowledge graph API.

**Example:**

```bash
gkg server start --register-mcp ~/.gitlab/duo/mcp.json
```

### `--enable-reindexing`

Enable automatic file watching and re-indexing.

- **Type**: Flag
- **Default**: `false`

When enabled, the server monitors registered workspaces for file changes and automatically queues re-indexing jobs. This keeps the knowledge graph up-to-date as you develop.

**Example:**

```bash
gkg server start --enable-reindexing
```

### `--detached`

Starts the server in detached (background) mode. This is useful when the server should not be bound to a terminal session, for example, when running it in a CI pipeline for integration testing or registering it for system autostart.

**Example:**

```bash
gkg server start --detached
```

## Stopping the server

You can stop both foreground and background servers from any terminal session with:

```bash
gkg server stop
```

The server also respects `SIGINT` and `SIGTERM` signals on Unix-like systems. You can gracefully stop the server from the attached terminal by pressing `Ctrl+C`.
