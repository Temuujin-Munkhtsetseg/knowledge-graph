use crate::AppState;
use crate::contract::{EmptyRequest, EndpointConfigTypes};
use crate::define_endpoint;
use crate::endpoints::shared::StatusResponse;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Deserialize, Serialize, TS, Default, Clone)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct WorkspaceDeleteBodyRequest {
    pub workspace_folder_path: String,
}

#[derive(Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct WorkspaceDeleteResponses {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_folder_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bad_request: Option<StatusResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_found: Option<StatusResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_server_error: Option<StatusResponse>,
}

pub struct WorkspaceDeleteEndpointConfig;

impl EndpointConfigTypes for WorkspaceDeleteEndpointConfig {
    type PathRequest = EmptyRequest;
    type BodyRequest = WorkspaceDeleteBodyRequest;
    type QueryRequest = EmptyRequest;
    type Response = WorkspaceDeleteResponses;
}

define_endpoint! {
    WorkspaceDeleteEndpoint,
    WorkspaceDeleteEndpointDef,
    Delete,
    "/workspace/delete",
    ts_path_type = "\"/api/workspace/delete\"",
    config = WorkspaceDeleteEndpointConfig,
    export_to = "../../../packages/gkg/src/api.ts"
}

impl WorkspaceDeleteEndpoint {
    pub fn create_success_response(
        workspace_folder_path: String,
        removed: bool,
    ) -> WorkspaceDeleteResponses {
        WorkspaceDeleteResponses {
            workspace_folder_path: Some(workspace_folder_path),
            removed: Some(removed),
            bad_request: None,
            not_found: None,
            internal_server_error: None,
        }
    }

    pub fn create_error_response(status: String) -> StatusResponse {
        StatusResponse { status }
    }
}

