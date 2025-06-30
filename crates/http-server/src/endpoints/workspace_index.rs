use crate::AppState;
use crate::contract::{EmptyRequest, EndpointConfigTypes};
use crate::define_endpoint;
use crate::endpoints::shared::StatusResponse;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use indexer::runner::run_client_indexer;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use ts_rs::TS;
use workspace_manager::WorkspaceFolderInfo;
use workspace_manager::WorkspaceManager;

#[derive(Deserialize, Serialize, TS, Default, Clone)]
#[ts(export, export_to = "api.ts")]
pub struct WorkspaceIndexBodyRequest {
    pub workspace: String,
}

#[derive(Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "api.ts")]
pub struct WorkspaceIndexSuccessResponse {
    pub workspace_folder_path: String,
    pub data_directory_name: String,
    pub status: String,
    pub last_indexed_at: Option<String>,
    pub project_count: usize,
}

#[derive(Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "api.ts")]
pub struct WorkspaceIndexResponses {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<WorkspaceIndexSuccessResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bad_request: Option<StatusResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_server_error: Option<StatusResponse>,
}

pub struct WorkspaceIndexEndpointConfig;

impl EndpointConfigTypes for WorkspaceIndexEndpointConfig {
    type PathRequest = EmptyRequest;
    type BodyRequest = WorkspaceIndexBodyRequest;
    type QueryRequest = EmptyRequest;
    type Response = WorkspaceIndexResponses;
}

define_endpoint! {
    WorkspaceIndexEndpoint,
    WorkspaceIndexEndpointDef,
    Post,
    "/workspace/index",
    ts_path_type = "\"/workspace/index\"",
    config = WorkspaceIndexEndpointConfig,
    export_to = "api.ts"
}

impl WorkspaceIndexEndpoint {
    pub fn create_success_response(
        workspace_info: &WorkspaceFolderInfo,
    ) -> WorkspaceIndexSuccessResponse {
        WorkspaceIndexSuccessResponse {
            workspace_folder_path: workspace_info.workspace_folder_path.clone(),
            data_directory_name: workspace_info.data_directory_name.clone(),
            status: workspace_info.status.to_string(),
            last_indexed_at: workspace_info.last_indexed_at.map(|dt| dt.to_rfc3339()),
            project_count: workspace_info.project_count,
        }
    }

    pub fn create_error_response(status: String) -> StatusResponse {
        StatusResponse { status }
    }
}

pub async fn index_handler(
    State(state): State<AppState>,
    Json(payload): Json<WorkspaceIndexBodyRequest>,
) -> impl IntoResponse {
    let workspace_path = PathBuf::from(&payload.workspace);

    if !workspace_path.exists() {
        return (
            StatusCode::BAD_REQUEST,
            Json(WorkspaceIndexResponses {
                bad_request: Some(WorkspaceIndexEndpoint::create_error_response(
                    "invalid_workspace_path".to_string(),
                )),
                ..Default::default()
            }),
        )
            .into_response();
    }

    let workspace_info = match state
        .workspace_manager
        .get_workspace_folder_info(&payload.workspace)
    {
        Some(info) => info,
        None => {
            match state
                .workspace_manager
                .register_workspace_folder(&workspace_path)
            {
                Ok(info) => info,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(WorkspaceIndexResponses {
                            internal_server_error: Some(
                                WorkspaceIndexEndpoint::create_error_response(format!(
                                    "Failed to register workspace: {e}"
                                )),
                            ),
                            ..Default::default()
                        }),
                    )
                        .into_response();
                }
            }
        }
    };

    if workspace_info.project_count == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(WorkspaceIndexResponses {
                bad_request: Some(WorkspaceIndexEndpoint::create_error_response(
                    "no_projects_found_in_workspace".to_string(),
                )),
                ..Default::default()
            }),
        )
            .into_response();
    }

    spawn_indexing_task(Arc::clone(&state.workspace_manager), payload.workspace);

    (
        StatusCode::OK,
        Json(WorkspaceIndexResponses {
            ok: Some(WorkspaceIndexEndpoint::create_success_response(
                &workspace_info,
            )),
            ..Default::default()
        }),
    )
        .into_response()
}

