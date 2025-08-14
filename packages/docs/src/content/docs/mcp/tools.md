---
title: Tools
description: Documentation for the gkg MCP tools.
sidebar:
  order: 2
---

### analyze_code_files

List the imports and definitions for a one or multiple code files.

Input:

- `project_absolute_path` (string): The absolute path to an indexed project.
- `files` (string[]): One or more file absolute path to search accross the code base.

Output: An array of file entries containing all their definitions and imports.

### search_codebase

Search for specific terms accross the indexed content and get contextual results.

Input:

- `project_absolute_path` (string): The absolute path to an indexed project.
- `search_terms` (string[]): One or more terms to search accross the code base.
- `limit` (int, optional) (default: 50): The maximum amount of results to include in the response.

Output: An array containing the code entries matching any of the search terms.

### index_project

Creates new or rebuilds the Knowledge Graph index for a project to reflect recent changes.

Input:

- `project_absolute_path` (string): The absolute path to the current project directory to index synchronously.

Output: An object containing:

- `status` (string): "ok" when indexing completes successfully
- `stats` (object): Project indexing statistics including total files processed and project path
