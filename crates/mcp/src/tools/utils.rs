use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use rmcp::model::ErrorCode;
use workspace_manager::WorkspaceManager;

// File management utils

pub fn resolve_paths(
    workspace_manager: &WorkspaceManager,
    input_file_path: &str,
) -> Result<(PathBuf, workspace_manager::ProjectInfo, String), rmcp::ErrorData> {
    // Try as absolute first
    let abs_path = if Path::new(input_file_path).is_absolute() {
        PathBuf::from(input_file_path)
    } else {
        // Try to find a project containing this relative path
        let mut found: Option<PathBuf> = None;
        for project in workspace_manager.list_all_projects() {
            let candidate = Path::new(&project.project_path).join(input_file_path);
            if candidate.exists() {
                found = Some(candidate);
                break;
            }
        }
        found.ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                format!(
                    "File not found in any registered project: {}",
                    input_file_path
                ),
                None,
            )
        })?
    };

    let abs_path_str = abs_path
        .canonicalize()
        .map_err(|e| rmcp::ErrorData::new(ErrorCode::INVALID_REQUEST, e.to_string(), None))?;
    let abs_path_str = abs_path_str.to_string_lossy().to_string();

    let project_info = workspace_manager
        .get_project_for_file(&abs_path_str)
        .ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                "File not found in workspace manager".to_string(),
                None,
            )
        })?;

    let relative = Path::new(&abs_path_str)
        .strip_prefix(&project_info.project_path)
        .map_err(|_| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                "Failed to compute relative file path".to_string(),
                None,
            )
        })?
        .to_string_lossy()
        .to_string();

    Ok((PathBuf::from(abs_path_str), project_info, relative))
}

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
