use clap::{Parser, Subcommand};
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
    },
    /// Start the gkg server
    Server {
        /// Path to MCP configuration file (example: ~/.gitlab/duo/mcp.json)
        #[arg(long)]
        register_mcp: Option<PathBuf>,
    },
}
