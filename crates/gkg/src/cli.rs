use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "gkg",
    author = "GitLab Inc.",
    // Use the default attributes feature of clap to set the proper version of gkg at compile time
    version,
    about = "GitLab Knowledge Graph CLI",
    long_about = "Creates a structured, queryable representation of code repositories."
)]
pub struct GkgCli {
    #[command(subcommand)]
    pub command: Commands,
}

impl GkgCli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

#[cfg(not(any(debug_assertions, feature = "dev-tools")))]
const DEV_TOOLS_ENABLED: bool = false;

#[cfg(any(debug_assertions, feature = "dev-tools"))]
const DEV_TOOLS_ENABLED: bool = true;

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Index repositories in a workspace
    Index {
        /// Directory to scan for repositories
        #[arg(default_value = ".")]
        workspace_path: PathBuf,

        /// Number of worker threads (0 means auto-detect based on CPU cores)
        #[arg(short, long, default_value_t = 0)]
        threads: usize,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,

        /// Output statistics. Optionally specify a file path to save to.
        #[arg(long, value_name = "FILE", num_args = 0..=1, require_equals = true)]
        stats: Option<Option<PathBuf>>,
    },
    /// Manage the gkg server
    Server {
        #[command(subcommand)]
        action: Option<ServerCommands>,
    },
    /// Remove all indexed data
    Clean,
    /// Developer tools (enabled for debug builds or with --features dev-tools in release builds)
    #[command(hide = !DEV_TOOLS_ENABLED, name="devtools")]
    DevTools {
        #[command(subcommand)]
        command: DevToolsCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum DevToolsCommands {
    /// Query the knowledge graph with a query string or a query file
    Query {
        /// Project path for query to be executed against
        #[arg(long)]
        project: String,
        /// Query string or file path containing the query
        #[arg(value_name = "QUERY_OR_FILE")]
        query_or_file: String,
    },
    /// List all indexed repositories
    List {
        /// List projects in a workspace folder
        #[arg(long, default_value_t = true)]
        projects: bool,
        /// List workspace folders
        #[arg(long, default_value_t = false)]
        workspace_folders: bool,
        /// Don't print headers
        #[arg(long, default_value_t = false)]
        header: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ServerCommands {
    /// Start the gkg server
    Start(ServerStartArgs),
    /// Stop the running gkg server
    Stop,
}

#[derive(Args, Debug)]
pub struct ServerStartArgs {
    /// Path to MCP configuration file (example: ~/.gitlab/duo/mcp.json)
    #[arg(long)]
    pub register_mcp: Option<PathBuf>,

    /// Enable reindexing
    #[arg(long, default_value_t = false)]
    pub enable_reindexing: bool,

    /// Start the server in detached mode (Unix only)
    #[arg(long, default_value_t = false)]
    pub detached: bool,

    /// Internal: specify port to bind (used by detached launcher)
    #[arg(long, hide = true)]
    pub port: Option<u16>,

    /// Path to MCP configuration file (example: ~/.gkg/mcp.settings.json)
    #[arg(long)]
    pub mcp_configuration_path: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(long)]
    pub verbose: bool,
}
