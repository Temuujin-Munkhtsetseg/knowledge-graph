---
title: HTTP Server API
description: REST API endpoints and Server-Sent Events for GitLab Knowledge Graph server
---

The GitLab Knowledge Graph server provides an HTTP API for programmatic access to the Knowledge Graph, along with real-time SSE events for monitoring progress.

## Base URL

When running locally (default port):

```none
http://localhost:27495
```

The server automatically selects port `27495` (0x6b67 - "knowledge graph") when available, or finds an unused port if it's busy.

## Authentication

Currently, the server runs without authentication for local development. The server does check against CORS headers.

## REST API Endpoints

### Server Information

#### `GET /api/info`

Get basic server information including version and port.

**Response:**

```json
{
  "port": 27495,
  "version": "0.10.0"
}
```

### Workspace Management

#### `GET /api/workspace/list`

List all indexed workspace folders and their projects.

**Response:**

```json
{
  "workspaces": [
    {
      "workspace_info": {
        "workspace_folder_path": "/path/to/workspace",
        "data_directory_name": "workspace_hash",
        "status": "indexed",
        "last_indexed_at": "2024-01-01T00:00:00Z",
        "project_count": 2
      },
      "projects": [
        {
          "project_path": "/path/to/workspace/project1",
          "project_hash": "project_hash_1",
          "workspace_folder_path": "/path/to/workspace",
          "status": "indexed",
          "database_path": "/data/workspace_hash/project_hash_1/kuzu_db",
          "parquet_directory": "/data/workspace_hash/project_hash_1/parquet_files"
        }
      ]
    }
  ]
}
```

#### `POST /api/workspace/index`

Index a new workspace folder or re-index an existing one.

**Request Body:**

```json
{
  "workspace_folder_path": "/path/to/workspace"
}
```

**Response (Success):**

```json
{
  "workspace_folder_path": "/path/to/workspace",
  "data_directory_name": "workspace_hash",
  "status": "indexing",
  "last_indexed_at": null,
  "project_count": 2
}
```

**Error Responses:**

- `400 Bad Request`: Invalid workspace path or no projects found
- `500 Internal Server Error`: Failed to register workspace or dispatch indexing job

#### `DELETE /api/workspace/delete`

Delete a workspace and all its associated data.

### Graph Queries

#### `GET /api/graph/initial`

Get initial graph data for visualization.

#### `GET /api/graph/neighbors`

Get neighboring nodes for graph exploration.

#### `GET /api/graph/search`

Search the knowledge graph for specific patterns.

#### `GET /api/graph/stats`

Get statistics about the knowledge graph.

### Server-Sent Events (SSE)

#### `GET /api/events`

Connect to the events endpoint for real-time Server-Sent Events during indexing operations.

**Headers:**

- `Content-Type: text/event-stream`
- `Cache-Control: no-cache`

**Connection Event:**

When you first connect, you'll receive a connection confirmation:

```http
event: gkg-connection
data: {"type":"connection-established","timestamp":"2024-01-01T00:00:00Z","message":"SSE connection established"}
```

**System Events:**

All system events are sent with the `gkg-event` event type:

```http
event: gkg-event
data: {"WorkspaceIndexing":{"Started":{"workspace_folder_info":{...},"projects_to_process":[...],"started_at":"2024-01-01T00:00:00Z"}}}
```

Events include workspace indexing progress, project processing updates, and completion notifications. The event data follows the internal event bus schema for real-time system monitoring.

## Error Handling

All endpoints return standard HTTP status codes:

- `200`: Success
- `400`: Bad Request - Invalid parameters
- `404`: Not Found - Resource doesn't exist
- `500`: Internal Server Error - Processing failed

Error responses include details:

```json
{
  "error": "Invalid workspace path",
  "code": "INVALID_PATH",
  "details": "Path does not exist or is not accessible"
}
```

## CORS Configuration

The server is configured to accept requests from localhost origins for local development. CORS is handled automatically for cross-origin requests from localhost.

## Example Usage

### Index a Workspace

```bash
# Start indexing a workspace
curl -X POST http://localhost:27495/api/workspace/index \
  -H "Content-Type: application/json" \
  -d '{"workspace_folder_path": "/path/to/my/workspace"}'
```

### List Workspaces and Projects

```bash
# Get all workspaces and their projects
curl http://localhost:27495/api/workspace/list
```

### Server Information

```bash
# Get server info
curl http://localhost:27495/api/info
```

### Real-time Events with SSE

```javascript
// Connect to Server-Sent Events
const eventSource = new EventSource("http://localhost:27495/api/events");

eventSource.onopen = () => {
  console.log("Connected to SSE stream");
};

eventSource.addEventListener("gkg-connection", (event) => {
  const data = JSON.parse(event.data);
  console.log("Connection established:", data);
});

eventSource.addEventListener("gkg-event", (event) => {
  const data = JSON.parse(event.data);
  console.log("System event:", data);
});

eventSource.onerror = (error) => {
  console.error("SSE connection error:", error);
};
```

### MCP Integration

The server also provides [Model Context Protocol endpoints](/mcp/endpoints) at `/mcp` and `/mcp/sse` for AI tool integration.
