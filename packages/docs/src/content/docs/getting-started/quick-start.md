---
title: Quick Start
description: Get up and running with GitLab Knowledge Graph in minutes
---

# Quick Start

This guide will help you get GitLab Knowledge Graph up and running on your first project in just a few minutes.

## Prerequisites

- GitLab Knowledge Graph [installed](/getting-started/installation)
- A Git repository or workspace folder to index

## Basic Indexing

The simplest way to start is by indexing a single repository or workspace folder.

### Index Current Directory

Navigate to your project directory and run:

```bash
gkg index
```

This will:

1. Discover all Git repositories in the current directory
2. Parse and analyze the code structure
3. Store the results in a local graph database
4. Display progress and completion statistics

### Index Specific Directory

You can also specify a path to index:

```bash
gkg index /path/to/your/workspace
```

### Example Output

```bash
$ gkg index my-project
✅ Workspace indexing completed in 12.34 seconds

Workspace Statistics:
- Projects indexed: 3
- Files processed: 1,247
- Code entities extracted: 5,832
- Relationships found: 12,156
```

## Server Mode

For more advanced usage, you can start GitLab Knowledge Graph in server mode, which provides:

- Web-based interface
- HTTP API for programmatic access
- Real-time file watching and re-indexing
- Background job processing

### Start the Server

```bash
gkg server
```

The server will start on `http://localhost:3000` by default. You'll see output like:

```bash
INFO HTTP server listening on 127.0.0.1:3000
```

### Access the Web Interface

Open your web browser and navigate to `http://localhost:3000` to access the GitLab Knowledge Graph web interface. From here you can:

- View indexed workspaces and projects
- Browse the knowledge graph visually
- Search for code entities and relationships
- Monitor indexing progress in real-time

### Enable Auto-Reindexing

To automatically re-index files when they change:

```bash
gkg server --enable-reindexing
```

This will watch for file changes and queue re-indexing jobs automatically.
