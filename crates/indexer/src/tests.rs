use crate::indexer::{IndexingConfig, RepositoryIndexer};
use crate::project::file_info::FileInfo;
use crate::project::source::{GitaliskFileSource, PathFileSource};
use gitalisk_core::repository::gitalisk_repository::CoreGitaliskRepository;
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

    println!("\n📁 === PARQUET FILE STRUCTURE VERIFICATION ===");

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

    // Check for core node files
    let required_node_files = vec![
        "directories",
        "files",
        "definitions",
        "file_definition_relationships",
    ];
    for required_file in required_node_files {
        assert!(
            file_types.contains(&required_file),
            "Should have created {required_file} Parquet file"
        );
    }

    // Check for directory relationship files (separated by type)
    let has_dir_relationships = file_types.iter().any(|f| f.contains("dir_contains"));
    assert!(
        has_dir_relationships,
        "Should have created directory relationship files"
    );

    // Check for definition relationship files (separated by type)
    let definition_rel_types = [
        "MODULE_TO_CLASS",
        "CLASS_TO_METHOD",
        "MODULE_TO_MODULE",
        "CLASS_TO_SINGLETON_METHOD",
        "MODULE_TO_SINGLETON_METHOD",
    ];
    let has_def_relationships = definition_rel_types
        .iter()
        .any(|rel_type| file_types.contains(rel_type));
    assert!(
        has_def_relationships,
        "Should have created definition relationship files"
    );

    // Focus on definitions file (should contain flattened structure)
    let definitions_file = writer_result
        .files_written
        .iter()
        .find(|f| f.file_type == "definitions")
        .expect("Should have definitions file");

    println!("\n📊 Definitions File Analysis:");
    println!("  📄 File: {}", definitions_file.file_path.display());
    println!("  📊 Records: {}", definitions_file.record_count);
    println!("  💾 Size: {} bytes", definitions_file.file_size_bytes);

    // Verify we have more records than definition nodes (due to flattening)
    let graph_data = result.graph_data.expect("Should have graph data");
    let unique_definitions = graph_data.definition_nodes.len();
    let total_locations: usize = graph_data
        .definition_nodes
        .iter()
        .map(|d| d.file_locations.len())
        .sum();

    println!("  🔢 Unique definitions: {unique_definitions}");
    println!("  🔢 Total locations (flattened): {total_locations}");

    // The Parquet file should have one record per unique definition (not per location)
    assert_eq!(
        definitions_file.record_count, unique_definitions,
        "Parquet records should equal unique definitions (using primary location)"
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
        "Authentication should have 3 locations but only 1 Parquet record"
    );

    // Verify file-definition relationships count
    let file_def_rels_file = writer_result
        .files_written
        .iter()
        .find(|f| f.file_type == "file_definition_relationships")
        .expect("Should have file_definition_relationships file");

    println!("\n📊 File-Definition Relationships:");
    println!("  📊 Records: {}", file_def_rels_file.record_count);

    // Should be one relationship per definition location (this remains flattened)
    assert_eq!(
        file_def_rels_file.record_count, total_locations,
        "File-definition relationships should equal total locations"
    );

    println!("\n✅ Parquet file structure verification completed!");
    println!("📁 Output directory: {}", output_dir.display());
}
