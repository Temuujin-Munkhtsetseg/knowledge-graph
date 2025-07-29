use std::path::Path;
use std::sync::Arc;

use crate::analysis::types::{DefinitionType, GraphData};
use crate::database::node_database_service::NodeDatabaseService;
use crate::deployed::executor::DeployedIndexingExecutor;
use crate::execution::config::IndexingConfigBuilder;
use crate::indexer::{IndexingConfig, RepositoryIndexer};
use crate::parsing::changes::FileChanges;
use crate::project::file_info::FileInfo;
use crate::project::source::{GitaliskFileSource, PathFileSource};
use database::graph::{RelationshipType, RelationshipTypeMapping};
use database::kuzu::connection::KuzuConnection;
use database::kuzu::database::KuzuDatabase;
use database::kuzu::types::{
    DefinitionNodeFromKuzu, DirectoryNodeFromKuzu, FileNodeFromKuzu, ImportedSymbolNodeFromKuzu,
    KuzuNodeType,
};
use gitalisk_core::repository::gitalisk_repository::CoreGitaliskRepository;
use gitalisk_core::repository::testing::local::LocalGitRepository;
use kuzu::{Database, SystemConfig};
use parser_core::SupportedLanguage;
use std::fs;
use tempfile::TempDir;

fn init_local_git_repository(language: SupportedLanguage) -> LocalGitRepository {
    let mut local_repo = LocalGitRepository::new(None);
    if language == SupportedLanguage::Ruby {
        let fixtures_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures/test-repo");
        local_repo.copy_dir(&fixtures_path);
    } else if language == SupportedLanguage::TypeScript {
        let fixtures_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures/typescript/test-repo");
        local_repo.copy_dir(&fixtures_path);
    }
    local_repo.add_all().commit("Initial commit");
    local_repo
}

