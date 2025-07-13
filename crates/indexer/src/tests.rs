use crate::database::node_database_service::NodeDatabaseService;
use crate::database::types::{
    DefinitionNodeFromKuzu, DirectoryNodeFromKuzu, FileNodeFromKuzu, KuzuNodeType,
};
use crate::indexer::{IndexingConfig, RepositoryIndexer};
use crate::parsing::changes::FileChanges;
use crate::project::file_info::FileInfo;
use crate::project::source::{GitaliskFileSource, PathFileSource};
use database::graph::{RelationshipType, RelationshipTypeMapping};
use database::kuzu::config::DatabaseConfig;
use database::kuzu::connection::KuzuConnection;
use database::kuzu::database::KuzuDatabase;
use database::kuzu::schema::SchemaManager;
use database::kuzu::schema::SchemaManagerImportMode;
use gitalisk_core::repository::gitalisk_repository::CoreGitaliskRepository;
use kuzu::{Database, SystemConfig};
use miette::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use tempfile::TempDir;
use watchexec::Watchexec;
use watchexec_events::{Event, Priority};

fn init_test_workspace_with_repo(workspace_path: &Path, repo_name: &str) -> PathBuf {
    let repo_path = workspace_path.join(repo_name);

    // Initialize a new git repo
    fs::create_dir_all(&repo_path).unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Configure git author for this repository
    Command::new("git")
        .args(["config", "--local", "user.name", "test-gl-user"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "--local", "user.email", "test-gl-user@gitlab.com"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Copy fixture files from the existing fixtures directory
    let fixtures_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/test-repo");

    println!("fixtures_path: {fixtures_path:?}");

    copy_dir_all(&fixtures_path, &repo_path).expect("Failed to copy fixture files");

    // Run git add .
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Run git commit
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    repo_path
}

pub async fn modify_test_repo(
    workspace_path: &Path,
    repo_name: &str,
) -> Result<(), std::io::Error> {
    let repo_path = workspace_path.join(repo_name);

    // 1. Modify existing file - add whitespace and a new method
    let base_model_path = repo_path.join("app/models/base_model.rb");
    let content = tokio::fs::read_to_string(&base_model_path).await?;

    // Insert the new method after the existing class methods (after self.create)
    let modified_content = content.replace(
        "  def self.create(attributes)\n    instance = new(attributes)\n    instance.save\n    instance\n  end",
        "  def self.create(attributes)\n    instance = new(attributes)\n    instance.save\n    instance\n  end\n\n  def self.find_by_attributes(attrs)\n    where(attrs)\n  end"
    );

    // Add some whitespace at the top
    let modified_content = format!("\n\n{modified_content}");
    tokio::fs::write(&base_model_path, modified_content).await?;

    // Simulate some processing time
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 2. Add a new utility file
    let utils_path = repo_path.join("app/utils/string_utils.rb");
    tokio::fs::create_dir_all(utils_path.parent().unwrap()).await?;
    let utils_content = r#"module StringUtils
  def self.sanitize(str)
    str.strip.downcase
  end

  def self.titleize(str)
    str.split(' ').map(&:capitalize).join(' ')
  end
end"#;
    tokio::fs::write(utils_path, utils_content).await?;

    // Simulate some processing time
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 3. Modify another existing file to use the new utils
    let user_model_path = repo_path.join("app/models/user_model.rb");
    let user_content = tokio::fs::read_to_string(&user_model_path).await?;
    let modified_user_content = format!(
        "require_relative '../utils/string_utils'\n\n{user_content}\n  # Add name formatting\n  def format_name\n    StringUtils.titleize(name)\n  end"
    );
    tokio::fs::write(user_model_path, modified_user_content).await?;

    // Simulate some processing time
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    //4. Delete a method
    let base_model_content = tokio::fs::read_to_string(&base_model_path).await?;
    let modified_base_model = base_model_content.replace(
        r#"  def to_h
    instance_variables.each_with_object({}) do |var, hash|
      key = var.to_s.delete('@').to_sym
      hash[key] = instance_variable_get(var)
    end
  end

"#,
        "",
    );
    tokio::fs::write(&base_model_path, modified_base_model).await?;

    Ok(())
}

/// Helper function to create a temporary git repository by copying existing fixture files
fn create_test_repository() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    // Create .git directory to make it look like a git repo
    fs::create_dir_all(repo_path.join(".git")).expect("Failed to create .git directory");
    fs::write(
        repo_path.join(".git/config"),
        "[core]\n    repositoryformatversion = 0\n",
    )
    .expect("Failed to write git config");

    // Copy fixture files from the existing fixtures directory
    let fixtures_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/test-repo");

    copy_dir_all(&fixtures_path, repo_path).expect("Failed to copy fixture files");

    temp_dir
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

async fn simulate_debounced_watchexec_file_changes(
    repo_path: &Path,
) -> miette::Result<Vec<String>> {
    println!("\n=== MULTIPLE FILE CHANGES TEST WITH WATCHEXEC ===");

    // Track if we've seen any events and collect them
    let collected_events = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Event>::new()));
    let events_seen = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let events_seen_clone = events_seen.clone();
    let collected_events_clone = collected_events.clone();

    // Key insight: The action handler receives multiple types of events:
    // 1. Initial startup/control events (empty tags)
    // 2. Real filesystem events (with Path tags)
    // We want to collect all filesystem events until timeout, not quit on first event.
    let wx = Watchexec::new(move |mut action| {
        // Print out the events as they come in
        let mut has_real_events = false;
        for event in action.events.iter() {
            println!("File event: {event:?}");

            // Check if this event has actual file paths (real file events)
            if event.paths().next().is_some() {
                println!("This event has file paths - marking as real event");
                has_real_events = true;
                events_seen_clone.store(true, std::sync::atomic::Ordering::Relaxed);

                // Collect the event for later processing
                if let Ok(mut events) = collected_events_clone.lock() {
                    events.push(event.clone());
                    println!("Collected event #{} for later processing", events.len());
                }
            } else {
                println!("This event has no file paths - likely startup/control event");
            }
        }

        // Only quit if we receive an interrupt signal (let timeout handle the rest)
        if action
            .signals()
            .any(|sig| sig == watchexec_signals::Signal::Interrupt)
        {
            println!("Received interrupt signal - quitting watchexec...");
            action.quit();
        } else if has_real_events {
            println!("Collected real events - continuing to watch for more...");
        } else {
            println!("Continuing to watch for events...");
        }

        action
    })?;

    // Watch the repository path
    wx.config.pathset([repo_path.to_string_lossy().to_string()]);

    // Start the engine
    let main = wx.main();

    // Send an event to start (this triggers the initial startup event)
    wx.send_event(Event::default(), Priority::Urgent).await?;

    // Give the watcher time to initialize before making changes
    println!("Waiting for watcher to initialize...");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Make comprehensive file changes to trigger real filesystem events
    println!("Making file changes...");
    let expected_changes = make_comprehensive_file_changes(repo_path).await;

    println!(
        "Expected {} file system operations to be detected",
        expected_changes.len()
    );

    // Give extra time for filesystem events to propagate in CI
    println!("Allowing time for filesystem events to propagate...");
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Add a timeout to prevent hanging forever (safety net) - increased for CI
    println!("Waiting for events (with 10s timeout)...");
    let timeout_result = tokio::time::timeout(tokio::time::Duration::from_secs(10), main).await;

    match timeout_result {
        Ok(result) => {
            println!("Watchexec completed: {result:?}");
        }
        Err(_) => {
            println!("Timeout reached - sending quit signal");
            wx.send_event(
                Event {
                    tags: vec![watchexec_events::Tag::Signal(
                        watchexec_signals::Signal::Interrupt,
                    )],
                    metadata: std::collections::HashMap::new(),
                },
                Priority::Urgent,
            )
            .await?;

            // Give it a moment to process the quit
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    // Process the collected events
    let events = collected_events.lock().unwrap();
    let event_count = events.len();
    let mut affected_files = std::collections::HashSet::new();

    if events_seen.load(std::sync::atomic::Ordering::Relaxed) {
        println!("Successfully detected file events!");
        println!("Collected {event_count} events for processing:");

        // Analyze collected events by type
        let mut event_summary = std::collections::HashMap::new();

        // Process each collected event
        for (i, event) in events.iter().enumerate() {
            println!("\n--- Event #{} ---", i + 1);

            // Extract file paths from the event
            let paths: Vec<_> = event.paths().collect();
            for (path, file_type) in paths {
                let relative_path = path
                    .strip_prefix("/private")
                    .map(|p| format!("/{}", p.display()))
                    .unwrap_or(path.display().to_string());
                println!("Path: {relative_path}");

                // Note this is temporary. We should be using a proper ignore tracker, like watchexec-filterer-ignore. But this issue only shows up in the CI.
                if !relative_path.contains(".git") {
                    affected_files.insert(relative_path);
                }

                if let Some(ft) = file_type {
                    println!("File type: {ft:?}");
                }
            }

            // Extract event kinds and count them
            for tag in &event.tags {
                match tag {
                    watchexec_events::Tag::FileEventKind(kind) => {
                        let kind_str = format!("{kind:?}");
                        println!("Event kind: {kind_str}");
                        *event_summary.entry(kind_str).or_insert(0) += 1;
                    }
                    watchexec_events::Tag::Source(source) => {
                        println!("Source: {source:?}");
                    }
                    _ => {}
                }
            }
        }
    } else {
        println!("No events were detected");
    }

    Ok(affected_files.iter().map(|s| s.to_string()).collect())
}

struct ReindexingPipelineSetup {
    repo_path: PathBuf,
    indexer: RepositoryIndexer,
    file_source: GitaliskFileSource,
    config: IndexingConfig,
    database_path: String,
    output_path: String,
}

fn setup_reindexing_pipeline(
    database: &Arc<KuzuDatabase>,
    temp_dir: &TempDir,
) -> ReindexingPipelineSetup {
    // Create temporary repository with test files
    let repo_path = init_test_workspace_with_repo(temp_dir.path(), "test-repo");
    let repo_path_str = repo_path.to_str().unwrap();

    // Create a gitalisk repository wrapper
    let gitalisk_repo =
        CoreGitaliskRepository::new(repo_path_str.to_string(), repo_path_str.to_string());

    // Create our RepositoryIndexer wrapper
    let indexer = RepositoryIndexer::new("test-repo".to_string(), repo_path_str.to_string());
    let file_source: GitaliskFileSource = GitaliskFileSource::new(gitalisk_repo.clone());

    // Configure indexing for Ruby files
    let config = IndexingConfig {
        worker_threads: 1, // Use single thread for deterministic testing
        max_file_size: 5_000_000,
        respect_gitignore: false, // Don't use gitignore in tests
    };

    // Create output directory for this test
    let output_dir = temp_dir.path().join("output");
    let output_path = output_dir.to_str().unwrap();
    let database_path: String = output_dir.join("kuzu_db").to_str().unwrap().to_string();

    // Run the full processing pipeline (to index the repo once)
    let indexing_result = indexer
        .process_files_full_with_database(
            database,
            file_source.clone(),
            &config,
            output_path,
            Some(&database_path),
        )
        .expect("Failed to process repository");

    // Verify we have graph data and that the database path is set
    assert!(
        indexing_result.graph_data.is_some(),
        "Should have graph data"
    );
    assert_eq!(*database_path, indexing_result.database_path.unwrap());

    let database_instance =
        Database::new(&database_path, SystemConfig::default()).expect("Failed to create database");

    let node_database_service = NodeDatabaseService::new(&database_instance);

    let all_definition_count = node_database_service.count_nodes::<DefinitionNodeFromKuzu>();
    println!("all_definition_count: {all_definition_count}");
    assert_eq!(
        all_definition_count, 94,
        "Should have 94 definitions globally after initial indexing"
    );

    // file_paths: ["app/models/user_model.rb", "app/models/base_model.rb"]
    let file_paths = vec![
        "app/models/user_model.rb".to_string(),
        "app/models/base_model.rb".to_string(),
    ];
    let definition_count = node_database_service
        .count_node_by(
            KuzuNodeType::DefinitionNode,
            "primary_file_path",
            &file_paths,
        )
        .unwrap();
    assert_eq!(
        definition_count, 33,
        "Should have 33 definitions after initial indexing (user_model.rb and base_model.rb)"
    );

    println!("repo_path: {repo_path:?}");
    println!("file_source: {file_source:?}");
    println!("config: {config:?}");
    println!("database_path: {database_path:?}");
    println!("output_path: {output_path:?}");

    ReindexingPipelineSetup {
        repo_path,
        indexer,
        file_source,
        config,
        database_path,
        output_path: output_path.to_string(),
    }
}

#[tokio::test]
async fn test_full_reindexing_pipeline_git_status() {
    let temp_dir: TempDir = TempDir::new().expect("Failed to create temp directory");
    let database = Arc::new(KuzuDatabase::new());

    let mut setup = setup_reindexing_pipeline(&database, &temp_dir);

    // Modify the test repo, we should optionally allow
    modify_test_repo(temp_dir.path(), "test-repo")
        .await
        .expect("Failed to modify test repo");
    let git_status = setup
        .file_source
        .repository
        .get_status()
        .expect("Failed to get git status");
    let reindexer_file_changes = FileChanges::from_git_status(git_status);
    reindexer_file_changes.pretty_print();

    // Run the full processing pipeline (to reindex the repo)
    let result = setup
        .indexer
        .reindex_repository(
            &database,
            reindexer_file_changes,
            &setup.config,
            &setup.database_path,
            &setup.output_path,
        )
        .expect("Failed to reindex repository");

    println!("result: {:?}", result.writer_result);

    let database_instance = database
        .get_or_create_database(&setup.database_path)
        .expect("Failed to create database");
    let node_database_service = NodeDatabaseService::new(&database_instance);

    let definition_count = node_database_service.count_nodes::<DefinitionNodeFromKuzu>();
    println!("definition_count: {definition_count}");
    assert_eq!(
        definition_count, 95,
        "Should have 95 definitions globally after reindexing"
    );

    let file_paths = vec![
        "app/models/user_model.rb".to_string(),
        "app/models/base_model.rb".to_string(),
    ];
    let definition_count = node_database_service
        .count_node_by(
            KuzuNodeType::DefinitionNode,
            "primary_file_path",
            &file_paths,
        )
        .unwrap();
    assert_eq!(
        definition_count, 34,
        "Should have 34 definitions after reindexing (user_model.rb and base_model.rb)"
    );
}

// TODO: fix this test https://gitlab.com/gitlab-org/rust/knowledge-graph/-/issues/46
#[tokio::test]
#[ignore]
async fn test_full_reindexing_pipeline_watchexec() {
    let temp_dir: TempDir = TempDir::new().expect("Failed to create temp directory");
    let database = Arc::new(KuzuDatabase::new());
    let mut setup = setup_reindexing_pipeline(&database, &temp_dir);
    let repo_path = setup.repo_path;

    // Modify the test repo, we should optionally allow
    let affected_files = simulate_debounced_watchexec_file_changes(&repo_path)
        .await
        .expect("Failed to simulate watchexec file changes");
    let reindexer_file_changes = FileChanges::from_watched_files(affected_files);
    reindexer_file_changes.pretty_print();

    // Run the full processing pipeline (to reindex the repo)
    let result = setup
        .indexer
        .reindex_repository(
            &database,
            reindexer_file_changes,
            &setup.config,
            &setup.database_path,
            &setup.output_path,
        )
        .expect("Failed to reindex repository");

    println!("result: {:?}", result.writer_result);

    // re-open a database connection to verify the definition counts
    let database_instance = database
        .get_or_create_database(&setup.database_path)
        .expect("Failed to create database");

    let node_database_service = NodeDatabaseService::new(&database_instance);

    let all_definition_count = node_database_service.count_nodes::<DefinitionNodeFromKuzu>();
    println!("all_definition_count: {all_definition_count}");

    let definitions = node_database_service
        .get_all::<DefinitionNodeFromKuzu>(KuzuNodeType::DefinitionNode)
        .unwrap();
    let ids_paths = definitions
        .iter()
        .map(|d| (d.id, d.primary_file_path.clone(), d.fqn.clone()))
        .collect::<Vec<_>>();

    // Create a HashMap to track duplicates
    let mut path_fqn_map: std::collections::HashMap<(String, String), Vec<u32>> =
        std::collections::HashMap::new();

    // Collect all entries with the same path+fqn combination
    for (id, path, fqn) in ids_paths {
        path_fqn_map
            .entry((path.clone(), fqn.clone()))
            .or_default()
            .push(id);
        println!("id: {id}, path: {path}, fqn: {fqn}");
    }

    // Check for duplicates (rigourous, because we have a ton of moves, deletes, and renames)
    let mut duplicates = Vec::new();
    println!("\nChecking for duplicates by path+fqn combination");
    for ((path, fqn), ids) in path_fqn_map {
        if ids.len() > 1 {
            duplicates.push((path, fqn, ids));
        }
    }
    if !duplicates.is_empty() {
        for (path, fqn, ids) in &duplicates {
            println!("DUPLICATE FOUND! path: {path}, fqn: {fqn}");
            println!("  IDs: {ids:?}");
        }
    } else {
        println!("No duplicates found");
    }

    assert_eq!(duplicates.len(), 0, "Should have no duplicates");

    // Check that we have the expected number of definition nodes
    let expected_definition_count = 142;
    let definition_count = node_database_service.count_nodes::<DefinitionNodeFromKuzu>();
    assert_eq!(
        definition_count, expected_definition_count,
        "Should have {expected_definition_count} definition nodes"
    );
}

fn setup_end_to_end_kuzu(temp_repo: &TempDir) {
    // Create temporary repository with test files
    let repo_path = temp_repo.path().to_str().unwrap();

    // Create a gitalisk repository wrapper
    let gitalisk_repo = CoreGitaliskRepository::new(repo_path.to_string(), repo_path.to_string());

    // Create our RepositoryIndexer wrapper
    let indexer = RepositoryIndexer::new("test-repo".to_string(), repo_path.to_string());
    let file_source = GitaliskFileSource::new(gitalisk_repo);

    // Configure indexing for Ruby files
    let config = IndexingConfig {
        worker_threads: 1,
        max_file_size: 5_000_000,
        respect_gitignore: false,
    };

    // Run full processing pipeline
    let output_dir = temp_repo.path().join("output");
    let output_path = output_dir.to_str().unwrap();

    // Create database as done in the working example
    let database = Arc::new(KuzuDatabase::new());
    let _result = indexer
        .process_files_full(&database, file_source, &config, output_path)
        .expect("Failed to process repository");

    // Create Kuzu database
    let db_dir = temp_repo.path().join("kuzu_db");

    let config = DatabaseConfig::new(db_dir.to_string_lossy())
        .with_buffer_size(512 * 1024 * 1024)
        .with_compression(true);

    let database_instance = database
        .create_temporary_database(config)
        .expect("Failed to create Kuzu database");

    // Initialize schema
    let schema_manager = SchemaManager::new(&database_instance);
    schema_manager
        .initialize_schema()
        .expect("Failed to initialize schema");

    // Import Parquet data
    schema_manager
        .import_graph_data(output_path, SchemaManagerImportMode::Indexing)
        .expect("Failed to import graph data");

    println!("✅ Kuzu database created and data imported successfully");
}

#[test]
fn test_new_indexer_with_gitalisk_file_source() {
    let temp_repo = create_test_repository();
    let repo_path = temp_repo.path().to_str().unwrap();

    let gitalisk_repo = CoreGitaliskRepository::new(repo_path.to_string(), repo_path.to_string());

    let indexer = RepositoryIndexer::new("test-repo".to_string(), repo_path.to_string());
    let file_source = GitaliskFileSource::new(gitalisk_repo);

    let config = IndexingConfig {
        worker_threads: 1,
        max_file_size: 5_000_000,
        respect_gitignore: false,
    };

    let result = indexer
        .index_files(file_source, &config)
        .expect("Failed to index files");

    assert!(
        result.total_files_processed > 0,
        "Should have processed some files"
    );
    assert_eq!(result.total_files_errored, 0, "Should have no errors");

    println!("✅ New indexer test completed successfully!");
    println!("📊 Processed {} files", result.total_files_processed);
}

#[test]
fn test_new_indexer_with_path_file_source() {
    let temp_repo = create_test_repository();
    let repo_path = temp_repo.path();

    let mut ruby_files = Vec::new();
    for entry in walkdir::WalkDir::new(repo_path) {
        let entry = entry.unwrap();
        if entry.path().extension().and_then(|s| s.to_str()) == Some("rb") {
            ruby_files.push(FileInfo::from_path(entry.path().to_path_buf()));
        }
    }

    let indexer = RepositoryIndexer::new(
        "test-repo".to_string(),
        repo_path.to_string_lossy().to_string(),
    );
    let file_source = PathFileSource::new(ruby_files);

    let config = IndexingConfig {
        worker_threads: 1,
        max_file_size: 5_000_000,
        respect_gitignore: false,
    };

    let result = indexer
        .index_files(file_source, &config)
        .expect("Failed to index files");

    assert!(
        result.total_files_processed > 0,
        "Should have processed some files"
    );
    assert_eq!(result.total_files_errored, 0, "Should have no errors");

    println!("✅ Path file source test completed successfully!");
    println!("📊 Processed {} files", result.total_files_processed);
}

#[test]
fn test_full_indexing_pipeline() {
    // Create temporary repository with test files
    let temp_repo = create_test_repository();
    let repo_path = temp_repo.path().to_str().unwrap();

    // Create a gitalisk repository wrapper
    let gitalisk_repo = CoreGitaliskRepository::new(repo_path.to_string(), repo_path.to_string());

    // Create our RepositoryIndexer wrapper
    let indexer = RepositoryIndexer::new("test-repo".to_string(), repo_path.to_string());
    let file_source = GitaliskFileSource::new(gitalisk_repo);

    // Configure indexing for Ruby files
    let config = IndexingConfig {
        worker_threads: 1, // Use single thread for deterministic testing
        max_file_size: 5_000_000,
        respect_gitignore: false, // Don't use gitignore in tests
    };

    // Create output directory for this test
    let output_dir = temp_repo.path().join("output");
    let output_path = output_dir.to_str().unwrap();

    // Run the full processing pipeline
    let database = Arc::new(KuzuDatabase::new());
    let result = indexer
        .process_files_full(&database, file_source, &config, output_path)
        .expect("Failed to process repository");

    // Verify we processed files
    assert!(
        result.total_files_processed > 0,
        "Should have processed some files"
    );
    assert_eq!(result.total_files_errored, 0, "Should have no errors");

    // Verify graph data was created
    let graph_data: crate::analysis::GraphData = result.graph_data.expect("Should have graph data");

    // Check we have the expected file nodes
    assert!(
        graph_data.file_nodes.len() >= 6,
        "Should have at least 6 file nodes"
    );

    // Check we have definition nodes
    assert!(
        !graph_data.definition_nodes.is_empty(),
        "Should have definition nodes"
    );

    // Verify Authentication module is properly merged (should have multiple file locations)
    let auth_module = graph_data
        .definition_nodes
        .iter()
        .find(|def| def.fqn == "Authentication")
        .expect("Should find Authentication module");

    assert!(
        auth_module.file_locations.len() >= 3,
        "Authentication module should have multiple file locations (got {})",
        auth_module.file_locations.len()
    );

    // Check that we have file-definition relationships
    assert!(
        !graph_data.file_definition_relationships.is_empty(),
        "Should have file-definition relationships"
    );

    // Check that we have definition relationships (parent-child)
    assert!(
        !graph_data.definition_relationships.is_empty(),
        "Should have definition relationships"
    );

    // Verify writer result
    let writer_result = result.writer_result.expect("Should have writer result");
    assert!(
        !writer_result.files_written.is_empty(),
        "Should have written Parquet files"
    );

    // Verify Parquet files exist
    for written_file in &writer_result.files_written {
        assert!(
            written_file.file_path.exists(),
            "Parquet file should exist: {}",
            written_file.file_path.display()
        );
        assert!(
            written_file.file_size_bytes > 0,
            "Parquet file should not be empty: {}",
            written_file.file_path.display()
        );
    }

    println!("✅ Test completed successfully!");
    println!("📊 Processed {} files", result.total_files_processed);
    println!(
        "📊 Created {} definition nodes",
        graph_data.definition_nodes.len()
    );
    println!(
        "📊 Created {} file-definition relationships",
        graph_data.file_definition_relationships.len()
    );
    println!(
        "📊 Created {} definition relationships",
        graph_data.definition_relationships.len()
    );
    println!(
        "📁 Wrote {} Parquet files",
        writer_result.files_written.len()
    );

    // === PART 2: End-to-end Kuzu database verification ===
    println!("\n🏗️ === KUZU DATABASE END-TO-END VERIFICATION ===");

    // Create Kuzu database
    let db_dir = temp_repo.path().join("kuzu_db");

    let config = DatabaseConfig::new(db_dir.to_string_lossy())
        .with_buffer_size(512 * 1024 * 1024)
        .with_compression(true);

    let database_instance = database
        .create_temporary_database(config)
        .expect("Failed to create Kuzu database");

    // Initialize schema
    let schema_manager = SchemaManager::new(&database_instance);

    schema_manager
        .initialize_schema()
        .expect("Failed to initialize schema");

    // Import Parquet data
    schema_manager
        .import_graph_data(output_path, SchemaManagerImportMode::Indexing)
        .expect("Failed to import graph data");

    println!("✅ Kuzu database created and data imported successfully");

    // Verify basic node counts
    println!("\n📊 Kuzu Database Node Counts:");
    let node_database_service = NodeDatabaseService::new(&database_instance);
    let node_counts = node_database_service
        .get_node_counts()
        .expect("Failed to get node counts");

    println!("  📁 Directory nodes: {}", node_counts.directory_count);
    println!("  📄 File nodes: {}", node_counts.file_count);
    println!("  🏗️  Definition nodes: {}", node_counts.definition_count);

    // Verify relationship counts
    println!("\n📊 Kuzu Database Relationship Counts:");
    let rel_counts = node_database_service
        .get_relationship_counts()
        .expect("Failed to get relationship counts");

    println!(
        "  📁 Directory relationships: {}",
        rel_counts.directory_relationships
    );
    println!("  📄 File relationships: {}", rel_counts.file_relationships);
    println!(
        "  🏗️  Definition relationships: {}",
        rel_counts.definition_relationships
    );
}

#[test]
fn test_module_reopening_merge() {
    // Create temporary repository with test files
    let temp_repo = create_test_repository();
    let repo_path = temp_repo.path().to_str().unwrap();

    // Create a gitalisk repository wrapper
    let gitalisk_repo = CoreGitaliskRepository::new(repo_path.to_string(), repo_path.to_string());

    // Create our RepositoryIndexer wrapper
    let indexer = RepositoryIndexer::new("test-repo".to_string(), repo_path.to_string());
    let file_source = GitaliskFileSource::new(gitalisk_repo);

    // Configure indexing for Ruby files
    let config = IndexingConfig {
        worker_threads: 1,
        max_file_size: 5_000_000,
        respect_gitignore: false,
    };

    // Index files and get results
    let index_result = indexer
        .index_files(file_source, &config)
        .expect("Failed to index repository");

    // Verify we have files
    assert!(
        index_result.total_files_processed >= 6,
        "Should have processed at least 6 Ruby files"
    );

    // Run analysis to get graph data
    let analysis_service =
        crate::analysis::AnalysisService::new(indexer.name.clone(), indexer.path.clone());

    let graph_data = analysis_service
        .analyze_results(&index_result.file_results)
        .expect("Failed to analyze results");

    // Find the Authentication module definition
    let auth_definitions: Vec<_> = graph_data
        .definition_nodes
        .iter()
        .filter(|def| def.fqn == "Authentication")
        .collect();

    // Should have exactly one merged definition for Authentication module
    assert_eq!(
        auth_definitions.len(),
        1,
        "Should have exactly one Authentication module definition"
    );

    let auth_def = auth_definitions[0];

    // Should have multiple file locations (from module reopening)
    assert!(
        auth_def.file_locations.len() >= 3,
        "Authentication module should be defined in at least 3 files, got {}",
        auth_def.file_locations.len()
    );

    // Verify the file locations include the expected files
    let file_paths: Vec<_> = auth_def
        .file_locations
        .iter()
        .map(|loc| {
            Path::new(&loc.file_path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        })
        .collect();

    assert!(
        file_paths.contains(&"authentication.rb"),
        "Should include authentication.rb, got: {file_paths:?}"
    );
    assert!(
        file_paths.contains(&"providers.rb"),
        "Should include providers.rb, got: {file_paths:?}"
    );
    assert!(
        file_paths.contains(&"tokens.rb"),
        "Should include tokens.rb, got: {file_paths:?}"
    );

    // Verify we have Authentication::Providers module
    let providers_def = graph_data
        .definition_nodes
        .iter()
        .find(|def| def.fqn == "Authentication::Providers")
        .expect("Should find Authentication::Providers module");

    assert!(
        !providers_def.file_locations.is_empty(),
        "Providers module should be defined in at least 1 file"
    );

    // Verify we have relationships between Authentication and its nested modules
    let module_relationships: Vec<_> = graph_data
        .definition_relationships
        .iter()
        .filter(|rel| {
            rel.from_definition_fqn == "Authentication"
                && rel.relationship_type == "MODULE_TO_MODULE"
        })
        .collect();

    assert!(
        !module_relationships.is_empty(),
        "Should have MODULE_TO_MODULE relationships from Authentication"
    );

    println!("✅ Module reopening test completed successfully!");
    println!(
        "📊 Authentication module has {} file locations",
        auth_def.file_locations.len()
    );
    println!(
        "📊 Found {} module-to-module relationships from Authentication",
        module_relationships.len()
    );
}

#[test]
fn test_inheritance_relationships() {
    // Create temporary repository with test files
    let temp_repo = create_test_repository();
    let repo_path = temp_repo.path().to_str().unwrap();

    // Create a gitalisk repository wrapper
    let gitalisk_repo = CoreGitaliskRepository::new(repo_path.to_string(), repo_path.to_string());

    // Create our RepositoryIndexer wrapper
    let indexer = RepositoryIndexer::new("test-repo".to_string(), repo_path.to_string());
    let file_source = GitaliskFileSource::new(gitalisk_repo);

    // Configure indexing for Ruby files
    let config = IndexingConfig {
        worker_threads: 1,
        max_file_size: 5_000_000,
        respect_gitignore: false,
    };

    // Run full processing
    let output_dir = temp_repo.path().join("output");
    let output_path = output_dir.to_str().unwrap();

    let database = Arc::new(KuzuDatabase::new());
    let result = indexer
        .process_files_full(&database, file_source, &config, output_path)
        .expect("Failed to process repository");

    let graph_data = result.graph_data.expect("Should have graph data");

    // Find BaseModel and UserModel classes
    let base_model = graph_data
        .definition_nodes
        .iter()
        .find(|def| def.fqn == "BaseModel")
        .expect("Should find BaseModel class");

    let user_model = graph_data
        .definition_nodes
        .iter()
        .find(|def| def.fqn == "UserModel")
        .expect("Should find UserModel class");

    assert_eq!(
        base_model.definition_type,
        parser_core::ruby::types::RubyDefinitionType::Class
    );
    assert_eq!(
        user_model.definition_type,
        parser_core::ruby::types::RubyDefinitionType::Class
    );

    // Verify we have class-to-method relationships
    let class_method_rels: Vec<_> = graph_data
        .definition_relationships
        .iter()
        .filter(|rel| rel.relationship_type == "CLASS_TO_METHOD")
        .collect();

    assert!(
        !class_method_rels.is_empty(),
        "Should have CLASS_TO_METHOD relationships"
    );

    // Check for methods in BaseModel
    let base_model_methods: Vec<_> = graph_data
        .definition_relationships
        .iter()
        .filter(|rel| {
            rel.from_definition_fqn == "BaseModel" && rel.relationship_type == "CLASS_TO_METHOD"
        })
        .collect();

    assert!(
        !base_model_methods.is_empty(),
        "BaseModel should have methods"
    );

    println!("✅ Inheritance relationships test completed successfully!");
    println!(
        "📊 Found {} class-to-method relationships",
        class_method_rels.len()
    );
    println!("📊 BaseModel has {} methods", base_model_methods.len());
}

#[test]
fn test_simple_end_to_end_kuzu() {
    // Create temporary repository with test files
    let temp_repo = create_test_repository();
    setup_end_to_end_kuzu(&temp_repo);

    let db_dir = temp_repo.path().join("kuzu_db");
    let database = Arc::new(KuzuDatabase::new());
    let database_instance = database
        .get_or_create_database(&db_dir.to_string_lossy())
        .expect("Failed to create database");
    let connection = KuzuConnection::new(&database_instance).expect("Failed to create connection");

    let relationship_type_map = RelationshipTypeMapping::new();
    let node_database_service = NodeDatabaseService::new(&database_instance);

    // Get definition node count
    let defn_node_count = node_database_service.count_nodes::<DefinitionNodeFromKuzu>();
    println!("Definition node count: {defn_node_count}");
    assert_eq!(defn_node_count, 94);

    // Get file node count
    let file_node_count = node_database_service.count_nodes::<FileNodeFromKuzu>();
    println!("File node count: {file_node_count}");
    assert_eq!(file_node_count, 7);

    // Get module -> class relationships count
    let module_class_rel_count =
        node_database_service.count_relationships_of_type(RelationshipType::ModuleToClass);
    println!("Module -> class relationship count: {module_class_rel_count}");
    assert_eq!(module_class_rel_count, 7);

    // Get file definition relationships count
    let file_defn_rel_count =
        node_database_service.count_relationships_of_type(RelationshipType::FileDefines);
    println!("File defines relationship count: {file_defn_rel_count}");
    assert_eq!(file_defn_rel_count, 96);

    // Get directory node count
    let dir_node_count = node_database_service.count_nodes::<DirectoryNodeFromKuzu>();
    println!("Directory node count: {dir_node_count}");
    assert_eq!(dir_node_count, 4);

    // get directory -> file relationships count
    let dir_file_rel_count =
        node_database_service.count_relationships_of_type(RelationshipType::DirContainsFile);
    println!("Directory -> file relationship count: {dir_file_rel_count}");
    assert_eq!(dir_file_rel_count, 6);

    // get directory -> directory relationships count
    let dir_dir_rel_count =
        node_database_service.count_relationships_of_type(RelationshipType::DirContainsDir);
    println!("Directory -> directory relationship count: {dir_dir_rel_count}");
    assert_eq!(dir_dir_rel_count, 2);

    // get definition relationships count
    let def_rel_count =
        node_database_service.count_relationships_of_node_type(KuzuNodeType::DefinitionNode);
    println!("Definition relationship count: {def_rel_count}");
    assert_eq!(def_rel_count, 88);

    // Get all relationships in the definition_relationships table
    let m2m_rel_type = relationship_type_map.get_type_id(RelationshipType::ModuleToClass);
    let query_module_to_class = format!(
        "MATCH (d:DefinitionNode)-[r:DEFINITION_RELATIONSHIPS]->(c:DefinitionNode) WHERE r.type = {m2m_rel_type} RETURN d, c, r.type"
    );

    let result = connection
        .query(&query_module_to_class)
        .expect("Failed to query module to class");
    for row in result {
        if let (Some(from_node_value), Some(to_node_value), Some(kuzu::Value::UInt8(rel_type))) =
            (row.first(), row.get(1), row.get(2))
        {
            let from_node = DefinitionNodeFromKuzu::from_kuzu_node(from_node_value);
            let to_node = DefinitionNodeFromKuzu::from_kuzu_node(to_node_value);
            let rel_type_name = relationship_type_map.get_type_name(*rel_type);
            println!(
                "Module to class relationship: {} -[type: {}]-> {}",
                from_node.fqn, rel_type_name, to_node.fqn
            );
            if from_node.fqn.as_str() == "Authentication::Providers" {
                match to_node.fqn.as_str() {
                    "Authentication::Providers::LdapProvider" => {
                        assert_eq!(to_node.definition_type, "Class");
                        assert_eq!(to_node.primary_file_path, "lib/authentication/providers.rb");
                    }
                    "Authentication::Providers::OAuthProvider" => {
                        assert_eq!(to_node.definition_type, "Class");
                        assert_eq!(to_node.primary_file_path, "lib/authentication/providers.rb");
                    }
                    _ => {}
                }
            }
        }
    }

    println!("--------------------------------");

    // Query file relationships
    let file_rel_type = relationship_type_map.get_type_id(RelationshipType::FileDefines);
    let query_file_rels = format!(
        "MATCH (f:FileNode)-[r:FILE_RELATIONSHIPS]->(d:DefinitionNode) WHERE r.type = {file_rel_type} RETURN f, d, r.type"
    );

    let result = connection
        .query(&query_file_rels)
        .expect("Failed to query file relationships");
    for row in result {
        if let (Some(file_value), Some(def_value), Some(kuzu::Value::UInt8(rel_type))) =
            (row.first(), row.get(1), row.get(2))
        {
            let file_node = FileNodeFromKuzu::from_kuzu_node(file_value);
            let def_node = DefinitionNodeFromKuzu::from_kuzu_node(def_value);
            let rel_type_name = relationship_type_map.get_type_name(*rel_type);
            println!(
                "File relationship: {} -[type: {}]-> {}",
                file_node.path, rel_type_name, def_node.fqn
            );
            match file_node.path.as_str() {
                "main.rb" => {
                    if def_node.fqn.as_str() == "Application::test_authentication_providers" {
                        assert_eq!(rel_type_name, RelationshipType::FileDefines.as_str());
                    }
                }
                "app/models/user_model.rb" => {
                    if def_node.fqn.as_str() == "UserModel::valid?" {
                        assert_eq!(rel_type_name, RelationshipType::FileDefines.as_str());
                    }
                }
                _ => {}
            }
        }
    }

    println!("--------------------------------");

    // Query directory relationships
    let dir_file_rel_type = relationship_type_map.get_type_id(RelationshipType::DirContainsFile);

    // Query directory -> file relationships
    let query_dir_file_rels = format!(
        "MATCH (d:DirectoryNode)-[r:DIRECTORY_RELATIONSHIPS]->(f:FileNode) WHERE r.type = {dir_file_rel_type} RETURN d, f, r.type"
    );

    let result = connection
        .query(&query_dir_file_rels)
        .expect("Failed to query directory-file relationships");
    for row in result {
        if let (Some(dir_value), Some(file_value), Some(kuzu::Value::UInt8(rel_type))) =
            (row.first(), row.get(1), row.get(2))
        {
            let dir_node = DirectoryNodeFromKuzu::from_kuzu_node(dir_value);
            let file_node = FileNodeFromKuzu::from_kuzu_node(file_value);
            let rel_type_name = relationship_type_map.get_type_name(*rel_type);
            println!(
                "Directory-File relationship: {} -[type: {}]-> {}",
                dir_node.path, rel_type_name, file_node.path
            );
            if dir_node.path.as_str() == "app/models"
                && file_node.path.as_str() == "app/models/user_model.rb"
            {
                assert_eq!(rel_type_name, RelationshipType::DirContainsFile.as_str());
            }
            if dir_node.path.as_str() == "lib/authentication"
                && file_node.path.as_str() == "lib/authentication/providers.rb"
            {
                assert_eq!(rel_type_name, RelationshipType::DirContainsFile.as_str());
            }
        }
    }

    println!("--------------------------------");

    // Query directory -> directory relationships
    let dir_dir_rel_type = relationship_type_map.get_type_id(RelationshipType::DirContainsDir);
    let query_dir_dir_rels = format!(
        "MATCH (d1:DirectoryNode)-[r:DIRECTORY_RELATIONSHIPS]->(d2:DirectoryNode) WHERE r.type = {dir_dir_rel_type} RETURN d1, d2, r.type"
    );

    let result = connection
        .query(&query_dir_dir_rels)
        .expect("Failed to query directory-directory relationships");
    for row in result {
        if let (Some(dir1_value), Some(dir2_value), Some(kuzu::Value::UInt8(rel_type))) =
            (row.first(), row.get(1), row.get(2))
        {
            let dir1_node = DirectoryNodeFromKuzu::from_kuzu_node(dir1_value);
            let dir2_node = DirectoryNodeFromKuzu::from_kuzu_node(dir2_value);
            let rel_type_name = relationship_type_map.get_type_name(*rel_type);
            println!(
                "Directory-Directory relationship: {} -[type: {}]-> {}",
                dir1_node.path, rel_type_name, dir2_node.path
            );
            match dir1_node.path.as_str() {
                "lib" => {
                    if dir2_node.path.as_str() == "lib/authentication" {
                        assert_eq!(rel_type_name, RelationshipType::DirContainsDir.as_str());
                    }
                }
                "app" => {
                    if dir2_node.path.as_str() == "app/models" {
                        assert_eq!(rel_type_name, RelationshipType::DirContainsDir.as_str());
                    }
                }
                _ => {}
            }
        }
    }
}

#[test]
fn test_detailed_data_inspection() {
    // Create temporary repository with test files
    let temp_repo = create_test_repository();
    let repo_path = temp_repo.path().to_str().unwrap();

    // Create a gitalisk repository wrapper
    let gitalisk_repo = CoreGitaliskRepository::new(repo_path.to_string(), repo_path.to_string());

    // Create our RepositoryIndexer wrapper
    let indexer = RepositoryIndexer::new("test-repo".to_string(), repo_path.to_string());
    let file_source = GitaliskFileSource::new(gitalisk_repo);

    // Configure indexing for Ruby files
    let config = IndexingConfig {
        worker_threads: 1,
        max_file_size: 5_000_000,
        respect_gitignore: false,
    };

    // Run full processing pipeline
    let output_dir = temp_repo.path().join("output");
    let output_path = output_dir.to_str().unwrap();

    let database = Arc::new(KuzuDatabase::new());
    let result = indexer
        .process_files_full(&database, file_source, &config, output_path)
        .expect("Failed to process repository");

    let graph_data = result.graph_data.expect("Should have graph data");

    println!("\n🔍 === DETAILED DATA INSPECTION ===");

    // === PART 1: In-memory graph data verification (existing) ===

    // 1. Inspect Authentication module specifically
    println!("\n📊 Authentication Module Analysis:");
    let auth_modules: Vec<_> = graph_data
        .definition_nodes
        .iter()
        .filter(|def| def.fqn.contains("Authentication"))
        .collect();

    for auth_def in &auth_modules {
        println!(
            "  📁 FQN: {} ({:?})",
            auth_def.fqn, auth_def.definition_type
        );
        println!("     Locations: {}", auth_def.file_locations.len());
        for (i, location) in auth_def.file_locations.iter().enumerate() {
            let file_name = std::path::Path::new(&location.file_path)
                .file_name()
                .map(|n| n.to_str().unwrap_or("unknown"))
                .unwrap_or("unknown");
            println!(
                "     [{i}] {file_name} (line: {}, bytes: {}-{})",
                location.line_number, location.start_byte, location.end_byte
            );
        }
    }

    // 2. Inspect all module definitions and their reopening behavior
    println!("\n📊 Module Reopening Analysis:");
    let modules: Vec<_> = graph_data
        .definition_nodes
        .iter()
        .filter(|def| {
            matches!(
                def.definition_type,
                parser_core::ruby::types::RubyDefinitionType::Module
            )
        })
        .collect();

    for module_def in &modules {
        if module_def.file_locations.len() > 1 {
            println!(
                "  🔄 REOPENED: {} ({} locations)",
                module_def.fqn,
                module_def.file_locations.len()
            );
            for location in &module_def.file_locations {
                let file_name = std::path::Path::new(&location.file_path)
                    .file_name()
                    .map(|n| n.to_str().unwrap_or("unknown"))
                    .unwrap_or("unknown");
                println!("      └─ {file_name}");
            }
        } else {
            println!("  📦 Single: {}", module_def.fqn);
        }
    }

    // 3. Inspect class hierarchies
    println!("\n📊 Class Hierarchy Analysis:");
    let classes: Vec<_> = graph_data
        .definition_nodes
        .iter()
        .filter(|def| {
            matches!(
                def.definition_type,
                parser_core::ruby::types::RubyDefinitionType::Class
            )
        })
        .collect();

    for class_def in &classes {
        println!("  🏗️  Class: {}", class_def.fqn);

        // Find methods in this class
        let class_methods: Vec<_> = graph_data
            .definition_relationships
            .iter()
            .filter(|rel| {
                rel.from_definition_fqn == class_def.fqn
                    && rel.relationship_type == "CLASS_TO_METHOD"
            })
            .collect();

        println!("     Methods: {}", class_methods.len());
        for method_rel in class_methods.iter().take(5) {
            // Show first 5 methods
            let method_name = method_rel
                .to_definition_fqn
                .split("::")
                .last()
                .unwrap_or("unknown");
            println!("     └─ {method_name}");
        }
        if class_methods.len() > 5 {
            println!("     └─ ... and {} more", class_methods.len() - 5);
        }
    }

    // 4. Inspect parent-child relationships
    println!("\n📊 Definition Relationships Analysis:");
    let relationship_counts: std::collections::HashMap<String, usize> = graph_data
        .definition_relationships
        .iter()
        .fold(std::collections::HashMap::new(), |mut acc, rel| {
            *acc.entry(rel.relationship_type.clone()).or_insert(0) += 1;
            acc
        });

    for (rel_type, count) in &relationship_counts {
        println!("  🔗 {rel_type}: {count}");
    }

    // 5. Verify specific expected definitions exist
    println!("\n📊 Expected Definitions Verification:");
    let expected_definitions = vec![
        ("Authentication", "Module"),
        ("Authentication::Providers", "Module"),
        ("Authentication::Providers::LdapProvider", "Class"),
        ("Authentication::Token", "Class"),
        ("UserManagement", "Module"),
        ("UserManagement::User", "Class"),
        ("BaseModel", "Class"),
        ("UserModel", "Class"),
    ];

    for (expected_fqn, expected_type) in expected_definitions {
        if let Some(def) = graph_data
            .definition_nodes
            .iter()
            .find(|d| d.fqn == expected_fqn)
        {
            println!("  ✅ Found: {} ({:?})", expected_fqn, def.definition_type);
            if expected_type == "Module" && def.file_locations.len() > 1 {
                println!("     🔄 Reopened in {} files", def.file_locations.len());
            }
        } else {
            println!("  ❌ Missing: {expected_fqn} ({expected_type})");
        }
    }

    // 6. Inspect file-definition relationships
    println!("\n📊 File-Definition Relationships:");
    let auth_file_rels: Vec<_> = graph_data
        .file_definition_relationships
        .iter()
        .filter(|rel| rel.definition_fqn.contains("Authentication"))
        .collect();

    println!(
        "  Authentication-related file relationships: {}",
        auth_file_rels.len()
    );
    for rel in auth_file_rels.iter().take(10) {
        let file_name = std::path::Path::new(&rel.file_path)
            .file_name()
            .map(|n| n.to_str().unwrap_or("unknown"))
            .unwrap_or("unknown");
        println!("    {} → {}", file_name, rel.definition_fqn);
    }

    println!("\n🎯 === SUMMARY ===");
    println!(
        "Total definition nodes: {}",
        graph_data.definition_nodes.len()
    );
    println!(
        "Total file-definition relationships: {}",
        graph_data.file_definition_relationships.len()
    );
    println!(
        "Total definition relationships: {}",
        graph_data.definition_relationships.len()
    );

    // Verify the specific Authentication module has exactly 3 locations
    let auth_main = graph_data
        .definition_nodes
        .iter()
        .find(|def| def.fqn == "Authentication")
        .expect("Should find Authentication module");

    assert_eq!(
        auth_main.file_locations.len(),
        3,
        "Authentication module should have exactly 3 file locations"
    );

    let file_names: Vec<_> = auth_main
        .file_locations
        .iter()
        .map(|loc| {
            std::path::Path::new(&loc.file_path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        })
        .collect();

    assert!(
        file_names.contains(&"authentication.rb"),
        "Should include authentication.rb"
    );
    assert!(
        file_names.contains(&"providers.rb"),
        "Should include providers.rb"
    );
    assert!(
        file_names.contains(&"tokens.rb"),
        "Should include tokens.rb"
    );

    println!("✅ All verification checks passed!");
}

#[test]
fn test_parquet_file_structure() {
    use std::fs;

    // Create temporary repository with test files
    let temp_repo = create_test_repository();
    let repo_path = temp_repo.path().to_str().unwrap();

    // Create a gitalisk repository wrapper
    let gitalisk_repo = CoreGitaliskRepository::new(repo_path.to_string(), repo_path.to_string());

    // Create our RepositoryIndexer wrapper
    let indexer = RepositoryIndexer::new("test-repo".to_string(), repo_path.to_string());
    let file_source = GitaliskFileSource::new(gitalisk_repo);

    // Configure indexing for Ruby files
    let config = IndexingConfig {
        worker_threads: 1,
        max_file_size: 5_000_000,
        respect_gitignore: false,
    };

    // Create a known output directory
    let output_dir = temp_repo.path().join("parquet_test_output");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");
    let output_path = output_dir.to_str().unwrap();

    // Run full processing pipeline
    let database = Arc::new(KuzuDatabase::new());
    let result = indexer
        .process_files_full(&database, file_source, &config, output_path)
        .expect("Failed to process repository");

    let writer_result = result.writer_result.expect("Should have writer result");

    println!("\n📁 === CONSOLIDATED PARQUET FILE STRUCTURE VERIFICATION ===");

    // List all generated Parquet files
    println!("\n📊 Generated Parquet Files:");
    for written_file in &writer_result.files_written {
        println!(
            "  📄 {} ({} records, {} bytes)",
            written_file
                .file_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap(),
            written_file.record_count,
            written_file.file_size_bytes
        );

        // Verify file exists and is not empty
        assert!(written_file.file_path.exists(), "Parquet file should exist");
        assert!(
            written_file.file_size_bytes > 0,
            "Parquet file should not be empty"
        );
    }

    // Check specific file types were created
    let file_types: Vec<_> = writer_result
        .files_written
        .iter()
        .map(|f| f.file_type.as_str())
        .collect();

    // Check for core node files (now with integer IDs)
    let required_node_files = vec!["directories", "files", "definitions"];
    for required_file in required_node_files {
        assert!(
            file_types.contains(&required_file),
            "Should have created {required_file} Parquet file"
        );
    }

    // Check for consolidated relationship files (NEW STRUCTURE)
    let required_relationship_files = vec![
        "directory_to_directory_relationships", // Replaces dir_contains_dir
        "directory_to_file_relationships",      // dir contains file
        "file_relationships",                   // Replaces file_definition_relationships
        "definition_relationships", // Replaces all MODULE_TO_*, CLASS_TO_*, METHOD_* files
    ];

    for required_file in required_relationship_files {
        assert!(
            file_types.contains(&required_file),
            "Should have created {required_file} Parquet file (consolidated schema)"
        );
    }

    // Focus on definitions file (should contain flattened structure with IDs)
    let definitions_file = writer_result
        .files_written
        .iter()
        .find(|f| f.file_type == "definitions")
        .expect("Should have definitions file");

    println!("\n📊 Definitions File Analysis (with Integer IDs):");
    println!("  📄 File: {}", definitions_file.file_path.display());
    println!("  📊 Records: {}", definitions_file.record_count);
    println!("  💾 Size: {} bytes", definitions_file.file_size_bytes);

    // Verify we have the correct number of records
    let graph_data = result.graph_data.expect("Should have graph data");
    let unique_definitions = graph_data.definition_nodes.len();
    let total_locations: usize = graph_data
        .definition_nodes
        .iter()
        .map(|d| d.file_locations.len())
        .sum();

    println!("  🔢 Unique definitions: {unique_definitions}");
    println!("  🔢 Total locations (flattened): {total_locations}");

    // The Parquet file should have one record per unique definition (using primary location + ID)
    assert_eq!(
        definitions_file.record_count, unique_definitions,
        "Parquet records should equal unique definitions (one per unique FQN with integer ID)"
    );

    // Verify Authentication module contributes 1 record (using primary location)
    let auth_def = graph_data
        .definition_nodes
        .iter()
        .find(|d| d.fqn == "Authentication")
        .expect("Should find Authentication module");

    println!(
        "  🔄 Authentication module locations: {}",
        auth_def.file_locations.len()
    );
    assert_eq!(
        auth_def.file_locations.len(),
        3,
        "Authentication should have 3 locations but only 1 Parquet record (with integer ID)"
    );

    // Verify consolidated relationship files contain expected data
    println!("\n📊 Consolidated Relationship Files:");

    // Directory relationships (DIR_CONTAINS_DIR + DIR_CONTAINS_FILE)
    let dir_rels_file = writer_result
        .files_written
        .iter()
        .find(|f| f.file_type == "directory_to_directory_relationships")
        .expect("Should have directory_to_directory_relationships file");

    println!(
        "  📁 Directory relationships: {} records",
        dir_rels_file.record_count
    );
    assert!(
        dir_rels_file.record_count > 0,
        "Should have directory relationship records"
    );

    // Directory to file relationships (DIR_CONTAINS_FILE)
    let dir_file_rels_file = writer_result
        .files_written
        .iter()
        .find(|f| f.file_type == "directory_to_file_relationships")
        .expect("Should have directory_to_file_relationships file");

    println!(
        "  📁 Directory to file relationships: {} records",
        dir_file_rels_file.record_count
    );
    assert!(
        dir_file_rels_file.record_count > 0,
        "Should have directory to file relationship records"
    );

    // File relationships (FILE_DEFINES)
    let file_rels_file = writer_result
        .files_written
        .iter()
        .find(|f| f.file_type == "file_relationships")
        .expect("Should have file_relationships file");

    println!(
        "  📄 File relationships: {} records",
        file_rels_file.record_count
    );

    // Should equal total definition locations (one FILE_DEFINES relationship per definition location)
    assert_eq!(
        file_rels_file.record_count, total_locations,
        "File relationships should equal total definition locations"
    );

    // Definition relationships (all MODULE_TO_*, CLASS_TO_*, METHOD_*)
    let def_rels_file = writer_result
        .files_written
        .iter()
        .find(|f| f.file_type == "definition_relationships")
        .expect("Should have definition_relationships file");

    println!(
        "  🔗 Definition relationships: {} records",
        def_rels_file.record_count
    );
    assert!(
        def_rels_file.record_count > 0,
        "Should have definition relationship records"
    );

    // Verify total relationship count matches expectation
    let total_relationship_records = dir_rels_file.record_count
        + dir_file_rels_file.record_count
        + file_rels_file.record_count
        + def_rels_file.record_count;

    let expected_total_relationships = writer_result.total_directory_relationships
        + writer_result.total_file_definition_relationships
        + writer_result.total_definition_relationships;

    assert_eq!(
        total_relationship_records, expected_total_relationships,
        "Total relationship records should match expected count"
    );

    println!("\n📊 Consolidated Schema Summary:");
    println!("  📁 Node files: 3");
    println!("  🔗 Relationship files: 3 (consolidated from 20+ separate files)");
    println!("  📋 Relationship types: mapped in relationship_types.json");
    println!("  🚀 Storage efficiency: Much improved with integer IDs and consolidated tables");

    println!("\n✅ Consolidated Parquet file structure verification completed!");
    println!("📁 Output directory: {}", output_dir.display());
}

/// Performs a comprehensive set of file system operations to test file watching
/// Returns a vector of (description, relative_path) for each change made
async fn make_comprehensive_file_changes(repo_path: &Path) -> Vec<(String, String)> {
    let mut changes_made = Vec::new();

    println!("Making comprehensive file changes...");

    // 1. Create new config file
    {
        println!("  Creating new config file");
        let config_path = repo_path.join("config.rb");
        tokio::fs::write(
            config_path,
            "# New config file\nmodule Config\n  VERSION = '1.0'\nend",
        )
        .await
        .unwrap();
        changes_made.push(("CREATE_FILE".to_string(), "config.rb".to_string()));
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    // 2. Modify existing file
    {
        println!("  Modifying existing base_model.rb");
        let base_model_path = repo_path.join("app/models/base_model.rb");
        let content = tokio::fs::read_to_string(&base_model_path).await.unwrap();
        tokio::fs::write(
            base_model_path,
            format!("{content}\n  # Added timestamp method\n  def timestamp\n    Time.now\n  end"),
        )
        .await
        .unwrap();
        changes_made.push((
            "MODIFY_FILE".to_string(),
            "app/models/base_model.rb".to_string(),
        ));
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    // 3. Create a utility file
    {
        println!("  Creating utility file");
        let utils_path = repo_path.join("app/utils/string_utils.rb");
        tokio::fs::create_dir_all(repo_path.join("app/utils"))
            .await
            .unwrap();
        let utils_content = r#"module StringUtils
  def self.sanitize(str)
    str.strip.downcase
  end

  def self.titleize(str)
    str.split(' ').map(&:capitalize).join(' ')
  end
end"#;
        tokio::fs::write(utils_path, utils_content).await.unwrap();
        changes_made.push((
            "CREATE_FILE".to_string(),
            "app/utils/string_utils.rb".to_string(),
        ));
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    // 4. Move file to new location
    {
        println!("  Moving user_model.rb to new location");
        let old_path = repo_path.join("app/models/user_model.rb");
        let new_path = repo_path.join("lib/models/user_model.rb");
        tokio::fs::create_dir_all(repo_path.join("lib/models"))
            .await
            .unwrap();
        tokio::fs::rename(old_path, new_path).await.unwrap();
        changes_made.push((
            "MOVE_FILE".to_string(),
            "app/models/user_model.rb -> lib/models/user_model.rb".to_string(),
        ));
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    // 5. Create another new file
    {
        println!("  Creating helper file");
        let helper_path = repo_path.join("lib/helper.rb");
        tokio::fs::write(
            helper_path,
            "# Helper module\nmodule Helper\n  def self.help\n    'helping'\n  end\nend",
        )
        .await
        .unwrap();
        changes_made.push(("CREATE_FILE".to_string(), "lib/helper.rb".to_string()));
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    // 6. Move directory with files
    {
        println!("  Moving authentication directory");
        let old_dir = repo_path.join("lib/authentication");
        let new_dir = repo_path.join("app/auth");
        tokio::fs::create_dir_all(&new_dir).await.unwrap();

        let files = vec!["providers.rb", "tokens.rb"];
        for file in files {
            let old_file = old_dir.join(file);
            let new_file = new_dir.join(file);
            if tokio::fs::try_exists(&old_file).await.unwrap_or(false) {
                tokio::fs::rename(old_file, new_file).await.unwrap();
                changes_made.push((
                    "MOVE_FILE".to_string(),
                    format!("lib/authentication/{file} -> app/auth/{file}"),
                ));
            }
        }

        // Remove old directory if empty
        if tokio::fs::try_exists(&old_dir).await.unwrap_or(false) {
            if let Ok(mut entries) = tokio::fs::read_dir(&old_dir).await {
                if entries.next_entry().await.unwrap().is_none() {
                    tokio::fs::remove_dir(old_dir).await.unwrap();
                    changes_made.push(("REMOVE_DIR".to_string(), "lib/authentication".to_string()));
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    // 7. Modify moved file
    {
        println!("  Modifying moved user_model.rb");
        let moved_model_path = repo_path.join("lib/models/user_model.rb");
        let content = tokio::fs::read_to_string(&moved_model_path).await.unwrap();
        tokio::fs::write(
            moved_model_path,
            format!("{content}\n  # Another added method\n  def updated_at\n    Time.now\n  end"),
        )
        .await
        .unwrap();
        changes_made.push((
            "MODIFY_FILE".to_string(),
            "lib/models/user_model.rb".to_string(),
        ));
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    // 8. Delete a file
    {
        println!("  Deleting helper file");
        let helper_path = repo_path.join("lib/helper.rb");
        if tokio::fs::try_exists(&helper_path).await.unwrap_or(false) {
            tokio::fs::remove_file(helper_path).await.unwrap();
            changes_made.push(("DELETE_FILE".to_string(), "lib/helper.rb".to_string()));
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    println!("Completed {} file system operations", changes_made.len());
    changes_made
}
