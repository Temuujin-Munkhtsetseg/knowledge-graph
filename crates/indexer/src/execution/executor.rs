use crate::indexer::{IndexingConfig, RepositoryIndexer};
use crate::project::source::GitaliskFileSource;
use anyhow::Result;
use chrono::Utc;
use event_bus::types::project_info::to_ts_project_info;
use event_bus::types::workspace_folder::to_ts_workspace_folder_info;
use event_bus::{
    EventBus, GkgEvent, ProjectIndexingCompleted, ProjectIndexingEvent, ProjectIndexingFailed,
    ProjectIndexingStarted, WorkspaceIndexingCompleted, WorkspaceIndexingEvent,
    WorkspaceIndexingStarted,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::error;
use workspace_manager::{Status, WorkspaceManager};

pub struct IndexingExecutor {
    event_bus: Arc<EventBus>,
    workspace_manager: Arc<WorkspaceManager>,
    config: IndexingConfig,
}

impl IndexingExecutor {
    pub fn new(
        workspace_manager: Arc<WorkspaceManager>,
        event_bus: Arc<EventBus>,
        config: IndexingConfig,
    ) -> Self {
        Self {
            workspace_manager,
            event_bus,
            config,
        }
    }

    pub fn execute_workspace_indexing(
        &mut self,
        workspace_folder_path: PathBuf,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<()> {
        self.check_cancellation(&cancellation_token, "before starting")?;

        let workspace_folder_path_str = workspace_folder_path.to_string_lossy().to_string();

        let projects = self
            .workspace_manager
            .list_projects_in_workspace(&workspace_folder_path_str);

        let workspace_folder_info = self
            .workspace_manager
            .get_workspace_folder_info(&workspace_folder_path_str)
            .ok_or_else(|| anyhow::anyhow!("Workspace folder not found"))?;

        if projects.is_empty() {
            self.event_bus.send(&GkgEvent::WorkspaceIndexing(
                WorkspaceIndexingEvent::Completed(WorkspaceIndexingCompleted {
                    workspace_folder_info: to_ts_workspace_folder_info(&workspace_folder_info),
                    projects_indexed: projects.iter().map(|p| p.project_path.clone()).collect(),
                    completed_at: Utc::now(),
                }),
            ));
            return Ok(());
        }
        self.event_bus.send(&GkgEvent::WorkspaceIndexing(
            WorkspaceIndexingEvent::Started(WorkspaceIndexingStarted {
                workspace_folder_info: to_ts_workspace_folder_info(&workspace_folder_info),
                projects_to_process: projects.iter().map(|p| p.project_path.clone()).collect(),
                started_at: Utc::now(),
            }),
        ));

        for project_discovery in projects.iter() {
            self.check_cancellation(&cancellation_token, "during project iteration")?;

            match self.execute_project_indexing(
                &workspace_folder_path_str,
                &project_discovery.project_path,
                cancellation_token.clone(),
            ) {
                Ok(_) => {
                    // Event sent inside process_single_project
                }
                Err(e) => {
                    let error_msg = format!("Failed to index repository: {e}");
                    self.mark_project_status(
                        &workspace_folder_path_str,
                        &project_discovery.project_path,
                        Status::Error,
                        Some(error_msg.clone()),
                    )?;
                    self.event_bus.send(&GkgEvent::WorkspaceIndexing(
                        WorkspaceIndexingEvent::Project(ProjectIndexingEvent::Failed(
                            ProjectIndexingFailed {
                                project_info: to_ts_project_info(project_discovery),
                                error: error_msg.clone(),
                                failed_at: Utc::now(),
                            },
                        )),
                    ));
                    error!(
                        "  ❌ Failed to index repository '{}': {}",
                        &project_discovery.project_path, error_msg
                    );
                    continue;
                }
            }
        }

        self.event_bus.send(&GkgEvent::WorkspaceIndexing(
            WorkspaceIndexingEvent::Completed(WorkspaceIndexingCompleted {
                workspace_folder_info: to_ts_workspace_folder_info(&workspace_folder_info),
                projects_indexed: projects.iter().map(|p| p.project_path.clone()).collect(),
                completed_at: Utc::now(),
            }),
        ));

        Ok(())
    }

    // TODO: abstract this into its own executor
    // So that the server side, who cannot use `gitalisk` or the `workspace-manager`
    // can use this executor to index projects.
    pub fn execute_project_indexing(
        &mut self,
        workspace_folder_path: &str,
        project_path: &str,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<()> {
        self.check_cancellation(&cancellation_token, "before starting")?;

        self.mark_project_status(workspace_folder_path, project_path, Status::Indexing, None)?;

        let project_info = self
            .workspace_manager
            .get_project_info(workspace_folder_path, project_path)
            .ok_or_else(|| anyhow::anyhow!("Project not found"))?;

        self.event_bus.send(&GkgEvent::WorkspaceIndexing(
            WorkspaceIndexingEvent::Project(ProjectIndexingEvent::Started(
                ProjectIndexingStarted {
                    project_info: to_ts_project_info(&project_info),
                    started_at: Utc::now(),
                },
            )),
        ));

        let parquet_directory = project_info.parquet_directory.to_string_lossy();
        let database_path = project_info.database_path.to_string_lossy();
        let repo_name = std::path::Path::new(&project_info.project_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        let indexer = RepositoryIndexer::new(repo_name, project_info.project_path.clone());
        let file_source = GitaliskFileSource::new(project_info.repository.clone());

        match indexer.process_files_full_with_database(
            file_source,
            &self.config,
            &parquet_directory,
            Some(&database_path),
        ) {
            Ok(_) => {
                self.check_cancellation(&cancellation_token, "after indexing completed")?;
                self.mark_project_status(
                    workspace_folder_path,
                    project_path,
                    Status::Indexed,
                    None,
                )?;
                self.event_bus.send(&GkgEvent::WorkspaceIndexing(
                    WorkspaceIndexingEvent::Project(ProjectIndexingEvent::Completed(
                        ProjectIndexingCompleted {
                            project_info: to_ts_project_info(&project_info),
                            completed_at: Utc::now(),
                        },
                    )),
                ));
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to index project: {e}");
                self.mark_project_status(
                    workspace_folder_path,
                    project_path,
                    Status::Error,
                    Some(error_msg.clone()),
                )?;
                self.event_bus.send(&GkgEvent::WorkspaceIndexing(
                    WorkspaceIndexingEvent::Project(ProjectIndexingEvent::Failed(
                        ProjectIndexingFailed {
                            project_info: to_ts_project_info(&project_info),
                            error: error_msg.clone(),
                            failed_at: Utc::now(),
                        },
                    )),
                ));
                Err(anyhow::anyhow!("Project indexing failed: {error_msg}"))
            }
        }
    }
    pub fn mark_workspace_status(&self, workspace_folder_path: &str, status: Status) -> Result<()> {
        self.workspace_manager
            .update_workspace_folder_status(workspace_folder_path, Some(status))
            .map_err(|e| anyhow::anyhow!("Failed to mark workspace as indexing: {}", e))
            .map(|_| ())
    }

    pub fn mark_project_status(
        &self,
        workspace_folder_path: &str,
        project_path: &str,
        status: Status,
        error_message: Option<String>,
    ) -> Result<()> {
        self.workspace_manager
            .update_project_indexing_status(
                workspace_folder_path,
                project_path,
                status,
                error_message,
            )
            .map_err(|e| anyhow::anyhow!("Failed to mark project as indexing: {}", e))
            .map(|_| ())
    }

    pub fn check_cancellation(
        &self,
        cancellation_token: &Option<CancellationToken>,
        stage: &str,
    ) -> Result<()> {
        if let Some(token) = cancellation_token {
            if token.is_cancelled() {
                return Err(anyhow::anyhow!("Operation cancelled {}", stage));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::config::IndexingConfigBuilder;
    use event_bus::{EventBus, GkgEvent, ProjectIndexingEvent, WorkspaceIndexingEvent};
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;
    use workspace_manager::Status;

    fn create_test_workspace_manager() -> (Arc<WorkspaceManager>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_manager =
            Arc::new(WorkspaceManager::new_with_directory(temp_dir.path().to_path_buf()).unwrap());
        (workspace_manager, temp_dir)
    }

    fn create_test_git_repo(path: &std::path::Path) {
        std::fs::create_dir_all(path).unwrap();

        std::process::Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .unwrap();

        std::fs::write(path.join("README.md"), "# Test Repo").unwrap();
        std::fs::write(path.join("main.rb"), "puts 'Hello, World!'").unwrap();

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    fn create_test_workspace() -> (Arc<WorkspaceManager>, TempDir, PathBuf) {
        let (workspace_manager, temp_dir) = create_test_workspace_manager();

        let workspace_path = temp_dir.path().join("test_workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        for i in 1..=2 {
            let project_path = workspace_path.join(format!("test_project{i}"));
            create_test_git_repo(&project_path);
        }

        (workspace_manager, temp_dir, workspace_path)
    }

    fn create_test_workspace_with_projects(
        project_count: usize,
    ) -> (Arc<WorkspaceManager>, TempDir, PathBuf) {
        let (workspace_manager, temp_dir) = create_test_workspace_manager();

        let workspace_path = temp_dir.path().join("test_workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        for i in 1..=project_count {
            let project_path = workspace_path.join(format!("test_project{i}"));
            create_test_git_repo(&project_path);
        }

        (workspace_manager, temp_dir, workspace_path)
    }

    #[tokio::test]
    async fn test_run_workspace_indexing_empty_workspace() {
        let (workspace_manager, temp_dir) = create_test_workspace_manager();
        let event_bus = Arc::new(EventBus::new());
        let mut execution = IndexingExecutor::new(
            workspace_manager,
            event_bus,
            IndexingConfigBuilder::build(4),
        );

        let empty_workspace = temp_dir.path().join("empty_workspace");
        std::fs::create_dir_all(&empty_workspace).unwrap();

        let result = execution.execute_workspace_indexing(empty_workspace, None);

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_workspace_indexing_successful() {
        let (workspace_manager, _temp_dir, workspace_path) = create_test_workspace_with_projects(1);
        let event_bus = Arc::new(EventBus::new());
        let mut execution = IndexingExecutor::new(
            Arc::clone(&workspace_manager),
            Arc::clone(&event_bus),
            IndexingConfigBuilder::build(4),
        );

        let mut event_receiver = event_bus.subscribe();

        workspace_manager
            .register_workspace_folder(&workspace_path)
            .unwrap();

        // Use canonicalized path since workspace manager stores canonicalized paths
        let canonical_workspace_path = workspace_path.canonicalize().unwrap();
        let result = execution.execute_workspace_indexing(canonical_workspace_path.clone(), None);

        assert!(result.is_ok());

        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        assert!(!events.is_empty(), "Should have received events");

        let completed_event = events.iter().find(|event| {
            matches!(
                event,
                GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Completed(_))
            )
        });

        assert!(
            completed_event.is_some(),
            "Should have received WorkspaceIndexingCompleted event"
        );

        if let Some(GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Completed(completed))) =
            completed_event
        {
            let expected_path = canonical_workspace_path.to_string_lossy().to_string();
            assert_eq!(
                completed.workspace_folder_info.workspace_folder_path,
                expected_path
            );
            assert!(completed.completed_at <= chrono::Utc::now());
        }
    }

    #[tokio::test]
    async fn test_run_workspace_indexing_with_projects_events() {
        let (workspace_manager, _temp_dir, workspace_path) = create_test_workspace_with_projects(2);
        let event_bus = Arc::new(EventBus::new());
        let mut execution = IndexingExecutor::new(
            Arc::clone(&workspace_manager),
            Arc::clone(&event_bus),
            IndexingConfigBuilder::build(4),
        );

        let mut event_receiver = event_bus.subscribe();

        workspace_manager
            .register_workspace_folder(&workspace_path)
            .unwrap();

        let canonical_workspace_path = workspace_path.canonicalize().unwrap();
        let canonical_workspace_path_str = canonical_workspace_path.to_string_lossy().to_string();

        let discovered_projects =
            workspace_manager.list_projects_in_workspace(&canonical_workspace_path_str);

        let result = execution.execute_workspace_indexing(canonical_workspace_path, None);
        assert!(result.is_ok());

        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        if discovered_projects.is_empty() {
            assert_eq!(events.len(), 1);
            assert!(matches!(
                events[0],
                GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Completed(_))
            ));
        } else {
            assert!(!events.is_empty(), "Should have received events");

            assert!(matches!(
                events[0],
                GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Started(_))
            ));

            assert!(matches!(
                events.last().unwrap(),
                GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Completed(_))
            ));

            let project_started_count = events
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Project(
                            ProjectIndexingEvent::Started(_)
                        ))
                    )
                })
                .count();

            let project_completed_count = events
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Project(
                            ProjectIndexingEvent::Completed(_)
                        ))
                    )
                })
                .count();

            assert_eq!(project_started_count, discovered_projects.len());
            assert_eq!(project_completed_count, discovered_projects.len());
        }
    }

    #[tokio::test]
    async fn test_run_project_indexing_successful() {
        let (workspace_manager, _temp_dir, workspace_path) = create_test_workspace_with_projects(1);
        let event_bus = Arc::new(EventBus::new());
        let mut execution = IndexingExecutor::new(
            Arc::clone(&workspace_manager),
            Arc::clone(&event_bus),
            IndexingConfigBuilder::build(4),
        );

        let mut event_receiver = event_bus.subscribe();

        workspace_manager
            .register_workspace_folder(&workspace_path)
            .unwrap();

        let workspace_str = workspace_path.to_string_lossy().to_string();
        let discovered_projects = workspace_manager.list_projects_in_workspace(&workspace_str);

        if discovered_projects.is_empty() {
            let result =
                execution.execute_project_indexing(&workspace_str, "nonexistent_project", None);
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Project not found")
            );

            let event = event_receiver.try_recv();

            if let Ok(event) = event {
                if let GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Project(
                    ProjectIndexingEvent::Failed(failed),
                )) = event
                {
                    assert_eq!(failed.project_info.project_path, "nonexistent_project");
                    assert!(failed.error.contains("Project not found"));
                } else {
                    panic!("Expected ProjectIndexingFailed event, got: {event:?}");
                }
            }
            return;
        }

        let project = &discovered_projects[0];
        let result =
            execution.execute_project_indexing(&workspace_str, &project.project_path, None);

        assert!(result.is_ok());

        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        assert!(events.len() >= 2, "Should have received at least 2 events");

        if let GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Project(
            ProjectIndexingEvent::Started(started),
        )) = &events[0]
        {
            assert_eq!(started.project_info.project_path, project.project_path);
        } else {
            panic!("Expected ProjectIndexingStarted event as first event");
        }

        if let GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Project(
            ProjectIndexingEvent::Completed(completed),
        )) = events.last().unwrap()
        {
            assert_eq!(completed.project_info.project_path, project.project_path);
        } else {
            panic!("Expected ProjectIndexingCompleted event as last event");
        }
    }

    #[tokio::test]
    async fn test_run_project_indexing_project_not_found() {
        let (workspace_manager, _temp_dir) = create_test_workspace_manager();
        let event_bus = Arc::new(EventBus::new());
        let mut execution = IndexingExecutor::new(
            workspace_manager,
            Arc::clone(&event_bus),
            IndexingConfigBuilder::build(4),
        );

        let mut event_receiver = event_bus.subscribe();

        let result = execution.execute_project_indexing(
            "nonexistent_workspace",
            "nonexistent_project",
            None,
        );

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Project not found")
        );

        // For a completely nonexistent workspace, no events are emitted
        // because the error occurs before event emission in mark_project_status
        let event = event_receiver.try_recv();
        // This should fail to receive an event since no workspace exists
        assert!(
            event.is_err(),
            "Should not have received any events for nonexistent workspace"
        );
    }

    #[tokio::test]
    async fn test_workspace_indexing_event_sequence() {
        let (workspace_manager, _temp_dir, workspace_path) = create_test_workspace_with_projects(2);
        let event_bus = Arc::new(EventBus::new());
        let mut execution = IndexingExecutor::new(
            Arc::clone(&workspace_manager),
            Arc::clone(&event_bus),
            IndexingConfigBuilder::build(4),
        );

        let mut event_receiver = event_bus.subscribe();

        workspace_manager
            .register_workspace_folder(&workspace_path)
            .unwrap();

        let canonical_workspace_path = workspace_path.canonicalize().unwrap();
        let canonical_workspace_path_str = canonical_workspace_path.to_string_lossy().to_string();

        let discovered_projects =
            workspace_manager.list_projects_in_workspace(&canonical_workspace_path_str);

        let _result = execution.execute_workspace_indexing(canonical_workspace_path, None);

        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        if discovered_projects.is_empty() {
            assert_eq!(events.len(), 1);
            assert!(matches!(
                events[0],
                GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Completed(_))
            ));
        } else {
            assert!(!events.is_empty());

            let workspace_started_events = events
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Started(_))
                    )
                })
                .count();
            let workspace_completed_events = events
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Completed(_))
                    )
                })
                .count();

            assert_eq!(
                workspace_started_events, 1,
                "Should have exactly one workspace started event"
            );
            assert_eq!(
                workspace_completed_events, 1,
                "Should have exactly one workspace completed event"
            );

            let started_pos = events.iter().position(|e| {
                matches!(
                    e,
                    GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Started(_))
                )
            });
            let completed_pos = events.iter().position(|e| {
                matches!(
                    e,
                    GkgEvent::WorkspaceIndexing(WorkspaceIndexingEvent::Completed(_))
                )
            });

            if let (Some(started), Some(completed)) = (started_pos, completed_pos) {
                assert!(
                    started < completed,
                    "Started event should come before completed event"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_run_workspace_indexing_cancellation_early() {
        let (workspace_manager, _temp_dir, workspace_path) = create_test_workspace();
        let event_bus = Arc::new(EventBus::new());
        let mut execution = IndexingExecutor::new(
            workspace_manager,
            event_bus,
            IndexingConfigBuilder::build(4),
        );

        let token = CancellationToken::new();
        token.cancel();

        let result = execution.execute_workspace_indexing(workspace_path, Some(token));

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Operation cancelled before starting")
        );
    }

    #[tokio::test]
    async fn test_run_project_indexing_cancellation_early() {
        let (workspace_manager, _temp_dir, _workspace_path) = create_test_workspace();
        let event_bus = Arc::new(EventBus::new());
        let mut execution = IndexingExecutor::new(
            workspace_manager,
            Arc::clone(&event_bus),
            IndexingConfigBuilder::build(4),
        );

        let token = CancellationToken::new();
        token.cancel();

        let result =
            execution.execute_project_indexing("some_workspace", "some_project", Some(token));

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Operation cancelled before starting")
        );
    }

    #[tokio::test]
    async fn test_mark_status_invalid_workspace() {
        let (workspace_manager, _temp_dir) = create_test_workspace_manager();
        let event_bus = Arc::new(EventBus::new());
        let execution = IndexingExecutor::new(
            workspace_manager,
            event_bus,
            IndexingConfigBuilder::build(4),
        );

        let result = execution.mark_workspace_status("nonexistent", Status::Indexing);
        assert!(result.is_err());

        let result =
            execution.mark_project_status("nonexistent", "nonexistent", Status::Indexing, None);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_cancellation_not_cancelled() {
        let (workspace_manager, _temp_dir) = create_test_workspace_manager();
        let event_bus = Arc::new(EventBus::new());
        let execution = IndexingExecutor::new(
            workspace_manager,
            event_bus,
            IndexingConfigBuilder::build(4),
        );

        let token = CancellationToken::new();
        let result = execution.check_cancellation(&Some(token), "test stage");
        assert!(result.is_ok());

        let result = execution.check_cancellation(&None, "test stage");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_cancellation_cancelled() {
        let (workspace_manager, _temp_dir) = create_test_workspace_manager();
        let event_bus = Arc::new(EventBus::new());
        let execution = IndexingExecutor::new(
            workspace_manager,
            event_bus,
            IndexingConfigBuilder::build(4),
        );

        let token = CancellationToken::new();
        token.cancel();

        let result = execution.check_cancellation(&Some(token), "test stage");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Operation cancelled test stage")
        );
    }
}
