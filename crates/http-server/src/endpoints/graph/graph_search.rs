use super::shared::{TypedGraphNode, create_error_response, create_typed_node, extract_node_data};
use crate::AppState;
use crate::contract::{EmptyRequest, EndpointConfigTypes};
use crate::decode_url_param;
use crate::define_endpoint;
use crate::endpoints::shared::StatusResponse;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use database::querying::{
    QueryLibrary, QueryResult, QueryingService, service::DatabaseQueryingService,
};
use event_bus::types::project_info::{TSProjectInfo, to_ts_project_info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use ts_rs::TS;
use urlencoding;

#[derive(Deserialize, Serialize, TS, Default, Clone, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphSearchPathRequest {
    pub workspace_folder_path: String,
    pub project_path: String,
}

#[derive(Deserialize, Serialize, TS, Default, Clone, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphSearchQueryRequest {
    pub search_term: String,
    pub limit: Option<i32>,
}

#[derive(Serialize, Deserialize, TS, Default, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphSearchSuccessResponse {
    pub nodes: Vec<TypedGraphNode>,
    pub project_info: TSProjectInfo,
}

#[derive(Serialize, Deserialize, TS, Default, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphSearchResponses {
    #[serde(rename = "200")]
    pub ok: Option<GraphSearchSuccessResponse>,
    #[serde(rename = "404")]
    pub not_found: Option<StatusResponse>,
    #[serde(rename = "400")]
    pub bad_request: Option<StatusResponse>,
    #[serde(rename = "500")]
    pub internal_server_error: Option<StatusResponse>,
}

pub struct GraphSearchEndpointConfig;

impl EndpointConfigTypes for GraphSearchEndpointConfig {
    type PathRequest = GraphSearchPathRequest;
    type BodyRequest = EmptyRequest;
    type QueryRequest = GraphSearchQueryRequest;
    type Response = GraphSearchSuccessResponse;
}

define_endpoint! {
    GraphSearchEndpoint,
    GraphSearchEndpointDef,
    Get,
    "/graph/search/{workspace_folder_path}/{project_path}",
    ts_path_type = "\"/api/graph/search/{workspace_folder_path}/{project_path}\"",
    config = GraphSearchEndpointConfig,
    export_to = "../../../packages/gkg/src/api.ts"
}

impl GraphSearchEndpoint {
    pub fn create_success_response(
        nodes: Vec<TypedGraphNode>,
        project_info: TSProjectInfo,
    ) -> GraphSearchSuccessResponse {
        GraphSearchSuccessResponse {
            nodes,
            project_info,
        }
    }

    pub fn create_error_response(status: String) -> StatusResponse {
        create_error_response(status)
    }
}

pub async fn graph_search_handler(
    State(state): State<AppState>,
    Path(path_params): Path<GraphSearchPathRequest>,
    Query(query_params): Query<GraphSearchQueryRequest>,
) -> impl IntoResponse {
    let input_project_path = decode_url_param!(
        &path_params.project_path,
        "project_path",
        GraphSearchEndpoint::create_error_response
    );
    let input_workspace_folder_path = decode_url_param!(
        &path_params.workspace_folder_path,
        "workspace_folder_path",
        GraphSearchEndpoint::create_error_response
    );

    let search_term = query_params.search_term.trim();
    let limit = query_params.limit.unwrap_or(100);

    if input_project_path.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(GraphSearchEndpoint::create_error_response(
                "empty_project_path".to_string(),
            )),
        )
            .into_response();
    }

    if search_term.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(GraphSearchEndpoint::create_error_response(
                "empty_search_term".to_string(),
            )),
        )
            .into_response();
    }

    info!(
        "Received search request {workspace_folder_path} {project_path} search_term=\"{search_term}\" limit={limit}",
        workspace_folder_path = input_workspace_folder_path,
        project_path = input_project_path,
        search_term = search_term,
        limit = limit
    );

    let project_info = match state
        .workspace_manager
        .get_project_info(&input_workspace_folder_path, &input_project_path)
    {
        Some(info) => info,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(GraphSearchEndpoint::create_error_response(
                    "project_not_found".to_string(),
                )),
            )
                .into_response();
        }
    };

    let query = QueryLibrary::get_search_nodes_query();

    let mut query_params = serde_json::Map::new();
    query_params.insert(
        "search_term".to_string(),
        serde_json::Value::String(search_term.to_string()),
    );
    query_params.insert("limit".to_string(), serde_json::Value::Number(limit.into()));

    let query_service = DatabaseQueryingService::new(Arc::clone(&state.database));

    info!(
        "Executing search query for project {} and workspace folder {}, search_term=\"{}\", limit={}",
        project_info.project_path, input_workspace_folder_path, search_term, limit
    );
    let mut query_result = match query_service.execute_query(
        project_info.database_path.clone(),
        query.query,
        query_params,
    ) {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to execute search query: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GraphSearchEndpoint::create_error_response(format!(
                    "Failed to execute search query: {e}"
                ))),
            )
                .into_response();
        }
    };

    let nodes = match convert_query_result_to_nodes(&mut query_result) {
        Ok(nodes) => nodes,
        Err(e) => {
            error!("Failed to convert query result to nodes: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GraphSearchEndpoint::create_error_response(format!(
                    "Failed to process search results: {e}"
                ))),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(GraphSearchEndpoint::create_success_response(
            nodes,
            to_ts_project_info(&project_info),
        )),
    )
        .into_response()
}

