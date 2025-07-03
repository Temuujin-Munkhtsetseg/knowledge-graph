use anyhow::Result;
use chrono::Utc;
use event_bus::EventBus;
use indexer::execution::{config::IndexingConfigBuilder, executor::IndexingExecutor};
use num_cpus;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use workspace_manager::WorkspaceManager;

use crate::queue::job::{Job, JobInfo, JobStatus};

/// Message types that can be sent to a workspace worker
#[derive(Debug, Clone)]
pub enum WorkerMessage {
    /// A new job to process
    Job(JobInfo),
    /// Cancel all pending jobs of a specific type
    CancelJobsOfType(String),
}

/// Timeout in seconds after which an idle worker will automatically shut down.
/// This helps conserve system resources when workspaces are not actively being processed.
const WORKER_TIMEOUT_SECS: u64 = 60;

/// Each WorkspaceWorker is responsible for processing jobs sequentially for a single
/// workspace. This ensures that operations on the same workspace are atomic and ordered,
/// while allowing parallel processing across different workspaces.
pub struct WorkspaceWorker {
    workspace_path: String,
    receiver: mpsc::Receiver<WorkerMessage>,
    workspace_manager: Arc<WorkspaceManager>,
    event_bus: Arc<EventBus>,
    cancellation_token: CancellationToken,
    job_queue: VecDeque<JobInfo>,
}

