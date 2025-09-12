use std::{path::PathBuf, sync::Arc};

use rmcp::model::ErrorCode;
use workspace_manager::WorkspaceManager;

// Database management utils

pub fn get_database_path(
    workspace_manager: &Arc<WorkspaceManager>,
    project_absolute_path: &str,
) -> Result<PathBuf, rmcp::ErrorData> {
    let database_path = workspace_manager
        .get_project_for_path(project_absolute_path)
        .map(|p| p.database_path);

    if database_path.is_none() {
        return Err(rmcp::ErrorData::new(
            ErrorCode::INVALID_REQUEST,
            "Project not found in workspace manager".to_string(),
            None,
        ));
    }

    Ok(database_path.unwrap())
}
