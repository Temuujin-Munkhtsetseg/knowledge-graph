use anyhow::Result;
use mcp::configuration::add_local_http_server_to_mcp_config;
use serde_json;
use std::env;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::{fs, process};

use crate::utils::{ServerInfo, get_lock_file_path, get_single_instance, is_server_running};
use database::kuzu::database::KuzuDatabase;
use event_bus::EventBus;
use workspace_manager::WorkspaceManager;

pub fn print_server_info(port: u16) -> Result<()> {
    let server_info = ServerInfo { port };
    println!("{}", serde_json::to_string(&server_info)?);
    Ok(())
}

pub async fn run(
    register_mcp: Option<std::path::PathBuf>,
    enable_reindexing: bool,
    detached: bool,
    port_override: Option<u16>,
    database: Arc<KuzuDatabase>,
    workspace_manager: Arc<WorkspaceManager>,
    event_bus: Arc<EventBus>,
) -> Result<()> {
    if detached {
        let instance = get_single_instance()?;
        if !instance.is_single() {
            if let Some(existing_port) = is_server_running()? {
                if let Some(mcp_config_path) = register_mcp {
                    add_local_http_server_to_mcp_config(mcp_config_path, existing_port)?;
                }
                print_server_info(existing_port)?;
                return Ok(());
            } else {
                eprintln!(
                    "gkg server is in an inconsistent state. Please check for stale processes and remove ~/.gkg/gkg.lock."
                );
                process::exit(1);
            }
        }

        // Preselect a port and create lock file before forking
        let port = port_override.unwrap_or(http_server::find_unused_port()?);
        let lock_file_path = get_lock_file_path()?;
        let mut file = fs::File::create(&lock_file_path)?;
        write!(file, "{port}")?;
        // Ensure the lock file contents are flushed before we print JSON
        file.flush()?;
        print_server_info(port)?;
        // Release instance lock before spawning the child
        drop(instance);

        #[cfg(unix)]
        {
            let current_exe = env::current_exe()?;
            let mut args: Vec<String> = vec!["server".to_string()];
            if let Some(path) = register_mcp.as_ref() {
                args.push("--register-mcp".to_string());
                args.push(path.display().to_string());
            }
            if enable_reindexing {
                args.push("--enable-reindexing".to_string());
            }
            args.push("--port".to_string());
            args.push(port.to_string());

            let mut cmd = Command::new(current_exe);
            cmd.args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            unsafe {
                cmd.pre_exec(|| {
                    // Create a new session to fully detach from the controlling terminal
                    libc::setsid();
                    Ok(())
                });
            }

            let _child = cmd.spawn()?;
            return Ok(());
        }

        #[cfg(not(unix))]
        {
            eprintln!("Detached mode is only supported on Unix-like systems");
            process::exit(1);
        }
    }

    let instance = get_single_instance()?;
    if instance.is_single() {
        let port = port_override.unwrap_or(http_server::find_unused_port()?);
        let lock_file_path = get_lock_file_path()?;
        let mut file = fs::File::create(&lock_file_path)?;
        // write port to lock file for other services to detect the running server
        write!(file, "{port}")?;
        // Ensure the lock file contents are flushed before we print JSON
        file.flush()?;
        // print server info to stdout for caller to allow connection
        print_server_info(port)?;

        if let Some(mcp_config_path) = register_mcp {
            // TODO: Add logging when this happens
            add_local_http_server_to_mcp_config(mcp_config_path, port)?;
        }

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
        // print server info to stdout for caller to allow connection
        print_server_info(port)?;
        Ok(())
    } else {
        eprintln!(
            "gkg server is in an inconsistent state. Please check for stale processes and remove ~/.gkg/gkg.lock."
        );
        process::exit(1);
    }
}
