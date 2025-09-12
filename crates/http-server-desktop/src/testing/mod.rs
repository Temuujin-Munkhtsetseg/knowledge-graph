use crate::AppState;
use database::kuzu::database::KuzuDatabase;
use event_bus::EventBus;
use indexer::execution::{config::IndexingConfigBuilder, executor::IndexingExecutor};
use std::{path::PathBuf, sync::Arc};
use tempfile::TempDir;
use tracing::info;
use workspace_manager::WorkspaceManager;

pub struct TestServerBuilderDependencies {
    pub workspace_manager: Option<Arc<WorkspaceManager>>,
    pub event_bus: Option<Arc<EventBus>>,
    pub database: Option<Arc<KuzuDatabase>>,
}

/// Build the test app state and return both AppState and TempDir.
/// The caller is responsible for keeping the TempDir alive for the duration of the test.
pub fn build_app_state(
    temp_data_dir: TempDir,
    workspace_folder_paths: Vec<PathBuf>,
    dependencies: Option<TestServerBuilderDependencies>,
) -> anyhow::Result<(AppState, TempDir)> {
    let workspace_manager = if let Some(deps) = &dependencies {
        deps.workspace_manager.clone().unwrap_or_else(|| {
            Arc::new(
                WorkspaceManager::new_with_directory(temp_data_dir.path().to_path_buf()).unwrap(),
            )
        })
    } else {
        Arc::new(WorkspaceManager::new_with_directory(temp_data_dir.path().to_path_buf()).unwrap())
    };

    for workspace_folder_path in workspace_folder_paths {
        let _ = workspace_manager.register_workspace_folder(workspace_folder_path.as_path());
    }

    let event_bus = if let Some(deps) = &dependencies {
        deps.event_bus
            .clone()
            .unwrap_or_else(|| Arc::new(EventBus::new()))
    } else {
        Arc::new(EventBus::new())
    };

    let database = if let Some(deps) = &dependencies {
        deps.database
            .clone()
            .unwrap_or_else(|| Arc::new(KuzuDatabase::new()))
    } else {
        Arc::new(KuzuDatabase::new())
    };

    let job_dispatcher = Arc::new(crate::queue::dispatch::JobDispatcher::new(
        workspace_manager.clone(),
        event_bus.clone(),
        database.clone(),
    ));

    let app_state = AppState {
        database,
        workspace_manager,
        event_bus,
        job_dispatcher,
    };

    Ok((app_state, temp_data_dir))
}

// Test helper function to index data for the test app state
// This writes to the file system, so it should be called after the temp dir is created
pub async fn index_data(app_state: &AppState, workspace_folder_paths: Vec<PathBuf>) {
    let threads = num_cpus::get();
    let config = IndexingConfigBuilder::build(threads);
    let mut executor = IndexingExecutor::new(
        app_state.database.clone(),
        app_state.workspace_manager.clone(),
        app_state.event_bus.clone(),
        config,
    );

    for workspace_folder_path in workspace_folder_paths {
        match executor
            .execute_workspace_indexing(workspace_folder_path.clone(), None)
            .await
        {
            Ok(_stats) => {
                info!("Successfully indexed workspace: {workspace_folder_path:?}");
            }
            Err(e) => {
                panic!("Failed to index workspace {workspace_folder_path:?}: {e}");
            }
        }
    }
}
