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
use database::querying::mappers::RELATIONSHIP_TYPE_MAPPER;
use database::querying::{
    QueryLibrary, QueryResult, QueryResultRow, QueryingService, service::DatabaseQueryingService,
};
use event_bus::types::project_info::{TSProjectInfo, to_ts_project_info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use ts_rs::TS;
use urlencoding;

#[derive(Deserialize, Serialize, TS, Default, Clone, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphInitialPathRequest {
    pub workspace_folder_path: String,
    pub project_path: String,
}

#[derive(Deserialize, Serialize, TS, Default, Clone, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphInitialQueryRequest {
    pub directory_limit: Option<i32>,
    pub file_limit: Option<i32>,
    pub definition_limit: Option<i32>,
    pub imported_symbol_limit: Option<i32>,
}

#[derive(Serialize, Deserialize, TS, Default, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphInitialSuccessResponse {
    pub nodes: Vec<TypedGraphNode>,
    pub relationships: Vec<GraphRelationship>,
    pub project_info: TSProjectInfo,
}

#[derive(Serialize, Deserialize, TS, Default, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphInitialResponses {
    #[serde(rename = "200")]
    pub ok: Option<GraphInitialSuccessResponse>,
    #[serde(rename = "404")]
    pub not_found: Option<StatusResponse>,
    #[serde(rename = "400")]
    pub bad_request: Option<StatusResponse>,
    #[serde(rename = "500")]
    pub internal_server_error: Option<StatusResponse>,
}

pub struct GraphInitialEndpointConfig;

impl EndpointConfigTypes for GraphInitialEndpointConfig {
    type PathRequest = GraphInitialPathRequest;
    type BodyRequest = EmptyRequest;
    type QueryRequest = GraphInitialQueryRequest;
    type Response = GraphInitialSuccessResponse;
}

define_endpoint! {
    GraphInitialEndpoint,
    GraphInitialEndpointDef,
    Get,
    "/graph/initial/{workspace_folder_path}/{project_path}",
    ts_path_type = "\"/api/graph/initial/{workspace_folder_path}/{project_path}\"",
    config = GraphInitialEndpointConfig,
    export_to = "../../../packages/gkg/src/api.ts"
}

impl GraphInitialEndpoint {
    pub fn create_success_response(
        nodes: Vec<TypedGraphNode>,
        relationships: Vec<GraphRelationship>,
        project_info: TSProjectInfo,
    ) -> GraphInitialSuccessResponse {
        GraphInitialSuccessResponse {
            nodes,
            relationships,
            project_info,
        }
    }

    pub fn create_error_response(status: String) -> StatusResponse {
        create_error_response(status)
    }
}

