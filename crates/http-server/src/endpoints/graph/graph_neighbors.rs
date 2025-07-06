use super::shared::{
    GraphRelationship, TypedGraphNode, create_error_response, create_typed_node, extract_node_data,
};
use crate::AppState;
use crate::contract::{EmptyRequest, EndpointConfigTypes};
use crate::decode_url_param;
use crate::define_endpoint;
use crate::endpoints::shared::StatusResponse;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use database::querying::{
    QueryLibrary, QueryResult, QueryResultRow, QueryingService, service::DatabaseQueryingService,
};
use event_bus::types::project_info::{TSProjectInfo, to_ts_project_info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use ts_rs::TS;
use urlencoding;

#[derive(Deserialize, Serialize, TS, Default, Clone, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphNeighborsPathRequest {
    pub workspace_folder_path: String,
    pub project_path: String,
    pub node_id: String,
}

#[derive(Deserialize, Serialize, TS, Default, Clone, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphNeighborsQueryRequest {
    pub limit: Option<i32>,
}

#[derive(Serialize, Deserialize, TS, Default, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphNeighborsSuccessResponse {
    pub nodes: Vec<TypedGraphNode>,
    pub relationships: Vec<GraphRelationship>,
    pub project_info: TSProjectInfo,
}

#[derive(Serialize, Deserialize, TS, Default, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphNeighborsResponses {
    #[serde(rename = "200")]
    pub ok: Option<GraphNeighborsSuccessResponse>,
    #[serde(rename = "404")]
    pub not_found: Option<StatusResponse>,
    #[serde(rename = "400")]
    pub bad_request: Option<StatusResponse>,
    #[serde(rename = "500")]
    pub internal_server_error: Option<StatusResponse>,
}

pub struct GraphNeighborsEndpointConfig;

impl EndpointConfigTypes for GraphNeighborsEndpointConfig {
    type PathRequest = GraphNeighborsPathRequest;
    type BodyRequest = EmptyRequest;
    type QueryRequest = GraphNeighborsQueryRequest;
    type Response = GraphNeighborsSuccessResponse;
}

define_endpoint! {
    GraphNeighborsEndpoint,
    GraphNeighborsEndpointDef,
    Get,
    "/graph/neighbors/{workspace_folder_path}/{project_path}/{node_id}",
    ts_path_type = "\"/api/graph/neighbors/{workspace_folder_path}/{project_path}/{node_id}\"",
    config = GraphNeighborsEndpointConfig,
    export_to = "../../../packages/gkg/src/api.ts"
}

impl GraphNeighborsEndpoint {
    pub fn create_success_response(
        nodes: Vec<TypedGraphNode>,
        relationships: Vec<GraphRelationship>,
        project_info: TSProjectInfo,
    ) -> GraphNeighborsSuccessResponse {
        GraphNeighborsSuccessResponse {
            nodes,
            relationships,
            project_info,
        }
    }

    pub fn create_error_response(status: String) -> StatusResponse {
        create_error_response(status)
    }
}

pub async fn graph_neighbors_handler(
    State(state): State<AppState>,
    Path(path_params): Path<GraphNeighborsPathRequest>,
    Query(query_params): Query<GraphNeighborsQueryRequest>,
) -> impl IntoResponse {
    let input_project_path = decode_url_param!(
        &path_params.project_path,
        "project_path",
        GraphNeighborsEndpoint::create_error_response
    );
    let input_workspace_folder_path = decode_url_param!(
        &path_params.workspace_folder_path,
        "workspace_folder_path",
        GraphNeighborsEndpoint::create_error_response
    );
    let input_node_id = decode_url_param!(
        &path_params.node_id,
        "node_id",
        GraphNeighborsEndpoint::create_error_response
    );

    let limit = query_params.limit.unwrap_or(100);

    if input_project_path.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(GraphNeighborsEndpoint::create_error_response(
                "empty_project_path".to_string(),
            )),
        )
            .into_response();
    }

    if input_node_id.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(GraphNeighborsEndpoint::create_error_response(
                "empty_node_id".to_string(),
            )),
        )
            .into_response();
    }

    let project_info = match state
        .workspace_manager
        .get_project_info(&input_workspace_folder_path, &input_project_path)
    {
        Some(info) => info,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(GraphNeighborsEndpoint::create_error_response(
                    "project_not_found".to_string(),
                )),
            )
                .into_response();
        }
    };

    let query = QueryLibrary::get_node_neighbors_query();

    let mut query_params = serde_json::Map::new();
    query_params.insert(
        "node_id".to_string(),
        serde_json::Value::String(input_node_id.clone()),
    );
    query_params.insert("limit".to_string(), serde_json::Value::Number(limit.into()));

    let query_service = DatabaseQueryingService::new(Arc::clone(&state.database));

    let mut query_result = match query_service.execute_query(
        project_info.database_path.clone(),
        query.query,
        query_params,
    ) {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Failed to execute neighbors query: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GraphNeighborsEndpoint::create_error_response(format!(
                    "Failed to execute graph query: {e}"
                ))),
            )
                .into_response();
        }
    };

    let graph_data = match convert_query_result_to_graph(&mut query_result) {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to convert query result to graph: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GraphNeighborsEndpoint::create_error_response(format!(
                    "Failed to process graph data: {e}"
                ))),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(GraphNeighborsEndpoint::create_success_response(
            graph_data.0,
            graph_data.1,
            to_ts_project_info(&project_info),
        )),
    )
        .into_response()
}

