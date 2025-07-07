pub mod api;
pub mod contract;
pub mod endpoints;
pub mod queue;

#[cfg(test)]
pub mod testing;

use crate::{
    contract::EndpointContract,
    endpoints::{
        events::{EventsEndpoint, events_handler},
        graph::{
            graph_initial::{GraphInitialEndpoint, graph_initial_handler},
            graph_neighbors::{GraphNeighborsEndpoint, graph_neighbors_handler},
            graph_search::{GraphSearchEndpoint, graph_search_handler},
        },
        info::{InfoEndpoint, info_handler},
        workspace_delete::{WorkspaceDeleteEndpoint, delete_handler},
        workspace_index::{WorkspaceIndexEndpoint, index_handler},
        workspace_list::{WorkspaceListEndpoint, workspace_list_handler},
    },
    queue::dispatch::JobDispatcher,
};
use anyhow::Result;
use axum::http::HeaderValue;
use axum::{
    Router,
    routing::{delete, get, post},
};
use axum_embed::ServeEmbed;
use database::querying::service::DatabaseQueryingService;
use database::{kuzu::database::KuzuDatabase, querying::QueryingService};
use event_bus::EventBus;
use mcp::{http::mcp_http_service, sse::mcp_sse_router};
use rust_embed::Embed;
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use workspace_manager::WorkspaceManager;

#[derive(Clone)]
pub struct AppState {
    pub database: Arc<KuzuDatabase>,
    pub workspace_manager: Arc<WorkspaceManager>,
    pub event_bus: Arc<EventBus>,
    pub job_dispatcher: Arc<JobDispatcher>,
}

#[derive(Embed, Clone)]
#[folder = "../../packages/frontend/dist"]
struct Assets;

pub async fn run(
    port: u16,
    database: Arc<KuzuDatabase>,
    workspace_manager: Arc<WorkspaceManager>,
    event_bus: Arc<EventBus>,
) -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
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

    let job_dispatcher = Arc::new(JobDispatcher::new(
        workspace_manager.clone(),
        event_bus.clone(),
        Arc::clone(&database),
    ));

    let query_service: Arc<dyn QueryingService> =
        Arc::new(DatabaseQueryingService::new(Arc::clone(&database)));

    let state = AppState {
        database: Arc::clone(&database),
        workspace_manager: workspace_manager.clone(),
        event_bus,
        job_dispatcher,
    };

    let serve_assets = ServeEmbed::<Assets>::new();

    let mcp_http_router =
        mcp_http_service(Arc::clone(&query_service), Arc::clone(&workspace_manager));
    let (mcp_sse_router, mcp_sse_cancellation_token) = mcp_sse_router(
        addr,
        Arc::clone(&query_service),
        Arc::clone(&workspace_manager),
    );

    let api_router = Router::new()
        .route(
            InfoEndpoint::PATH,
            get({
                let shared_port = port;
                move || info_handler(shared_port)
            }),
        )
        .route(WorkspaceIndexEndpoint::PATH, post(index_handler))
        .route(WorkspaceDeleteEndpoint::PATH, delete(delete_handler))
        .route(EventsEndpoint::PATH, get(events_handler))
        .route(WorkspaceListEndpoint::PATH, get(workspace_list_handler))
        .route(GraphInitialEndpoint::PATH, get(graph_initial_handler))
        .route(GraphNeighborsEndpoint::PATH, get(graph_neighbors_handler))
        .route(GraphSearchEndpoint::PATH, get(graph_search_handler))
        .with_state(state);

    let app = Router::new()
        .nest("/api", api_router)
        .nest_service("/mcp", mcp_http_router)
        .nest_service("/mcp/sse", mcp_sse_router)
        .fallback_service(serve_assets)
        .layer(ServiceBuilder::new().layer(cors_layer));

    tracing::info!("HTTP server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    mcp_sse_cancellation_token.cancel();
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
