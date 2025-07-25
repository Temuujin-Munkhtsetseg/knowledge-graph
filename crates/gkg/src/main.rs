mod cli;

use anyhow::Result;
use cli::{Commands, GkgCli};
use database::kuzu::database::KuzuDatabase;
use event_bus::EventBus;
use home::home_dir;
use indexer::execution::config::IndexingConfigBuilder;
use indexer::execution::executor::IndexingExecutor;
use indexer::stats::WorkspaceStatistics;
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
use tracing::{error, info};
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

#[cfg(target_os = "macos")]
fn get_single_instance() -> Result<SingleInstance> {
    let home_dir = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let single_instance_path = home_dir.join(GKG_HTTP_SERVER);
    // On macOS, SingleInstance uses a file-based lock to ensure only one instance of the server is running.
    Ok(SingleInstance::new(single_instance_path.to_str().unwrap())?)
}

#[cfg(not(target_os = "macos"))]
fn get_single_instance() -> Result<SingleInstance> {
    // SingleInstance has a different implementation for Windows and Linux.
    // On Windows, it will create a mutex object using CreateMutexW.
    // On Linux, it will create a socket based on the name.
    Ok(SingleInstance::new(GKG_HTTP_SERVER)?)
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

fn handle_statistics_output(
    workspace_stats: &WorkspaceStatistics,
    stats_output: Option<Option<PathBuf>>,
) {
    match stats_output {
        Some(stats_path_option) => {
            // Save to file if path provided
            if let Some(stats_path) = stats_path_option {
                match workspace_stats.export_to_file(&stats_path) {
                    Ok(_) => {
                        info!("Statistics saved to: {}", stats_path.display());
                    }
                    Err(e) => {
                        error!("Failed to save statistics: {e}");
                    }
                }
            }

            // Display statistics (both when file path provided and when not)
            info!("Indexing Summary:");
            info!("  - Total Projects: {}", workspace_stats.total_projects);
            info!("  - Total Files: {}", workspace_stats.total_files);
            info!(
                "  - Total Definitions: {}",
                workspace_stats.total_definitions
            );

            if !workspace_stats.projects.is_empty() {
                info!("Project Timing:");
                for project in &workspace_stats.projects {
                    info!(
                        "  - {}: {:.2}s ({} files, {} definitions)",
                        project.project_name,
                        project.indexing_duration_seconds,
                        project.total_files,
                        project.total_definitions
                    );
                }
            }

            if !workspace_stats.total_languages.is_empty() {
                info!("Language Breakdown:");
                let mut languages: Vec<(&String, &indexer::stats::LanguageSummary)> =
                    workspace_stats.total_languages.iter().collect();
                languages.sort_by(|a, b| b.1.file_count.cmp(&a.1.file_count));

                for (language, summary) in languages.iter().take(10) {
                    info!(
                        "  - {}: {} files, {} definitions",
                        language, summary.file_count, summary.definitions_count
                    );
                }

                if languages.len() > 10 {
                    info!("  ... and {} more languages", languages.len() - 10);
                }
            }
        }
        None => {
            // Do not display statistics when option is not specified
        }
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
            stats_output,
        } => {
            if let Some(port) = is_server_running()? {
                error!(
                    "Error: gkg server is running on port {port}. Please stop it to run indexing."
                );
                process::exit(1);
            }

            let mut rx = event_bus.subscribe();
            tokio::spawn(async move {
                while (rx.recv().await).is_ok() {
                    // TODO: Add a CLI frontend consumer for this.
                    // disabled for now
                    // println!("[EVENT] {event:?}");
                }
            });

            let config = IndexingConfigBuilder::build(threads);
            let mut executor = IndexingExecutor::new(
                database.clone(),
                workspace_manager.clone(),
                event_bus,
                config,
            );

            let canonical_workspace_path = workspace_path.canonicalize()?;
            let start_time = std::time::Instant::now();

            match executor.execute_workspace_indexing(canonical_workspace_path.clone(), None) {
                Ok(workspace_stats) => {
                    let indexing_duration = start_time.elapsed();
                    println!(
                        "✅ Workspace indexing completed in {:.2} seconds",
                        indexing_duration.as_secs_f64()
                    );

                    // Handle statistics output based on user request
                    handle_statistics_output(&workspace_stats, stats_output);
                }
                Err(e) => {
                    error!("❌ Indexing failed: {e}");
                    process::exit(1);
                }
            }

            Ok(())
        }
        Commands::Server {
            register_mcp,
            enable_reindexing,
            ..
        } => {
            let instance = get_single_instance()?;
            if instance.is_single() {
                let port = http_server::find_unused_port()?;

                let lock_file_path = get_lock_file_path()?;
                let mut file = fs::File::create(&lock_file_path)?;
                write!(file, "{port}")?;

                if let Some(mcp_config_path) = register_mcp {
                    add_local_http_server_to_mcp_config(mcp_config_path, port)?;
                }

                let server_info = ServerInfo { port };
                info!("{}", serde_json::to_string(&server_info)?);

                let l_file = lock_file_path.clone();
                ctrlc::set_handler(move || {
                    let _ = fs::remove_file(&l_file);
                    process::exit(0);
                })?;

                http_server::run(
                    port,
                    enable_reindexing,
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
                info!("{}", serde_json::to_string(&server_info)?);
                Ok(())
            } else {
                error!(
                    "gkg server is in an inconsistent state. Please remove ~/.gkg/gkg-server.lock and try again."
                );
                process::exit(1);
            }
        }
    }
}
