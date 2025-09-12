//! Job dispatching and queue management.
//!
//! The JobDispatcher is the central orchestrator of the job queue system. It manages
//! per-workspace job queues, dynamically creates and destroys workers, and handles
//! job prioritization and cancellation logic.

use anyhow::Result;
use chrono::Utc;
use dashmap::DashMap;
use database::kuzu::database::KuzuDatabase;
use event_bus::EventBus;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;
use workspace_manager::WorkspaceManager;

use crate::queue::{
    job::{Job, JobInfo, JobPriority, JobStatus},
    worker::{WorkerMessage, WorkspaceWorker},
};

/// Maximum number of jobs that can be queued per workspace before backpressure kicks in.
/// TODO: Make this configurable or dynamic based on system resources, business logic, etc.
const JOB_QUEUE_CAPACITY: usize = 1000;

pub struct JobDispatcher {
    pub workspace_queues: Arc<DashMap<String, mpsc::Sender<WorkerMessage>>>,
    pub workspace_manager: Arc<WorkspaceManager>,
    pub event_bus: Arc<EventBus>,
    pub database: Arc<KuzuDatabase>,
    pub worker_cancellation_tokens: Arc<DashMap<String, CancellationToken>>,
}

impl JobDispatcher {
    /// The dispatcher starts with no active workers - they are created dynamically
    /// as jobs are submitted for each workspace.
    pub fn new(
        workspace_manager: Arc<WorkspaceManager>,
        event_bus: Arc<EventBus>,
        database: Arc<KuzuDatabase>,
    ) -> Self {
        Self {
            workspace_queues: Arc::new(DashMap::new()),
            workspace_manager,
            event_bus,
            database,
            worker_cancellation_tokens: Arc::new(DashMap::new()),
        }
    }

