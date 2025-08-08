use crate::AppState;
use crate::contract::{EmptyRequest, EndpointConfigTypes};
use crate::define_endpoint;
use crate::endpoints::shared::StatusResponse;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use database::kuzu::database::KuzuDatabase;
use event_bus::{
    EventBus,
    types::workspace_folder::{TSWorkspaceFolderInfo, to_ts_workspace_folder_info},
};
use indexer::execution::{config::IndexingConfigBuilder, executor::IndexingExecutor};
use num_cpus;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};
use ts_rs::TS;
use workspace_manager::WorkspaceFolderInfo;
use workspace_manager::WorkspaceManager;

#[derive(Deserialize, Serialize, TS, Default, Clone)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct WorkspaceIndexBodyRequest {
    pub workspace_folder_path: String,
}

#[derive(Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct WorkspaceIndexResponses {
    #[serde(rename = "200")]
    pub ok: TSWorkspaceFolderInfo,
    #[serde(rename = "400")]
    pub bad_request: StatusResponse,
    #[serde(rename = "500")]
    pub internal_server_error: StatusResponse,
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
    ts_path_type = "\"/api/workspace/index\"",
    config = WorkspaceIndexEndpointConfig,
    export_to = "../../../packages/gkg/src/api.ts"
}

impl WorkspaceIndexEndpoint {
    pub fn create_success_response(workspace_info: &WorkspaceFolderInfo) -> TSWorkspaceFolderInfo {
        to_ts_workspace_folder_info(workspace_info)
    }

    pub fn create_error_response(status: String) -> StatusResponse {
        StatusResponse { status }
    }
}

pub async fn index_handler(
    State(state): State<AppState>,
    Json(payload): Json<WorkspaceIndexBodyRequest>,
) -> impl IntoResponse {
    let workspace_folder_path = PathBuf::from(&payload.workspace_folder_path);

    if !workspace_folder_path.exists() {
        return (
            StatusCode::BAD_REQUEST,
            Json(WorkspaceIndexEndpoint::create_error_response(
                "invalid_workspace_path".to_string(),
            )),
        )
            .into_response();
    }

    let workspace_info = match state
        .workspace_manager
        .get_or_register_workspace_folder(&workspace_folder_path)
    {
        Ok(info) => info,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(WorkspaceIndexEndpoint::create_error_response(format!(
                    "Failed to get or register workspace: {e}"
                ))),
            )
                .into_response();
        }
    };

    if workspace_info.project_count == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(WorkspaceIndexEndpoint::create_error_response(
                "no_projects_found_in_workspace".to_string(),
            )),
        )
            .into_response();
    }

    // Dispatch indexing job to the job queue with high priority
    let job = crate::queue::job::Job::IndexWorkspaceFolder {
        workspace_folder_path: payload.workspace_folder_path.clone(),
        priority: crate::queue::job::JobPriority::High,
    };

    if let Err(e) = state.job_dispatcher.dispatch(job).await {
        error!("Failed to dispatch indexing job: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(WorkspaceIndexEndpoint::create_error_response(format!(
                "Failed to schedule indexing job: {e}"
            ))),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(WorkspaceIndexEndpoint::create_success_response(
            &workspace_info,
        )),
    )
        .into_response()
}

pub fn spawn_indexing_task(
    database: Arc<KuzuDatabase>,
    workspace_manager: Arc<WorkspaceManager>,
    event_bus: Arc<EventBus>,
    workspace_folder_path: String,
) {
    tokio::spawn(async move {
        let workspace_path_buf = PathBuf::from(workspace_folder_path.clone());
        let threads = num_cpus::get();
        let config = IndexingConfigBuilder::build(threads);
        let mut executor = IndexingExecutor::new(database, workspace_manager, event_bus, config);
        let result = tokio::task::spawn(async move {
            executor
                .execute_workspace_indexing(workspace_path_buf, None)
                .await
        })
        .await;

        match result {
            Ok(Ok(_stats)) => {
                info!("Workspace indexing succeeded for {}", workspace_folder_path)
            }
            Ok(Err(e)) => {
                error!(
                    "Indexing failed for workspace '{}': {}",
                    workspace_folder_path, e
                )
            }
            Err(e) => error!(
                "Indexing task panicked for workspace '{}': {}",
                workspace_folder_path, e
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
        let event_bus = Arc::new(EventBus::new());
        let database = Arc::new(KuzuDatabase::new());

        let job_dispatcher = Arc::new(crate::queue::dispatch::JobDispatcher::new(
            workspace_manager.clone(),
            event_bus.clone(),
            Arc::clone(&database),
        ));

        let state = crate::AppState {
            database: Arc::clone(&database),
            workspace_manager,
            event_bus,
            job_dispatcher,
        };
        let app = Router::new()
            .route("/workspace/index", post(index_handler))
            .with_state(state);
        (TestServer::new(app).unwrap(), temp_data_dir)
    }

    #[tokio::test]
    async fn test_workspace_index_invalid_path() {
        let (server, _temp_dir) = create_test_app().await;

        let request_body = WorkspaceIndexBodyRequest {
            workspace_folder_path: "/nonexistent/path".to_string(),
        };

        let response = server.post("/workspace/index").json(&request_body).await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let body: StatusResponse = response.json();
        assert_eq!(body.status, "invalid_workspace_path");
    }

    #[tokio::test]
    async fn test_workspace_index_empty_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let (server, _temp_data_dir) = create_test_app().await;

        let request_body = WorkspaceIndexBodyRequest {
            workspace_folder_path: temp_dir.path().to_string_lossy().to_string(),
        };

        let response = server.post("/workspace/index").json(&request_body).await;

        let status = response.status_code();
        if status == StatusCode::BAD_REQUEST {
            let body: StatusResponse = response.json();
            assert_eq!(body.status, "no_projects_found_in_workspace");
        } else {
            panic!("Expected BAD_REQUEST but got: {status}");
        }
    }

    #[tokio::test]
    async fn test_workspace_index_new_workspace_registration() {
        let temp_workspace = create_test_workspace();
        let (server, _temp_data_dir) = create_test_app().await;

        let request_body = WorkspaceIndexBodyRequest {
            workspace_folder_path: temp_workspace.path().to_string_lossy().to_string(),
        };

        let response = server.post("/workspace/index").json(&request_body).await;

        response.assert_status_ok();
        let body: TSWorkspaceFolderInfo = response.json();
        assert_eq!(body.project_count, 2);
        assert!(!body.workspace_folder_path.is_empty());
        assert!(!body.data_directory_name.is_empty());
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

        let workspace_folder_path = temp_workspace.path().to_string_lossy().to_string();
        let request_body = WorkspaceIndexBodyRequest {
            workspace_folder_path,
        };

        let start_time = std::time::Instant::now();
        let response = server.post("/workspace/index").json(&request_body).await;
        let duration = start_time.elapsed();

        response.assert_status_ok();
        assert!(
            duration.as_millis() < 1000,
            "Indexing took too long: {duration:?}"
        );

        let body: TSWorkspaceFolderInfo = response.json();
        assert_eq!(body.project_count, 2);
    }
}
