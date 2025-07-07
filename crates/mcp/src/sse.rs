use crate::service::DefaultMcpService;
use axum::Router;
use database::querying::types::QueryingService;
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use std::{net::SocketAddr, sync::Arc};
use tokio_util::sync::CancellationToken;
use workspace_manager::WorkspaceManager;

pub fn mcp_sse_router(
    bind: SocketAddr,
    query_service: Arc<dyn QueryingService>,
    workspace_manager: Arc<WorkspaceManager>,
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
        DefaultMcpService::new(Arc::clone(&query_service), Arc::clone(&workspace_manager))
    });

    (router, cancellation_token)
}