/// Handler for the graph initial endpoint
/// Fetches the initial graph structure for a project including top-level directories, files, and definitions
pub async fn graph_initial_handler(
    State(state): State<AppState>,
    Path(path_params): Path<GraphInitialPathRequest>,
    Query(query_params): Query<GraphInitialQueryRequest>,
) -> impl IntoResponse {
    let input_project_path = decode_url_param!(
        &path_params.project_path,
        "project_path",
        GraphInitialEndpoint::create_error_response
    );
    let input_workspace_folder_path = decode_url_param!(
        &path_params.workspace_folder_path,
        "workspace_folder_path",
        GraphInitialEndpoint::create_error_response
    );

    let directory_limit = query_params.directory_limit.unwrap_or(100);
    let file_limit = query_params.file_limit.unwrap_or(200);
    let definition_limit = query_params.definition_limit.unwrap_or(500);
    let imported_symbol_limit = query_params.imported_symbol_limit.unwrap_or(50);

    if input_project_path.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(GraphInitialEndpoint::create_error_response(
                "empty_project_path".to_string(),
            )),
        )
            .into_response();
    }

    info!(
        "Received request {workspace_folder_path} {project_path} {query_params:?}",
        workspace_folder_path = input_workspace_folder_path,
        project_path = input_project_path,
        query_params = query_params
    );

    let project_info = match state
        .workspace_manager
        .get_project_info(&input_workspace_folder_path, &input_project_path)
    {
        Some(info) => info,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(GraphInitialEndpoint::create_error_response(
                    "project_not_found".to_string(),
                )),
            )
                .into_response();
        }
    };

    let query = QueryLibrary::get_initial_project_graph_query();

    let mut query_params = serde_json::Map::new();
    query_params.insert(
        "directory_limit".to_string(),
        serde_json::Value::Number(directory_limit.into()),
    );
    query_params.insert(
        "file_limit".to_string(),
        serde_json::Value::Number(file_limit.into()),
    );
    query_params.insert(
        "definition_limit".to_string(),
        serde_json::Value::Number(definition_limit.into()),
    );
    query_params.insert(
        "imported_symbol_limit".to_string(),
        serde_json::Value::Number(imported_symbol_limit.into()),
    );

    let query_service = DatabaseQueryingService::new(Arc::clone(&state.database));

    info!(
        "Executing initial graph query for project {} and workspace folder {}, query params: {:?}",
        project_info.project_path, input_workspace_folder_path, query_params
    );
    let mut query_result = match query_service.execute_query(
        project_info.database_path.clone(),
        query.query,
        query_params,
    ) {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to execute initial graph query: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GraphInitialEndpoint::create_error_response(format!(
                    "Failed to execute graph query: {e}"
                ))),
            )
                .into_response();
        }
    };

    let graph_data = match convert_query_result_to_graph(&mut query_result) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to convert query result to graph: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GraphInitialEndpoint::create_error_response(format!(
                    "Failed to process graph data: {e}"
                ))),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(GraphInitialEndpoint::create_success_response(
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
        let order_priority = row.get_int_value(37)?;
        all_rows.push((row, order_priority));
    }

    all_rows.sort_by_key(|(_, priority)| *priority);

    for (row, _) in all_rows {
        process_graph_row(
            row,
            &mut nodes,
            &mut relationships,
            &mut node_ids,
            &mut relationship_ids,
        )?;
    }

    Ok((nodes, relationships))
}

