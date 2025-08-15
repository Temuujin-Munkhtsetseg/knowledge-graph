---
title: Endpoints
description: Documentation for the gkg MCP endpoints.
sidebar:
  order: 1
---

The Knowledge Graph exposes Model Context Protocol endpoints to interact with its data. This server provides [specialized tools](/mcp/tools) to search and analyze code bases via LLMs.

## Available Transports

### HTTP Transport

A streamable HTTP endpoint is available [when the server is running](/cli/server) at `/mcp`. If the Knowledge Graph started normally, the endpoint will be:

```
http://localhost:27495/mcp
```

This endpoint supports the MCP HTTP transport protocol for synchronous tool execution.

### Server-Sent Events (SSE) Transport

An SSE endpoint is available [when the server is running](/cli/server) at `/mcp/sse`. If the Knowledge Graph started normally, the endpoint will be:

```
http://localhost:27495/mcp/sse
```

This endpoint supports the MCP SSE transport protocol for streaming tool execution and real-time communication.

## Integration

These endpoints are designed to be used by AI development tools and IDEs that support the Model Context Protocol. The server automatically registers these endpoints when started with the `--register-mcp` flag.

For detailed information about available tools, see the [MCP Tools documentation](/mcp/tools).
