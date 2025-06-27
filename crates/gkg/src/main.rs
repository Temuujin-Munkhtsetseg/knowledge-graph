mod cli;

use anyhow::Result;
use cli::{Commands, GkgCli};
use indexer::runner::run_client_indexer;

fn main() -> Result<()> {
    let cli = GkgCli::parse_args();

    match cli.command {
        Commands::Index {
            workspace_path,
            threads,
            verbose,
        } => {
            if verbose {
                tracing_subscriber::fmt()
                    .with_max_level(tracing::Level::DEBUG)
                    .init();
            } else {
                tracing_subscriber::fmt()
                    .with_max_level(tracing::Level::INFO)
                    .init();
            }

            run_client_indexer(workspace_path, threads)
        }
    }
}
