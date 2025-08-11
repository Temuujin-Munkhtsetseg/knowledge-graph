use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "gkg",
    author = "GitLab Inc.",
    version = "0.1.0",
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
        stats_output: Option<Option<PathBuf>>,
    },
    /// Manage the gkg server
    Server {
        #[command(subcommand)]
        action: ServerCommands,
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
}
