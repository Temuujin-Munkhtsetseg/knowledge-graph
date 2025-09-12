//! Job queue system for workspace indexing operations.
//!
//! This module provides a job queue system designed to handle indexing
//! operations across multiple workspaces. The system aims to ensure
//! that operations within the same workspace are processed sequentially while allowing
//! parallel processing across different workspaces.
//!
//! ## Architecture Overview
//!
//! ```text
//! HTTP Endpoint
//!      │
//!      ▼
//! JobDispatcher ────┐
//!      │            │
//!      ▼            ▼
//! Workspace A    Workspace B
//! Queue + Worker Queue + Worker
//!      │            │
//!      ▼            ▼
//! IndexingExecutor  IndexingExecutor
//! ```
//!
//! ## Modules
//!
//! - **[`job`]**: Defines job types, priorities, and metadata structures
//! - **[`dispatch`]**: Central dispatching and queue management logic  
//! - **[`worker`]**: Per-workspace job processing workers
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use crate::queue::{dispatch::JobDispatcher, job::{Job, JobPriority}};
//!
//! // Create dispatcher with shared resources
//! let dispatcher = JobDispatcher::new(workspace_manager, event_bus);
//!
//! // Dispatch a high-priority indexing job
//! let job = Job::IndexWorkspaceFolder {
//!     workspace_folder_path: "/path/to/workspace".to_string(),
//!     priority: JobPriority::High,
//! };
//!
//! let job_id = dispatcher.dispatch(job).await?;
//! ```
//! ## Priority System
//!
//! - **Low**: Background operations, file watching events
//! - **Normal**: Regular indexing requests  
//! - **High**: User-triggered operations that should preempt existing work
//!
//! High-priority jobs will cancel any existing worker for the same workspace.

pub mod dispatch;
pub mod job;
pub mod worker;

pub use dispatch::JobDispatcher;
pub use job::{Job, JobInfo, JobPriority, JobStatus};
pub use worker::WorkspaceWorker;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use database::kuzu::database::KuzuDatabase;
    use event_bus::EventBus;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::time::{Duration, sleep};
    use workspace_manager::WorkspaceManager;

    fn create_test_setup() -> (
        Arc<WorkspaceManager>,
        Arc<EventBus>,
        Arc<KuzuDatabase>,
        TempDir,
    ) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_manager =
            Arc::new(WorkspaceManager::new_with_directory(temp_dir.path().to_path_buf()).unwrap());
        let event_bus = Arc::new(EventBus::new());
        let database = Arc::new(KuzuDatabase::new());
        (workspace_manager, event_bus, database, temp_dir)
    }

    #[tokio::test]
    async fn test_end_to_end_job_processing() {
        let (workspace_manager, event_bus, database, _temp_dir) = create_test_setup();
        let dispatcher = JobDispatcher::new(workspace_manager, event_bus, database);

        let job = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/workspace".to_string(),
            priority: JobPriority::Normal,
        };

        let job_id = dispatcher.dispatch(job).await;
        assert!(job_id.is_ok());

        sleep(Duration::from_millis(100)).await;

        assert_eq!(dispatcher.workspace_queues.len(), 1);
    }

    #[tokio::test]
    async fn test_priority_based_cancellation_integration() {
        let (workspace_manager, event_bus, database, _temp_dir) = create_test_setup();
        let dispatcher = JobDispatcher::new(workspace_manager, event_bus, database);

        let normal_job = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/workspace".to_string(),
            priority: JobPriority::Normal,
        };

        let job_id1 = dispatcher.dispatch(normal_job).await;
        assert!(job_id1.is_ok());

        sleep(Duration::from_millis(50)).await;
        assert_eq!(dispatcher.workspace_queues.len(), 1);

        let high_priority_job = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/workspace".to_string(),
            priority: JobPriority::High,
        };

        let job_id2 = dispatcher.dispatch(high_priority_job).await;
        assert!(job_id2.is_ok());

        assert_ne!(job_id1.unwrap(), job_id2.unwrap());

        sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_multi_workspace_parallel_processing() {
        let (workspace_manager, event_bus, database, _temp_dir) = create_test_setup();
        let dispatcher = JobDispatcher::new(workspace_manager, event_bus, database);

        let workspaces = vec!["/workspace1", "/workspace2", "/workspace3"];

        let mut job_ids = Vec::new();

        for workspace in &workspaces {
            let job = Job::IndexWorkspaceFolder {
                workspace_folder_path: workspace.to_string(),
                priority: JobPriority::Normal,
            };

            let job_id = dispatcher.dispatch(job).await;
            assert!(job_id.is_ok());
            job_ids.push(job_id.unwrap());
        }

        assert_eq!(job_ids.len(), 3);
        for i in 0..job_ids.len() {
            for j in i + 1..job_ids.len() {
                assert_ne!(job_ids[i], job_ids[j]);
            }
        }

        sleep(Duration::from_millis(100)).await;

        assert_eq!(dispatcher.workspace_queues.len(), 3);
    }

    #[tokio::test]
    async fn test_job_queue_capacity_and_resilience() {
        let (workspace_manager, event_bus, database, _temp_dir) = create_test_setup();
        let dispatcher = JobDispatcher::new(workspace_manager, event_bus, database);

        let workspace_path = "/test/workspace";

        let mut job_ids = Vec::new();
        for i in 0..5 {
            let job = Job::IndexWorkspaceFolder {
                workspace_folder_path: format!("{workspace_path}-{i}"),
                priority: JobPriority::Normal,
            };

            let job_id = dispatcher.dispatch(job).await;
            assert!(job_id.is_ok());
            job_ids.push(job_id.unwrap());
        }

        assert_eq!(job_ids.len(), 5);

        sleep(Duration::from_millis(100)).await;

        assert_eq!(dispatcher.workspace_queues.len(), 5);
    }

    #[tokio::test]
    async fn test_queue_system_component_integration() {
        let job = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/integration/test".to_string(),
            priority: JobPriority::High,
        };

        assert_eq!(job.workspace_path(), "/integration/test");
        assert_eq!(job.priority(), JobPriority::High);
        assert_eq!(job.job_type(), "IndexWorkspaceFolder");

        let job_info = JobInfo {
            id: "integration-test".to_string(),
            job: job.clone(),
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            status: JobStatus::Pending,
            error: None,
        };

        assert_eq!(job_info.status, JobStatus::Pending);
        assert_eq!(job_info.job.workspace_path(), "/integration/test");

        let serialized = serde_json::to_string(&job_info).unwrap();
        let deserialized: JobInfo = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, job_info.id);
        assert_eq!(deserialized.status, job_info.status);
        assert_eq!(
            deserialized.job.workspace_path(),
            job_info.job.workspace_path()
        );
    }
}
