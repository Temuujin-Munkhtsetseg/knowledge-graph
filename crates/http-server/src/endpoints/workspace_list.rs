use crate::AppState;
use crate::contract::{EmptyRequest, EndpointConfigTypes};
use crate::define_endpoint;
use crate::endpoints::shared::StatusResponse;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use event_bus::types::{
    project_info::{TSProjectInfo, to_ts_project_info},
    workspace_folder::{TSWorkspaceFolderInfo, to_ts_workspace_folder_info},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct WorkspaceListResponses {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<WorkspaceListSuccessResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_server_error: Option<StatusResponse>,
}

#[derive(Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct WorkspaceListSuccessResponse {
    pub workspaces: Vec<WorkspaceWithProjects>,
}

#[derive(Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct WorkspaceWithProjects {
    pub workspace_info: TSWorkspaceFolderInfo,
    pub projects: Vec<TSProjectInfo>,
}

pub struct WorkspaceListEndpointConfig;

impl EndpointConfigTypes for WorkspaceListEndpointConfig {
    type PathRequest = EmptyRequest;
    type BodyRequest = EmptyRequest;
    type QueryRequest = EmptyRequest;
    type Response = WorkspaceListResponses;
}

define_endpoint! {
    WorkspaceListEndpoint,
    WorkspaceListEndpointDef,
    Get,
    "/workspace/list",
    ts_path_type = "\"/api/workspace/list\"",
    config = WorkspaceListEndpointConfig,
    export_to = "../../../packages/gkg/src/api.ts"
}

impl WorkspaceListEndpoint {
    pub fn create_success_response(
        workspaces: Vec<WorkspaceWithProjects>,
    ) -> WorkspaceListSuccessResponse {
        WorkspaceListSuccessResponse { workspaces }
    }

    pub fn create_error_response(status: String) -> StatusResponse {
        StatusResponse { status }
    }
}

