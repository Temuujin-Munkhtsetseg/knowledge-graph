use crate::AppState;
use crate::contract::{EmptyRequest, EndpointConfigTypes};
use crate::define_endpoint;
use crate::endpoints::shared::StatusResponse;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Sse, sse::Event};
use indexer::runner::run_client_indexer;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};
use ts_rs::TS;
use workspace_manager::WorkspaceManager;
use workspace_manager::manifest::Status;

#[derive(Deserialize, Serialize, TS, Default)]
#[ts(export, export_to = "api.ts")]
pub struct WorkspaceIndexBodyRequest {
    pub workspace: String,
}

#[derive(Serialize, TS, Default)]
#[ts(export, export_to = "api.ts")]
pub struct IndexingProgressResponse {
    pub message: String,
}

#[derive(Serialize, TS, Default)]
#[ts(export, export_to = "api.ts")]
pub struct WorkspaceIndexResponses {
    #[serde(rename = "200")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<IndexingProgressResponse>,
    #[serde(rename = "400")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bad_request: Option<StatusResponse>,
    #[serde(rename = "409")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict: Option<StatusResponse>,
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
    pub fn create_progress_response(message: String) -> IndexingProgressResponse {
        IndexingProgressResponse { message }
    }

    pub fn create_error_response(status: String) -> StatusResponse {
        StatusResponse { status }
    }
}

/// Handler for workspace indexing
/// Validates workspace, checks status, and starts indexing with SSE progress updates
pub async fn index_handler(
    State(state): State<AppState>,
    Json(payload): Json<WorkspaceIndexBodyRequest>,
) -> impl IntoResponse {
    let path = std::path::Path::new(&payload.workspace);
    if !path.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            Json(WorkspaceIndexResponses {
                bad_request: Some(WorkspaceIndexEndpoint::create_error_response(
                    "path_is_not_a_directory".to_string(),
                )),
                ..Default::default()
            }),
        )
            .into_response();
    }

    match state
        .workspace_manager
        .get_workspace_folder_info(&payload.workspace)
    {
        Some(info) => {
            if matches!(info.status, Status::Indexing) {
                return (
                    StatusCode::CONFLICT,
                    Json(WorkspaceIndexResponses {
                        conflict: Some(WorkspaceIndexEndpoint::create_error_response(format!(
                            "Workspace is already being processed. Current status: {}",
                            info.status
                        ))),
                        ..Default::default()
                    }),
                )
                    .into_response();
            }
        }
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(WorkspaceIndexResponses {
                    bad_request: Some(WorkspaceIndexEndpoint::create_error_response(
                        "workspace_not_loaded".to_string(),
                    )),
                    ..Default::default()
                }),
            )
                .into_response();
        }
    }

    let (tx, rx) = broadcast::channel(100);
    spawn_indexing_task(
        Arc::clone(&state.workspace_manager),
        payload.workspace,
        Some(tx),
    );

    let stream = BroadcastStream::new(rx)
        .map(|result: Result<String, BroadcastStreamRecvError>| {
            let progress = match result {
                Ok(p) => p,
                Err(e) => format!("Error receiving progress: {e}"),
            };
            Event::default()
                .json_data(WorkspaceIndexResponses {
                    ok: Some(WorkspaceIndexEndpoint::create_progress_response(progress)),
                    ..Default::default()
                })
                .unwrap()
        })
        .map(|e| Ok(e) as Result<Event, Infallible>);

    Sse::new(stream).into_response()
}

/// Spawns an indexing task for the given workspace
/// Optionally sends progress updates via the provided broadcast channel
pub fn spawn_indexing_task(
    workspace_manager: Arc<WorkspaceManager>,
    workspace_path: String,
    progress_tx: Option<broadcast::Sender<String>>,
) {
    tokio::spawn(async move {
        let workspace_path_buf = PathBuf::from(workspace_path);
        let progress_tx_clone = progress_tx.clone();

        let task_result = tokio::task::spawn_blocking(move || {
            run_client_indexer(workspace_manager, workspace_path_buf, 0, move |msg| {
                if let Some(tx) = &progress_tx_clone {
                    if tx.send(msg.to_string()).is_err() {
                        tracing::warn!("SSE client disconnected.");
                    }
                }
            })
        })
        .await;

        match task_result {
            Ok(Ok(())) => tracing::info!("Indexing completed successfully."),
            Ok(Err(e)) => {
                let error_msg = format!("Indexing failed: {e}");
                tracing::error!("{}", error_msg);
                if let Some(tx) = progress_tx {
                    let _ = tx.send(error_msg);
                }
            }
            Err(e) => {
                let error_msg = format!("Indexing task panicked: {e}");
                tracing::error!("{}", error_msg);
                if let Some(tx) = progress_tx {
                    let _ = tx.send(error_msg);
                }
            }
        }
    });
}
