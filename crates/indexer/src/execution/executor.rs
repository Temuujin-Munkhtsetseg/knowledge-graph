use crate::indexer::{IndexingConfig, RepositoryIndexer};
use crate::parsing::changes::FileChanges;
use crate::project::source::GitaliskFileSource;
use crate::stats::{ProjectStatistics, WorkspaceStatistics, finalize_project_statistics};

use anyhow::Result;
use chrono::Utc;
use database::kuzu::database::KuzuDatabase;
use event_bus::types::project_info::to_ts_project_info;
use event_bus::types::workspace_folder::to_ts_workspace_folder_info;
use event_bus::{
    EventBus, GkgEvent, ProjectIndexingCompleted, ProjectIndexingEvent, ProjectIndexingFailed,
    ProjectIndexingStarted, ProjectReindexingCompleted, ProjectReindexingEvent,
    ProjectReindexingFailed, ProjectReindexingStarted, WorkspaceIndexingCompleted,
    WorkspaceIndexingEvent, WorkspaceIndexingStarted, WorkspaceReindexingCompleted,
    WorkspaceReindexingEvent, WorkspaceReindexingStarted,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use workspace_manager::{Status, WorkspaceManager};

pub struct IndexingExecutor {
    database: Arc<KuzuDatabase>,
    event_bus: Arc<EventBus>,
    workspace_manager: Arc<WorkspaceManager>,
    config: IndexingConfig,
}

impl IndexingExecutor {
    pub fn new(
        database: Arc<KuzuDatabase>,
        workspace_manager: Arc<WorkspaceManager>,
        event_bus: Arc<EventBus>,
        config: IndexingConfig,
    ) -> Self {
        Self {
            database,
            workspace_manager,
            event_bus,
            config,
        }
    }

    pub fn execute_workspace_indexing(
        &mut self,
        workspace_folder_path: PathBuf,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<WorkspaceStatistics> {
        let start_time = std::time::Instant::now();
        self.check_cancellation(&cancellation_token, "before starting")?;

        let workspace_folder_info = self
            .workspace_manager
            .get_or_register_workspace_folder(&workspace_folder_path)
            .map_err(|e| anyhow::anyhow!("Failed to get or register workspace folder: {}", e))?;

        let workspace_folder_path_str = &workspace_folder_info.workspace_folder_path;
        let projects = self
            .workspace_manager
            .list_projects_in_workspace(workspace_folder_path_str);

        if projects.is_empty() {
            self.event_bus.send(&GkgEvent::WorkspaceIndexing(
                WorkspaceIndexingEvent::Completed(WorkspaceIndexingCompleted {
                    workspace_folder_info: to_ts_workspace_folder_info(&workspace_folder_info),
                    projects_indexed: projects.iter().map(|p| p.project_path.clone()).collect(),
                    completed_at: Utc::now(),
                }),
            ));

            // Return empty statistics
            let indexing_duration = start_time.elapsed().as_secs_f64();
            return Ok(WorkspaceStatistics::new(
                workspace_folder_path_str.clone(),
                indexing_duration,
            ));
        }
        self.event_bus.send(&GkgEvent::WorkspaceIndexing(
            WorkspaceIndexingEvent::Started(WorkspaceIndexingStarted {
                workspace_folder_info: to_ts_workspace_folder_info(&workspace_folder_info),
                projects_to_process: projects.iter().map(|p| p.project_path.clone()).collect(),
                started_at: Utc::now(),
            }),
        ));

        // Create statistics collector
        let indexing_duration = start_time.elapsed().as_secs_f64();
        let mut workspace_stats =
            WorkspaceStatistics::new(workspace_folder_path_str.clone(), indexing_duration);

        for project_discovery in projects.iter() {
            self.check_cancellation(&cancellation_token, "during project iteration")?;

            match self.execute_project_indexing(
                workspace_folder_path_str,
                &project_discovery.project_path,
                cancellation_token.clone(),
            ) {
                Ok(project_stats) => {
                    // Event sent inside process_single_project
                    println!("Project reindexed: {}", &project_discovery.project_path);
                    workspace_stats.add_project(project_stats);
                }
                Err(e) => {
                    let error_msg = format!("Failed to index repository: {e}");
                    self.mark_project_status(
                        workspace_folder_path_str,
                        &project_discovery.project_path,
                        Status::Error,
                        Some(error_msg.clone()),
                    )?;
                    self.event_bus
                        .send(&GkgEvent::ProjectIndexing(ProjectIndexingEvent::Failed(
                            ProjectIndexingFailed {
                                project_info: to_ts_project_info(project_discovery),
                                error: error_msg.clone(),
                                failed_at: Utc::now(),
                            },
                        )));
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

        // Update duration after all processing
        workspace_stats.metadata.indexing_duration_seconds = start_time.elapsed().as_secs_f64();
        Ok(workspace_stats)
    }

    pub fn execute_workspace_reindexing(
        &mut self,
        workspace_folder_path: PathBuf,
        workspace_changes: Vec<PathBuf>,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<()> {
        self.check_cancellation(&cancellation_token, "before starting")?;

        let workspace_folder_info = self
            .workspace_manager
            .get_or_register_workspace_folder(&workspace_folder_path)
            .map_err(|e| anyhow::anyhow!("Failed to get or register workspace folder: {}", e))?;

        let workspace_folder_path_str = &workspace_folder_info.workspace_folder_path;
        let projects = self
            .workspace_manager
            .list_projects_in_workspace(workspace_folder_path_str);

        if projects.is_empty() {
            self.event_bus.send(&GkgEvent::WorkspaceReindexing(
                WorkspaceReindexingEvent::Completed(WorkspaceReindexingCompleted {
                    workspace_folder_info: to_ts_workspace_folder_info(&workspace_folder_info),
                    projects_indexed: projects.iter().map(|p| p.project_path.clone()).collect(),
                    completed_at: Utc::now(),
                }),
            ));
            return Ok(());
        }
        self.event_bus.send(&GkgEvent::WorkspaceReindexing(
            WorkspaceReindexingEvent::Started(WorkspaceReindexingStarted {
                workspace_folder_info: to_ts_workspace_folder_info(&workspace_folder_info),
                projects_to_process: projects.iter().map(|p| p.project_path.clone()).collect(),
                started_at: Utc::now(),
            }),
        ));

        for project_discovery in projects.iter() {
            self.check_cancellation(&cancellation_token, "during project iteration")?;

            // Filter changes to only those within this project's path
            let project_path = Path::new(&project_discovery.project_path);
            let project_changes: Vec<PathBuf> = workspace_changes
                .iter()
                .filter(|path| path.starts_with(project_path))
                .cloned()
                .collect();

            match self.execute_project_reindexing(
                workspace_folder_path_str,
                &project_discovery.project_path,
                project_changes,
                cancellation_token.clone(),
            ) {
                Ok(_) => {
                    // Event sent inside process_single_project
                    println!("Project reindexed: {}", &project_discovery.project_path);
                }
                Err(e) => {
                    let error_msg = format!("Failed to re-index repository: {e}");
                    self.mark_project_status(
                        workspace_folder_path_str,
                        &project_discovery.project_path,
                        Status::Error,
                        Some(error_msg.clone()),
                    )?;
                    self.event_bus.send(&GkgEvent::ProjectReindexing(
                        ProjectReindexingEvent::Failed(ProjectReindexingFailed {
                            project_info: to_ts_project_info(project_discovery),
                            error: error_msg.clone(),
                            failed_at: Utc::now(),
                        }),
                    ));
                    error!(
                        "  ❌ Failed to re-index repository '{}': {}",
                        &project_discovery.project_path, error_msg
                    );
                    continue;
                }
            }
        }

        self.event_bus.send(&GkgEvent::WorkspaceReindexing(
            WorkspaceReindexingEvent::Completed(WorkspaceReindexingCompleted {
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
    ) -> Result<ProjectStatistics> {
        self.check_cancellation(&cancellation_token, "before starting")?;

        self.mark_project_status(workspace_folder_path, project_path, Status::Indexing, None)?;

        let project_info = self
            .workspace_manager
            .get_project_info(workspace_folder_path, project_path)
            .ok_or_else(|| anyhow::anyhow!("Project not found"))?;

        self.event_bus
            .send(&GkgEvent::ProjectIndexing(ProjectIndexingEvent::Started(
                ProjectIndexingStarted {
                    project_info: to_ts_project_info(&project_info),
                    started_at: Utc::now(),
                },
            )));

        let parquet_directory = project_info.parquet_directory.to_string_lossy();
        let database_path = project_info.database_path.to_string_lossy();
        let repo_name = std::path::Path::new(&project_info.project_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        let indexer = RepositoryIndexer::new(repo_name.clone(), project_info.project_path.clone());
        let file_source = GitaliskFileSource::new(project_info.repository.clone());

        match indexer.process_files_full_with_database(
            &self.database,
            file_source,
            &self.config,
            &parquet_directory,
            &database_path,
        ) {
            Ok(project_stats) => {
                self.check_cancellation(&cancellation_token, "after re-indexing completed")?;
                self.mark_project_status(
                    workspace_folder_path,
                    project_path,
                    Status::Indexed,
                    None,
                )?;
                self.event_bus
                    .send(&GkgEvent::ProjectIndexing(ProjectIndexingEvent::Completed(
                        ProjectIndexingCompleted {
                            project_info: to_ts_project_info(&project_info),
                            completed_at: Utc::now(),
                        },
                    )));
                // Use finalize_project_statistics to build ProjectStatistics from written data
                let stats = finalize_project_statistics(
                    project_info.project_path.clone(),
                    project_info.project_path.clone(),
                    project_stats.total_processing_time,
                    project_stats
                        .graph_data
                        .as_ref()
                        .expect("graph_data should exist"),
                    project_stats
                        .writer_result
                        .as_ref()
                        .expect("writer_result should exist"),
                );
                Ok(stats)
            }
            Err(e) => {
                let error_msg = format!("Failed to re-index project: {e}");
                self.mark_project_status(
                    workspace_folder_path,
                    project_path,
                    Status::Error,
                    Some(error_msg.clone()),
                )?;
                self.event_bus
                    .send(&GkgEvent::ProjectIndexing(ProjectIndexingEvent::Failed(
                        ProjectIndexingFailed {
                            project_info: to_ts_project_info(&project_info),
                            error: error_msg.clone(),
                            failed_at: Utc::now(),
                        },
                    )));
                Err(anyhow::anyhow!("Project re-indexing failed: {error_msg}"))
            }
        }
    }

    pub fn execute_project_reindexing(
        &mut self,
        workspace_folder_path: &str,
        project_path: &str,
        project_changes: Vec<PathBuf>,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<()> {
        self.check_cancellation(&cancellation_token, "before starting")?;

        self.mark_project_status(
            workspace_folder_path,
            project_path,
            Status::Reindexing,
            None,
        )?;

        let project_info = self
            .workspace_manager
            .get_project_info(workspace_folder_path, project_path)
            .ok_or_else(|| anyhow::anyhow!("Project not found"))?;

        self.event_bus.send(&GkgEvent::ProjectReindexing(
            ProjectReindexingEvent::Started(ProjectReindexingStarted {
                project_info: to_ts_project_info(&project_info),
                started_at: Utc::now(),
            }),
        ));

        let parquet_directory = project_info.parquet_directory.to_string_lossy();
        let database_path = project_info.database_path.to_string_lossy();
        let repo_name = std::path::Path::new(&project_info.project_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();

        let changes_as_strs: Vec<String> = project_changes
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect();
        let mut indexer =
            RepositoryIndexer::new(repo_name.clone(), project_info.project_path.clone());
        let changes = FileChanges::from_watched_files(changes_as_strs);

        info!("Re-indexing project with changes: {:?}", changes);
        info!("Re-indexing project with path: {:?}", project_path);
        info!(
            "Re-indexing project with workspace folder path: {:?}",
            workspace_folder_path
        );
        info!(
            "Re-indexing project with database path: {:?}",
            database_path
        );
        info!(
            "Re-indexing project with parquet directory: {:?}",
            parquet_directory
        );
        info!("Re-indexing project with repo name: {:?}", repo_name);

        match indexer.reindex_repository(
            &self.database,
            changes,
            &self.config,
            &database_path,
            &parquet_directory,
        ) {
            Ok(_) => {
                self.check_cancellation(&cancellation_token, "after re-indexing completed")?;
                self.mark_project_status(
                    workspace_folder_path,
                    project_path,
                    Status::Indexed,
                    None,
                )?;
                self.event_bus.send(&GkgEvent::ProjectReindexing(
                    ProjectReindexingEvent::Completed(ProjectReindexingCompleted {
                        project_info: to_ts_project_info(&project_info),
                        completed_at: Utc::now(),
                    }),
                ));
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to re-index project: {e}");
                self.mark_project_status(
                    workspace_folder_path,
                    project_path,
                    Status::Error,
                    Some(error_msg.clone()),
                )?;
                self.event_bus.send(&GkgEvent::ProjectReindexing(
                    ProjectReindexingEvent::Failed(ProjectReindexingFailed {
                        project_info: to_ts_project_info(&project_info),
                        error: error_msg.clone(),
                        failed_at: Utc::now(),
                    }),
                ));
                Err(anyhow::anyhow!("Project re-indexing failed: {error_msg}"))
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
    use database::kuzu::service::NodeDatabaseService;
    use event_bus::{EventBus, GkgEvent, ProjectIndexingEvent, WorkspaceIndexingEvent};
    use kuzu::{Database, SystemConfig};
    use std::fs;
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

    /// Recursively copy a directory and all its contents
    fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
        if !dst.exists() {
            fs::create_dir_all(dst)?;
        }

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                copy_dir_all(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
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

        // Copy fixture files from the existing fixtures directory
        let fixtures_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures/test-repo");

        copy_dir_all(&fixtures_path, path).expect("Failed to copy fixture files");

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
        let database = Arc::new(KuzuDatabase::new());

        let mut execution = IndexingExecutor::new(
            database,
            workspace_manager,
            event_bus,
            IndexingConfigBuilder::build(4),
        );

        let empty_workspace = temp_dir.path().join("empty_workspace");
        std::fs::create_dir_all(&empty_workspace).unwrap();

        let result = execution.execute_workspace_indexing(empty_workspace, None);

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_workspace_indexing_successful() {
        let (workspace_manager, _temp_dir, workspace_path) = create_test_workspace_with_projects(1);
        let event_bus = Arc::new(EventBus::new());
        let database = Arc::new(KuzuDatabase::new());
        let mut execution = IndexingExecutor::new(
            database,
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
        let database = Arc::new(KuzuDatabase::new());
        let mut execution = IndexingExecutor::new(
            database,
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
                        GkgEvent::ProjectIndexing(ProjectIndexingEvent::Started(_))
                    )
                })
                .count();

            let project_completed_count = events
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        GkgEvent::ProjectIndexing(ProjectIndexingEvent::Completed(_))
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
        let database = Arc::new(KuzuDatabase::new());
        let mut execution = IndexingExecutor::new(
            database,
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
                if let GkgEvent::ProjectIndexing(ProjectIndexingEvent::Failed(failed)) = event {
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

        if let GkgEvent::ProjectIndexing(ProjectIndexingEvent::Started(started)) = &events[0] {
            assert_eq!(started.project_info.project_path, project.project_path);
        } else {
            panic!("Expected ProjectIndexingStarted event as first event");
        }

        if let GkgEvent::ProjectIndexing(ProjectIndexingEvent::Completed(completed)) =
            events.last().unwrap()
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
        let database = Arc::new(KuzuDatabase::new());
        let mut execution = IndexingExecutor::new(
            database,
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
        let database = Arc::new(KuzuDatabase::new());
        let mut execution = IndexingExecutor::new(
            database,
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
        let database = Arc::new(KuzuDatabase::new());
        let mut execution = IndexingExecutor::new(
            database,
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
        let database = Arc::new(KuzuDatabase::new());
        let mut execution = IndexingExecutor::new(
            database,
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
        let database = Arc::new(KuzuDatabase::new());
        let execution = IndexingExecutor::new(
            database,
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
        let database = Arc::new(KuzuDatabase::new());
        let execution = IndexingExecutor::new(
            database,
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
        let database = Arc::new(KuzuDatabase::new());
        let execution = IndexingExecutor::new(
            database,
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

    fn check_db_def_count(database_path: &Path, expected_definition_count: u32) {
        let database_instance = Database::new(database_path, SystemConfig::default())
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);
        match node_database_service.get_node_counts() {
            Ok(node_counts) => {
                assert_eq!(node_counts.definition_count, expected_definition_count);
            }
            Err(e) => {
                println!("Error getting node counts: {e}");
            }
        }
    }

    #[tokio::test]
    async fn test_run_workspace_reindexing_comprehensive() {
        let (workspace_manager, _temp_dir, workspace_path) = create_test_workspace_with_projects(3);
        let event_bus = Arc::new(EventBus::new());
        let database = Arc::new(KuzuDatabase::new());
        let mut execution = IndexingExecutor::new(
            database,
            Arc::clone(&workspace_manager),
            Arc::clone(&event_bus),
            IndexingConfigBuilder::build(4),
        );

        // Register workspace and get canonical path
        workspace_manager
            .register_workspace_folder(&workspace_path)
            .unwrap();
        let canonical_workspace_path = workspace_path.canonicalize().unwrap();
        let canonical_workspace_path_str = canonical_workspace_path.to_string_lossy().to_string();

        // Get discovered projects
        let discovered_projects =
            workspace_manager.list_projects_in_workspace(&canonical_workspace_path_str);
        assert!(
            !discovered_projects.is_empty(),
            "Should have discovered projects"
        );

        // FIRST: Perform initial workspace indexing to set up the database
        let initial_indexing_result =
            execution.execute_workspace_indexing(canonical_workspace_path.clone(), None);
        assert!(
            initial_indexing_result.is_ok(),
            "Initial workspace indexing should succeed"
        );

        // Get discovered projects after initial indexing
        let discovered_projects_after_initial_indexing =
            workspace_manager.list_projects_in_workspace(&canonical_workspace_path_str);
        assert!(
            !discovered_projects_after_initial_indexing.is_empty(),
            "Should have discovered projects after initial indexing"
        );

        // TODO: Actually verify the database has the correct data
        // Get first database path
        let database_path = discovered_projects_after_initial_indexing
            .first()
            .unwrap()
            .database_path
            .clone();
        check_db_def_count(&database_path, 90);

        // Clear any events from initial indexing and set up fresh event receiver
        let mut event_receiver = event_bus.subscribe();

        // Create file changes for different projects
        let mut workspace_changes = Vec::new();

        // Add changes for first project
        if let Some(project) = discovered_projects.first() {
            let project_path = std::path::Path::new(&project.project_path);
            workspace_changes.push(project_path.join("main.rb"));
            workspace_changes.push(project_path.join("new_file.rb"));
        }

        // Add changes for second project (if exists)
        if discovered_projects.len() > 1 {
            let project_path = std::path::Path::new(&discovered_projects[1].project_path);
            workspace_changes.push(project_path.join("README.md"));
        }

        // Add a change outside any project (should be ignored)
        workspace_changes.push(canonical_workspace_path.join("global_config.txt"));

        // Execute workspace reindexing
        let result = execution.execute_workspace_reindexing(
            canonical_workspace_path.clone(),
            workspace_changes.clone(),
            None,
        );

        assert!(result.is_ok(), "Workspace reindexing should succeed");

        check_db_def_count(&database_path, 90);

        // Collect all events
        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        assert!(!events.is_empty(), "Should have received events");

        // Verify workspace reindexing events
        let workspace_started_events = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    GkgEvent::WorkspaceReindexing(WorkspaceReindexingEvent::Started(_))
                )
            })
            .count();

        let workspace_completed_events = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    GkgEvent::WorkspaceReindexing(WorkspaceReindexingEvent::Completed(_))
                )
            })
            .count();

        assert_eq!(
            workspace_started_events, 1,
            "Should have exactly one workspace reindexing started event"
        );
        assert_eq!(
            workspace_completed_events, 1,
            "Should have exactly one workspace reindexing completed event"
        );

        // Verify project reindexing events
        let project_started_events = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    GkgEvent::ProjectReindexing(ProjectReindexingEvent::Started(_))
                )
            })
            .count();

        let project_completed_events = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    GkgEvent::ProjectReindexing(ProjectReindexingEvent::Completed(_))
                )
            })
            .count();

        // All projects get reindexed during workspace reindexing, regardless of changes
        // This is the intended behavior - workspace reindexing processes all projects
        assert_eq!(
            project_started_events,
            discovered_projects.len(),
            "Should have project started events for all projects in workspace"
        );
        assert_eq!(
            project_completed_events,
            discovered_projects.len(),
            "Should have project completed events for all projects in workspace"
        );

        // Verify event sequence
        let started_pos = events.iter().position(|e| {
            matches!(
                e,
                GkgEvent::WorkspaceReindexing(WorkspaceReindexingEvent::Started(_))
            )
        });
        let completed_pos = events.iter().position(|e| {
            matches!(
                e,
                GkgEvent::WorkspaceReindexing(WorkspaceReindexingEvent::Completed(_))
            )
        });

        if let (Some(started), Some(completed)) = (started_pos, completed_pos) {
            assert!(
                started < completed,
                "Workspace started event should come before completed event"
            );
        }

        // Verify workspace reindexing started event details
        if let Some(GkgEvent::WorkspaceReindexing(WorkspaceReindexingEvent::Started(started))) =
            events.iter().find(|e| {
                matches!(
                    e,
                    GkgEvent::WorkspaceReindexing(WorkspaceReindexingEvent::Started(_))
                )
            })
        {
            assert_eq!(
                started.workspace_folder_info.workspace_folder_path,
                canonical_workspace_path_str
            );
            assert_eq!(started.projects_to_process.len(), discovered_projects.len());
            assert!(started.started_at <= chrono::Utc::now());
        }

        // Verify workspace reindexing completed event details
        if let Some(GkgEvent::WorkspaceReindexing(WorkspaceReindexingEvent::Completed(completed))) =
            events.iter().find(|e| {
                matches!(
                    e,
                    GkgEvent::WorkspaceReindexing(WorkspaceReindexingEvent::Completed(_))
                )
            })
        {
            assert_eq!(
                completed.workspace_folder_info.workspace_folder_path,
                canonical_workspace_path_str
            );
            assert_eq!(completed.projects_indexed.len(), discovered_projects.len());
            assert!(completed.completed_at <= chrono::Utc::now());
        }

        // Verify project reindexing event details
        for event in events.iter() {
            if let GkgEvent::ProjectReindexing(ProjectReindexingEvent::Started(started)) = event {
                // Verify project info is correct
                let project = discovered_projects
                    .iter()
                    .find(|p| p.project_path == started.project_info.project_path)
                    .expect("Project should exist in discovered projects");

                assert_eq!(started.project_info.project_path, project.project_path);
                assert!(started.started_at <= chrono::Utc::now());
            }

            if let GkgEvent::ProjectReindexing(ProjectReindexingEvent::Completed(completed)) = event
            {
                // Verify project info is correct
                let project = discovered_projects
                    .iter()
                    .find(|p| p.project_path == completed.project_info.project_path)
                    .expect("Project should exist in discovered projects");

                assert_eq!(completed.project_info.project_path, project.project_path);
                assert!(completed.completed_at <= chrono::Utc::now());
            }
        }

        // Verify no failed events
        let failed_events = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    GkgEvent::WorkspaceReindexing(WorkspaceReindexingEvent::Failed(_))
                        | GkgEvent::ProjectReindexing(ProjectReindexingEvent::Failed(_))
                )
            })
            .count();

        assert_eq!(failed_events, 0, "Should have no failed events");

        // Verify project statuses after reindexing
        for project in discovered_projects.iter() {
            let project_info = workspace_manager
                .get_project_info(&canonical_workspace_path_str, &project.project_path)
                .expect("Project should exist");

            // Projects should be marked as indexed after successful reindexing
            assert_eq!(
                project_info.status,
                Status::Indexed,
                "Project should be marked as indexed after successful reindexing"
            );
        }
    }
}
