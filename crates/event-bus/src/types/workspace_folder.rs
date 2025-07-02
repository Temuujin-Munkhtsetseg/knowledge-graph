use serde::{Deserialize, Serialize};
use ts_rs::TS;
use workspace_manager::WorkspaceFolderInfo;

#[derive(Serialize, Deserialize, TS, Default, Clone, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/workspace_folder.ts")]
pub struct TSWorkspaceFolderInfo {
    pub workspace_folder_path: String,
    pub data_directory_name: String,
    pub status: String,
    pub last_indexed_at: Option<String>,
    pub project_count: usize,
}

pub fn to_ts_workspace_folder_info(
    workspace_folder_info: &WorkspaceFolderInfo,
) -> TSWorkspaceFolderInfo {
    TSWorkspaceFolderInfo {
        workspace_folder_path: workspace_folder_info.workspace_folder_path.clone(),
        data_directory_name: workspace_folder_info.data_directory_name.clone(),
        status: workspace_folder_info.status.to_string(),
        last_indexed_at: workspace_folder_info
            .last_indexed_at
            .map(|dt| dt.to_rfc3339()),
        project_count: workspace_folder_info.project_count,
    }
}
