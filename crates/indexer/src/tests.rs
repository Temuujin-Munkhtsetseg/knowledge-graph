use crate::database::schema::SchemaManager;
use crate::database::types::KuzuNodeType;
use crate::database::types::{DefinitionNodeFromKuzu, DirectoryNodeFromKuzu, FileNodeFromKuzu};
use crate::database::utils::{RelationshipType, RelationshipTypeMapping};
use crate::database::{DatabaseConfig, KuzuConnection};
use crate::indexer::{IndexingConfig, RepositoryIndexer};
use crate::project::file_info::FileInfo;
use crate::project::source::{GitaliskFileSource, PathFileSource};
use gitalisk_core::repository::gitalisk_repository::CoreGitaliskRepository;
use kuzu::{Database, SystemConfig};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

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

    let _result = indexer
        .process_files_full(file_source, &config, output_path)
        .expect("Failed to process repository");

    // Create Kuzu database
    let db_dir = temp_repo.path().join("kuzu_db");

    let config = DatabaseConfig::new(db_dir.to_string_lossy())
        .with_buffer_size(512 * 1024 * 1024)
        .with_compression(true);

    let database = KuzuConnection::create_database(config).expect("Failed to create Kuzu database");
    let connection = KuzuConnection::new(&database, db_dir.to_string_lossy().to_string())
        .expect("Failed to create Kuzu connection");

    // Initialize schema
    let schema_manager = SchemaManager::new();
    schema_manager
        .initialize_schema(&connection)
        .expect("Failed to initialize schema");

    // Import Parquet data
    schema_manager
        .import_graph_data(&connection, output_path)
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
    let result = indexer
        .process_files_full(file_source, &config, output_path)
        .expect("Failed to process repository");

    // Verify we processed files
    assert!(
        result.total_files_processed > 0,
        "Should have processed some files"
    );
    assert_eq!(result.total_files_errored, 0, "Should have no errors");

    // Verify graph data was created
    let graph_data = result.graph_data.expect("Should have graph data");

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

    let database = KuzuConnection::create_database(config).expect("Failed to create Kuzu database");

    let connection = KuzuConnection::new(&database, db_dir.to_string_lossy().to_string())
        .expect("Failed to create Kuzu connection");

    // Initialize schema
    let schema_manager = SchemaManager::new();
    schema_manager
        .initialize_schema(&connection)
        .expect("Failed to initialize schema");

    // Import Parquet data
    schema_manager
        .import_graph_data(&connection, output_path)
        .expect("Failed to import graph data");

    println!("✅ Kuzu database created and data imported successfully");

    // Verify basic node counts
    println!("\n📊 Kuzu Database Node Counts:");
    let node_counts = connection
        .get_node_counts()
        .expect("Failed to get node counts");

    println!("  📁 Directory nodes: {}", node_counts.directory_count);
    println!("  📄 File nodes: {}", node_counts.file_count);
    println!("  🏗️  Definition nodes: {}", node_counts.definition_count);

    // Verify relationship counts
    println!("\n📊 Kuzu Database Relationship Counts:");
    let rel_counts = connection
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

    let result = indexer
        .process_files_full(file_source, &config, output_path)
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
    let database = Database::new(
        db_dir.to_string_lossy().to_string(),
        SystemConfig::default(),
    )
    .expect("Failed to create database");
    let connection = KuzuConnection::new(&database, db_dir.to_string_lossy().to_string())
        .expect("Failed to create connection");

    let relationship_type_map = RelationshipTypeMapping::new();

    // Get definition node count
    let defn_node_count = connection.count_nodes(KuzuNodeType::DefinitionNode);
    println!("Definition node count: {defn_node_count}");
    assert_eq!(defn_node_count, 94);

    // Get file node count
    let file_node_count = connection.count_nodes(KuzuNodeType::FileNode);
    println!("File node count: {file_node_count}");
    assert_eq!(file_node_count, 7);

    // Get module -> class relationships count
    let module_class_rel_count =
        connection.count_relationships_of_type(RelationshipType::ModuleToClass);
    println!("Module -> class relationship count: {module_class_rel_count}");
    assert_eq!(module_class_rel_count, 7);

    // Get file definition relationships count
    let file_defn_rel_count = connection.count_relationships_of_type(RelationshipType::FileDefines);
    println!("File defines relationship count: {file_defn_rel_count}");
    assert_eq!(file_defn_rel_count, 96);

    // Get directory node count
    let dir_node_count = connection.count_nodes(KuzuNodeType::DirectoryNode);
    println!("Directory node count: {dir_node_count}");
    assert_eq!(dir_node_count, 4);

    // get directory -> file relationships count
    let dir_file_rel_count =
        connection.count_relationships_of_type(RelationshipType::DirContainsFile);
    println!("Directory -> file relationship count: {dir_file_rel_count}");
    assert_eq!(dir_file_rel_count, 6);

    // get directory -> directory relationships count
    let dir_dir_rel_count =
        connection.count_relationships_of_type(RelationshipType::DirContainsDir);
    println!("Directory -> directory relationship count: {dir_dir_rel_count}");
    assert_eq!(dir_dir_rel_count, 2);

    // get definition relationships count
    let def_rel_count = connection.count_relationships_of_node_type(KuzuNodeType::DefinitionNode);
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

    let result = indexer
        .process_files_full(file_source, &config, output_path)
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
    let result = indexer
        .process_files_full(file_source, &config, output_path)
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
