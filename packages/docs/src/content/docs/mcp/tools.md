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

### get_symbol_references

Finds all locations where a symbol is referenced throughout the codebase to assess change impact. This tool is helpful for:

- Planning to modify, rename, or delete a function, class, variable, or other symbol
- Need to understand the blast radius of a potential change before implementing it
- Investigating which parts of the codebase depend on a specific symbol
- Performing impact analysis for refactoring or deprecation decisions
- Tracing usage patterns to understand how a symbol is being used across the project

Input:

- `absolute_file_path` (string): The absolute path to the file containing the symbol
- `symbol_name` (string): The name of the symbol to find references for
- `depth` (integer, optional) (default: 1, maximum: 3): Maximum depth to traverse for finding references
- `limit` (integer, optional) (default: 50, maximum: 100): The maximum number of results to return

Output: An object containing:

- `references` (array): Array of symbol references, each containing:
  - `name` (string): The name of the symbol
  - `location` (string): File path and line number where the symbol is defined (format: "file:line")
  - `fqn` (string): Fully qualified name of the symbol
  - `referenced_by` (array): Array of references that call this symbol, each containing:
    - `name` (string): Name of the calling symbol
    - `location` (string): File path and line number where the call occurs
    - `fqn` (string): Fully qualified name of the calling symbol
    - `referenced_by` (array): Recursive references (up to the specified depth)
