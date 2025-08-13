---
title: Endpoints
description: Documentation for the gkg MCP endpoints.
sidebar:
  order: 1
---

The Knowledge Graph exposes a Model Context Protocol endpoints to interact with its data. This server provides tools to search and analyze code bases via LLMs.

### Streamable HTTP transport

A Streamable HTTP endpoint is available [when the server is running](/getting-started/usage/#start-the-server) at `/mcp`. If the Knowledge Graph started normally, the endpoint will be:

```
http:/localhost:27495/mcp
```

### SSE Transport

An SSE endpoint is available [when the server is running](/getting-started/usage/#start-the-server) at `/mcp/sse`. If the Knowledge Graph started normally, the endpoint will be:

```
http:/localhost:27495/mcp/sse
```