fn convert_query_result_to_nodes(
    query_result: &mut Box<dyn QueryResult>,
) -> Result<Vec<TypedGraphNode>, Box<dyn std::error::Error>> {
    let mut nodes = Vec::new();

    while let Some(row) = query_result.next() {
        let node_data = extract_node_data(&*row, 0)?;
        nodes.push(create_typed_node(node_data)?);
    }

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use crate::testing::{build_app_state, index_data};
    use testing::repository::TestRepository;

    use super::*;
    use axum::{Router, routing::get};
    use axum_test::TestServer;
    use std::path::PathBuf;
    use tempfile::TempDir;

    async fn create_test_app_with_indexed_data() -> (Router, AppState, TempDir) {
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
                "/graph/search/{workspace_folder_path}/{project_path}",
                get(graph_search_handler),
            )
            .with_state(app_state.clone());

        (app, app_state, temp_dir)
    }

    #[tokio::test]
    async fn test_graph_search_empty_project_path() {
        let (app, _app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let response = server
            .get("/graph/search/workspace/%20?search_term=test")
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let body: StatusResponse = response.json();
        assert_eq!(body.status, "empty_project_path");
    }

    #[tokio::test]
    async fn test_graph_search_empty_search_term() {
        let (app, _app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let response = server
            .get("/graph/search/workspace/project?search_term=")
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let body: StatusResponse = response.json();
        assert_eq!(body.status, "empty_search_term");
    }

    #[tokio::test]
    async fn test_graph_search_malformed_request() {
        let (app, _app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/graph/search/missing_project_path").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_graph_search_with_real_indexed_data() {
        let (app, app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let workspaces = app_state.workspace_manager.list_workspace_folders();
        assert!(!workspaces.is_empty(), "Should have at least one workspace");

        let workspace_info = &workspaces[0];
        let workspace_folder_path = &workspace_info.workspace_folder_path;

        let projects = app_state
            .workspace_manager
            .list_projects_in_workspace(workspace_folder_path);
        assert!(!projects.is_empty(), "Should have at least one project");

        let project_path = &projects[0].project_path;

        let encoded_project_path = urlencoding::encode(project_path);
        let encoded_workspace_folder_path = urlencoding::encode(workspace_folder_path);

        let url_string = format!(
            "/graph/search/{encoded_workspace_folder_path}/{encoded_project_path}?search_term=main&limit=50"
        );

        let response = server.get(&url_string).await;

        response.assert_status(StatusCode::OK);
        let body = response.json::<GraphSearchSuccessResponse>();

        assert_eq!(body.project_info.project_path, *project_path);

        for node in &body.nodes {
            match node {
                TypedGraphNode::DirectoryNode {
                    label, properties, ..
                } => {
                    assert!(
                        label.to_lowercase().contains("main")
                            || properties.path.to_lowercase().contains("main"),
                        "DirectoryNode should match search term"
                    );
                }
                TypedGraphNode::FileNode {
                    label, properties, ..
                } => {
                    assert!(
                        label.to_lowercase().contains("main")
                            || properties.path.to_lowercase().contains("main"),
                        "FileNode should match search term"
                    );
                }
                TypedGraphNode::DefinitionNode {
                    label, properties, ..
                } => {
                    assert!(
                        label.to_lowercase().contains("main")
                            || properties.fqn.to_lowercase().contains("main"),
                        "DefinitionNode should match search term"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_graph_search_case_insensitive() {
        let (app, app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let workspaces = app_state.workspace_manager.list_workspace_folders();
        let workspace_folder_path = &workspaces[0].workspace_folder_path;
        let projects = app_state
            .workspace_manager
            .list_projects_in_workspace(workspace_folder_path);
        let project_path = &projects[0].project_path;

        let encoded_project_path = urlencoding::encode(project_path);
        let encoded_workspace_folder_path = urlencoding::encode(workspace_folder_path);

        let test_cases = vec!["MAIN", "Main", "main", "MAIN.rb"];

        for search_term in test_cases {
            let url_string = format!(
                "/graph/search/{encoded_workspace_folder_path}/{encoded_project_path}?search_term={search_term}&limit=50"
            );

            let response = server.get(&url_string).await;

            if response.status_code() == StatusCode::OK {
                let body = response.json::<GraphSearchSuccessResponse>();
                assert_eq!(body.project_info.project_path, *project_path);
            }
        }
    }

    #[tokio::test]
    async fn test_graph_search_with_limit() {
        let (app, app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let workspaces = app_state.workspace_manager.list_workspace_folders();
        let workspace_folder_path = &workspaces[0].workspace_folder_path;
        let projects = app_state
            .workspace_manager
            .list_projects_in_workspace(workspace_folder_path);
        let project_path = &projects[0].project_path;

        let encoded_project_path = urlencoding::encode(project_path);
        let encoded_workspace_folder_path = urlencoding::encode(workspace_folder_path);

        let url_string = format!(
            "/graph/search/{encoded_workspace_folder_path}/{encoded_project_path}?search_term=main&limit=2"
        );

        let response = server.get(&url_string).await;

        if response.status_code() == StatusCode::OK {
            let body = response.json::<GraphSearchSuccessResponse>();
            assert!(body.nodes.len() <= 2, "Should respect limit parameter");
        }
    }
}
