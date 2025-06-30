use crate::indexer::{IndexingConfig, RepositoryIndexer};
use crate::project::source::GitaliskFileSource;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tracing::{Level, error, info, warn};
use workspace_manager::WorkspaceManager;

fn progress_with_tracing<F>(message: &str, progress: &mut F, level: Level)
where
    F: FnMut(&str),
{
    progress(message);
    match level {
        Level::INFO => info!("{message}"),
        Level::WARN => warn!("{message}"),
        Level::ERROR => error!("{message}"),
        _ => info!("{message}"),
    }
}

pub fn run_client_indexer<F>(
    workspace_manager: Arc<WorkspaceManager>,
    workspace_path: PathBuf,
    threads: usize,
    mut progress: F,
) -> Result<()>
where
    F: FnMut(&str),
{
    let start_time = Instant::now();
    progress_with_tracing(
        "🚀 Starting knowledge graph indexing...",
        &mut progress,
        Level::INFO,
    );
    progress_with_tracing(
        &format!("📂 Workspace: {}", workspace_path.display()),
        &mut progress,
        Level::INFO,
    );
    progress_with_tracing(
        &format!(
            "🧵 Threads: {}",
            if threads == 0 {
                num_cpus::get()
            } else {
                threads
            }
        ),
        &mut progress,
        Level::INFO,
    );

    progress_with_tracing(
        "🔍 Initializing workspace manager...",
        &mut progress,
        Level::INFO,
    );

    progress_with_tracing(
        "📚 Discovering and registering workspace...",
        &mut progress,
        Level::INFO,
    );
    let discovery_result = workspace_manager
        .register_workspace_folder(&workspace_path)
        .map_err(|e| anyhow::anyhow!("Failed to register workspace: {}", e))?;

    progress_with_tracing(
        &format!(
            "📚 Found {} repositories in workspace",
            discovery_result.project_count
        ),
        &mut progress,
        Level::INFO,
    );

    if discovery_result.project_count == 0 {
        progress_with_tracing(
            "ℹ️ No repositories found in workspace",
            &mut progress,
            Level::INFO,
        );
        return Ok(());
    }

    let config = IndexingConfig {
        worker_threads: threads,
        max_file_size: 5_000_000,
        respect_gitignore: true,
    };

    progress_with_tracing("⚙️ Indexing configuration:", &mut progress, Level::INFO);
    progress_with_tracing(
        &format!(
            "  • Worker threads: {}",
            if config.worker_threads == 0 {
                num_cpus::get()
            } else {
                config.worker_threads
            }
        ),
        &mut progress,
        Level::INFO,
    );
    progress_with_tracing(
        &format!("  • Max file size: {} MB", config.max_file_size / 1_000_000),
        &mut progress,
        Level::INFO,
    );
    progress_with_tracing(
        &format!("  • Respect .gitignore: {}", config.respect_gitignore),
        &mut progress,
        Level::INFO,
    );

    let mut total_files_processed = 0;
    let mut total_files_skipped = 0;
    let mut total_files_errored = 0;
    let mut total_errors = Vec::new();
    let mut repositories_processed = 0;

    let workspace_folder_path = discovery_result.workspace_folder_path;
    let projects = workspace_manager.list_projects_in_workspace(&workspace_folder_path);

    for (index, project_discovery) in projects.iter().enumerate() {
        let repo_progress = (index + 1) as f64 / projects.len() as f64 * 100.0;
        progress_with_tracing(
            &format!(
                "\n📖 Processing repository {}/{} ({:.1}%): {}",
                index + 1,
                projects.len(),
                repo_progress,
                project_discovery.project_path
            ),
            &mut progress,
            Level::INFO,
        );

        workspace_manager
            .mark_project_indexing(&workspace_folder_path, &project_discovery.project_path)
            .map_err(|e| anyhow::anyhow!("Failed to mark project as indexing: {}", e))?;

        let project_info = workspace_manager
            .get_project_info(&workspace_folder_path, &project_discovery.project_path)
            .ok_or_else(|| anyhow::anyhow!("Project not found after registration"))?;

        let repo_name = project_info
            .project_path
            .split('/')
            .next_back()
            .unwrap_or("unknown")
            .to_string();

        // Use workspace manager's managed paths
        let parquet_directory = project_info.parquet_directory.to_string_lossy().to_string();
        let database_path = project_info.database_path.to_string_lossy().to_string();

        progress_with_tracing(
            &format!("  📁 Parquet directory (workspace-managed): {parquet_directory}"),
            &mut progress,
            Level::INFO,
        );
        progress_with_tracing(
            &format!("  🗄️ Database path (workspace-managed): {database_path}"),
            &mut progress,
            Level::INFO,
        );

        let indexer = RepositoryIndexer::new(repo_name.clone(), project_info.project_path.clone());
        let file_source = GitaliskFileSource::new(project_info.repository);

        progress_with_tracing(
            "  🚀 Starting full processing (index → analyze → write → database)...",
            &mut progress,
            Level::INFO,
        );

        match indexer.process_files_full_with_database(
            file_source,
            &config,
            &parquet_directory,
            Some(&database_path),
        ) {
            Ok(result) => {
                repositories_processed += 1;

                workspace_manager
                    .mark_project_indexed(&workspace_folder_path, &project_discovery.project_path)
                    .map_err(|e| anyhow::anyhow!("Failed to mark project as indexed: {}", e))?;

                progress_with_tracing(
                    &format!(
                        "  ✅ Completed repository {}/{}: {} processed, {} skipped, {} errors in {:?}",
                        repositories_processed,
                        projects.len(),
                        result.total_files_processed,
                        result.total_files_skipped,
                        result.total_files_errored,
                        result.total_processing_time
                    ),
                    &mut progress,
                    Level::INFO,
                );

                if let Some(ref graph_data) = result.graph_data {
                    progress_with_tracing(
                        &format!(
                            "  📊 Graph data: {} files, {} definitions, {} relationships",
                            graph_data.file_nodes.len(),
                            graph_data.definition_nodes.len(),
                            graph_data.file_definition_relationships.len()
                        ),
                        &mut progress,
                        Level::INFO,
                    );
                }

                if let Some(ref writer_result) = result.writer_result {
                    progress_with_tracing(
                        &format!(
                            "  📁 Parquet files: {} files written to {}",
                            writer_result.files_written.len(),
                            parquet_directory
                        ),
                        &mut progress,
                        Level::INFO,
                    );
                }

                if let Some(ref db_path) = result.database_path {
                    if result.database_loaded {
                        progress_with_tracing(
                            &format!(
                                "  🗄️ Database: Successfully loaded graph data into {db_path}"
                            ),
                            &mut progress,
                            Level::INFO,
                        );
                    } else {
                        progress_with_tracing(
                            &format!("  ⚠️ Database: Failed to load graph data into {db_path}"),
                            &mut progress,
                            Level::WARN,
                        );
                    }
                }

                if !result.errors.is_empty() {
                    progress_with_tracing(
                        &format!("  ⚠️ Errors encountered ({} total):", result.errors.len()),
                        &mut progress,
                        Level::WARN,
                    );
                    for (file_path, error_msg) in result.errors.iter().take(5) {
                        progress_with_tracing(
                            &format!("    • {file_path}: {error_msg}"),
                            &mut progress,
                            Level::WARN,
                        );
                    }
                    if result.errors.len() > 5 {
                        progress_with_tracing(
                            &format!("    • ... and {} more errors", result.errors.len() - 5),
                            &mut progress,
                            Level::WARN,
                        );
                    }
                }

                total_files_processed += result.total_files_processed;
                total_files_skipped += result.total_files_skipped;
                total_files_errored += result.total_files_errored;
                total_errors.extend(result.errors);

                let overall_progress =
                    repositories_processed as f64 / projects.len() as f64 * 100.0;
                progress_with_tracing(
                    &format!(
                        "  📊 Overall progress: {:.1}% ({}/{} repositories completed)",
                        overall_progress,
                        repositories_processed,
                        projects.len()
                    ),
                    &mut progress,
                    Level::INFO,
                );
            }
            Err(e) => {
                let error_msg = format!("Failed to index repository: {e}");

                workspace_manager
                    .mark_project_error(
                        &workspace_folder_path,
                        &project_discovery.project_path,
                        error_msg.clone(),
                    )
                    .map_err(|e| anyhow::anyhow!("Failed to mark project error: {}", e))?;

                error!("  ❌ Failed to index repository '{repo_name}': {error_msg}");
                continue;
            }
        }
    }

    let total_time = start_time.elapsed();
    progress_with_tracing(
        &format!("\n🎉 Indexing completed in {total_time:?}"),
        &mut progress,
        Level::INFO,
    );
    progress_with_tracing("📊 Summary:", &mut progress, Level::INFO);
    progress_with_tracing(
        &format!("  • Repositories processed: {repositories_processed}"),
        &mut progress,
        Level::INFO,
    );
    progress_with_tracing(
        &format!("  • Files processed: {total_files_processed}"),
        &mut progress,
        Level::INFO,
    );
    progress_with_tracing(
        &format!("  • Files skipped: {total_files_skipped}"),
        &mut progress,
        Level::INFO,
    );
    progress_with_tracing(
        &format!("  • Files with errors: {total_files_errored}"),
        &mut progress,
        Level::INFO,
    );

    if total_files_processed > 0 {
        let files_per_sec = total_files_processed as f64 / total_time.as_secs_f64();
        progress_with_tracing(
            &format!("  • Processing rate: {files_per_sec:.1} files/second"),
            &mut progress,
            Level::INFO,
        );
    }

    if !total_errors.is_empty() && total_errors.len() <= 10 {
        progress_with_tracing("  • Recent errors:", &mut progress, Level::WARN);
        for (file_path, error_msg) in total_errors.iter().take(10) {
            progress_with_tracing(
                &format!("    • {file_path}: {error_msg}"),
                &mut progress,
                Level::WARN,
            );
        }
        if total_errors.len() > 10 {
            progress_with_tracing(
                &format!("    • ... and {} more errors", total_errors.len() - 10),
                &mut progress,
                Level::WARN,
            );
        }
    }

    progress_with_tracing("\n📈 Workspace status:", &mut progress, Level::INFO);
    let workspace_info = workspace_manager
        .get_workspace_folder_info(&workspace_folder_path)
        .ok_or_else(|| anyhow::anyhow!("Failed to get workspace info"))?;

    progress_with_tracing(
        &format!("  • Workspace: {}", workspace_info.workspace_folder_path),
        &mut progress,
        Level::INFO,
    );
    progress_with_tracing(
        &format!("  • Total projects: {}", workspace_info.project_count),
        &mut progress,
        Level::INFO,
    );
    progress_with_tracing(
        &format!("  • Status: {:?}", workspace_info.status),
        &mut progress,
        Level::INFO,
    );

    // Show data directory information
    if let Ok(data_info) = workspace_manager.get_data_directory_info() {
        progress_with_tracing(
            &format!("\n📁 Data directory: {}", data_info.root_path.display()),
            &mut progress,
            Level::INFO,
        );
        progress_with_tracing(
            &format!("  • Total size: {}", data_info.format_total_size()),
            &mut progress,
            Level::INFO,
        );
        progress_with_tracing(
            &format!(
                "  • Workspace folders: {}",
                data_info.workspace_folder_count
            ),
            &mut progress,
            Level::INFO,
        );
    }

    Ok(())
}
