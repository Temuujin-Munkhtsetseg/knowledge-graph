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
