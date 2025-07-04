//! # Knowledge Graph Event Bus
//!
//! The event bus provides a centralized, real-time communication system for broadcasting
//! structured information about major events happening during the knowledge graph indexing process.
//!
//! ## Purpose
//!
//! The event bus enables real-time monitoring and feedback for knowledge graph operations by:
//!
//! - **Broadcasting workspace-level events** when workspaces are discovered, registered, or updated
//! - **Streaming project-level progress** as individual repositories are being indexed
//! - **Providing structured payloads** containing full state information
//! - **Generating TypeScript types** for frontend consumption via ts-rs
//!
//! ## Event Flow Architecture
//!
//! ```text
//! ┌─────────────────┐    ┌──────────────┐    ┌─────────────────┐
//! │   Indexing      │    │  Event Bus   │    │   Consumers     │
//! │   Operations    │───▶│  (Broadcast) │───▶│   • CLI         │
//! │                 │    │              │    │   • HTTP/SSE    │
//! │ • Workspace     │    │              │    │   • Frontend    │
//! │   Discovery     │    │              │    │                 │
//! │ • Project       │    │              │    │                 │
//! │   Indexing      │    │              │    │                 │
//! │ • File          │    │              │    │                 │
//! │   Processing    │    │              │    │                 │
//! └─────────────────┘    └──────────────┘    └─────────────────┘
//! ```
//!
//! ## Event-Bus vs Logging
//!
//! While logging helps developers understand *what the system is doing*, the event bus enables
//! clients to react to *what the system has accomplished* with complete state information.

use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::broadcast::{self, Sender};
use ts_rs::TS;

use crate::types::{project_info::TSProjectInfo, workspace_folder::TSWorkspaceFolderInfo};
pub mod types;

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/events.ts")]
#[serde(tag = "type", content = "payload")]
pub enum GkgEvent {
    WorkspaceIndexing(WorkspaceIndexingEvent),
    ProjectIndexing(ProjectIndexingEvent),
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/events.ts")]
#[serde(tag = "status")]
pub enum WorkspaceIndexingEvent {
    Started(WorkspaceIndexingStarted),
    Completed(WorkspaceIndexingCompleted),
    Failed(WorkspaceIndexingFailed),
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/events.ts")]
pub struct WorkspaceIndexingStarted {
    pub workspace_folder_info: TSWorkspaceFolderInfo,
    pub projects_to_process: Vec<String>,
    pub started_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/events.ts")]
#[serde(tag = "status")]
pub enum ProjectIndexingEvent {
    Started(ProjectIndexingStarted),
    Completed(ProjectIndexingCompleted),
    Failed(ProjectIndexingFailed),
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/events.ts")]
pub struct ProjectIndexingStarted {
    pub project_info: TSProjectInfo,
    pub started_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/events.ts")]
pub struct ProjectIndexingCompleted {
    pub project_info: TSProjectInfo,
    pub completed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/events.ts")]
pub struct ProjectIndexingFailed {
    pub project_info: TSProjectInfo,
    pub error: String,
    pub failed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/events.ts")]
pub struct WorkspaceIndexingCompleted {
    pub workspace_folder_info: TSWorkspaceFolderInfo,
    pub projects_indexed: Vec<String>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/events.ts")]
pub struct WorkspaceIndexingFailed {
    pub workspace_folder_info: TSWorkspaceFolderInfo,
    pub projects_indexed: Vec<String>,
    pub error: String,
    pub failed_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct EventBus {
    sender: Sender<GkgEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self { sender }
    }

    pub fn send(&self, event: &GkgEvent) {
        if self.sender.send(event.clone()).is_err() {
            // This can happen if there are no receivers.
            // In our case, this is fine, we can just ignore the error for now.
            tracing::info!("No receivers for event bus, ignoring event: {:?}", &event);
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<GkgEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
