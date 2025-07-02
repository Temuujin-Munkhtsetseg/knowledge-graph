use serde::{Deserialize, Serialize};
use ts_rs::TS;
use workspace_manager::ProjectInfo;

#[derive(Serialize, Deserialize, TS, Default, Clone, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/project_info.ts")]
pub struct TSProjectInfo {
    pub project_path: String,
    pub workspace_folder_path: String,
    pub project_hash: String,
    pub status: String,
    pub last_indexed_at: Option<String>,
    pub error_message: Option<String>,
    pub database_path: String,
    pub parquet_directory: String,
}

pub fn to_ts_project_info(project_info: &ProjectInfo) -> TSProjectInfo {
    TSProjectInfo {
        project_path: project_info.project_path.clone(),
        workspace_folder_path: project_info.workspace_folder_path.clone(),
        project_hash: project_info.project_hash.clone(),
        status: project_info.status.to_string(),
        last_indexed_at: project_info.last_indexed_at.map(|dt| dt.to_rfc3339()),
        error_message: project_info.error_message.clone(),
        database_path: project_info.database_path.to_string_lossy().to_string(),
        parquet_directory: project_info.parquet_directory.to_string_lossy().to_string(),
    }
}