pub fn spawn_indexing_task(workspace_manager: Arc<WorkspaceManager>, workspace_path: String) {
    tokio::spawn(async move {
        let workspace_path_buf = PathBuf::from(workspace_path.clone());
        let result = tokio::task::spawn_blocking(move || {
            run_client_indexer(workspace_manager, workspace_path_buf, 0, |msg| {
                tracing::info!("Indexing progress: {}", msg);
            })
        })
        .await;

        match result {
            Ok(Ok(())) => tracing::info!(
                "Indexing completed successfully for workspace '{}'",
                workspace_path
            ),
            Ok(Err(e)) => {
                tracing::error!("Indexing failed for workspace '{}': {}", workspace_path, e)
            }
            Err(e) => tracing::error!(
                "Indexing task panicked for workspace '{}': {}",
                workspace_path,
                e
            ),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::{Router, routing::post};
    use axum_test::TestServer;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_workspace() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // Create repo1 with proper git structure
        let repo1_path = temp_dir.path().join("repo1");
        fs::create_dir_all(repo1_path.join(".git/refs/heads")).unwrap();
        fs::create_dir_all(repo1_path.join(".git/objects/info")).unwrap();
        fs::create_dir_all(repo1_path.join(".git/objects/pack")).unwrap();
        fs::write(repo1_path.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(repo1_path.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n\tlogallrefupdates = true\n").unwrap();
        fs::write(
            repo1_path.join(".git/description"),
            "Unnamed repository; edit this file 'description' to name the repository.\n",
        )
        .unwrap();
        fs::write(repo1_path.join("test.rb"), "puts 'hello'").unwrap();

        // Create repo2 with proper git structure
        let repo2_path = temp_dir.path().join("repo2");
        fs::create_dir_all(repo2_path.join(".git/refs/heads")).unwrap();
        fs::create_dir_all(repo2_path.join(".git/objects/info")).unwrap();
        fs::create_dir_all(repo2_path.join(".git/objects/pack")).unwrap();
        fs::write(repo2_path.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(repo2_path.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n\tlogallrefupdates = true\n").unwrap();
        fs::write(
            repo2_path.join(".git/description"),
            "Unnamed repository; edit this file 'description' to name the repository.\n",
        )
        .unwrap();
        fs::write(repo2_path.join("main.rb"), "class Test; end").unwrap();

        temp_dir
    }

    async fn create_test_app() -> (TestServer, TempDir) {
        let temp_data_dir = TempDir::new().unwrap();
        let workspace_manager = Arc::new(
            WorkspaceManager::new_with_directory(temp_data_dir.path().to_path_buf()).unwrap(),
        );
        let state = crate::AppState { workspace_manager };
        let app = Router::new()
            .route("/workspace/index", post(index_handler))
            .with_state(state);
        (TestServer::new(app).unwrap(), temp_data_dir)
    }

    #[tokio::test]
    async fn test_workspace_index_invalid_path() {
        let (server, _temp_dir) = create_test_app().await;

        let request_body = WorkspaceIndexBodyRequest {
            workspace: "/nonexistent/path".to_string(),
        };

        let response = server.post("/workspace/index").json(&request_body).await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let body: WorkspaceIndexResponses = response.json();
        assert!(body.bad_request.is_some());
        assert_eq!(body.bad_request.unwrap().status, "invalid_workspace_path");
    }

    #[tokio::test]
    async fn test_workspace_index_empty_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let (server, _temp_data_dir) = create_test_app().await;

        let request_body = WorkspaceIndexBodyRequest {
            workspace: temp_dir.path().to_string_lossy().to_string(),
        };

        let response = server.post("/workspace/index").json(&request_body).await;

        let status = response.status_code();
        if status == StatusCode::BAD_REQUEST {
            let body: WorkspaceIndexResponses = response.json();
            assert!(body.bad_request.is_some());
            assert_eq!(
                body.bad_request.unwrap().status,
                "no_projects_found_in_workspace"
            );
        } else {
            panic!("Expected BAD_REQUEST but got: {status}");
        }
    }

    #[tokio::test]
    async fn test_workspace_index_new_workspace_registration() {
        let temp_workspace = create_test_workspace();
        let (server, _temp_data_dir) = create_test_app().await;

        let request_body = WorkspaceIndexBodyRequest {
            workspace: temp_workspace.path().to_string_lossy().to_string(),
        };

        let response = server.post("/workspace/index").json(&request_body).await;

        response.assert_status_ok();
        let body: WorkspaceIndexResponses = response.json();
        assert!(body.ok.is_some());

        let workspace_info = body.ok.unwrap();
        assert_eq!(workspace_info.project_count, 2);
        assert!(!workspace_info.workspace_folder_path.is_empty());
        assert!(!workspace_info.data_directory_name.is_empty());
    }

    #[tokio::test]
    async fn test_workspace_index_malformed_request() {
        let (server, _temp_dir) = create_test_app().await;

        let response = server.post("/workspace/index").text("invalid json").await;

        response.assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn test_workspace_index_performance() {
        let temp_workspace = create_test_workspace();
        let (server, _temp_data_dir) = create_test_app().await;

        let workspace_path = temp_workspace.path().to_string_lossy().to_string();
        let request_body = WorkspaceIndexBodyRequest {
            workspace: workspace_path,
        };

        let start_time = std::time::Instant::now();
        let response = server.post("/workspace/index").json(&request_body).await;
        let duration = start_time.elapsed();

        response.assert_status_ok();
        assert!(
            duration.as_millis() < 1000,
            "Indexing took too long: {duration:?}"
        );

        let body: WorkspaceIndexResponses = response.json();
        assert!(body.ok.is_some());
        assert_eq!(body.ok.unwrap().project_count, 2);
    }
}
