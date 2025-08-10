use anyhow::Result;
use indexer::execution::config::IndexingConfigBuilder;
use indexer::execution::executor::IndexingExecutor;
use indexer::stats::WorkspaceStatistics;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use tracing::{error, info};

use crate::utils::is_server_running;
use database::kuzu::database::KuzuDatabase;
use event_bus::EventBus;
use workspace_manager::WorkspaceManager;

fn handle_statistics_output(
    workspace_stats: &WorkspaceStatistics,
    stats_output: Option<Option<PathBuf>>,
) {
    if let Some(stats_path_option) = stats_output {
        // Save to file if path provided
        if let Some(stats_path) = stats_path_option {
            match workspace_stats.export_to_file(&stats_path) {
                Ok(_) => {
                    info!("Statistics saved to: {}", stats_path.display());
                }
                Err(e) => {
                    error!("Failed to save statistics: {e}");
                }
            }
        }

        // Display summary and top breakdowns when user requested stats
        info!("Indexing Summary:");
        info!("  - Total Projects: {}", workspace_stats.total_projects);
        info!(
            "  - Total Definitions: {}",
            workspace_stats.total_definitions
        );

        if !workspace_stats.projects.is_empty() {
            info!("Project Timing:");
            for project in &workspace_stats.projects {
                info!(
                    "  - {}: {:.2}s ({} files, {} definitions)",
                    project.project_name,
                    project.indexing_duration_seconds,
                    project.total_files,
                    project.total_definitions
                );
            }
        }

        if !workspace_stats.total_languages.is_empty() {
            info!("Language Breakdown:");
            let mut languages: Vec<(&String, &indexer::stats::LanguageSummary)> =
                workspace_stats.total_languages.iter().collect();
            languages.sort_by(|a, b| b.1.file_count.cmp(&a.1.file_count));

            for (language, summary) in languages.iter().take(10) {
                info!(
                    "  - {}: {} files, {} definitions",
                    language, summary.file_count, summary.definitions_count
                );
            }

            if languages.len() > 10 {
                info!("  ... and {} more languages", languages.len() - 10);
            }
        }
    }
}

pub async fn run(
    workspace_path: PathBuf,
    threads: usize,
    stats_output: Option<Option<PathBuf>>,
    workspace_manager: Arc<WorkspaceManager>,
    event_bus: Arc<EventBus>,
    database: Arc<KuzuDatabase>,
) -> Result<()> {
    if let Some(port) = is_server_running()? {
        error!(
            "Error: gkg server is running on port {port}. Please stop it to run indexing from the CLI."
        );
        process::exit(1);
    }

    // Subscribe to events; CLI frontend consumer is currently disabled.
    let mut rx = event_bus.subscribe();
    // TODO: implement CLI frontend consumer
    tokio::spawn(async move { while (rx.recv().await).is_ok() {} });

    let config = IndexingConfigBuilder::build(threads);
    let mut executor = IndexingExecutor::new(
        database.clone(),
        workspace_manager.clone(),
        event_bus,
        config,
    );

    let canonical_workspace_path = workspace_path.canonicalize()?;
    let start_time = std::time::Instant::now();

    match executor
        .execute_workspace_indexing(canonical_workspace_path.clone(), None)
        .await
    {
        Ok(workspace_stats) => {
            let indexing_duration = start_time.elapsed();
            info!(
                "✅ Workspace indexing completed in {:.2} seconds",
                indexing_duration.as_secs_f64()
            );

            handle_statistics_output(&workspace_stats, stats_output);
        }
        Err(e) => {
            error!("❌ Indexing failed: {e}");
            process::exit(1);
        }
    }

    Ok(())
}
