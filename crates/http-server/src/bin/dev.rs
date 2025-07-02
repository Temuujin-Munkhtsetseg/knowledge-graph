use anyhow::Result;
use event_bus::EventBus;
use http_server::{find_unused_port, run};
use logging::{LogMode, init};
use std::env;
use std::sync::Arc;
use workspace_manager::WorkspaceManager;

#[tokio::main]
async fn main() -> Result<()> {
    init(LogMode::Cli, true).unwrap();

    let port = env::var("DEV_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or_else(|| find_unused_port().unwrap_or(27495));

    println!("🚀 Development server starting on port {port}");

    let workspace_manager = Arc::new(WorkspaceManager::new_system_default().unwrap());
    let event_bus = Arc::new(EventBus::new());

    run(port, workspace_manager, event_bus).await
}
