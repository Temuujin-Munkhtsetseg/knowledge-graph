pub mod handlers;
pub mod tools;
pub mod types;

// Re-export commonly used items for easier importing
pub use handlers::*;
pub use types::*;

pub const MCP_NAME: &str = "knowledge-graph";
pub const MCP_LOCAL_FILE: &str = "mcp.json";
