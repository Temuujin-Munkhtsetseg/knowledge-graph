mod cli;

use crate::cli::{Commands, GkgCli};
use anyhow::Result;
use home::home_dir;
use indexer::runner::run_client_indexer;
use serde::{Deserialize, Serialize};
use single_instance::SingleInstance;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;
use std::{fs, process};

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

    match cli.command {
        Commands::Index {
            workspace_path,
            threads,
            verbose,
        } => {
            if let Some(port) = is_server_running()? {
                eprintln!(
                    "Error: gkg server is running on port {port}. Please stop it to run indexing."
                );
                process::exit(1);
            }
            init_logging(verbose);
            run_client_indexer(workspace_path, threads, |msg| println!("{msg}"))
        }
        Commands::Server => {
            let instance = SingleInstance::new(GKG_HTTP_SERVER)?;
            if instance.is_single() {
                let port = http_server::find_unused_port()?;

                let lock_file_path = get_lock_file_path()?;
                let mut file = fs::File::create(&lock_file_path)?;
                write!(file, "{port}")?;

                let server_info = ServerInfo { port };
                println!("{}", serde_json::to_string(&server_info)?);

                let l_file = lock_file_path.clone();
                ctrlc::set_handler(move || {
                    let _ = fs::remove_file(&l_file);
                    process::exit(0);
                })?;

                http_server::run(port).await
            } else if let Some(port) = is_server_running()? {
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

fn init_logging(verbose: bool) {
    let log_level = if verbose { "debug" } else { "info" };

    // Initialize tracing subscriber
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    println!("Logging level set to: {log_level}");
}