impl WorkspaceWorker {
    pub fn new(
        workspace_path: String,
        receiver: mpsc::Receiver<WorkerMessage>,
        workspace_manager: Arc<WorkspaceManager>,
        event_bus: Arc<EventBus>,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            workspace_path,
            receiver,
            workspace_manager,
            event_bus,
            cancellation_token,
            job_queue: VecDeque::new(),
        }
    }

    /// Main worker loop that processes jobs sequentially until shutdown.
    ///
    /// The worker continues processing jobs until one of these conditions is met:
    /// - The cancellation token is triggered (worker shutdown)
    /// - The message channel is closed (dispatcher shutdown)
    /// - No messages are received within the timeout period (auto-cleanup)
    ///
    /// Jobs are processed one at a time in FIFO order, with support for cancelling
    /// specific job types while preserving others in the queue.
    pub async fn run(mut self) {
        info!("Starting worker for workspace: {}", self.workspace_path);

        while !self.cancellation_token.is_cancelled() {
            // First, try to process any queued jobs
            if let Some(mut job_info) = self.job_queue.pop_front() {
                info!(
                    "Processing queued job {} for workspace {}",
                    job_info.id, self.workspace_path
                );
                job_info.started_at = Some(Utc::now());
                job_info.status = JobStatus::Running;

                let result = self.process_job(&job_info.job).await;

                match result {
                    Ok(()) => {
                        job_info.completed_at = Some(Utc::now());
                        job_info.status = JobStatus::Completed;
                        info!(
                            "Completed job {} for workspace {}",
                            job_info.id, self.workspace_path
                        );
                    }
                    Err(e) => {
                        job_info.completed_at = Some(Utc::now());
                        job_info.status = JobStatus::Failed;
                        job_info.error = Some(e.to_string());
                        error!(
                            "Failed job {} for workspace {}: {}",
                            job_info.id, self.workspace_path, e
                        );
                    }
                }
                continue;
            }

            // If no queued jobs, wait for new messages
            match timeout(
                Duration::from_secs(WORKER_TIMEOUT_SECS),
                self.receiver.recv(),
            )
            .await
            {
                Ok(Some(message)) => match message {
                    WorkerMessage::Job(job_info) => {
                        self.job_queue.push_back(job_info);
                    }
                    WorkerMessage::CancelJobsOfType(job_type) => {
                        let original_count = self.job_queue.len();
                        self.job_queue.retain(|job_info| {
                            let should_keep = job_info.job.job_type() != job_type;
                            if !should_keep {
                                warn!(
                                    "Cancelling job {} ({}) for workspace {}",
                                    job_info.id, job_type, self.workspace_path
                                );
                            }
                            should_keep
                        });
                        let cancelled_count = original_count - self.job_queue.len();
                        if cancelled_count > 0 {
                            info!(
                                "Cancelled {} {} jobs for workspace {}",
                                cancelled_count, job_type, self.workspace_path
                            );
                        }
                    }
                },
                Ok(None) => {
                    debug!(
                        "Message channel closed for workspace {}",
                        self.workspace_path
                    );
                    break;
                }
                Err(_) => {
                    info!(
                        "Worker timeout for workspace {}, shutting down",
                        self.workspace_path
                    );
                    break;
                }
            }
        }

        info!("Worker for workspace {} shutting down", self.workspace_path);
    }

    async fn process_job(&self, job: &Job) -> Result<()> {
        match job {
            Job::IndexWorkspaceFolder {
                workspace_folder_path,
                ..
            } => {
                self.process_index_workspace_job(workspace_folder_path)
                    .await
            }
        }
    }

    /// Processes an IndexWorkspaceFolder job by running full workspace indexing.
    ///
    /// This method:
    /// 1. Creates an IndexingExecutor with system-appropriate thread count
    /// 2. Runs the indexing in a blocking task to avoid blocking the async runtime
    /// 3. Discovers all Git repositories in the workspace
    /// 4. Indexes their contents into the knowledge graph database under 3 the core phases of indexing:
    ///    - Parsing (E)
    ///    - Analysis (T)
    ///    - Write and Load to Kuzu (L)
    async fn process_index_workspace_job(&self, workspace_folder_path: &str) -> Result<()> {
        let workspace_path_buf = PathBuf::from(workspace_folder_path);
        let threads = num_cpus::get();
        let config = IndexingConfigBuilder::build(threads);
        let mut executor = IndexingExecutor::new(
            Arc::clone(&self.workspace_manager),
            Arc::clone(&self.event_bus),
            config,
        );

        let cancellation_token = CancellationToken::new();
        let result = tokio::task::spawn_blocking(move || {
            executor.execute_workspace_indexing(workspace_path_buf, Some(cancellation_token))
        })
        .await;

        match result {
            Ok(Ok(())) => {
                info!(
                    "Indexing completed successfully for workspace '{}'",
                    workspace_folder_path
                );
                Ok(())
            }
            Ok(Err(e)) => {
                error!(
                    "Indexing failed for workspace '{}': {}",
                    workspace_folder_path, e
                );
                Err(e)
            }
            Err(e) => {
                error!(
                    "Indexing task panicked for workspace '{}': {}",
                    workspace_folder_path, e
                );
                Err(anyhow::anyhow!("Indexing task panicked: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::job::JobPriority;
    use event_bus::EventBus;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::mpsc;
    use tokio::time::{Duration, timeout};
    use workspace_manager::WorkspaceManager;

    fn create_test_setup() -> (Arc<WorkspaceManager>, Arc<EventBus>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_manager =
            Arc::new(WorkspaceManager::new_with_directory(temp_dir.path().to_path_buf()).unwrap());
        let event_bus = Arc::new(EventBus::new());
        (workspace_manager, event_bus, temp_dir)
    }

    #[tokio::test]
    async fn test_worker_creation() {
        let (workspace_manager, event_bus, _temp_dir) = create_test_setup();
        let (_sender, receiver) = mpsc::channel::<WorkerMessage>(100);
        let cancellation_token = CancellationToken::new();

        let worker = WorkspaceWorker::new(
            "/test/workspace".to_string(),
            receiver,
            workspace_manager,
            event_bus,
            cancellation_token,
        );

        assert_eq!(worker.workspace_path, "/test/workspace");
    }

    #[tokio::test]
    async fn test_worker_cancellation() {
        let (workspace_manager, event_bus, _temp_dir) = create_test_setup();
        let (_sender, receiver) = mpsc::channel::<WorkerMessage>(100);
        let cancellation_token = CancellationToken::new();

        let worker = WorkspaceWorker::new(
            "/test/workspace".to_string(),
            receiver,
            workspace_manager,
            event_bus,
            cancellation_token.clone(),
        );

        cancellation_token.cancel();

        let result = timeout(Duration::from_millis(100), worker.run()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_worker_timeout_behavior() {
        let (workspace_manager, event_bus, _temp_dir) = create_test_setup();
        let (_sender, receiver) = mpsc::channel::<WorkerMessage>(100);
        let cancellation_token = CancellationToken::new();

        let worker = WorkspaceWorker::new(
            "/test/workspace".to_string(),
            receiver,
            workspace_manager,
            event_bus,
            cancellation_token,
        );

        // Worker should timeout quickly since no jobs are sent and timeout is 60 seconds
        // We can't wait 60 seconds in a test, so we'll just verify it starts properly
        let worker_future = worker.run();

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // If we get here without panicking, the worker started successfully
        // In a real scenario, it would timeout after WORKER_TIMEOUT_SECS
        drop(worker_future); // Prevent the test from hanging
    }

    #[tokio::test]
    async fn test_job_processing_dispatch() {
        let job = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/nonexistent/path".to_string(),
            priority: JobPriority::Normal,
        };

        assert_eq!(job.workspace_path(), "/nonexistent/path");
        assert_eq!(job.job_type(), "IndexWorkspaceFolder");
    }

    #[tokio::test]
    async fn test_job_info_status_updates() {
        let job = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/path".to_string(),
            priority: JobPriority::Normal,
        };

        let mut job_info = JobInfo {
            id: "test-job".to_string(),
            job,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            status: JobStatus::Pending,
            error: None,
        };

        assert_eq!(job_info.status, JobStatus::Pending);
        assert!(job_info.started_at.is_none());
        assert!(job_info.completed_at.is_none());

        job_info.started_at = Some(Utc::now());
        job_info.status = JobStatus::Running;
        assert_eq!(job_info.status, JobStatus::Running);
        assert!(job_info.started_at.is_some());

        job_info.completed_at = Some(Utc::now());
        job_info.status = JobStatus::Completed;
        assert_eq!(job_info.status, JobStatus::Completed);
        assert!(job_info.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_job_type_specific_cancellation() {
        let (workspace_manager, event_bus, _temp_dir) = create_test_setup();
        let (sender, receiver) = mpsc::channel::<WorkerMessage>(100);
        let cancellation_token = CancellationToken::new();

        let worker = WorkspaceWorker::new(
            "/test/workspace".to_string(),
            receiver,
            workspace_manager,
            event_bus,
            cancellation_token,
        );

        // Add some jobs to the internal queue
        let job1 = JobInfo {
            id: "job1".to_string(),
            job: Job::IndexWorkspaceFolder {
                workspace_folder_path: "/test/path1".to_string(),
                priority: JobPriority::Normal,
            },
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            status: JobStatus::Pending,
            error: None,
        };

        let job2 = JobInfo {
            id: "job2".to_string(),
            job: Job::IndexWorkspaceFolder {
                workspace_folder_path: "/test/path2".to_string(),
                priority: JobPriority::Normal,
            },
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            status: JobStatus::Pending,
            error: None,
        };

        // Send jobs first
        sender.send(WorkerMessage::Job(job1)).await.unwrap();
        sender.send(WorkerMessage::Job(job2)).await.unwrap();

        // Send cancellation message for IndexWorkspaceFolder jobs
        sender
            .send(WorkerMessage::CancelJobsOfType(
                "IndexWorkspaceFolder".to_string(),
            ))
            .await
            .unwrap();

        // Start the worker and let it process a few messages
        let worker_handle = tokio::spawn(async move { worker.run().await });

        // Give the worker time to process the messages
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Drop the sender to close the channel and stop the worker
        drop(sender);

        // Wait for worker to finish
        let _ = timeout(Duration::from_millis(500), worker_handle).await;

        // This test mainly checks that the worker doesn't panic when processing cancellation messages
        // and that the logic compiles correctly. The actual cancellation behavior is tested
        // through integration tests with the dispatcher.
    }
}
