---
title: HTTP Server API
description: REST API endpoints and WebSocket events for GitLab Knowledge Graph server
---

# HTTP Server API

The GitLab Knowledge Graph server provides a comprehensive HTTP API for programmatic access to your knowledge graphs, along with real-time WebSocket events for monitoring progress.

## Base URL

When running locally:

```
http://localhost:3000
```

## Authentication

Currently, the server runs without authentication for local development. Production deployments should implement proper authentication layers.

## REST API Endpoints

### Workspaces

#### `GET /api/workspaces`

List all indexed workspaces.

**Response:**

```json
{
  "workspaces": [
    {
      "id": "workspace_hash",
      "path": "/path/to/workspace",
      "created_at": "2024-01-01T00:00:00Z",
      "project_count": 5
    }
  ]
}
```

#### `POST /api/workspaces`

Index a new workspace.

**Request Body:**

```json
{
  "path": "/path/to/workspace",
  "options": {
    "watch": true,
    "languages": ["rust", "typescript", "python"]
  }
}
```

### Projects

#### `GET /api/workspaces/{workspace_id}/projects`

List projects within a workspace.

#### `GET /api/projects/{project_id}/graph`

Get the knowledge graph for a specific project.

**Query Parameters:**

- `format`: Response format (`json` | `cypher` | `graphml`)
- `include_git`: Include git commit information
- `depth`: Maximum relationship depth to traverse

### Query Interface

#### `POST /api/query`

Execute Cypher queries against the knowledge graph.

**Request Body:**

```json
{
  "query": "MATCH (f:File)-[:CONTAINS]->(fn:Function) RETURN f.name, fn.name LIMIT 10",
  "workspace_id": "workspace_hash",
  "project_id": "project_hash"
}
```

**Response:**

```json
{
  "results": [
    {
      "f.name": "src/main.rs",
      "fn.name": "main"
    }
  ],
  "execution_time_ms": 45
}
```

## WebSocket Events

Connect to `/ws` for real-time updates during indexing operations.

### Event Types

#### `indexing_started`

```json
{
  "type": "indexing_started",
  "workspace_id": "workspace_hash",
  "total_repositories": 5
}
```

#### `repository_progress`

```json
{
  "type": "repository_progress",
  "repository_name": "my-project",
  "files_processed": 150,
  "total_files": 200,
  "percentage": 75
}
```

#### `indexing_complete`

```json
{
  "type": "indexing_complete",
  "workspace_id": "workspace_hash",
  "duration_ms": 45000,
  "entities_created": 5000,
  "relationships_created": 12000
}
```

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

## Rate Limiting

The server implements basic rate limiting:

- API endpoints: 100 requests/minute per IP
- WebSocket connections: 5 concurrent connections per IP

## Example Usage

### Index and Query a Workspace

```bash
# Start indexing
curl -X POST http://localhost:3000/api/workspaces \
  -H "Content-Type: application/json" \
  -d '{"path": "/path/to/my/workspace"}'

# Query the results
curl -X POST http://localhost:3000/api/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "MATCH (f:File) RETURN f.name, f.language LIMIT 5",
    "workspace_id": "your_workspace_hash"
  }'
```

### WebSocket Connection

```javascript
const ws = new WebSocket("ws://localhost:3000/ws");

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log("Progress update:", data);
};
```
