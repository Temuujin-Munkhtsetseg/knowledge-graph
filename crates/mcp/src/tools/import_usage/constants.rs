pub const IMPORT_USAGE_TOOL_NAME: &str = "import_usage";
pub const IMPORT_USAGE_TOOL_DESCRIPTION: &str = r#"Analyze import usages across the project.

- Returns imports that match the requested paths (with file/line locations)
- Returns usages (call/reference sites) grouped by file with code snippets if found

Examples:
{
  "project_absolute_path": "/project/root",
  "packages": [
    { "import_path": "react", "name": "React" },
    { "import_path": "@vue/runtime-core" }
  ],
  "page": 1,
  "page_size": 50
}
"#;

pub const DEFAULT_PAGE: u64 = 1;
pub const DEFAULT_PAGE_SIZE: u64 = 50;
pub const MAX_PAGE_SIZE: u64 = 1000;
pub const FILE_READ_TIMEOUT_SECONDS: u64 = 10;
