use anyhow::{Result, bail};
use mcp::configuration::get_or_create_mcp_configuration;
use mcp::duo_configuration::add_local_http_server_to_mcp_config;
use serde_json;
use std::env;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::{fs, process};
use tracing::info;

#[cfg(unix)]
use nix::sys::signal::{Signal::SIGTERM, kill};
#[cfg(unix)]
use nix::unistd::Pid;

#[cfg(windows)]
use std::time::Duration;

use crate::utils::{
    ServerInfo, ServerLockInfo, get_lock_file_path, get_single_instance, is_server_running,
    read_lock_info, remove_lock_file, write_lock_info,
};
use database::kuzu::database::KuzuDatabase;
use event_bus::EventBus;
use workspace_manager::WorkspaceManager;

pub fn print_server_info(port: u16) -> Result<()> {
    let server_info = ServerInfo { port };
    println!("{}", serde_json::to_string(&server_info)?);
    std::io::stdout().flush().ok();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn start(
    register_duo_mcp: Option<std::path::PathBuf>,
    enable_reindexing: bool,
    detached: bool,
    port_override: Option<u16>,
    mcp_configuration_path: Option<std::path::PathBuf>,
    database: Arc<KuzuDatabase>,
    workspace_manager: Arc<WorkspaceManager>,
    event_bus: Arc<EventBus>,
) -> Result<()> {
    if detached {
        let instance = get_single_instance()?;
        if !instance.is_single() {
            if let Some(existing_port) = is_server_running()? {
                if let Some(mcp_config_path) = register_duo_mcp {
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
        let port = port_override.unwrap_or(http_server_desktop::find_unused_port()?);
        // Write a provisional lock (no pid yet) so other invocations can discover the port
        write_lock_info(&ServerLockInfo { port, pid: None })?;
        print_server_info(port)?;
        // Release instance lock before spawning the child
        drop(instance);

        #[cfg(unix)]
        {
            let current_exe = env::current_exe()?;
            let mut args: Vec<String> = vec!["server".to_string(), "start".to_string()];
            if let Some(path) = register_duo_mcp.as_ref() {
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

            let child = cmd.spawn()?;
            // Update lock with child PID
            let _ = write_lock_info(&ServerLockInfo {
                port,
                pid: Some(child.id()),
            });
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
        let port = port_override.unwrap_or(http_server_desktop::find_unused_port()?);
        let lock = ServerLockInfo {
            port,
            pid: Some(process::id()),
        };
        write_lock_info(&lock)?;
        // print server info to stdout for caller to allow connection
        print_server_info(port)?;

        if let Some(mcp_config_path) = register_duo_mcp {
            // TODO: Add logging when this happens
            add_local_http_server_to_mcp_config(mcp_config_path, port)?;
        }

        let mcp_configuration = match mcp_configuration_path {
            Some(path) => Arc::new(mcp::configuration::read_mcp_configuration(path)),
            None => Arc::new(get_or_create_mcp_configuration(workspace_manager.clone())),
        };

        let l_file = get_lock_file_path()?;
        ctrlc::set_handler(move || {
            let _ = fs::remove_file(&l_file);
            process::exit(0);
        })?;

        http_server_desktop::run(
            port,
            enable_reindexing,
            Arc::clone(&database),
            Arc::clone(&workspace_manager),
            Arc::clone(&event_bus),
            Arc::clone(&mcp_configuration),
        )
        .await
    } else if let Some(port) = is_server_running()? {
        if let Some(mcp_config_path) = register_duo_mcp {
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

pub async fn stop() -> Result<()> {
    if let Some(info) = read_lock_info()? {
        // Try graceful stop via SIGTERM on Unix, else remove lock if process gone
        #[cfg(unix)]
        {
            if let Some(pid) = info.pid
                && kill(Pid::from_raw(pid as i32), None).is_ok()
            {
                let _ = kill(Pid::from_raw(pid as i32), SIGTERM);
            }
        }

        // Best effort to stop the server on windows, on windows server cannot run in detached mode
        // so it is up to the caller to stop the server via ^C or taskkill
        // TODO: rework windows handling to use windows-service approach
        #[cfg(windows)]
        {
            if let Some(pid) = info.pid {
                // Temporary behavior: forceful termination only
                let _ = Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .status();
            }
        }

        let _ = remove_lock_file();
        info!("Server stopped");
        println!(
            "{}",
            serde_json::to_string(&ServerInfo { port: info.port })?
        );
        return Ok(());
    }
    bail!("No running server found");
}
