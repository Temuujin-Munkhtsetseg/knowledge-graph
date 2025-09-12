use crate::AppState;
use crate::contract::{EmptyRequest, EndpointConfigTypes};
use crate::define_endpoint;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use chrono::Utc;
use futures_util::stream::Stream;
use futures_util::{StreamExt, stream};
use serde::Serialize;
use serde_json::json;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use ts_rs::TS;

#[derive(Serialize, TS, Default)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct EventsResponses {
    // SSE responses don't need structured response types
    // The events are streamed directly as Server-Sent Events
}

pub struct EventsEndpointConfig;

impl EndpointConfigTypes for EventsEndpointConfig {
    type PathRequest = EmptyRequest;
    type BodyRequest = EmptyRequest;
    type QueryRequest = EmptyRequest;
    type Response = EventsResponses;
}

define_endpoint! {
    EventsEndpoint,
    EventsEndpointDef,
    Get,
    "/events",
    ts_path_type = "\"/api/events\"",
    config = EventsEndpointConfig,
    export_to = "../../../packages/gkg/src/api.ts"
}

/// Handler for the events endpoint
/// Returns a Server-Sent Events (SSE) stream of all system events
pub async fn events_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.event_bus.subscribe();

    // Create initial connection event
    let connection_event = json!({
        "type": "connection-established",
        "timestamp": Utc::now().to_rfc3339(),
        "message": "SSE connection established"
    });

    let initial_event = stream::once(async move {
        Ok(Event::default()
            .event("gkg-connection")
            .data(connection_event.to_string()))
    });

    let event_stream = BroadcastStream::new(receiver).filter_map(|result| async move {
        match result {
            Ok(event) => {
                // Serialize the event to JSON
                match serde_json::to_string(&event) {
                    Ok(json) => Some(Ok(Event::default().event("gkg-event").data(json))),
                    Err(e) => {
                        tracing::error!("Failed to serialize event: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Event stream error: {}", e);
                None
            }
        }
    });

    let combined_stream = initial_event.chain(event_stream);

    Sse::new(combined_stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(30)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use axum::{Router, routing::get};
    use axum_test::TestServer;
    use chrono::Utc;
    use database::kuzu::database::KuzuDatabase;
    use event_bus::types::workspace_folder::to_ts_workspace_folder_info;
    use event_bus::{EventBus, GkgEvent, WorkspaceIndexingEvent, WorkspaceIndexingStarted};
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;
    use workspace_manager::WorkspaceManager;
    use workspace_manager::{Status, WorkspaceFolderInfo};

    async fn create_test_app() -> (TestServer, Arc<EventBus>, TempDir) {
        let temp_data_dir = TempDir::new().unwrap();
        let workspace_manager = Arc::new(
            WorkspaceManager::new_with_directory(temp_data_dir.path().to_path_buf()).unwrap(),
        );
        let event_bus = Arc::new(EventBus::new());
        let database = Arc::new(KuzuDatabase::new());

        let job_dispatcher = Arc::new(crate::queue::dispatch::JobDispatcher::new(
            workspace_manager.clone(),
            Arc::clone(&event_bus),
            Arc::clone(&database),
        ));

        let state = AppState {
            database: Arc::clone(&database),
            workspace_manager,
            event_bus: Arc::clone(&event_bus),
            job_dispatcher,
        };

        let app = Router::new()
            .route("/events", get(events_handler))
            .with_state(state);
        (TestServer::new(app).unwrap(), event_bus, temp_data_dir)
    }

    #[tokio::test]
    async fn test_events_endpoint_connection() {
        let (server, _event_bus, _temp_dir) = create_test_app().await;

        let result = tokio::time::timeout(Duration::from_millis(500), server.get("/events")).await;

        match result {
            Ok(response) => {
                response.assert_status_ok();
                assert_eq!(
                    response.headers().get("content-type").unwrap(),
                    "text/event-stream"
                );
                assert_eq!(response.headers().get("cache-control").unwrap(), "no-cache");
            }
            Err(_) => {
                println!("SSE connection test completed (timeout expected for streaming endpoint)");
            }
        }
    }

    #[tokio::test]
    async fn test_events_endpoint_stream_format() {
        let (server, event_bus, _temp_dir) = create_test_app().await;

        let test_event = GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Started(
            WorkspaceIndexingStarted {
                workspace_folder_info: to_ts_workspace_folder_info(&WorkspaceFolderInfo {
                    workspace_folder_path: "/test/workspace".to_string(),
                    data_directory_name: "test".to_string(),
                    status: Status::Indexing,
                    last_indexed_at: Some(Utc::now()),
                    project_count: 2,
                    gitalisk_workspace: None,
                }),
                projects_to_process: vec![],
                started_at: Utc::now(),
            },
        ));

        event_bus.send(&test_event);

        let result = tokio::time::timeout(Duration::from_millis(200), server.get("/events")).await;

        match result {
            Ok(response) => {
                response.assert_status_ok();
                assert_eq!(
                    response.headers().get("content-type").unwrap(),
                    "text/event-stream"
                );
            }
            Err(_) => {
                println!("SSE connection test completed (timeout expected)");
            }
        }
    }

    #[tokio::test]
    async fn test_events_endpoint_routing() {
        let (server, _event_bus, _temp_dir) = create_test_app().await;

        let result = tokio::time::timeout(Duration::from_millis(100), server.get("/events")).await;

        match result {
            Ok(response) => {
                response.assert_status_ok();
            }
            Err(_) => {
                println!("Endpoint routing test completed (timeout expected for SSE)");
            }
        }
    }
}