fn process_graph_row(
    row: Box<dyn QueryResultRow>,
    nodes: &mut Vec<TypedGraphNode>,
    relationships: &mut Vec<GraphRelationship>,
    node_ids: &mut std::collections::HashSet<String>,
    relationship_ids: &mut std::collections::HashSet<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let source_data = extract_node_data(&*row, 0)?;
    let target_data = extract_node_data(&*row, 17)?;

    let relationship_name = row.get_string_value(34)?;
    let relationship_id = row.get_string_value(35)?;
    let relationship_type = RELATIONSHIP_TYPE_MAPPER(&*row, 36)?;

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
            relationship_name,
            relationship_type: relationship_type.to_string(),
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
        )
        .await;

        let app = Router::new()
            .route(
                "/graph/initial/{workspace_folder_path}/{project_path}",
                get(graph_initial_handler),
            )
            .with_state(app_state.clone());

        (app, app_state, temp_dir)
    }

    #[tokio::test]
    async fn test_graph_initial_empty_project_path() {
        let (app, _app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/graph/initial/placeholder_workspace/%20").await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let body: StatusResponse = response.json();
        assert_eq!(body.status, "empty_project_path");
    }

    #[tokio::test]
    async fn test_graph_initial_malformed_request() {
        let (app, _app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/graph/initial/missing_project_path").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_graph_initial_with_real_indexed_data() {
        let (app, app_state, _temp_dir) = create_test_app_with_indexed_data().await;
        let server = TestServer::new(app).unwrap();

        // Get the actual workspace and project paths that were registered with the WorkspaceManager
        // These are already canonicalized internally by the WorkspaceManager
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
            "/graph/initial/{encoded_workspace_folder_path}/{encoded_project_path}?directory_limit=100&file_limit=200&definition_limit=500&imported_symbol_limit=50"
        );

        let response = server.get(&url_string).await;

        let body = response.json::<GraphInitialSuccessResponse>();

        assert_eq!(body.project_info.project_path, *project_path);

        let directory_nodes: Vec<_> = body
            .nodes
            .iter()
            .filter(|n| matches!(n, TypedGraphNode::DirectoryNode { .. }))
            .collect();
        assert!(
            !directory_nodes.is_empty(),
            "Should have at least one directory node"
        );
        let file_nodes: Vec<_> = body
            .nodes
            .iter()
            .filter(|n| matches!(n, TypedGraphNode::FileNode { .. }))
            .collect();
        assert!(!file_nodes.is_empty(), "Should have at least one file node");

        let definition_nodes: Vec<_> = body
            .nodes
            .iter()
            .filter(|n| matches!(n, TypedGraphNode::DefinitionNode { .. }))
            .collect();
        assert!(
            !definition_nodes.is_empty(),
            "Should have at least one definition node"
        );

        if let Some(TypedGraphNode::DirectoryNode { properties, .. }) = directory_nodes.first() {
            assert!(
                !properties.path.is_empty(),
                "DirectoryNode path should not be empty"
            );
            assert!(
                !properties.absolute_path.is_empty(),
                "DirectoryNode absolute_path should not be empty"
            );
            assert!(
                !properties.repository_name.is_empty(),
                "DirectoryNode repository_name should not be empty"
            );
        }

        if let Some(TypedGraphNode::FileNode { properties, .. }) = file_nodes.first() {
            assert!(
                !properties.path.is_empty(),
                "FileNode path should not be empty"
            );
            assert!(
                !properties.absolute_path.is_empty(),
                "FileNode absolute_path should not be empty"
            );
            assert!(
                !properties.repository_name.is_empty(),
                "FileNode repository_name should not be empty"
            );
            assert!(
                !properties.language.is_empty(),
                "FileNode language should not be empty"
            );
            assert!(
                !properties.extension.is_empty(),
                "FileNode extension should not be empty"
            );
        }

        if let Some(TypedGraphNode::DefinitionNode { properties, .. }) = definition_nodes.first() {
            assert!(
                !properties.path.is_empty(),
                "DefinitionNode path should not be empty"
            );
            assert!(
                !properties.fqn.is_empty(),
                "DefinitionNode fqn should not be empty"
            );
            assert!(
                !properties.definition_type.is_empty(),
                "DefinitionNode definition_type should not be empty"
            );
            assert!(
                properties.start_line > 0,
                "DefinitionNode start_line should be positive"
            );
            assert!(
                properties.primary_start_byte >= 0,
                "DefinitionNode primary_start_byte should be non-negative"
            );
            assert!(
                properties.primary_end_byte >= properties.primary_start_byte,
                "DefinitionNode primary_end_byte should be >= start_byte"
            );
            assert!(
                properties.total_locations > 0,
                "DefinitionNode total_locations should be positive"
            );
        }

        // Validate relationships between different node types
        let directory_node_ids: std::collections::HashSet<String> = body
            .nodes
            .iter()
            .filter_map(|n| match n {
                TypedGraphNode::DirectoryNode { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect();

        let file_node_ids: std::collections::HashSet<String> = body
            .nodes
            .iter()
            .filter_map(|n| match n {
                TypedGraphNode::FileNode { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect();

        let definition_node_ids: std::collections::HashSet<String> = body
            .nodes
            .iter()
            .filter_map(|n| match n {
                TypedGraphNode::DefinitionNode { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect();

        let dir_to_dir_relationships: Vec<_> = body
            .relationships
            .iter()
            .filter(|r| {
                directory_node_ids.contains(&r.source) && directory_node_ids.contains(&r.target)
            })
            .collect();

        assert!(
            !dir_to_dir_relationships.is_empty(),
            "Should have at least one Directory to Directory relationship"
        );

        let dir_to_file_relationships: Vec<_> = body
            .relationships
            .iter()
            .filter(|r| directory_node_ids.contains(&r.source) && file_node_ids.contains(&r.target))
            .collect();

        assert!(
            !dir_to_file_relationships.is_empty(),
            "Should have at least one Directory to File relationship"
        );

        let file_to_def_relationships: Vec<_> = body
            .relationships
            .iter()
            .filter(|r| {
                file_node_ids.contains(&r.source) && definition_node_ids.contains(&r.target)
            })
            .collect();

        assert!(
            !file_to_def_relationships.is_empty(),
            "Should have at least one File to Definition relationship"
        );

        let def_to_def_relationships: Vec<_> = body
            .relationships
            .iter()
            .filter(|r| {
                definition_node_ids.contains(&r.source) && definition_node_ids.contains(&r.target)
            })
            .collect();

        assert!(
            !def_to_def_relationships.is_empty(),
            "Should have at least one Definition to Definition relationship"
        );
    }
}