fn convert_query_result_to_graph(
    query_result: &mut Box<dyn QueryResult>,
) -> Result<(Vec<TypedGraphNode>, Vec<GraphRelationship>), Box<dyn std::error::Error>> {
    let mut nodes = Vec::new();
    let mut relationships = Vec::new();
    let mut node_ids = std::collections::HashSet::new();
    let mut relationship_ids = std::collections::HashSet::new();

    let mut all_rows = Vec::new();
    while let Some(row) = query_result.next() {
        all_rows.push(row);
    }

    for row in all_rows {
        process_neighbors_row(
            row,
            &mut nodes,
            &mut relationships,
            &mut node_ids,
            &mut relationship_ids,
        )?;
    }

    Ok((nodes, relationships))
}

fn process_neighbors_row(
    row: Box<dyn QueryResultRow>,
    nodes: &mut Vec<TypedGraphNode>,
    relationships: &mut Vec<GraphRelationship>,
    node_ids: &mut std::collections::HashSet<String>,
    relationship_ids: &mut std::collections::HashSet<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let source_data = extract_node_data(&*row, 0)?;
    let target_data = extract_node_data(&*row, 14)?;

    let relationship_type = row.get_string_value(28)?;
    let relationship_id = row.get_string_value(29)?;

    let source_id = source_data.id.clone();
    let target_id = target_data.id.clone();

    if node_ids.insert(source_id.clone()) {
        nodes.push(create_typed_node(source_data)?);
    }

    if node_ids.insert(target_id.clone()) {
        nodes.push(create_typed_node(target_data)?);
    }

    if relationship_ids.insert(relationship_id.clone()) {
        relationships.push(GraphRelationship {
            id: relationship_id,
            source: source_id,
            target: target_id,
            relationship_type,
            properties: HashMap::new(),
        });
    }

    Ok(())
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
        );

        let app = Router::new()
            .route(
                "/graph/neighbors/{workspace_folder_path}/{project_path}/{node_id}",
                get(graph_neighbors_handler),
            )
            .with_state(app_state.clone());

        (app, app_state, temp_dir)
    }

    #[tokio::test]
    async fn test_graph_neighbors_empty_project_path() {
        let (app, _app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let response = server
            .get("/graph/neighbors/workspace/%20/some_node_id")
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let body: StatusResponse = response.json();
        assert_eq!(body.status, "empty_project_path");
    }

    #[tokio::test]
    async fn test_graph_neighbors_empty_node_id() {
        let (app, _app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/graph/neighbors/workspace/project/%20").await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let body: StatusResponse = response.json();
        assert_eq!(body.status, "empty_node_id");
    }

    #[tokio::test]
    async fn test_graph_neighbors_malformed_request() {
        let (app, _app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/graph/neighbors/missing_project_path").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_graph_neighbors_with_real_indexed_data() {
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

        let query_service = DatabaseQueryingService::new(Arc::clone(&app_state.database));
        let query = QueryLibrary::get_initial_project_graph_query();
        let mut query_params = serde_json::Map::new();
        query_params.insert(
            "directory_limit".to_string(),
            serde_json::Value::Number(10.into()),
        );
        query_params.insert(
            "file_limit".to_string(),
            serde_json::Value::Number(10.into()),
        );
        query_params.insert(
            "definition_limit".to_string(),
            serde_json::Value::Number(10.into()),
        );

        let project_info = app_state
            .workspace_manager
            .get_project_info(workspace_folder_path, project_path)
            .expect("Should have project info");

        let mut initial_result = query_service
            .execute_query(
                project_info.database_path.clone(),
                query.query,
                query_params,
            )
            .expect("Should execute initial query");

        let first_node_id = if let Some(row) = initial_result.next() {
            row.get_string_value(0).expect("Should have node ID")
        } else {
            panic!("Should have at least one node");
        };

        let encoded_project_path = urlencoding::encode(project_path);
        let encoded_workspace_folder_path = urlencoding::encode(workspace_folder_path);
        let encoded_node_id = urlencoding::encode(&first_node_id);

        let url_string = format!(
            "/graph/neighbors/{encoded_workspace_folder_path}/{encoded_project_path}/{encoded_node_id}?limit=50"
        );

        let response = server.get(&url_string).await;

        response.assert_status(StatusCode::OK);
        let body = response.json::<GraphNeighborsSuccessResponse>();

        assert_eq!(body.project_info.project_path, *project_path);
        assert!(!body.nodes.is_empty(), "Should have at least one node");
    }

    async fn setup_test_environment() -> (Router, String, String, crate::AppState) {
        use crate::testing::{build_app_state, index_data};
        use std::path::PathBuf;
        use tempfile::TempDir;
        use testing::repository::TestRepository;

        let temp_dir = TempDir::new().unwrap();
        let workspace_folder = temp_dir.path().join("test_workspace");
        std::fs::create_dir_all(&workspace_folder).unwrap();

        let _repository =
            TestRepository::new(&workspace_folder.join("test-repo"), Some("test-repo"));
        let (app_state, _temp_dir) =
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
        );

        let app = Router::new()
            .route(
                "/graph/neighbors/{workspace_folder_path}/{project_path}/{node_id}",
                axum::routing::get(graph_neighbors_handler),
            )
            .with_state(app_state.clone());

        let workspaces = app_state.workspace_manager.list_workspace_folders();
        let workspace_folder_path = workspaces[0].workspace_folder_path.clone();
        let projects = app_state
            .workspace_manager
            .list_projects_in_workspace(&workspace_folder_path);
        let project_path = projects[0].project_path.clone();

        (app, workspace_folder_path, project_path, app_state)
    }

    #[tokio::test]
    async fn test_graph_neighbors_directory_node() {
        let (app, workspace_folder_path, project_path, _app_state) = setup_test_environment().await;
        let server = TestServer::new(app).unwrap();

        let test_cases = vec!["app", "lib"];

        for directory_name in test_cases {
            let encoded_workspace = urlencoding::encode(&workspace_folder_path);
            let encoded_project = urlencoding::encode(&project_path);
            let encoded_node_id = urlencoding::encode(directory_name);
            let uri = format!(
                "/graph/neighbors/{encoded_workspace}/{encoded_project}/{encoded_node_id}?limit=50"
            );

            let response = server.get(&uri).await;

            if response.status_code() == StatusCode::OK {
                let response_json = response.json::<GraphNeighborsSuccessResponse>();
                assert_eq!(response_json.project_info.project_path, project_path);
            }
        }
    }

    #[tokio::test]
    async fn test_graph_neighbors_file_node() {
        let (app, workspace_folder_path, project_path, _app_state) = setup_test_environment().await;
        let server = TestServer::new(app).unwrap();

        let test_cases = vec![
            "main.rb",
            "app/models/user_model.rb",
            "lib/authentication.rb",
        ];

        for file_path in test_cases {
            let encoded_workspace = urlencoding::encode(&workspace_folder_path);
            let encoded_project = urlencoding::encode(&project_path);
            let encoded_node_id = urlencoding::encode(file_path);
            let uri = format!(
                "/graph/neighbors/{encoded_workspace}/{encoded_project}/{encoded_node_id}?limit=50"
            );

            let response = server.get(&uri).await;

            if response.status_code() == StatusCode::OK {
                let response_json = response.json::<GraphNeighborsSuccessResponse>();
                assert_eq!(response_json.project_info.project_path, project_path);
            }
        }
    }

    #[tokio::test]
    async fn test_graph_neighbors_definition_node() {
        let (app, workspace_folder_path, project_path, _app_state) = setup_test_environment().await;
        let server = TestServer::new(app).unwrap();

        let test_cases = vec!["main.rb::Application", "main.rb::ApplicationUtils"];

        for definition_fqn in test_cases {
            let encoded_workspace = urlencoding::encode(&workspace_folder_path);
            let encoded_project = urlencoding::encode(&project_path);
            let encoded_node_id = urlencoding::encode(definition_fqn);
            let uri = format!(
                "/graph/neighbors/{encoded_workspace}/{encoded_project}/{encoded_node_id}?limit=50"
            );

            let response = server.get(&uri).await;

            if response.status_code() == StatusCode::OK {
                let response_json = response.json::<GraphNeighborsSuccessResponse>();
                assert_eq!(response_json.project_info.project_path, project_path);
            }
        }
    }

    #[tokio::test]
    async fn test_graph_neighbors_with_known_node() {
        use crate::testing::{build_app_state, index_data};
        use database::querying::{QueryLibrary, QueryingService, service::DatabaseQueryingService};
        use std::path::PathBuf;
        use std::sync::Arc;
        use tempfile::TempDir;
        use testing::repository::TestRepository;

        let temp_dir = TempDir::new().unwrap();
        let workspace_folder = temp_dir.path().join("test_workspace");
        std::fs::create_dir_all(&workspace_folder).unwrap();

        let _repository =
            TestRepository::new(&workspace_folder.join("test-repo"), Some("test-repo"));
        let (app_state, _temp_dir) =
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
        );

        let app = Router::new()
            .route(
                "/graph/neighbors/{workspace_folder_path}/{project_path}/{node_id}",
                axum::routing::get(graph_neighbors_handler),
            )
            .with_state(app_state.clone());

        let workspaces = app_state.workspace_manager.list_workspace_folders();
        let workspace_folder_path = &workspaces[0].workspace_folder_path;
        let projects = app_state
            .workspace_manager
            .list_projects_in_workspace(workspace_folder_path);
        let project_path = &projects[0].project_path;

        let query_service = DatabaseQueryingService::new(Arc::clone(&app_state.database));
        let query = QueryLibrary::get_initial_project_graph_query();
        let mut query_params = serde_json::Map::new();
        query_params.insert(
            "directory_limit".to_string(),
            serde_json::Value::Number(10.into()),
        );
        query_params.insert(
            "file_limit".to_string(),
            serde_json::Value::Number(10.into()),
        );
        query_params.insert(
            "definition_limit".to_string(),
            serde_json::Value::Number(10.into()),
        );

        let project_info = app_state
            .workspace_manager
            .get_project_info(workspace_folder_path, project_path)
            .expect("Should have project info");

        let mut initial_result = query_service
            .execute_query(
                project_info.database_path.clone(),
                query.query,
                query_params,
            )
            .expect("Should execute initial query");

        let first_node_id = if let Some(row) = initial_result.next() {
            row.get_string_value(0).expect("Should have node ID")
        } else {
            panic!("Should have at least one node");
        };

        let server = TestServer::new(app).unwrap();
        let encoded_workspace = urlencoding::encode(workspace_folder_path);
        let encoded_project = urlencoding::encode(project_path);
        let encoded_node_id = urlencoding::encode(&first_node_id);
        let uri = format!(
            "/graph/neighbors/{encoded_workspace}/{encoded_project}/{encoded_node_id}?limit=50"
        );

        let response = server.get(&uri).await;

        response.assert_status(StatusCode::OK);
        let response_json = response.json::<GraphNeighborsSuccessResponse>();
        assert_eq!(response_json.project_info.project_path, *project_path);
    }

    #[tokio::test]
    async fn test_graph_neighbors_error_cases() {
        let (app, workspace_folder_path, project_path, _app_state) = setup_test_environment().await;
        let server = TestServer::new(app).unwrap();

        let encoded_workspace = urlencoding::encode(&workspace_folder_path);
        let encoded_project = urlencoding::encode(&project_path);
        let encoded_node_id = "%20";
        let uri = format!(
            "/graph/neighbors/{encoded_workspace}/{encoded_project}/{encoded_node_id}?limit=50"
        );

        let response = server.get(&uri).await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }
}
