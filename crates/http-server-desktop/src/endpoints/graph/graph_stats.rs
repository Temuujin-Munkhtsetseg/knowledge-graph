use super::shared::create_error_response;
use crate::contract::{EmptyRequest, EndpointConfigTypes};
use crate::define_endpoint;
use crate::{AppState, decode_url_param};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use database::kuzu::service::NodeDatabaseService;
use event_bus::types::project_info::TSProjectInfo;
use event_bus::types::project_info::to_ts_project_info;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use ts_rs::TS;

#[derive(Deserialize, Serialize, TS, Default, Clone, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphStatsPathRequest {
    pub workspace_folder_path: String,
    pub project_path: String,
}

#[derive(Serialize, Deserialize, TS, Default, Debug, Clone)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphNodeCountsResponse {
    pub directory_count: u32,
    pub file_count: u32,
    pub definition_count: u32,
    pub imported_symbol_count: u32,
}

#[derive(Serialize, Deserialize, TS, Default, Debug, Clone)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphRelationshipCountsResponse {
    pub directory_relationships: u32,
    pub file_relationships: u32,
    pub definition_relationships: u32,
}

#[derive(Serialize, Deserialize, TS, Default, Debug, Clone)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphStatsSuccessResponse {
    pub total_nodes: u32,
    pub total_relationships: u32,
    pub node_counts: GraphNodeCountsResponse,
    pub relationship_counts: GraphRelationshipCountsResponse,
    pub project_info: TSProjectInfo,
}

#[derive(Serialize, Deserialize, TS, Default, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphStatsResponses {
    #[serde(rename = "200")]
    pub ok: Option<GraphStatsSuccessResponse>,
    #[serde(rename = "404")]
    pub not_found: Option<crate::endpoints::shared::StatusResponse>,
    #[serde(rename = "400")]
    pub bad_request: Option<crate::endpoints::shared::StatusResponse>,
    #[serde(rename = "500")]
    pub internal_server_error: Option<crate::endpoints::shared::StatusResponse>,
}

pub struct GraphStatsEndpointConfig;

impl EndpointConfigTypes for GraphStatsEndpointConfig {
    type PathRequest = GraphStatsPathRequest;
    type BodyRequest = EmptyRequest;
    type QueryRequest = EmptyRequest;
    type Response = GraphStatsSuccessResponse;
}

define_endpoint! {
    GraphStatsEndpoint,
    GraphStatsEndpointDef,
    Get,
    "/graph/stats/{workspace_folder_path}/{project_path}",
    ts_path_type = "\"/api/graph/stats/{workspace_folder_path}/{project_path}\"",
    config = GraphStatsEndpointConfig,
    export_to = "../../../packages/gkg/src/api.ts"
}

impl GraphStatsEndpoint {
    pub fn create_success_response(
        total_nodes: u32,
        total_relationships: u32,
        node_counts: GraphNodeCountsResponse,
        relationship_counts: GraphRelationshipCountsResponse,
        project_info: TSProjectInfo,
    ) -> GraphStatsSuccessResponse {
        GraphStatsSuccessResponse {
            total_nodes,
            total_relationships,
            node_counts,
            relationship_counts,
            project_info,
        }
    }

    pub fn create_error_response(status: String) -> crate::endpoints::shared::StatusResponse {
        create_error_response(status)
    }
}

