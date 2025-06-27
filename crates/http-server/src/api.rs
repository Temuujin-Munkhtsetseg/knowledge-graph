use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct ServerInfoResponse {
    pub port: u16,
}

#[derive(Deserialize)]
pub struct WorkspacePathRequest {
    pub workspace: String,
}

#[derive(Serialize)]
pub struct WorkspaceResponse {
    /// The path to the workspace.
    pub path: String,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
}

#[derive(Serialize)]
pub struct IndexingProgressResponse {
    pub message: String,
}

#[derive(Deserialize)]
pub struct IndexRequest {
    pub workspace: String,
}
