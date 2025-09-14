use crate::{configuration::McpConfiguration, service::DefaultMcpService};
use axum::Router;
use database::kuzu::database::KuzuDatabase;
use database::querying::types::QueryingService;
use event_bus::EventBus;
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use std::{net::SocketAddr, sync::Arc};
use tokio_util::sync::CancellationToken;
use workspace_manager::WorkspaceManager;

pub fn mcp_sse_router(
    bind: SocketAddr,
    query_service: Arc<dyn QueryingService>,
    workspace_manager: Arc<WorkspaceManager>,
    database: Arc<KuzuDatabase>,
    event_bus: Arc<EventBus>,
    configuration: Arc<McpConfiguration>,
) -> (Router, CancellationToken) {
    let (sse_server, router) = SseServer::new(SseServerConfig {
        bind,
        sse_path: "/".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    });

    let cancellation_token = sse_server.config.ct.child_token();

    sse_server.with_service(move || {
        DefaultMcpService::new(
            Arc::clone(&query_service),
            Arc::clone(&workspace_manager),
            Arc::clone(&database),
            Arc::clone(&event_bus),
            Arc::clone(&configuration),
        )
    });

    (router, cancellation_token)
}
