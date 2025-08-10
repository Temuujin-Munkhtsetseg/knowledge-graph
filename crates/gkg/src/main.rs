mod cli;
mod commands;
mod utils;

use crate::commands::{index, server};
use cli::{Commands, GkgCli};
use database::kuzu::database::KuzuDatabase;
use event_bus::EventBus;
use logging::LogMode;
use std::sync::Arc;
use workspace_manager::WorkspaceManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = GkgCli::parse_args();

    let verbose = match cli.command {
        Commands::Index { verbose, .. } => verbose,
        Commands::Server { .. } => false,
    };

    let mode = match cli.command {
        Commands::Index { .. } => LogMode::Cli,
        Commands::Server { .. } => LogMode::Server,
    };

    let _guard = logging::init(mode, verbose)?;

    let workspace_manager = Arc::new(WorkspaceManager::new_system_default()?);
    let event_bus = Arc::new(EventBus::new());
    let database = Arc::new(KuzuDatabase::new());

    match cli.command {
        Commands::Index {
            workspace_path,
            threads,
            verbose: _,
            stats_output,
        } => {
            index::run(
                workspace_path,
                threads,
                stats_output,
                Arc::clone(&workspace_manager),
                Arc::clone(&event_bus),
                Arc::clone(&database),
            )
            .await
        }
        Commands::Server {
            register_mcp,
            enable_reindexing,
            detached,
            port: port_override,
            ..
        } => {
            server::run(
                register_mcp,
                enable_reindexing,
                detached,
                port_override,
                Arc::clone(&database),
                Arc::clone(&workspace_manager),
                Arc::clone(&event_bus),
            )
            .await
        }
    }
}
