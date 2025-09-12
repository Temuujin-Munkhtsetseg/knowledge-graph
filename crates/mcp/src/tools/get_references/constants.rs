pub const GET_REFERENCES_TOOL_NAME: &str = "get_references";
pub(in crate::tools::get_references) const GET_REFERENCES_TOOL_DESCRIPTION: &str = r#"Find all references to a code definition (function, class, constant, etc.) across the entire codebase.

Behavior:
- Searches for every location where a given symbol is called.
- Returns file paths, line numbers, and context around each usage.
- Large result sets are paginated with the `page` parameter.

Requirements:
- Provide the exact symbol name as it appears in code (case-sensitive).
- Specify the file path where the definition is declared.

Use cases:
- Impact analysis before refactoring
- Finding all callers of a function
- Dependency mapping

Example:
Function definition: `export const = calculateTotal(param) => {...}` in `/project/src/utils/math.js`
Page: 1 (first page)
Call:
{
  "definition_name": "calculateTotal",
  "file_path": "/project/src/utils/math.js",
  "page": 1,
}

This will find all places where `calculateTotal` is called throughout the codebase.
Tip: Use with `search_codebase_definitions` first to locate the definition, then use this tool to find all its references."#;

// Schema field names
pub(in crate::tools::get_references) const DEFINITION_NAME_FIELD: &str = "definition_name";
pub(in crate::tools::get_references) const FILE_PATH_FIELD: &str = "file_path";
pub(in crate::tools::get_references) const PAGE_FIELD: &str = "page";

// Default values
pub(in crate::tools::get_references) const DEFAULT_PAGE: u64 = 1;

// Limits
pub(in crate::tools::get_references) const MIN_PAGE: u64 = 1;
