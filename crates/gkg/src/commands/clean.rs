use anyhow::Result;
use std::process;
use std::sync::Arc;
use tracing::{error, info};

use crate::utils::is_server_running;
use workspace_manager::WorkspaceManager;

pub fn run(workspace_manager: Arc<WorkspaceManager>) -> Result<()> {
    if let Some(port) = is_server_running()? {
        error!("Error: gkg server is running on port {port}. Stop it before running clean.");
        process::exit(1);
    }

    workspace_manager.clean()?;
    info!("Clean completed");
    Ok(())
}
