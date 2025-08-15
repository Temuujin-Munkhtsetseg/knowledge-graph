---
title: Architecture Overview
description: The architecture of the GitLab Knowledge Graph
---

The GitLab Knowledge Graph is built on a modular, crate-based architecture:

- **`gkg`**: The main command-line interface (CLI) that brings all the components together.
- **`gitlab-code-parser`**: A language-agnostic parsing engine powered by `tree-sitter` and other native Rust parsers.
- **`indexer`**: The core component responsible for processing repositories and extracting structured data.
- **`database`**: The persistence layer, powered by the Kuzu graph database, for storing and querying the knowledge graph.
- **`workspace-manager`**: A dedicated service for tracking projects and their indexing status.
- **`http-server`**: The web server, built with Axum, that provides the HTTP API and frontend interface.
- **`event-bus`**: A real-time event system for broadcasting progress and status updates.
- **`mcp`**: Implements the Model-Context-Provider protocol for seamless integration with AI tools.
- **`logging`**: A structured logging service for improved observability and debugging.

## How Indexing Works

The indexing process follows a multi-stage pipeline that resembles an ETL (Extract, Transform, Load) process:

1.  **(Extract) Workspace Discovery**: Scans the workspace directory for Git repositories and registers each as a "project".
2.  **(Extract) File Discovery**: Enumerates files in each repository, respecting `.gitignore` and filtering by supported language extensions.
3.  **(Transform) Semantic Analysis**: Extracts definitions, imports, and relationships from each file's Abstract Syntax Tree (AST).
4.  **(Transform) Resolution**: Resolves all references to definitions and imports into an in memory graph representing your code structure.
5.  **(Load) Graph Storage**: Writes the extracted data to Parquet files and loads it into a KuzuDB graph database.

## Threading Model

The indexer uses a hybrid threading model to optimize for both I/O and CPU-bound tasks:

- **Async I/O Pool**: A high-concurrency pool reads file contents asynchronously, with a concurrency level of `worker_threads * 2` (minimum 8).
- **CPU Worker Pool**: A thread pool sized to the number of CPU cores (or as specified by `--threads`) handles CPU-intensive parsing and analysis. A semaphore attempts to prevent this pool from being overwhelmed.

## Output & Storage

Indexed data is stored in a hierarchical directory structure under `~/.gkg/`:

```
~/.gkg/
└── workspace_folders/
    └── {workspace_hash}/          # SHA-256 hash of workspace path
        └── {project_hash}/        # SHA-256 hash of project path
            ├── database.kz           # KuzuDB graph database file
            └── parquet_files/     # Structured data in Parquet format
                ├── definitions.parquet
                ├── references.parquet
                ├── imports.parquet
                └── files.parquet
```