pub async fn graph_stats_handler(
    State(state): State<AppState>,
    Path(path_params): Path<GraphStatsPathRequest>,
) -> impl IntoResponse {
    let input_project_path = decode_url_param!(
        &path_params.project_path,
        "project_path",
        GraphStatsEndpoint::create_error_response
    );
    let input_workspace_folder_path = decode_url_param!(
        &path_params.workspace_folder_path,
        "workspace_folder_path",
        GraphStatsEndpoint::create_error_response
    );

    if input_project_path.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(GraphStatsEndpoint::create_error_response(
                "empty_project_path".to_string(),
            )),
        )
            .into_response();
    }

    info!(
        "Received graph stats request {workspace_folder_path} {project_path}",
        workspace_folder_path = input_workspace_folder_path,
        project_path = input_project_path,
    );

    let project_info = match state
        .workspace_manager
        .get_project_info(&input_workspace_folder_path, &input_project_path)
    {
        Some(info) => info,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(GraphStatsEndpoint::create_error_response(
                    "project_not_found".to_string(),
                )),
            )
                .into_response();
        }
    };

    let database = state
        .database
        .get_or_create_database(project_info.database_path.to_str().unwrap(), None);
    if database.is_none() {
        error!(
            "Failed to get database for project {} at {}",
            project_info.project_path,
            project_info.database_path.display()
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GraphStatsEndpoint::create_error_response(
                "database_not_found".to_string(),
            )),
        )
            .into_response();
    }

    let database = database.unwrap();
    let node_service = NodeDatabaseService::new(&database);

    let node_counts = match node_service.get_node_counts() {
        Ok(counts) => counts,
        Err(e) => {
            error!("Failed to get node counts: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GraphStatsEndpoint::create_error_response(format!(
                    "failed_to_get_node_counts: {e}"
                ))),
            )
                .into_response();
        }
    };

    let relationship_counts = match node_service.get_relationship_counts() {
        Ok(counts) => counts,
        Err(e) => {
            error!("Failed to get relationship counts: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GraphStatsEndpoint::create_error_response(format!(
                    "failed_to_get_relationship_counts: {e}"
                ))),
            )
                .into_response();
        }
    };

    let total_nodes = node_counts.directory_count
        + node_counts.file_count
        + node_counts.definition_count
        + node_counts.imported_symbol_count;

    let total_relationships = relationship_counts.directory_relationships
        + relationship_counts.file_relationships
        + relationship_counts.definition_relationships;

    (
        StatusCode::OK,
        Json(GraphStatsEndpoint::create_success_response(
            total_nodes,
            total_relationships,
            GraphNodeCountsResponse {
                directory_count: node_counts.directory_count,
                file_count: node_counts.file_count,
                definition_count: node_counts.definition_count,
                imported_symbol_count: node_counts.imported_symbol_count,
            },
            GraphRelationshipCountsResponse {
                directory_relationships: relationship_counts.directory_relationships,
                file_relationships: relationship_counts.file_relationships,
                definition_relationships: relationship_counts.definition_relationships,
            },
            to_ts_project_info(&project_info),
        )),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use axum_test::TestServer;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use testing::repository::TestRepository;

    async fn create_test_app_with_indexed_data() -> (Router, AppState, TempDir) {
        use crate::testing::{build_app_state, index_data};

        let temp_dir = TempDir::new().unwrap();
        let workspace_folder = temp_dir.path().join("test_workspace");
        std::fs::create_dir_all(&workspace_folder).unwrap();
        let _repository =
            TestRepository::new(&workspace_folder.join("test-repo"), Some("test-repo"));

        let (app_state, temp_dir) =
            build_app_state(temp_dir, vec![workspace_folder], None).unwrap();

        let workspace_folder_paths = app_state
            .workspace_manager
            .list_workspace_folders()
            .iter()
            .map(|w| w.workspace_folder_path.clone())
            .collect::<Vec<_>>();

        index_data(
            &app_state,
            workspace_folder_paths.iter().map(PathBuf::from).collect(),
        )
        .await;

        let app = Router::new()
            .route(
                "/graph/stats/{workspace_folder_path}/{project_path}",
                get(graph_stats_handler),
            )
            .with_state(app_state.clone());

        (app, app_state, temp_dir)
    }

    #[tokio::test]
    async fn test_graph_stats_empty_project_path() {
        use crate::endpoints::shared::StatusResponse;
        let (app, _app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/graph/stats/workspace/%20").await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let body: StatusResponse = response.json();
        assert_eq!(body.status, "empty_project_path");
    }

    #[tokio::test]
    async fn test_graph_stats_malformed_request() {
        let (app, _app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/graph/stats/missing_parts").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_graph_stats_with_real_indexed_data() {
        let (app, app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let workspaces = app_state.workspace_manager.list_workspace_folders();
        assert!(!workspaces.is_empty());
        let workspace_folder_path = &workspaces[0].workspace_folder_path;
        let projects = app_state
            .workspace_manager
            .list_projects_in_workspace(workspace_folder_path);
        assert!(!projects.is_empty());
        let project_path = &projects[0].project_path;

        let encoded_project_path = urlencoding::encode(project_path);
        let encoded_workspace_folder_path = urlencoding::encode(workspace_folder_path);
        let url_string =
            format!("/graph/stats/{encoded_workspace_folder_path}/{encoded_project_path}");

        let response = server.get(&url_string).await;
        response.assert_status(StatusCode::OK);
        let body = response.json::<GraphStatsSuccessResponse>();

        assert_eq!(body.project_info.project_path, *project_path);
        // Sanity checks
        assert!(body.total_nodes as i64 >= 0);
        assert!(body.total_relationships as i64 >= 0);
        assert_eq!(
            body.total_nodes,
            body.node_counts.directory_count
                + body.node_counts.file_count
                + body.node_counts.definition_count
                + body.node_counts.imported_symbol_count
        );
        assert_eq!(
            body.total_relationships,
            body.relationship_counts.directory_relationships
                + body.relationship_counts.file_relationships
                + body.relationship_counts.definition_relationships
        );
    }
}
