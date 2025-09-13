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

Efficiently searches the codebase for functions, classes, methods, constants, interfaces that contain one or more search terms. Returns the definition information for definitions matching the search terms. Supports exact matches, partial matches, and case-sensitive/insensitive search modes. Use this tool for code exploration, refactoring, debugging, and understanding code structure.

Input:

- `project` (string): Absolute filesystem path to the project root directory where code definitions should be searched.
- `search_terms` (string[]): List of definition names to search for. Can be names of functions, classes, constants, etc.
- `page` (integer, optional) (default: 1): Page number starting from 1. If the response's next_page field is greater than 1, more results are available at that page. You can use this to retrieve more results if more context is needed.

Output: An object containing:

- `definitions` (array): Array of matching code definitions, each containing:
  - `name` (string): The name of the definition
  - `fqn` (string): Fully qualified name of the definition
  - `definition_type` (string): Type of definition (e.g., "Function", "Class", "Method", "Constant")
  - `location` (string): File path and line range where the definition is located (format: "file:LstartLine-endLine")
  - `context` (string, optional): Code snippet showing the definition signature and a few lines of context
- `next_page` (integer, optional): Next page number if more results are available, null if this is the last page
- `system_message` (string): Informational message about the search results and suggested next steps

### index_project

Creates new or rebuilds the Knowledge Graph index for a project to reflect recent changes.

Input:

- `project` (string): The absolute path to the project root directory to index.

Output: An object containing:

- `stats` (object): Detailed statistics about the indexing process, including file counts, definition counts, relationships, and language-specific information.
- `system_message` (string, optional): A message indicating if there were any issues during indexing, for example if no definitions were found.

### get_references

Find all references to a code definition (function, class, constant, etc.) across the entire codebase. Given a definition name and its file location, this tool identifies all call sites of a function, class, etc. Ideal for impact analysis, dependency mapping, and ensuring safe, confident refactoring. Use in tandem with search_codebase_definitions: first find the definition, then discover where it's used.

Input:

- `definition_name` (string): The exact identifier name to search for (e.g., 'myFunction', 'MyClass'). Must match the symbol name exactly as it appears in code, without namespace prefixes or file extensions.
- `file_path` (string): Absolute or relative filesystem path to the file where the definition is declared.
- `page` (integer, optional) (default: 1): Page number for pagination, starting from 1.

Output: An object containing:

- `definitions` (array): Array of definitions that reference the target symbol, each containing:
  - `name` (string): Name of the definition that contains references to the target.
  - `location` (string): File path and line number where the referencing definition is declared.
  - `definition_type` (string): The type of the referencing definition (e.g., "Method", "Constructor", "Class").
  - `fqn` (string): Fully qualified name of the referencing definition.
  - `references` (array): Array of specific reference instances within this definition, each containing:
    - `reference_type` (string): The type of reference (e.g., "CALLS", "PropertyReference").
    - `location` (string): File path and line number where the reference occurs.
    - `context` (string): The lines of code surrounding the reference.
- `next_page` (integer, optional): The next page number for pagination. If this field is absent, you have reached the last page of results.
- `system_message` (string): Additional information about the search results and suggestions for next steps.

### read_definitions

Read the definition bodies for multiple definitions across the codebase. Optimizes token usage by allowing multiple definition names per file.

Input:

- `definitions` (array): Array of definition requests with names array and file path. Each object in the array has two required fields:
  - `names` (array): Array of exact identifier names to read from the same file. Must match symbol names exactly as they appear in code, without namespace prefixes or file extensions. Example: ['myFunction', 'MyClass'].
  - `file_path` (string): Absolute or project-relative path to the file that contains the definitions. Example: src/main/java/com/example/User.java

Output: An object containing:

- `definitions` (array): Array of matching code definitions, each containing:
  - `name` (string): The name of the definition.
  - `fqn` (string): The fully qualified name of the definition.
  - `definition_type` (string): The type of definition (e.g., "Method", "Class").
  - `location` (string): File path and line range where the definition is located.
  - `definition_body` (string): The full code of the definition's body.
- `system_message` (string): An informational message, for example if some definitions were not found.

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
