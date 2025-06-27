use crate::api::{
    IndexRequest, IndexingProgressResponse, ServerInfoResponse, StatusResponse,
    WorkspacePathRequest, WorkspaceResponse,
};
use anyhow::Result;
use axum::{
    http::StatusCode,
    response::{sse::Event, IntoResponse, Json, Sse},
    routing::{get, post},
    Router,
};
use http::HeaderValue;
use indexer::runner::run_client_indexer;
use once_cell::sync::Lazy;
use std::convert::Infallible;
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use tokio::sync::broadcast;
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;
use workspace_manager::manifest::Status;
use workspace_manager::WorkspaceManager;

mod api;

pub static WORKSPACE_MANAGER: Lazy<WorkspaceManager> =
    Lazy::new(|| WorkspaceManager::new_system_default().unwrap());

async fn root_handler(port: u16) -> Json<ServerInfoResponse> {
    Json(ServerInfoResponse { port })
}

async fn load_workspace_handler(Json(payload): Json<WorkspacePathRequest>) -> impl IntoResponse {
    let path = std::path::Path::new(&payload.workspace);
    if !path.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            Json(StatusResponse {
                status: "path_is_not_a_directory".to_string(),
            }),
        )
            .into_response();
    }

    match WORKSPACE_MANAGER.register_workspace_folder(path) {
        Ok(discovery_result) => {
            let workspace_path = discovery_result.workspace_folder_path.clone();
            spawn_indexing_task(workspace_path, None);

            (
                StatusCode::CREATED,
                Json(WorkspaceResponse {
                    path: discovery_result.workspace_folder_path,
                }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(StatusResponse {
                status: format!("failed_to_load_workspace: {e}"),
            }),
        )
            .into_response(),
    }
}

async fn index_handler(Json(payload): Json<IndexRequest>) -> impl IntoResponse {
    let path = std::path::Path::new(&payload.workspace);
    if !path.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            Json(StatusResponse {
                status: "path_is_not_a_directory".to_string(),
            }),
        )
            .into_response();
    }

    match WORKSPACE_MANAGER.get_workspace_folder_info(&payload.workspace) {
        Some(info) => {
            if matches!(info.status, Status::Indexing) {
                return (
                    StatusCode::CONFLICT,
                    Json(StatusResponse {
                        status: format!(
                            "Workspace is already being processed. Current status: {}",
                            info.status
                        ),
                    }),
                )
                    .into_response();
            }
        }
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(StatusResponse {
                    status: "workspace_not_loaded".to_string(),
                }),
            )
                .into_response()
        }
    }

    let (tx, rx) = broadcast::channel(100);
    spawn_indexing_task(payload.workspace, Some(tx));

    let stream = BroadcastStream::new(rx)
        .map(|result: Result<String, BroadcastStreamRecvError>| {
            let progress = match result {
                Ok(p) => p,
                Err(e) => format!("Error receiving progress: {e}"),
            };
            Event::default()
                .json_data(IndexingProgressResponse { message: progress })
                .unwrap()
        })
        .map(|e| Ok(e) as Result<Event, Infallible>);

    Sse::new(stream).into_response()
}

fn spawn_indexing_task(workspace_path: String, progress_tx: Option<broadcast::Sender<String>>) {
    tokio::spawn(async move {
        let workspace_path_buf = PathBuf::from(workspace_path);
        let progress_tx_clone = progress_tx.clone();

        let task_result = tokio::task::spawn_blocking(move || {
            run_client_indexer(workspace_path_buf, 0, move |msg| {
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

pub async fn run(port: u16) -> Result<()> {
    let cors_layer = CorsLayer::new().allow_origin(tower_http::cors::AllowOrigin::predicate(
        |origin: &HeaderValue, _| {
            if let Ok(origin_str) = origin.to_str() {
                if let Ok(uri) = origin_str.parse::<http::Uri>() {
                    return uri.host() == Some("localhost");
                }
            }
            false
        },
    ));

    let app = Router::new()
        .route(
            "/",
            get({
                let shared_port = port;
                move || root_handler(shared_port)
            }),
        )
        .route("/workspace/index", post(index_handler))
        .route("/workspace/load", post(load_workspace_handler))
        .layer(cors_layer);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("HTTP server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

pub fn find_unused_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok(port)
}