    /// Dispatches a job to the appropriate workspace queue.
    ///
    /// This method:
    /// 1. Generates a unique job ID
    /// 2. Checks if the job is high priority and cancels existing workers if needed
    /// 3. Creates a workspace worker if one doesn't exist
    /// 4. Queues the job for processing
    ///
    /// Returns the job ID on successful dispatch.
    ///
    /// # Cancellation Behavior
    ///
    /// High-priority jobs will cancel any existing worker for the same workspace.
    pub async fn dispatch(&self, job: Job) -> Result<String> {
        let job_id = Uuid::new_v4().to_string();
        let workspace_path = job.workspace_path().to_string();

        info!(
            "Dispatching job {} ({}) for workspace {}",
            job_id,
            job.job_type(),
            workspace_path
        );

        let job_info = JobInfo {
            id: job_id.clone(),
            job: job.clone(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            status: JobStatus::Pending,
            error: None,
        };

        if job.priority() == JobPriority::High {
            self.cancel_existing_jobs_of_type(&workspace_path, job.job_type())
                .await?;
        }

        let sender = self.get_or_create_workspace_queue(&workspace_path).await?;

        sender
            .send(WorkerMessage::Job(job_info))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send job to workspace queue: {}", e))?;

        info!(
            "Successfully dispatched job {} for workspace {}",
            job_id, workspace_path
        );
        Ok(job_id)
    }

    /// If a queue already exists for the workspace, returns the existing sender.
    /// Otherwise, creates a new mpsc channel, spawns a WorkspaceWorker to process jobs,
    /// and sets up automatic cleanup when the worker shuts down.
    async fn get_or_create_workspace_queue(
        &self,
        workspace_path: &str,
    ) -> Result<mpsc::Sender<WorkerMessage>> {
        if let Some(sender) = self.workspace_queues.get(workspace_path) {
            return Ok(sender.clone());
        }

        let (sender, receiver) = mpsc::channel::<WorkerMessage>(JOB_QUEUE_CAPACITY);
        let cancellation_token = CancellationToken::new();

        self.workspace_queues
            .insert(workspace_path.to_string(), sender.clone());
        self.worker_cancellation_tokens
            .insert(workspace_path.to_string(), cancellation_token.clone());

        let worker = WorkspaceWorker::new(
            workspace_path.to_string(),
            receiver,
            Arc::clone(&self.workspace_manager),
            Arc::clone(&self.event_bus),
            Arc::clone(&self.database),
            cancellation_token.clone(),
        );

        let workspace_path_for_cleanup = workspace_path.to_string();
        let queues_for_cleanup = Arc::clone(&self.workspace_queues);
        let tokens_for_cleanup = Arc::clone(&self.worker_cancellation_tokens);

        tokio::spawn(async move {
            worker.run().await;

            queues_for_cleanup.remove(&workspace_path_for_cleanup);
            tokens_for_cleanup.remove(&workspace_path_for_cleanup);
            info!(
                "Cleaned up worker resources for workspace {}",
                workspace_path_for_cleanup
            );
        });

        info!("Created new worker for workspace {}", workspace_path);
        Ok(sender)
    }

    pub async fn cancel_existing_jobs_of_type(
        &self,
        workspace_path: &str,
        job_type: &str,
    ) -> Result<()> {
        if let Some(sender_entry) = self.workspace_queues.get(workspace_path) {
            let sender = sender_entry.clone();
            if let Err(e) = sender
                .send(WorkerMessage::CancelJobsOfType(job_type.to_string()))
                .await
            {
                warn!(
                    "Failed to send cancellation message for {} jobs in workspace {}: {}",
                    job_type, workspace_path, e
                );
            } else {
                info!(
                    "Sent cancellation request for {} jobs in workspace {}",
                    job_type, workspace_path
                );
            }
        }
        Ok(())
    }
}

impl Drop for JobDispatcher {
    /// The Drop implementation cancels all active workers and clears internal state.
    /// While this is a synchronous operation (Drop trait cannot be async), the
    /// cancellation tokens will signal workers to shut down gracefully in the background.
    fn drop(&mut self) {
        info!("JobDispatcher dropping, shutting down all workers");

        let worker_count = self.worker_cancellation_tokens.len();

        // Cancel all active workers via their cancellation tokens
        for entry in self.worker_cancellation_tokens.iter() {
            entry.value().cancel();
        }

        // Clear internal data structures to release memory
        self.workspace_queues.clear();
        self.worker_cancellation_tokens.clear();

        info!(
            "JobDispatcher drop complete - {} workers cancelled",
            worker_count
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::job::{Job, JobPriority};
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
    async fn test_job_dispatcher_creation() {
        let (workspace_manager, event_bus, database, _temp_dir) = create_test_setup();
        let dispatcher = JobDispatcher::new(workspace_manager, event_bus, database);

        assert_eq!(dispatcher.workspace_queues.len(), 0);
    }

    #[tokio::test]
    async fn test_dispatch_creates_worker() {
        let (workspace_manager, event_bus, database, _temp_dir) = create_test_setup();
        let dispatcher = JobDispatcher::new(workspace_manager, event_bus, database);

        let job = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/nonexistent/path".to_string(),
            priority: JobPriority::Normal,
        };

        // This should fail because the workspace doesn't exist, but it should still create a worker
        let _result = dispatcher.dispatch(job).await;

        // Give the worker a moment to start up
        sleep(Duration::from_millis(100)).await;

        assert_eq!(dispatcher.workspace_queues.len(), 1);
    }

    #[tokio::test]
    async fn test_multiple_jobs_same_workspace() {
        let (workspace_manager, event_bus, database, _temp_dir) = create_test_setup();
        let dispatcher = JobDispatcher::new(workspace_manager, event_bus, database);

        let job1 = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/workspace".to_string(),
            priority: JobPriority::Normal,
        };

        let job2 = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/workspace".to_string(),
            priority: JobPriority::Low,
        };

        let _result1 = dispatcher.dispatch(job1).await;
        let _result2 = dispatcher.dispatch(job2).await;

        sleep(Duration::from_millis(100)).await;

        assert_eq!(dispatcher.workspace_queues.len(), 1);
    }

    #[tokio::test]
    async fn test_multiple_workspaces() {
        let (workspace_manager, event_bus, database, _temp_dir) = create_test_setup();
        let dispatcher = JobDispatcher::new(workspace_manager, event_bus, database);

        let job1 = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/workspace1".to_string(),
            priority: JobPriority::Normal,
        };

