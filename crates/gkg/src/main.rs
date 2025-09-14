#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod cli;
mod commands;
mod utils;

use crate::commands::{clean, index, list, query, server};
use cli::{Commands, DevToolsCommands, GkgCli, ServerCommands, ServerStartArgs};
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
        Commands::Clean => false,
        Commands::DevTools { .. } => false,
    };

    let mode = match cli.command {
        Commands::Index { .. } => LogMode::Cli,
        Commands::Server { ref action } => match action {
            Some(ServerCommands::Start(args)) => {
                if args.detached {
                    LogMode::ServerBackground
                } else {
                    LogMode::ServerForeground
                }
            }
            Some(ServerCommands::Stop) => LogMode::ServerForeground,
            None => LogMode::ServerForeground, // Default to start command
        },
        Commands::Clean => LogMode::Cli,
        Commands::DevTools { .. } => LogMode::Cli,
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
            stats,
        } => {
            index::run(
                workspace_path,
                threads,
                stats,
                Arc::clone(&workspace_manager),
                Arc::clone(&event_bus),
                Arc::clone(&database),
            )
            .await
        }
        Commands::Server { action } => match action {
            Some(ServerCommands::Start(args)) => {
                server::start(
                    args.register_mcp,
                    args.enable_reindexing,
                    args.detached,
                    args.port,
                    args.mcp_configuration_path,
                    Arc::clone(&database),
                    Arc::clone(&workspace_manager),
                    Arc::clone(&event_bus),
                )
                .await
            }
            Some(ServerCommands::Stop) => server::stop().await,
            None => {
                // Default behavior: start with default arguments
                // FIXME: This is a temporary fix to allow the server to start with default arguments
                let args = ServerStartArgs {
                    register_mcp: None,
                    enable_reindexing: false,
                    detached: false,
                    port: None,
                    mcp_configuration_path: None,
                };
                server::start(
                    args.register_mcp,
                    args.enable_reindexing,
                    args.detached,
                    args.port,
                    args.mcp_configuration_path,
                    Arc::clone(&database),
                    Arc::clone(&workspace_manager),
                    Arc::clone(&event_bus),
                )
                .await
            }
        },
        Commands::Clean => clean::run(Arc::clone(&workspace_manager)),
        Commands::DevTools { command } => match command {
            DevToolsCommands::Query {
                project,
                query_or_file,
            } => {
                use crate::commands::query::QueryArgs;
                query::run(
                    Arc::clone(&workspace_manager),
                    Arc::clone(&database),
                    QueryArgs {
                        project,
                        query_or_file,
                    },
                )
            }
            DevToolsCommands::List {
                projects,
                workspace_folders,
                header,
            } => {
                use crate::commands::list::ListArgs;
                list::run(
                    Arc::clone(&workspace_manager),
                    ListArgs {
                        projects,
                        workspace_folders,
                        header,
                    },
                )
            }
        },
    }
}
