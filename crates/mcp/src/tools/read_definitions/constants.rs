pub const READ_DEFINITIONS_TOOL_NAME: &str = "read_definitions";
pub(in crate::tools::read_definitions) const READ_DEFINITIONS_TOOL_DESCRIPTION: &str = r#"Read the definition bodies for multiple definitions across the codebase.

Behavior:
- Takes an array of definition groups, where each group can contains multiple definition names for a single file
- Returns the complete definition body/code for each found definition along with location information, fully qualified names, and definition types
- Handles multiple definitions efficiently in a single request, optimizing token usage by grouping definitions by file

Requirements:
- Provide exact definition names as they appear in code (case-sensitive)
- Specify absolute or project-relative file paths where definitions are expected to be found
- Group multiple definition names per file to optimize token usage

Use cases:
- Reading multiple related definitions at once
- Getting complete function/class/method/constant implementations
- Code analysis, understanding, preparation for refactoring operations

Example:
Definitions to read:
- Functions `calculateTotal` and `formatCurrency` in `/project/src/utils/math.js`
- Class `UserService` in `/project/src/services/user.js`

Call:
{
  "definitions": [
    {
      "names": ["calculateTotal", "formatCurrency"],
      "file_path": "/project/src/utils/math.js"
    },
    {
      "names": ["UserService"], 
      "file_path": "/project/src/services/user.js"
    }
  ]
}

This will return the complete code bodies for all definitions along with their metadata.
Tip: Use with `search_codebase_definitions` first to locate definitions, then use this tool to read their implementations."#;

pub(in crate::tools::read_definitions) const DEFINITIONS_FIELD: &str = "definitions";
pub(in crate::tools::read_definitions) const NAMES_FIELD: &str = "names";
pub(in crate::tools::read_definitions) const FILE_PATH_FIELD: &str = "file_path";