/// Handler for the workspace list endpoint
/// Returns a list of all registered workspace folders in the system with their projects
pub async fn workspace_list_handler(State(state): State<AppState>) -> impl IntoResponse {
    let workspace_folders = state.workspace_manager.list_workspace_folders();

    let mut workspaces_with_projects = Vec::with_capacity(workspace_folders.len());

    for workspace_folder in workspace_folders {
        let workspace_info = to_ts_workspace_folder_info(&workspace_folder);

        let projects = state
            .workspace_manager
            .list_projects_in_workspace(&workspace_folder.workspace_folder_path);

        let ts_projects: Vec<TSProjectInfo> = projects.iter().map(to_ts_project_info).collect();

        workspaces_with_projects.push(WorkspaceWithProjects {
            workspace_info,
            projects: ts_projects,
        });
    }

    (
        StatusCode::OK,
        Json(WorkspaceListResponses {
            ok: Some(WorkspaceListEndpoint::create_success_response(
                workspaces_with_projects,
            )),
            ..Default::default()
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use axum_test::TestServer;
    use event_bus::EventBus;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;
    use workspace_manager::WorkspaceManager;

    fn create_test_workspace(temp_dir: &TempDir, name: &str) {
        let repo_path = temp_dir.path().join(name);
        fs::create_dir_all(repo_path.join(".git/refs/heads")).unwrap();
        fs::create_dir_all(repo_path.join(".git/objects/info")).unwrap();
        fs::create_dir_all(repo_path.join(".git/objects/pack")).unwrap();
        fs::write(repo_path.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(repo_path.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n\tlogallrefupdates = true\n").unwrap();
        fs::write(
            repo_path.join(".git/description"),
            "Unnamed repository; edit this file 'description' to name the repository.\n",
        )
        .unwrap();
        fs::write(repo_path.join("test.rb"), "puts 'hello'").unwrap();
    }

    async fn create_test_app() -> (TestServer, TempDir) {
        let temp_data_dir = TempDir::new().unwrap();
        let workspace_manager = Arc::new(
            WorkspaceManager::new_with_directory(temp_data_dir.path().to_path_buf()).unwrap(),
        );
        let event_bus = Arc::new(EventBus::new());
        let job_dispatcher = Arc::new(crate::queue::dispatch::JobDispatcher::new(
            workspace_manager.clone(),
            event_bus.clone(),
        ));
        let state = AppState {
            workspace_manager,
            event_bus,
            job_dispatcher,
        };
        let app = Router::new()
            .route("/workspace/list", get(workspace_list_handler))
            .with_state(state);
        (TestServer::new(app).unwrap(), temp_data_dir)
    }

    async fn create_test_app_with_workspaces() -> (TestServer, TempDir, Arc<WorkspaceManager>) {
        let temp_data_dir = TempDir::new().unwrap();
        let workspace_manager = Arc::new(
            WorkspaceManager::new_with_directory(temp_data_dir.path().to_path_buf()).unwrap(),
        );
        let event_bus = Arc::new(EventBus::new());

        // Create and register test workspaces
        let temp_workspace1 = TempDir::new().unwrap();
        let temp_workspace2 = TempDir::new().unwrap();

        create_test_workspace(&temp_workspace1, "repo1");
        create_test_workspace(&temp_workspace2, "repo2");

        // Register workspaces
        let _result1 = workspace_manager
            .register_workspace_folder(temp_workspace1.path())
            .unwrap();
        let _result2 = workspace_manager
            .register_workspace_folder(temp_workspace2.path())
            .unwrap();

        let job_dispatcher = Arc::new(crate::queue::dispatch::JobDispatcher::new(
            workspace_manager.clone(),
            event_bus.clone(),
        ));
        let state = AppState {
            workspace_manager: Arc::clone(&workspace_manager),
            event_bus,
            job_dispatcher,
        };
        let app = Router::new()
            .route("/workspace/list", get(workspace_list_handler))
            .with_state(state);
        (
            TestServer::new(app).unwrap(),
            temp_data_dir,
            workspace_manager,
        )
    }

    #[tokio::test]
    async fn test_workspace_list_empty() {
        let (server, _temp_dir) = create_test_app().await;

        let response = server.get("/workspace/list").await;

        response.assert_status_ok();
        let body: WorkspaceListResponses = response.json();
        assert!(body.ok.is_some());
        let success_response = body.ok.unwrap();
        assert_eq!(success_response.workspaces.len(), 0);
    }

    #[tokio::test]
    async fn test_workspace_list_with_workspaces() {
        let (server, _temp_data_dir, _workspace_manager) = create_test_app_with_workspaces().await;

        let response = server.get("/workspace/list").await;

        response.assert_status_ok();
        let body: WorkspaceListResponses = response.json();
        assert!(body.ok.is_some());
        let success_response = body.ok.unwrap();
        assert_eq!(success_response.workspaces.len(), 2);

        for workspace in &success_response.workspaces {
            assert!(!workspace.workspace_info.workspace_folder_path.is_empty());
            assert!(!workspace.workspace_info.data_directory_name.is_empty());
            assert_eq!(workspace.projects.len(), 1);
            assert!(!workspace.workspace_info.status.is_empty());

            for project in &workspace.projects {
                assert!(!project.project_path.is_empty());
                assert!(!project.workspace_folder_path.is_empty());
                assert!(!project.project_hash.is_empty());
                assert!(!project.status.is_empty());
                assert!(!project.database_path.is_empty());
                assert!(!project.parquet_directory.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_workspace_list_performance() {
        let (server, _temp_data_dir, _workspace_manager) = create_test_app_with_workspaces().await;

        let start_time = std::time::Instant::now();
        let response = server.get("/workspace/list").await;
        let duration = start_time.elapsed();

        response.assert_status_ok();
        assert!(
            duration.as_millis() < 5000,
            "Workspace list took too long: {duration:?}"
        );

        let body: WorkspaceListResponses = response.json();
        assert!(body.ok.is_some());
        let success_response = body.ok.unwrap();
        assert_eq!(success_response.workspaces.len(), 2);

        for workspace in &success_response.workspaces {
            assert_eq!(workspace.projects.len(), 1);
        }
    }
}
