mod cli;

use crate::cli::{Commands, GkgCli};
use anyhow::Result;
use database::kuzu::database::KuzuDatabase;
use event_bus::EventBus;
use home::home_dir;
use indexer::execution::config::IndexingConfigBuilder;
use indexer::execution::executor::IndexingExecutor;
use logging::LogMode;
use mcp::configuration::add_local_http_server_to_mcp_config;
use serde::{Deserialize, Serialize};
use single_instance::SingleInstance;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::{fs, process};
use workspace_manager::WorkspaceManager;

const GKG_HTTP_SERVER: &str = "gkg-http-server";

#[derive(Serialize, Deserialize)]
struct ServerInfo {
    port: u16,
}

fn get_gkg_dir() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let gkg_dir = home.join(".gkg");
    fs::create_dir_all(&gkg_dir)?;
    Ok(gkg_dir)
}

fn get_lock_file_path() -> Result<PathBuf> {
    Ok(get_gkg_dir()?.join("gkg.lock"))
}

fn is_server_running() -> Result<Option<u16>> {
    let lock_file = get_lock_file_path()?;
    if !lock_file.exists() {
        return Ok(None);
    }

    let mut contents = String::new();
    fs::File::open(&lock_file)?
        .read_to_string(&mut contents)
        .map_err(|e| anyhow::anyhow!("Could not read lock file: {}", e))?;
    let port: u16 = contents
        .trim()
        .parse()
        .map_err(|e| anyhow::anyhow!("Could not parse port from file: {}", e))?;

    if TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse()?,
        Duration::from_millis(100),
    )
    .is_ok()
    {
        Ok(Some(port))
    } else {
        // Server is not running, so we can remove the stale port file.
        fs::remove_file(lock_file)?;
        Ok(None)
    }
}

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
        } => {
            if let Some(port) = is_server_running()? {
                eprintln!(
                    "Error: gkg server is running on port {port}. Please stop it to run indexing."
                );
                process::exit(1);
            }

            let mut rx = event_bus.subscribe();
            tokio::spawn(async move {
                while let Ok(event) = rx.recv().await {
                    // TODO: Add a CLI frontend consumer for this.
                    println!("[EVENT] {event:?}");
                }
            });

            let config = IndexingConfigBuilder::build(threads);
            let mut executor =
                IndexingExecutor::new(database, workspace_manager, event_bus, config);

            executor.execute_workspace_indexing(workspace_path, None)
        }
        Commands::Server { register_mcp } => {
            let instance = SingleInstance::new(GKG_HTTP_SERVER)?;
            if instance.is_single() {
                let port = http_server::find_unused_port()?;

                let lock_file_path = get_lock_file_path()?;
                let mut file = fs::File::create(&lock_file_path)?;
                write!(file, "{port}")?;

                if let Some(mcp_config_path) = register_mcp {
                    add_local_http_server_to_mcp_config(mcp_config_path, port)?;
                }

                let server_info = ServerInfo { port };
                println!("{}", serde_json::to_string(&server_info)?);

                let l_file = lock_file_path.clone();
                ctrlc::set_handler(move || {
                    let _ = fs::remove_file(&l_file);
                    process::exit(0);
                })?;

                http_server::run(
                    port,
                    Arc::clone(&database),
                    Arc::clone(&workspace_manager),
                    Arc::clone(&event_bus),
                )
                .await
            } else if let Some(port) = is_server_running()? {
                if let Some(mcp_config_path) = register_mcp {
                    add_local_http_server_to_mcp_config(mcp_config_path, port)?;
                }

                let server_info = ServerInfo { port };
                println!("{}", serde_json::to_string(&server_info)?);
                Ok(())
            } else {
                eprintln!(
                    "gkg server is in an inconsistent state. Please remove ~/.gkg/gkg-server.lock and try again."
                );
                process::exit(1);
            }
        }
    }
}
