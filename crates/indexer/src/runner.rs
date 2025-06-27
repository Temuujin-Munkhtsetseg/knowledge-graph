use crate::indexer::{IndexingConfig, RepositoryIndexer};
use crate::project::source::GitaliskFileSource;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Instant;
use workspace_manager::WorkspaceManager;

pub fn run_client_indexer<F>(workspace_path: PathBuf, threads: usize, mut progress: F) -> Result<()>
where
    F: FnMut(&str),
{
    let start_time = Instant::now();
    progress("🚀 Starting knowledge graph indexing...");
    progress(&format!("📂 Workspace: {}", workspace_path.display()));
    progress(&format!(
        "🧵 Threads: {}",
        if threads == 0 {
            num_cpus::get()
        } else {
            threads
        }
    ));

    progress("🔍 Initializing workspace manager...");
    let workspace_manager = WorkspaceManager::new_system_default()
        .map_err(|e| anyhow::anyhow!("Failed to create workspace manager: {}", e))?;

    progress("📚 Discovering and registering workspace...");
    let discovery_result = workspace_manager
        .register_workspace_folder(&workspace_path)
        .map_err(|e| anyhow::anyhow!("Failed to register workspace: {}", e))?;

    progress(&format!(
        "📚 Found {} repositories in workspace",
        discovery_result.projects_found.len()
    ));

    if discovery_result.projects_found.is_empty() {
        progress("ℹ️ No repositories found in workspace");
        return Ok(());
    }

    let config = IndexingConfig {
        worker_threads: threads,
        max_file_size: 5_000_000,
        respect_gitignore: true,
    };

    progress("⚙️ Indexing configuration:");
    progress(&format!(
        "  • Worker threads: {}",
        if config.worker_threads == 0 {
            num_cpus::get()
        } else {
            config.worker_threads
        }
    ));
    progress(&format!(
        "  • Max file size: {} MB",
        config.max_file_size / 1_000_000
    ));
    progress(&format!(
        "  • Respect .gitignore: {}",
        config.respect_gitignore
    ));

    let mut total_files_processed = 0;
    let mut total_files_skipped = 0;
    let mut total_files_errored = 0;
    let mut total_errors = Vec::new();
    let mut repositories_processed = 0;

    let workspace_folder_path = discovery_result.workspace_folder_path;
    let projects = discovery_result.projects_found;

    for (index, project_discovery) in projects.iter().enumerate() {
        let repo_progress = (index + 1) as f64 / projects.len() as f64 * 100.0;
        progress(&format!(
            "\n📖 Processing repository {}/{} ({:.1}%): {}",
            index + 1,
            projects.len(),
            repo_progress,
            project_discovery.project_path
        ));

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

        progress(&format!(
            "  📁 Parquet directory (workspace-managed): {parquet_directory}"
        ));
        progress(&format!(
            "  🗄️ Database path (workspace-managed): {database_path}"
        ));

        let indexer = RepositoryIndexer::new(repo_name.clone(), project_info.project_path.clone());
        let file_source = GitaliskFileSource::new(project_info.repository);

        progress("  🚀 Starting full processing (index → analyze → write → database)...");

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

                progress(&format!(
                    "  ✅ Completed repository {}/{}: {} processed, {} skipped, {} errors in {:?}",
                    repositories_processed,
                    projects.len(),
                    result.total_files_processed,
                    result.total_files_skipped,
                    result.total_files_errored,
                    result.total_processing_time
                ));

                if let Some(ref graph_data) = result.graph_data {
                    progress(&format!(
                        "  📊 Graph data: {} files, {} definitions, {} relationships",
                        graph_data.file_nodes.len(),
                        graph_data.definition_nodes.len(),
                        graph_data.file_definition_relationships.len()
                    ));
                }

                if let Some(ref writer_result) = result.writer_result {
                    progress(&format!(
                        "  📁 Parquet files: {} files written to {}",
                        writer_result.files_written.len(),
                        parquet_directory
                    ));
                }

                if let Some(ref db_path) = result.database_path {
                    if result.database_loaded {
                        progress(&format!(
                            "  🗄️ Database: Successfully loaded graph data into {db_path}"
                        ));
                    } else {
                        progress(&format!(
                            "  ⚠️ Database: Failed to load graph data into {db_path}"
                        ));
                    }
                }

                if !result.errors.is_empty() {
                    progress(&format!(
                        "  ⚠️ Errors encountered ({} total):",
                        result.errors.len()
                    ));
                    for (file_path, error_msg) in result.errors.iter().take(5) {
                        progress(&format!("    • {file_path}: {error_msg}"));
                    }
                    if result.errors.len() > 5 {
                        progress(&format!(
                            "    • ... and {} more errors",
                            result.errors.len() - 5
                        ));
                    }
                }

                total_files_processed += result.total_files_processed;
                total_files_skipped += result.total_files_skipped;
                total_files_errored += result.total_files_errored;
                total_errors.extend(result.errors);

                let overall_progress =
                    repositories_processed as f64 / projects.len() as f64 * 100.0;
                progress(&format!(
                    "  📊 Overall progress: {:.1}% ({}/{} repositories completed)",
                    overall_progress,
                    repositories_processed,
                    projects.len()
                ));
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

                eprintln!("  ❌ Failed to index repository '{repo_name}': {error_msg}");
                continue;
            }
        }
    }

    let total_time = start_time.elapsed();
    progress(&format!("\n🎉 Indexing completed in {total_time:?}"));
    progress("📊 Summary:");
    progress(&format!(
        "  • Repositories processed: {repositories_processed}"
    ));
    progress(&format!("  • Files processed: {total_files_processed}"));
    progress(&format!("  • Files skipped: {total_files_skipped}"));
    progress(&format!("  • Files with errors: {total_files_errored}"));

    if total_files_processed > 0 {
        let files_per_sec = total_files_processed as f64 / total_time.as_secs_f64();
        progress(&format!(
            "  • Processing rate: {files_per_sec:.1} files/second"
        ));
    }

    if !total_errors.is_empty() && total_errors.len() <= 10 {
        progress("  • Recent errors:");
        for (file_path, error_msg) in total_errors.iter().take(10) {
            progress(&format!("    • {file_path}: {error_msg}"));
        }
        if total_errors.len() > 10 {
            progress(&format!(
                "    • ... and {} more errors",
                total_errors.len() - 10
            ));
        }
    }

    progress("\n📈 Workspace status:");
    let workspace_info = workspace_manager
        .get_workspace_folder_info(&workspace_folder_path)
        .ok_or_else(|| anyhow::anyhow!("Failed to get workspace info"))?;

    progress(&format!(
        "  • Workspace: {}",
        workspace_info.workspace_folder_path
    ));
    progress(&format!(
        "  • Total projects: {}",
        workspace_info.project_count
    ));
    progress(&format!("  • Status: {:?}", workspace_info.status));

    // Show data directory information
    if let Ok(data_info) = workspace_manager.get_data_directory_info() {
        progress(&format!(
            "\n📁 Data directory: {}",
            data_info.root_path.display()
        ));
        progress(&format!(
            "  • Total size: {}",
            data_info.format_total_size()
        ));
        progress(&format!(
            "  • Workspace folders: {}",
            data_info.workspace_folder_count
        ));
    }

    Ok(())
}
