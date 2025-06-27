mod cli;

use crate::cli::{Commands, GkgCli};
use anyhow::Result;
use home::home_dir;
use indexer::runner::run_client_indexer;
use mcp::{MCP_LOCAL_FILE, MCP_NAME};
use serde::{Deserialize, Serialize};
use single_instance::SingleInstance;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;
use std::{fs, process, vec::Vec};

const GKG_HTTP_SERVER: &str = "gkg-http-server";

#[derive(Serialize, Deserialize)]
struct ServerInfo {
    port: u16,
}

#[derive(Serialize, Deserialize, Default)]
struct McpConfig {
    #[serde(rename = "mcpServers")]
    mcp_servers: HashMap<String, McpServer>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum McpServer {
    Url { url: String },
    Command { command: String, args: Vec<String> },
}

fn get_gkg_dir() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let gkg_dir = home.join(".gkg");
    fs::create_dir_all(&gkg_dir)?;
    Ok(gkg_dir)
}

fn get_mcp_config_path() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let mcp_dir = home.join(".gitlab").join("duo");
    fs::create_dir_all(&mcp_dir)?;
    Ok(mcp_dir.join(MCP_LOCAL_FILE))
}

fn get_lock_file_path() -> Result<PathBuf> {
    Ok(get_gkg_dir()?.join("gkg.lock"))
}

fn update_mcp_config(port: u16) -> Result<()> {
    let mcp_path = get_mcp_config_path()?;
    let mut config = if mcp_path.exists() {
        let content = fs::read_to_string(&mcp_path)?;

        serde_json::from_str(&content).unwrap_or_else(|_| {
            eprintln!("Warning: Could not parse existing MCP config, creating new one");
            McpConfig::default()
        })
    } else {
        McpConfig::default()
    };

    config.mcp_servers.insert(
        MCP_NAME.to_string(),
        McpServer::Url {
            url: format!("http://localhost:{port}/mcp"),
        },
    );

    let json = serde_json::to_string_pretty(&config)?;
    fs::write(&mcp_path, json)?;

    Ok(())
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

            if verbose {
                tracing_subscriber::fmt()
                    .with_max_level(tracing::Level::DEBUG)
                    .init();
            } else {
                tracing_subscriber::fmt()
                    .with_max_level(tracing::Level::INFO)
                    .init();
            }

            run_client_indexer(workspace_path, threads, |msg| println!("{msg}"))
        }
        Commands::Server { register_mcp } => {
            let instance = SingleInstance::new(GKG_HTTP_SERVER)?;
            if instance.is_single() {
                let port = http_server::find_unused_port()?;

                let lock_file_path = get_lock_file_path()?;
                let mut file = fs::File::create(&lock_file_path)?;
                write!(file, "{port}")?;

                // Update MCP configuration with the new server only if flag is provided
                if register_mcp {
                    update_mcp_config(port)?;
                }

                let server_info = ServerInfo { port };
                println!("{}", serde_json::to_string(&server_info)?);

                let l_file = lock_file_path.clone();
                ctrlc::set_handler(move || {
                    let _ = fs::remove_file(&l_file);
                    process::exit(0);
                })?;

                http_server::run(port).await
            } else if let Some(port) = is_server_running()? {
                // Update MCP configuration with existing server only if flag is provided
                if register_mcp {
                    update_mcp_config(port)?;
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