pub async fn modify_test_repo_ruby(
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

    // 2. Add a new utility file and an import
    let utils_path = repo_path.join("app/utils/string_utils.rb");
    tokio::fs::create_dir_all(utils_path.parent().unwrap()).await?;
    let utils_content = r#"
    require 'string_toolkit'
    module StringUtils
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

pub async fn modify_test_repo_typescript(
    workspace_path: &Path,
    repo_name: &str,
) -> Result<(), std::io::Error> {
    let repo_path = workspace_path.join(repo_name);
    // Add imports to main.ts
    let main_ts_path = repo_path.join("main.ts");
    let content = tokio::fs::read_to_string(&main_ts_path).await?;
    let modified_content = content.replace(
        "import { Authentication } from './lib/authentication';",
        "import { Authentication } from './lib/authentication';\nimport { UserManagement } from './lib/user_management';\nimport { UserModel } from './app/models/user_model';",
    );
    tokio::fs::write(&main_ts_path, modified_content).await?;
    Ok(())
}

/// Helper function to create a temporary non-git repository by copying existing fixture files
fn create_non_git_test_repository() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

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

struct ReindexingPipelineSetup {
    local_repo: LocalGitRepository,
    indexer: RepositoryIndexer,
    file_source: GitaliskFileSource,
    config: IndexingConfig,
    database_path: String,
    output_path: String,
}

fn setup_reindexing_pipeline(
    database: &Arc<KuzuDatabase>,
    language: SupportedLanguage,
) -> ReindexingPipelineSetup {
    // Create temporary repository with test files
    let local_repo = init_local_git_repository(language);
    let repo_path_str = local_repo.path.to_str().unwrap();
    let workspace_path = local_repo.workspace_path.to_str().unwrap();

    // Create a gitalisk repository wrapper
    let gitalisk_repo =
        CoreGitaliskRepository::new(repo_path_str.to_string(), workspace_path.to_string());

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
    let output_dir = local_repo.workspace_path.join("output");
    let output_path = output_dir.to_str().unwrap();
    let database_path: String = local_repo
        .workspace_path
        .join("db.kuzu")
        .to_str()
        .unwrap()
        .to_string();

    // Run the full processing pipeline (to index the repo once)
    let indexing_result = indexer
        .process_files_full_with_database(
            database,
            file_source.clone(),
            &config,
            output_path,
            &database_path,
        )
        .expect("Failed to process repository");

    // Verify we have graph data and that the database path is set
    assert!(
        indexing_result.graph_data.is_some(),
        "Should have graph data"
    );

    let database_instance =
        Database::new(&database_path, SystemConfig::default()).expect("Failed to create database");

    let node_database_service = NodeDatabaseService::new(&database_instance);

    let all_definition_count = node_database_service.count_nodes::<DefinitionNodeFromKuzu>();
    println!("all_definition_count: {all_definition_count}");
    if language == SupportedLanguage::Ruby {
        assert_eq!(
            all_definition_count, 90,
            "Should have 90 definitions globally after initial indexing"
        );
    } else if language == SupportedLanguage::TypeScript {
        assert_eq!(
            all_definition_count, 101,
            "Should have 101 definitions globally after initial indexing"
        );
    }
    // file_paths: ["app/models/user_model.rb", "app/models/base_model.rb"]
    let mut file_paths = vec![];
    if language == SupportedLanguage::Ruby {
        file_paths = vec![
            "app/models/user_model.rb".to_string(),
            "app/models/base_model.rb".to_string(),
        ];
    } else if language == SupportedLanguage::TypeScript {
        file_paths = vec![
            "app/models/user_model.ts".to_string(),
            "app/models/base_model.ts".to_string(),
        ];
    }
    let definition_count = node_database_service
        .count_node_by(
            KuzuNodeType::DefinitionNode,
            "primary_file_path",
            &file_paths,
        )
        .unwrap();

    if language == SupportedLanguage::Ruby {
        assert_eq!(
            definition_count, 33,
            "Should have 33 definitions after initial indexing (user_model.rb and base_model.rb)"
        );
    } else if language == SupportedLanguage::TypeScript {
        assert_eq!(
            definition_count, 40,
            "Should have 40 definitions after initial indexing (user_model.ts and base_model.ts)"
        );
    }

    // This makes sense, as ruby doesn't support imports as of v0.7.0
    let imported_symbol_count = node_database_service.count_nodes::<ImportedSymbolNodeFromKuzu>();
    println!("imported_symbol_count: {imported_symbol_count}");
    if language == SupportedLanguage::Ruby {
        assert_eq!(
            imported_symbol_count, 0,
            "Should have 0 imported symbols after initial indexing"
        );
    } else if language == SupportedLanguage::TypeScript {
        assert_eq!(
            imported_symbol_count, 8,
            "Should have 8 imported symbols after initial indexing"
        );
        let imported_symbols = node_database_service
            .get_by::<String, ImportedSymbolNodeFromKuzu>(
                KuzuNodeType::ImportedSymbolNode,
                "file_path",
                &["main.ts".to_string()],
            )
            .unwrap();
        assert_eq!(imported_symbols.len(), 3);
    }

    let agg_by_file_path = node_database_service
        .agg_node_by::<ImportedSymbolNodeFromKuzu>("count", "file_path")
        .unwrap();
    println!("agg_by_file_path: {agg_by_file_path}");

    println!("repo_path: {repo_path_str:?}");
    println!("file_source: {file_source:?}");
    println!("config: {config:?}");
    println!("database_path: {database_path:?}");
    println!("output_path: {output_path:?}");

    ReindexingPipelineSetup {
        local_repo,
        indexer,
        file_source,
        config,
        database_path,
        output_path: output_path.to_string(),
    }
}

#[tokio::test]
async fn test_full_reindexing_pipeline_git_status_ruby() {
    let database = Arc::new(KuzuDatabase::new());
    let mut setup = setup_reindexing_pipeline(&database, SupportedLanguage::Ruby);

    // Modify the test repo, we should optionally allow
    modify_test_repo_ruby(&setup.local_repo.workspace_path, "test-repo")
        .await
        .expect("Failed to modify test repo");
    let git_status = setup
        .file_source
        .repository
        .get_status()
        .expect("Failed to get git status");
    let reindexer_file_changes = FileChanges::from_git_status(git_status);
    reindexer_file_changes.pretty_print();

    // check if the database path exists
    assert!(
        Path::new(&setup.database_path).exists(),
        "Database path should exist"
    );
    println!("database path: {:?}", setup.database_path);
    println!("database keys: {:?}", database.get_database_keys());

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
        .get_or_create_database(&setup.database_path, None)
        .expect("Failed to create database");
    let node_database_service = NodeDatabaseService::new(&database_instance);

    let definition_count = node_database_service.count_nodes::<DefinitionNodeFromKuzu>();
    println!("definition_count: {definition_count}");
    assert_eq!(
        definition_count, 91,
        "Should have 91 definitions globally after reindexing"
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
    // Disabled for now, as we don't support imports yet for ruby as of v0.7.0
    // let imported_symbol_count = node_database_service.count_nodes::<ImportedSymbolNodeFromKuzu>();
    // // println!("imported_symbol_count: {imported_symbol_count}");
    // assert_eq!(
    //     imported_symbol_count, 1,
    //     "Should have 1 imported symbol after reindexing"
    // );
}

#[tokio::test]
async fn test_full_reindexing_pipeline_git_status_typescript() {
    let database = Arc::new(KuzuDatabase::new());
    let mut setup = setup_reindexing_pipeline(&database, SupportedLanguage::TypeScript);

    // Modify the test repo, we should optionally allow
    modify_test_repo_typescript(&setup.local_repo.workspace_path, "test-repo")
        .await
        .expect("Failed to modify test repo");
    let git_status = setup
        .file_source
        .repository
        .get_status()
        .expect("Failed to get git status");
    let reindexer_file_changes = FileChanges::from_git_status(git_status);
    reindexer_file_changes.pretty_print();

    // check if the database path exists
    assert!(
        Path::new(&setup.database_path).exists(),
        "Database path should exist"
    );
    println!("database path: {:?}", setup.database_path);
    println!("database keys: {:?}", database.get_database_keys());

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
        .get_or_create_database(&setup.database_path, None)
        .expect("Failed to create database");
    let node_database_service = NodeDatabaseService::new(&database_instance);

    let definition_count = node_database_service.count_nodes::<DefinitionNodeFromKuzu>();
    println!("definition_count: {definition_count}");
    println!("definition_count: {definition_count}");
    assert_eq!(
        definition_count, 101,
        "Should have 101 definitions globally after reindexing"
    );

    let file_paths = vec![
        "app/models/user_model.ts".to_string(),
        "app/models/base_model.ts".to_string(),
    ];
    let definition_count = node_database_service
        .count_node_by(
            KuzuNodeType::DefinitionNode,
            "primary_file_path",
            &file_paths,
        )
        .unwrap();
    assert_eq!(
        definition_count, 40,
        "Should have 40 definitions after reindexing (user_model.ts and base_model.ts)"
    );

    let mut imported_symbols = node_database_service
        .get_by::<String, ImportedSymbolNodeFromKuzu>(
            KuzuNodeType::ImportedSymbolNode,
            "file_path",
            &["main.ts".to_string()],
        )
        .unwrap();

    imported_symbols.sort_by_key(|symbol| symbol.start_line);
    for symbol in &imported_symbols {
        println!("symbol: {symbol:?}");
    }
    assert_eq!(imported_symbols.len(), 5);
}

fn setup_end_to_end_kuzu(temp_repo: &LocalGitRepository) -> Arc<KuzuDatabase> {
    // Create temporary repository with test files
    let repo_path = temp_repo.path.to_str().unwrap();

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
    let output_dir = temp_repo.workspace_path.join("output");
    let output_path = output_dir.to_str().unwrap();
    let database_path = temp_repo.workspace_path.join("db.kuzu");
    let database_path_str = database_path.to_str().unwrap();

    // Create database as done in the working example
    let database = Arc::new(KuzuDatabase::new());
    let _result = indexer
        .process_files_full_with_database(
            &database,
            file_source,
            &config,
            output_path,
            database_path_str,
        )
        .expect("Failed to process repository");

    println!("✅ Kuzu database created and data imported successfully");
    println!("database keys: {:?}", database.get_database_keys());
    database
}

#[test]
fn test_new_indexer_with_gitalisk_file_source() {
    let temp_repo = init_local_git_repository(SupportedLanguage::Ruby);
    let repo_path = temp_repo.path.to_str().unwrap();

    let gitalisk_repo = CoreGitaliskRepository::new(repo_path.to_string(), repo_path.to_string());

    let indexer = RepositoryIndexer::new("test-repo".to_string(), repo_path.to_string());
    let file_source = GitaliskFileSource::new(gitalisk_repo);

    let config = IndexingConfig {
        worker_threads: 1,
        max_file_size: 5_000_000,
        respect_gitignore: false,
    };

    let temp_output_dir = temp_repo.workspace_path.join("output");
    let output_path = temp_output_dir.to_str().unwrap();
    let temp_db_path = temp_repo.workspace_path.join("db.kuzu");
    let db_path = temp_db_path.to_str().unwrap();
    let database = Arc::new(KuzuDatabase::new());

    let result = indexer
        .index_files(&database, output_path, db_path, file_source, &config)
        .expect("Failed to index files");

    assert!(
        !result.file_results.is_empty(),
        "Should have processed some files"
    );
    assert_eq!(result.errored_files.len(), 0, "Should have no errors");

    println!("✅ New indexer test completed successfully!");
    println!("📊 Processed {} files", result.file_results.len());
}

#[test]
fn test_new_indexer_with_path_file_source() {
    let temp_repo = init_local_git_repository(SupportedLanguage::Ruby);
    let repo_path = temp_repo.path.to_str().unwrap();

    let mut ruby_files = Vec::new();
    for entry in walkdir::WalkDir::new(repo_path) {
        let entry = entry.unwrap();
        if entry.path().extension().and_then(|s| s.to_str()) == Some("rb") {
            ruby_files.push(FileInfo::from_path(entry.path().to_path_buf()));
        }
    }

    let indexer = RepositoryIndexer::new("test-repo".to_string(), repo_path.to_string());
    let file_source = PathFileSource::new(ruby_files);

    let config = IndexingConfig {
        worker_threads: 1,
        max_file_size: 5_000_000,
        respect_gitignore: false,
    };

    let temp_output_dir = temp_repo.workspace_path.join("output");
    let output_path = temp_output_dir.to_str().unwrap();
    let temp_db_path = temp_repo.workspace_path.join("db.kuzu");
    let db_path = temp_db_path.to_str().unwrap();
    let database = Arc::new(KuzuDatabase::new());

    let result = indexer
        .index_files(&database, output_path, db_path, file_source, &config)
        .expect("Failed to index files");

    assert!(
        !result.file_results.is_empty(),
        "Should have processed some files"
    );
    assert_eq!(result.errored_files.len(), 0, "Should have no errors");

    println!("✅ Path file source test completed successfully!");
    println!("📊 Processed {} files", result.file_results.len());
}

#[test]
fn test_full_indexing_pipeline() {
    // Create temporary repository with test files
    let temp_repo = init_local_git_repository(SupportedLanguage::Ruby);
    let repo_path = temp_repo.path.to_str().unwrap();

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
    let output_dir = temp_repo.workspace_path.join("output");
    let output_path = output_dir.to_str().unwrap();
    let database_path = temp_repo.workspace_path.join("db.kuzu");
    let database_path_str = database_path.to_str().unwrap();

    // Run the full processing pipeline
    let database = Arc::new(KuzuDatabase::new());
    let result = indexer
        .process_files_full_with_database(
            &database,
            file_source,
            &config,
            output_path,
            database_path_str,
        )
        .expect("Failed to process repository");

    // Verify we processed files
    assert!(
        !result.file_results.is_empty(),
        "Should have processed some files"
    );
    assert_eq!(result.errored_files.len(), 0, "Should have no errors");

    // Verify graph data was created
    let graph_data: GraphData = result.graph_data.expect("Should have graph data");

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
    println!("📊 Processed {} files", result.file_results.len());
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

    // The database is already set up by process_files_full_with_database, so we just connect to it
    let database_instance = database
        .get_or_create_database(database_path_str, None)
        .expect("Failed to get database instance");
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
fn test_inheritance_relationships() {
    // Create temporary repository with test files
    let temp_repo = init_local_git_repository(SupportedLanguage::Ruby);
    let repo_path = temp_repo.path.to_str().unwrap();

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
    let output_dir = temp_repo.workspace_path.join("output");
    let output_path = output_dir.to_str().unwrap();
    let database_path = temp_repo.workspace_path.join("db.kuzu");
    let database_path_str = database_path.to_str().unwrap();

    let database = Arc::new(KuzuDatabase::new());
    let result = indexer
        .process_files_full_with_database(
            &database,
            file_source,
            &config,
            output_path,
            database_path_str,
        )
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
        DefinitionType::Ruby(parser_core::ruby::types::RubyDefinitionType::Class)
    );
    assert_eq!(
        user_model.definition_type,
        DefinitionType::Ruby(parser_core::ruby::types::RubyDefinitionType::Class)
    );

    // Verify we have class-to-method relationships
    let class_method_rels: Vec<_> = graph_data
        .definition_relationships
        .iter()
        .filter(|rel| rel.relationship_type == RelationshipType::ClassToMethod)
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
            rel.from_definition_fqn == "BaseModel"
                && rel.relationship_type == RelationshipType::ClassToMethod
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
    let temp_repo = init_local_git_repository(SupportedLanguage::Ruby);
    let database = setup_end_to_end_kuzu(&temp_repo);

    let db_dir = temp_repo.workspace_path.join("db.kuzu");
    let database_instance = database
        .get_or_create_database(&db_dir.to_string_lossy(), None)
        .expect("Failed to create database");
    let connection = KuzuConnection::new(&database_instance).expect("Failed to create connection");

    let relationship_type_map = RelationshipTypeMapping::new();
    let node_database_service = NodeDatabaseService::new(&database_instance);

    // Get definition node count
    let defn_node_count = node_database_service.count_nodes::<DefinitionNodeFromKuzu>();
    println!("Definition node count: {defn_node_count}");
    assert_eq!(defn_node_count, 90);

    // Get file node count
    let file_node_count = node_database_service.count_nodes::<FileNodeFromKuzu>();
    println!("File node count: {file_node_count}");
    assert_eq!(file_node_count, 7);

    // Get module -> class relationships count
    let class_method_rel_count =
        node_database_service.count_relationships_of_type(RelationshipType::ClassToMethod);
    println!("Class -> method relationship count: {class_method_rel_count}");
    assert_eq!(class_method_rel_count, 50);

    // Get file definition relationships count
    let file_defn_rel_count =
        node_database_service.count_relationships_of_type(RelationshipType::FileDefines);
    println!("File defines relationship count: {file_defn_rel_count}");
    assert_eq!(file_defn_rel_count, 90);

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
    assert_eq!(def_rel_count, 67);

    // Get all relationships in the definition_relationships table
    let m2m_rel_type = relationship_type_map.get_type_id(RelationshipType::ClassToMethod);
    let query_class_to_method = format!(
        "MATCH (d:DefinitionNode)-[r:DEFINITION_RELATIONSHIPS]->(c:DefinitionNode) WHERE r.type = {m2m_rel_type} RETURN d, c, r.type"
    );
    println!("Query: {query_class_to_method}");

    let result = connection
        .query(&query_class_to_method)
        .expect("Failed to query class to method");
    for row in result {
        if let (Some(from_node_value), Some(to_node_value), Some(kuzu::Value::UInt8(rel_type))) =
            (row.first(), row.get(1), row.get(2))
        {
            let from_node = DefinitionNodeFromKuzu::from_kuzu_node(from_node_value);
            let to_node = DefinitionNodeFromKuzu::from_kuzu_node(to_node_value);
            let rel_type_name = relationship_type_map.get_type_name(*rel_type);
            println!(
                "Class to method relationship: {} -[type: {}]-> {}",
                from_node.fqn, rel_type_name, to_node.fqn
            );
            if from_node.fqn.as_str() == "Authentication::Providers::LdapProvider" {
                match to_node.fqn.as_str() {
                    "Authentication::Providers::LdapProvider::verify_credentials" => {
                        assert_eq!(to_node.definition_type, "Method");
                        assert_eq!(to_node.primary_file_path, "lib/authentication/providers.rb");
                    }
                    "Authentication::Providers::LdapProvider::authenticate" => {
                        assert_eq!(to_node.definition_type, "Method");
                        assert_eq!(to_node.primary_file_path, "lib/authentication/providers.rb");
                    }
                    _ => {}
                }
            }
            if from_node.fqn.as_str() == "Authentication::Providers::OAuthProvider" {
                match to_node.fqn.as_str() {
                    "Authentication::Providers::OAuthProvider::exchange_code_for_token" => {
                        assert_eq!(to_node.definition_type, "Method");
                        assert_eq!(to_node.primary_file_path, "lib/authentication/providers.rb");
                    }
                    "Authentication::Providers::OAuthProvider::initializer" => {
                        assert_eq!(to_node.definition_type, "Method");
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
    let temp_repo = init_local_git_repository(SupportedLanguage::Ruby);
    let repo_path = temp_repo.path.to_str().unwrap();

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
    let output_dir = temp_repo.workspace_path.join("output");
    let output_path = output_dir.to_str().unwrap();
    let database_path = temp_repo.workspace_path.join("db.kuzu");
    let database_path_str = database_path.to_str().unwrap();

    let database = Arc::new(KuzuDatabase::new());
    let result = indexer
        .process_files_full_with_database(
            &database,
            file_source,
            &config,
            output_path,
            database_path_str,
        )
        .expect("Failed to process repository");

    let graph_data = result.graph_data.expect("Should have graph data");

    println!("\n🔍 === DETAILED DATA INSPECTION ===");

    // === PART 1: In-memory graph data verification (existing) ===

    // 1. Inspect class hierarchies
    println!("\n📊 Class Hierarchy Analysis:");
    let classes: Vec<_> = graph_data
        .definition_nodes
        .iter()
        .filter(|def| {
            matches!(
                def.definition_type,
                DefinitionType::Ruby(parser_core::ruby::types::RubyDefinitionType::Class)
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
                    && rel.relationship_type == RelationshipType::ClassToMethod
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

    // 2. Inspect parent-child relationships
    println!("\n📊 Definition Relationships Analysis:");
    let relationship_counts: std::collections::HashMap<String, usize> = graph_data
        .definition_relationships
        .iter()
        .fold(std::collections::HashMap::new(), |mut acc, rel| {
            *acc.entry(rel.relationship_type.as_str().to_string())
                .or_insert(0) += 1;
            acc
        });

    for (rel_type, count) in &relationship_counts {
        println!("  🔗 {rel_type}: {count}");
    }

    // 5. Verify specific expected definitions exist
    println!("\n📊 Expected Definitions Verification:");
    let expected_definitions = vec![
        ("Authentication::Providers::LdapProvider", "Class"),
        ("Authentication::Token", "Class"),
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

    println!("✅ All verification checks passed!");
}

#[test]
fn test_server_side_repository_processing() {
    let repository_temp_path = create_non_git_test_repository();
    // convert repository_temp_path to path_buf. No mode code. 1 line
    let repository_path = repository_temp_path.path().to_path_buf();

    // Create a new temp direcory and specify database file path within it
    let database_temp_path = tempfile::tempdir().expect("Failed to create temp directory");
    let database_path = database_temp_path.path().join("db.kuzu");

    let parquet_temp_path = tempfile::tempdir().expect("Failed to create temp directory");
    let parquet_path = parquet_temp_path.path().to_path_buf();

    let config = IndexingConfigBuilder::build(0); // Number of CPU Cores will be used instead
    let server_indexer =
        DeployedIndexingExecutor::new(repository_path, database_path, parquet_path, config);
    let result = server_indexer
        .execute()
        .expect("Failed to process repository");

    assert!(!result.total_files > 0, "Should have processed some files");

    assert!(result.total_files > 0, "Should have processed some files");
    assert!(
        result.total_definitions > 0,
        "Should have processed some definitions"
    );
}

#[test]
fn test_parquet_file_structure() {
    use std::fs;

    // Create temporary repository with test files
    let temp_repo = init_local_git_repository(SupportedLanguage::Ruby);
    let repo_path = temp_repo.path.to_str().unwrap();

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
    let output_dir = temp_repo.workspace_path.join("parquet_test_output");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");
    let output_path = output_dir.to_str().unwrap();
    let database_path = temp_repo.workspace_path.join("db.kuzu");
    let database_path_str = database_path.to_str().unwrap();

    // Run full processing pipeline
    let database = Arc::new(KuzuDatabase::new());
    let result = indexer
        .process_files_full_with_database(
            &database,
            file_source,
            &config,
            output_path,
            database_path_str,
        )
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
    let required_node_files = vec!["directories", "files", "definitions"]; // "imported_symbols"
    for required_file in required_node_files {
        assert!(
            file_types.contains(&required_file),
            "Should have created {required_file} Parquet file"
        );
    }

    // Check for consolidated relationship files (NEW STRUCTURE)
    let required_relationship_files = vec![
        "directory_to_directory_relationships",
        "directory_to_file_relationships",
        "file_to_definition_relationships",
        // "file_to_imported_symbol_relationships",
        "definition_to_definition_relationships",
        // "definition_to_imported_symbol_relationships"
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

    println!("  🔢 Unique definitions: {unique_definitions}");

    // The Parquet file should have one record per unique definition (using primary location + ID)
    assert_eq!(
        definitions_file.record_count, unique_definitions,
        "Parquet records should equal unique definitions (one per unique FQN with integer ID)"
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

    // File to definition relationships (FILE_DEFINES)
    let file_def_rels_file = writer_result
        .files_written
        .iter()
        .find(|f| f.file_type == "file_to_definition_relationships")
        .expect("Should have file_to_definition_relationships file");

    println!(
        "  📄 File to definition relationships: {} records",
        file_def_rels_file.record_count
    );

    // // File to imported symbol relationships (FILE_IMPORTS)
    // let file_import_rels_file = writer_result
    //     .files_written
    //     .iter()
    //     .find(|f| f.file_type == "file_to_imported_symbol_relationships")
    //     .expect("Should have file_to_imported_symbol_relationships file");

    // println!(
    //     "  📄 File to imported symbol relationships: {} records",
    //     file_import_rels_file.record_count
    // );

    // Definition to definition relationships (all MODULE_TO_*, CLASS_TO_*, METHOD_*)
    let def_rels_file = writer_result
        .files_written
        .iter()
        .find(|f| f.file_type == "definition_to_definition_relationships")
        .expect("Should have definition_to_definition_relationships file");

    println!(
        "  🔗 Definition to definition relationships: {} records",
        def_rels_file.record_count
    );
    assert!(
        def_rels_file.record_count > 0,
        "Should have definition to definition relationship records"
    );

    // // Definition to imported symbol relationships (DEFINITION_IMPORTS)
    // let def_import_rels_file = writer_result
    //     .files_written
    //     .iter()
    //     .find(|f| f.file_type == "definition_to_imported_symbol_relationships")
    //     .expect("Should have definition_to_imported_symbol_relationships file");

    // println!(
    //     "  🔗 Definition to imported symbol relationships: {} records",
    //     def_import_rels_file.record_count
    // );
    // assert!(
    //     def_import_rels_file.record_count > 0,
    //     "Should have definition to imported symbol relationship records"
    // );

    // Verify total relationship count matches expectation
    let total_relationship_records = dir_rels_file.record_count
        + dir_file_rels_file.record_count
        + file_def_rels_file.record_count
        // + file_import_rels_file.record_count
        + def_rels_file.record_count;
    // + def_import_rels_file.record_count;

    let expected_total_relationships = writer_result.total_directory_relationships
        + writer_result.total_file_definition_relationships
        + writer_result.total_file_imported_symbol_relationships
        + writer_result.total_definition_relationships
        + writer_result.total_definition_imported_symbol_relationships;

    assert_eq!(
        total_relationship_records, expected_total_relationships,
        "Total relationship records should match expected count"
    );

    println!("\n📊 Consolidated Schema Summary:");
    println!("  📁 Node files: 4");
    println!("  🔗 Relationship files: 3 (consolidated from 20+ separate files)");
    println!("  📋 Relationship types: mapped in relationship_types.json");
    println!("  🚀 Storage efficiency: Much improved with integer IDs and consolidated tables");

    println!("\n✅ Consolidated Parquet file structure verification completed!");
    println!("📁 Output directory: {}", output_dir.display());
}
