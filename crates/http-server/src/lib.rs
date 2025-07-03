pub mod api;
pub mod contract;
pub mod endpoints;
pub mod queue;

use crate::{
    contract::EndpointContract,
    endpoints::{
        events::{EventsEndpoint, events_handler},
        info::{InfoEndpoint, info_handler},
        mcp::{mcp_batch_handler, mcp_handler},
        workspace_index::{WorkspaceIndexEndpoint, index_handler},
        workspace_list::{WorkspaceListEndpoint, workspace_list_handler},
    },
    queue::dispatch::JobDispatcher,
};
use anyhow::Result;
use axum::http::HeaderValue;
use axum::{
    Router,
    routing::{get, post},
};
use axum_embed::ServeEmbed;
use database::kuzu::database::KuzuDatabase;
use database::querying::service::DatabaseQueryingService;
use event_bus::EventBus;
use mcp::DefaultMcpService;
use rust_embed::Embed;
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use workspace_manager::WorkspaceManager;

#[derive(Clone)]
pub struct AppState {
    pub workspace_manager: Arc<WorkspaceManager>,
    pub event_bus: Arc<EventBus>,
    pub job_dispatcher: Arc<JobDispatcher>,
}

#[derive(Embed, Clone)]
#[folder = "../../packages/frontend/dist"]
struct Assets;

pub async fn run(
    port: u16,
    workspace_manager: Arc<WorkspaceManager>,
    event_bus: Arc<EventBus>,
) -> Result<()> {
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

    let database = KuzuDatabase::new();
    let query_service = Arc::new(DatabaseQueryingService::new(
        database,
        workspace_manager.clone(),
    ));

    let mcp_router = Router::new()
        .route("/", post(mcp_handler))
        .route("/batch", post(mcp_batch_handler))
        .with_state(Arc::new(DefaultMcpService::new(query_service)));

    let job_dispatcher = Arc::new(JobDispatcher::new(
        workspace_manager.clone(),
        event_bus.clone(),
    ));

    let state = AppState {
        workspace_manager,
        event_bus,
        job_dispatcher,
    };
    let serve_assets = ServeEmbed::<Assets>::new();

    let api_router = Router::new()
        .route(
            InfoEndpoint::PATH,
            get({
                let shared_port = port;
                move || info_handler(shared_port)
            }),
        )
        .route(WorkspaceIndexEndpoint::PATH, post(index_handler))
        .route(EventsEndpoint::PATH, get(events_handler))
        .route(WorkspaceListEndpoint::PATH, get(workspace_list_handler))
        .with_state(state);

    let app = Router::new()
        .nest("/api", api_router)
        .nest("/mcp", mcp_router)
        .fallback_service(serve_assets)
        .layer(ServiceBuilder::new().layer(cors_layer));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("HTTP server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// The preferred port is an easter egg from "knowledge graph":
// 'k' -> 0x6b, 'g' -> 0x67 => 0x6b67 => 27495
const PREFERRED_PORT: u16 = 27495;

pub fn find_unused_port() -> Result<u16> {
    match TcpListener::bind(("127.0.0.1", PREFERRED_PORT)) {
        Ok(listener) => Ok(listener.local_addr()?.port()),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            tracing::info!(
                "Preferred port {} is busy, finding a random unused port",
                PREFERRED_PORT
            );
            let listener = TcpListener::bind("127.0.0.1:0")?;
            let port = listener.local_addr()?.port();
            Ok(port)
        }
        Err(e) => {
            tracing::error!("Error finding unused port: {e}");
            Err(e.into())
        }
    }
}
