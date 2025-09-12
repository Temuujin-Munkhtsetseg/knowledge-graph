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

### search_codebase_definitions

Efficiently searches the codebase for functions, classes, methods, constants, interfaces that contain one or more search terms. Start your search with an overview of the signatures, then drill down into the full implementation bodies if needed. Returns the definition signatures with optional full implementation bodies. Supports exact matches, partial matches, and case-sensitive/insensitive search modes. Use this tool for code exploration, refactoring, debugging, and understanding code structure.

Input:

- `project_absolute_path` (string): Absolute filesystem path to the project root directory where code definitions should be searched. Must be a valid directory path.
- `search_terms` (string[]): List of code identifiers to search for definitions. Can include function names, class names, method names, constants, etc.
- `include_full_body` (boolean, optional) (default: false): Use false when requesting an overview of multiple definitions. Switch to true to see how full implementation of specific definitions. Start with false, then switch to true for the items you want to examine closely.
- `page` (integer, optional) (default: 1): Page number for pagination, starting from 1.

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

### get_definition

Navigates directly to the definition of a function or method call on a specific line. This tool is useful for:

- Quickly understanding what a specific function or method does without manual searching.
- Verifying the implementation details of a symbol encountered in the code.
- Efficiently exploring the codebase by jumping from usage to definition.

Input:

- `file_path` (string): Absolute or project-relative path to the file containing the symbol usage.
- `line` (string): The exact line of code containing the symbol (whitespace must be preserved).
- `symbol_name` (string): The name of the callable symbol (method/function) to resolve.

Output: An object containing:

- `definitions` (array): A list of definitions found for the symbol. Each entry can be one of two types:
  - **Definition**: For symbols defined within the workspace.
  - **ImportedSymbol**: For symbols imported from external dependencies.
- Both types include the following fields:
  - `type` (string): The type of the definition ("Definition" or "ImportedSymbol").
  - `name` (string): The name of the symbol.
  - `fqn` (string): The fully qualified name of the symbol.
  - `primary_file_path` (string): The project-relative file path where the symbol is defined or imported.
  - `absolute_file_path` (string): The absolute file path.
  - `start_line` (integer): The starting line number of the definition.
  - `end_line` (integer): The ending line number of the definition.
  - `code` (string): A snippet of the code for the definition.
  - `is_ambiguous` (boolean): A flag indicating if the found reference is ambiguous.
- `system_message` (string, optional): A message provided if multiple lines or symbol occurrences were found, which may affect the results.
