pub const REPO_MAP_TOOL_NAME: &str = "repo_map";
pub const REPO_MAP_TOOL_DESCRIPTION: &str = r#"Generate a compact, API-style map for selected repository paths.

Useful for:
- Summarize a repository segment for LLMs or code review
- Explore structure first (directories only), then fetch definitions
- Useful for other tools, like get_references

Recommendations:
- Keep depth at 1–2 for large repos to control output size
- Increase page_size or follow next-page if more results are needed
"#;

pub const DEFAULT_PAGE: u64 = 1;
pub const DEFAULT_PAGE_SIZE: u64 = 200;
pub const MAX_PAGE_SIZE: u64 = 1000;
pub const MIN_PAGE: u64 = 1;
pub const DEFAULT_DEPTH: u64 = 1;
pub const FILE_READ_TIMEOUT_SECONDS: u64 = 10;