/// Handler for the workspace delete endpoint
/// Removes a workspace folder and all its associated data from the system
pub async fn delete_handler(
    State(state): State<AppState>,
    Json(payload): Json<WorkspaceDeleteBodyRequest>,
) -> impl IntoResponse {
    // Validate workspace folder path
    if payload.workspace_folder_path.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(WorkspaceDeleteResponses {
                bad_request: Some(WorkspaceDeleteEndpoint::create_error_response(
                    "empty_workspace_path".to_string(),
                )),
                ..Default::default()
            }),
        )
            .into_response();
    }

    // Check if workspace exists before attempting deletion
    let workspace_info = state
        .workspace_manager
        .get_workspace_folder_info(&payload.workspace_folder_path);

    if workspace_info.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(WorkspaceDeleteResponses {
                not_found: Some(WorkspaceDeleteEndpoint::create_error_response(
                    "workspace_not_found".to_string(),
                )),
                ..Default::default()
            }),
        )
            .into_response();
    }

    // Attempt to remove the workspace
    match state
        .workspace_manager
        .remove_workspace_folder(&payload.workspace_folder_path)
    {
        Ok(removed) => (
            StatusCode::OK,
            Json(WorkspaceDeleteEndpoint::create_success_response(
                payload.workspace_folder_path,
                removed,
            )),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to remove workspace folder: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(WorkspaceDeleteResponses {
                    internal_server_error: Some(WorkspaceDeleteEndpoint::create_error_response(
                        format!("Failed to remove workspace: {e}"),
                    )),
                    ..Default::default()
                }),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::delete};
    use axum_test::TestServer;
    use database::kuzu::database::KuzuDatabase;
    use event_bus::EventBus;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;
    use workspace_manager::WorkspaceManager;

    fn create_test_workspace() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // Create repo with proper git structure
        let repo_path = temp_dir.path().join("repo1");
        fs::create_dir_all(repo_path.join(".git/refs/heads")).unwrap();
        fs::create_dir_all(repo_path.join(".git/objects/info")).unwrap();
        fs::create_dir_all(repo_path.join(".git/objects/pack")).unwrap();
        fs::write(repo_path.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(
            repo_path.join(".git/config"),
            "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n\tlogallrefupdates = true\n"
        ).unwrap();
        fs::write(
            repo_path.join(".git/description"),
            "Unnamed repository; edit this file 'description' to name the repository.\n",
        )
        .unwrap();
        fs::write(repo_path.join("test.rb"), "puts 'hello'").unwrap();

        temp_dir
    }

    async fn create_test_app() -> (TestServer, TempDir) {
        let temp_data_dir = TempDir::new().unwrap();
        let workspace_manager = Arc::new(
            WorkspaceManager::new_with_directory(temp_data_dir.path().to_path_buf()).unwrap(),
        );
        let event_bus = Arc::new(EventBus::new());
        let database = Arc::new(KuzuDatabase::new());
        let job_dispatcher = Arc::new(crate::queue::dispatch::JobDispatcher::new(
            workspace_manager.clone(),
            event_bus.clone(),
            database.clone(),
        ));
        let state = crate::AppState {
            workspace_manager,
            event_bus,
            job_dispatcher,
            database,
        };
        let app = Router::new()
            .route("/workspace/delete", delete(delete_handler))
            .with_state(state);
        (TestServer::new(app).unwrap(), temp_data_dir)
    }

    async fn create_test_app_with_workspace() -> (TestServer, TempDir, String) {
        let temp_workspace = create_test_workspace();
        let temp_data_dir = TempDir::new().unwrap();

        // Create workspace manager that will be shared between server and test
        let workspace_manager = Arc::new(
            WorkspaceManager::new_with_directory(temp_data_dir.path().to_path_buf()).unwrap(),
        );

        // Register workspace before creating the server
        let workspace_info = workspace_manager
            .register_workspace_folder(temp_workspace.path())
            .unwrap();

        let event_bus = Arc::new(EventBus::new());
        let database = Arc::new(KuzuDatabase::new());
        let job_dispatcher = Arc::new(crate::queue::dispatch::JobDispatcher::new(
            workspace_manager.clone(),
            event_bus.clone(),
            database.clone(),
        ));
        let state = crate::AppState {
            workspace_manager,
            event_bus,
            job_dispatcher,
            database,
        };
        let app = Router::new()
            .route("/workspace/delete", delete(delete_handler))
            .with_state(state);
        let server = TestServer::new(app).unwrap();

        (server, temp_data_dir, workspace_info.workspace_folder_path)
    }

    #[tokio::test]
    async fn test_workspace_delete_success() {
        let (server, _temp_data_dir, workspace_path) = create_test_app_with_workspace().await;

        let request_body = WorkspaceDeleteBodyRequest {
            workspace_folder_path: workspace_path.clone(),
        };

        let response = server.delete("/workspace/delete").json(&request_body).await;

        response.assert_status_ok();
        let body: WorkspaceDeleteResponses = response.json();
        assert!(body.workspace_folder_path.is_some());
        assert!(body.removed.is_some());

        assert_eq!(body.workspace_folder_path.unwrap(), workspace_path);
        assert!(body.removed.unwrap());
    }

    #[tokio::test]
    async fn test_workspace_delete_not_found() {
        let (server, _temp_dir) = create_test_app().await;

        let request_body = WorkspaceDeleteBodyRequest {
            workspace_folder_path: "/nonexistent/workspace".to_string(),
        };

        let response = server.delete("/workspace/delete").json(&request_body).await;

        response.assert_status(StatusCode::NOT_FOUND);
        let body: WorkspaceDeleteResponses = response.json();
        assert!(body.not_found.is_some());
        assert_eq!(body.not_found.unwrap().status, "workspace_not_found");
    }

    #[tokio::test]
    async fn test_workspace_delete_empty_path() {
        let (server, _temp_dir) = create_test_app().await;

        let request_body = WorkspaceDeleteBodyRequest {
            workspace_folder_path: "".to_string(),
        };

        let response = server.delete("/workspace/delete").json(&request_body).await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let body: WorkspaceDeleteResponses = response.json();
        assert!(body.bad_request.is_some());
        assert_eq!(body.bad_request.unwrap().status, "empty_workspace_path");
    }

    #[tokio::test]
    async fn test_workspace_delete_whitespace_path() {
        let (server, _temp_dir) = create_test_app().await;

        let request_body = WorkspaceDeleteBodyRequest {
            workspace_folder_path: "   ".to_string(),
        };

        let response = server.delete("/workspace/delete").json(&request_body).await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let body: WorkspaceDeleteResponses = response.json();
        assert!(body.bad_request.is_some());
        assert_eq!(body.bad_request.unwrap().status, "empty_workspace_path");
    }

    #[tokio::test]
    async fn test_workspace_delete_malformed_request() {
        let (server, _temp_dir) = create_test_app().await;

        let response = server
            .delete("/workspace/delete")
            .text("invalid json")
            .await;

        response.assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn test_workspace_delete_performance() {
        let (server, _temp_data_dir, workspace_path) = create_test_app_with_workspace().await;

        let request_body = WorkspaceDeleteBodyRequest {
            workspace_folder_path: workspace_path,
        };

        let start_time = std::time::Instant::now();
        let response = server.delete("/workspace/delete").json(&request_body).await;
        let duration = start_time.elapsed();

        response.assert_status_ok();
        assert!(
            duration.as_millis() < 1000,
            "Deletion took too long: {duration:?}"
        );

        let body: WorkspaceDeleteResponses = response.json();
        assert!(body.workspace_folder_path.is_some());
        assert!(body.removed.is_some());
        assert!(body.removed.unwrap());
    }

    #[tokio::test]
    async fn test_workspace_delete_twice() {
        let (server, _temp_data_dir, workspace_path) = create_test_app_with_workspace().await;

        let request_body = WorkspaceDeleteBodyRequest {
            workspace_folder_path: workspace_path.clone(),
        };

        // First deletion should succeed
        let response = server.delete("/workspace/delete").json(&request_body).await;
        response.assert_status_ok();

        // Second deletion should return not found
        let response = server.delete("/workspace/delete").json(&request_body).await;
        response.assert_status(StatusCode::NOT_FOUND);
        let body: WorkspaceDeleteResponses = response.json();
        assert!(body.not_found.is_some());
        assert_eq!(body.not_found.unwrap().status, "workspace_not_found");
    }
}
