use anyhow::Result;
use clap::{Parser, Subcommand};
use gitalisk_core::workspace_folder::gitalisk_workspace::CoreGitaliskWorkspaceFolder;
use indexer::repository::{IndexingConfig, Repository};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(
    name = "gkg",
    author = "GitLab Inc.",
    version = "0.1.0",
    about = "GitLab Knowledge Graph CLI",
    long_about = "Creates a structured, queryable representation of code repositories."
)]
pub struct GkgCli {
    #[command(subcommand)]
    pub command: Commands,
}

impl GkgCli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Index repositories in a workspace
    Index {
        /// Directory to scan for repositories
        #[arg(default_value = ".")]
        workspace_path: PathBuf,

        /// Number of worker threads (0 means auto-detect based on CPU cores)
        #[arg(short, long, default_value_t = 0)]
        threads: usize,

        /// Output directory for Parquet files
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,

        /// Optional path to Kuzu database for loading graph data
        #[arg(short = 'd', long)]
        database: Option<PathBuf>,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
}

pub fn run_indexing(
    workspace_path: PathBuf,
    threads: usize,
    output_path: PathBuf,
    database_path: Option<PathBuf>,
) -> Result<()> {
    let start_time = Instant::now();
    println!("🚀 Starting knowledge graph indexing...");
    println!("📂 Workspace: {}", workspace_path.display());
    println!("📁 Output: {}", output_path.display());
    println!(
        "🧵 Threads: {}",
        if threads == 0 {
            num_cpus::get()
        } else {
            threads
        }
    );

    // Convert PathBuf to String for gitalisk and output path
    let workspace_path_str = workspace_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid workspace path: contains non-UTF8 characters"))?;

    let _output_path_str = output_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid output path: contains non-UTF8 characters"))?;

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&output_path)
        .map_err(|e| anyhow::anyhow!("Failed to create output directory: {}", e))?;

    // Create gitalisk workspace
    println!("🔍 Discovering repositories...");
    let gitalisk_workspace = CoreGitaliskWorkspaceFolder::new(workspace_path_str.to_string());

    // Discover repositories in the workspace
    let stats = gitalisk_workspace
        .index_repositories()
        .map_err(|e| anyhow::anyhow!("Failed to discover repositories: {}", e))?;

    println!("📚 Found {} repositories", stats.repo_count);

    if stats.repo_count == 0 {
        println!("ℹ️ No repositories found in workspace");
        return Ok(());
    }

    // Configure indexing
    let config = IndexingConfig {
        worker_threads: threads,
        file_extensions: vec![
            "rb".to_string(),
            "rake".to_string(),
            "gemspec".to_string(),
            "rbw".to_string(),
        ],
        max_file_size: 5_000_000, // 5MB
        respect_gitignore: true,
    };

    println!("⚙️ Indexing configuration:");
    println!(
        "  • Worker threads: {}",
        if config.worker_threads == 0 {
            num_cpus::get()
        } else {
            config.worker_threads
        }
    );
    println!("  • File extensions: {}", config.file_extensions.join(", "));
    println!("  • Max file size: {} MB", config.max_file_size / 1_000_000);
    println!("  • Respect .gitignore: {}", config.respect_gitignore);

    // Process each repository
    let mut total_files_processed = 0;
    let mut total_files_skipped = 0;
    let mut total_files_errored = 0;
    let mut total_errors = Vec::new();
    let mut repositories_processed = 0;
    let repos = gitalisk_workspace.get_repositories();

    for (index, gitalisk_repo) in repos.iter().enumerate() {
        let repo_progress = (index + 1) as f64 / repos.len() as f64 * 100.0;
        println!(
            "\n📖 Processing repository {}/{} ({:.1}%): {}",
            index + 1,
            repos.len(),
            repo_progress,
            gitalisk_repo.path
        );

        // Create Repository wrapper
        let mut repository = Repository::new(gitalisk_repo.clone());

        // Create output directory for this repository
        let repo_output_dir = output_path.join(&repository.name);
        let repo_output_str = repo_output_dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid repository output path"))?;

        // Process repository: discover, index, analyze, write to Parquet, and optionally load to database
        println!(
            "  🚀 Starting full processing (discover → index → analyze → write → database)..."
        );

        // Set default database path to current working directory + "kuzu_db" if not provided
        let default_db_path = match &database_path {
            Some(path) => path.clone(),
            None => {
                // Create default database path: cwd + "kuzu_db"
                let cwd = std::env::current_dir()
                    .map_err(|e| anyhow::anyhow!("Failed to get current directory: {}", e))?;
                cwd.join("kuzu_db")
            }
        };

        let database_str = default_db_path.to_str();
        println!("database_path: {database_str:?}");

        match repository.process_repository_full_with_database(
            &config,
            repo_output_str,
            database_str,
        ) {
            Ok(result) => {
                repositories_processed += 1;
                println!(
                    "  ✅ Completed repository {}/{}: {} processed, {} skipped, {} errors in {:?}",
                    repositories_processed,
                    repos.len(),
                    result.total_files_processed,
                    result.total_files_skipped,
                    result.total_files_errored,
                    result.total_processing_time
                );

                // Show analysis and writer results if available
                if let Some(ref graph_data) = result.graph_data {
                    println!(
                        "  📊 Graph data: {} files, {} definitions, {} relationships",
                        graph_data.file_nodes.len(),
                        graph_data.definition_nodes.len(),
                        graph_data.file_definition_relationships.len()
                    );
                }

                if let Some(ref writer_result) = result.writer_result {
                    println!(
                        "  📁 Parquet files: {} files written to {}",
                        writer_result.files_written.len(),
                        repo_output_dir.display()
                    );
                }

                // Show database results if applicable
                if let Some(ref db_path) = result.database_path {
                    if result.database_loaded {
                        println!("  🗄️ Database: Successfully loaded graph data into {db_path}");
                    } else {
                        println!("  ⚠️ Database: Failed to load graph data into {db_path}");
                    }
                }

                // Log errors if any (limit to first 5 to avoid spam)
                if !result.errors.is_empty() {
                    println!("  ⚠️ Errors encountered ({} total):", result.errors.len());
                    for (file_path, error_msg) in result.errors.iter().take(5) {
                        println!("    • {file_path}: {error_msg}");
                    }
                    if result.errors.len() > 5 {
                        println!("    • ... and {} more errors", result.errors.len() - 5);
                    }
                }

                // Accumulate totals
                total_files_processed += result.total_files_processed;
                total_files_skipped += result.total_files_skipped;
                total_files_errored += result.total_files_errored;
                total_errors.extend(result.errors);

                // Show overall progress
                let overall_progress = repositories_processed as f64 / repos.len() as f64 * 100.0;
                println!(
                    "  📊 Overall progress: {:.1}% ({}/{} repositories completed)",
                    overall_progress,
                    repositories_processed,
                    repos.len()
                );
            }
            Err(e) => {
                eprintln!(
                    "  ❌ Failed to index repository '{}': {}",
                    repository.name, e
                );
                continue;
            }
        }
    }

    // Print final summary
    let total_time = start_time.elapsed();
    println!("\n🎉 Indexing completed in {total_time:?}");
    println!("📊 Summary:");
    println!("  • Repositories processed: {repositories_processed}");
    println!("  • Files processed: {total_files_processed}");
    println!("  • Files skipped: {total_files_skipped}");
    println!("  • Files with errors: {total_files_errored}");

    if total_files_processed > 0 {
        let files_per_sec = total_files_processed as f64 / total_time.as_secs_f64();
        println!("  • Processing rate: {files_per_sec:.1} files/second");
    }

    if !total_errors.is_empty() && total_errors.len() <= 10 {
        println!("  • Recent errors:");
        for (file_path, error_msg) in total_errors.iter().take(10) {
            println!("    • {file_path}: {error_msg}");
        }
        if total_errors.len() > 10 {
            println!("    • ... and {} more errors", total_errors.len() - 10);
        }
    }

    Ok(())
}