        let job2 = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/workspace2".to_string(),
            priority: JobPriority::Normal,
        };

        let _result1 = dispatcher.dispatch(job1).await;
        let _result2 = dispatcher.dispatch(job2).await;

        sleep(Duration::from_millis(100)).await;

        assert_eq!(dispatcher.workspace_queues.len(), 2);
    }

    #[tokio::test]
    async fn test_high_priority_job_cancellation() {
        let (workspace_manager, event_bus, database, _temp_dir) = create_test_setup();
        let dispatcher = JobDispatcher::new(workspace_manager, event_bus, database);

        let job1 = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/workspace".to_string(),
            priority: JobPriority::Normal,
        };

        let result1 = dispatcher.dispatch(job1).await;
        assert!(result1.is_ok());

        sleep(Duration::from_millis(100)).await;
        assert_eq!(dispatcher.workspace_queues.len(), 1);

        // Dispatch a high priority job - should cancel the existing worker and create a new one
        let job2 = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/workspace".to_string(),
            priority: JobPriority::High,
        };

        let result2 = dispatcher.dispatch(job2).await;
        assert!(result2.is_ok());

        // Give the cancellation and new worker creation a moment to process
        sleep(Duration::from_millis(200)).await;

        assert!(result1.is_ok() && result2.is_ok());
    }

    #[tokio::test]
    async fn test_job_id_generation() {
        let (workspace_manager, event_bus, database, _temp_dir) = create_test_setup();
        let dispatcher = JobDispatcher::new(workspace_manager, event_bus, database);

        let job = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/workspace".to_string(),
            priority: JobPriority::Normal,
        };

        let result1 = dispatcher.dispatch(job.clone()).await;
        let result2 = dispatcher.dispatch(job).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_ne!(result1.unwrap(), result2.unwrap());
    }

    #[tokio::test]
    async fn test_granular_job_type_cancellation() {
        let (workspace_manager, event_bus, database, _temp_dir) = create_test_setup();
        let dispatcher = JobDispatcher::new(workspace_manager, event_bus, database);

        // First, dispatch a normal priority job to create a worker
        let job1 = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/workspace".to_string(),
            priority: JobPriority::Normal,
        };

        let result1 = dispatcher.dispatch(job1).await;
        assert!(result1.is_ok());

        sleep(Duration::from_millis(50)).await;
        assert_eq!(dispatcher.workspace_queues.len(), 1);

        // Get the sender to verify we can still send messages after cancellation
        let sender = dispatcher
            .workspace_queues
            .get("/test/workspace")
            .expect("Workspace queue should exist")
            .clone();

        // Dispatch a high priority job of the same type - should trigger cancellation
        let job2 = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/workspace".to_string(),
            priority: JobPriority::High,
        };

        let result2 = dispatcher.dispatch(job2).await;
        assert!(result2.is_ok());

        // The queue should still exist (no worker termination)
        assert_eq!(dispatcher.workspace_queues.len(), 1);

        // We should be able to send more jobs to the same queue
        let job3 = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/workspace".to_string(),
            priority: JobPriority::Low,
        };

        let result3 = dispatcher.dispatch(job3).await;
        assert!(result3.is_ok());

        // All jobs should have been dispatched successfully
        assert!(result1.is_ok() && result2.is_ok() && result3.is_ok());

        // The sender should still be functional (queue not destroyed)
        assert!(!sender.is_closed());
    }

    #[tokio::test]
    async fn test_drop_trait_automatic_shutdown() {
        let (workspace_manager, event_bus, database, _temp_dir) = create_test_setup();

        let (queue_arc, token_arc, tokens) = {
            let dispatcher = JobDispatcher::new(workspace_manager, event_bus, database);

            let job1 = Job::IndexWorkspaceFolder {
                workspace_folder_path: "/test/workspace1".to_string(),
                priority: JobPriority::Normal,
            };

            let job2 = Job::IndexWorkspaceFolder {
                workspace_folder_path: "/test/workspace2".to_string(),
                priority: JobPriority::Normal,
            };

            let _result1 = dispatcher.dispatch(job1).await;
            let _result2 = dispatcher.dispatch(job2).await;

            sleep(Duration::from_millis(100)).await;

            assert_eq!(dispatcher.workspace_queues.len(), 2);
            assert_eq!(dispatcher.worker_cancellation_tokens.len(), 2);

            let queue_arc = Arc::clone(&dispatcher.workspace_queues);
            let token_arc = Arc::clone(&dispatcher.worker_cancellation_tokens);

            let mut tokens = Vec::new();
            for entry in dispatcher.worker_cancellation_tokens.iter() {
                tokens.push(entry.value().clone());
            }

            assert!(Arc::strong_count(&queue_arc) >= 2);
            assert!(Arc::strong_count(&token_arc) >= 2);
            for token in &tokens {
                assert!(!token.is_cancelled());
            }

            (queue_arc, token_arc, tokens)
        };

        sleep(Duration::from_millis(100)).await;

        assert_eq!(Arc::strong_count(&queue_arc), 1);
        assert_eq!(Arc::strong_count(&token_arc), 1);
        assert_eq!(queue_arc.len(), 0);
        assert_eq!(token_arc.len(), 0);
        for token in &tokens {
            assert!(token.is_cancelled());
        }
    }
}
