use std::path::Path;
use std::sync::Arc;

use crate::indexer::{IndexingConfig, RepositoryIndexer};
use crate::project::source::GitaliskFileSource;
use database::kuzu::database::KuzuDatabase;
use database::kuzu::service::NodeDatabaseService;
use gitalisk_core::repository::gitalisk_repository::CoreGitaliskRepository;
use gitalisk_core::repository::testing::local::LocalGitRepository;
use tracing_test::traced_test;

fn init_java_references_repository() -> LocalGitRepository {
    let mut local_repo = LocalGitRepository::new(None);
    let fixtures_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/java");
    local_repo.copy_dir(&fixtures_path);
    local_repo
        .add_all()
        .commit("Initial commit with Java reference examples");
    local_repo
}

struct JavaReferenceTestSetup {
    _local_repo: LocalGitRepository,
    database_path: String,
}

async fn setup_java_reference_pipeline(database: &Arc<KuzuDatabase>) -> JavaReferenceTestSetup {
    let local_repo = init_java_references_repository();
    let repo_path_str = local_repo.path.to_str().unwrap();
    let workspace_path = local_repo.workspace_path.to_str().unwrap();

    let gitalisk_repo =
        CoreGitaliskRepository::new(repo_path_str.to_string(), workspace_path.to_string());

    let indexer = RepositoryIndexer::new(
        "java-references-test".to_string(),
        repo_path_str.to_string(),
    );
    let file_source: GitaliskFileSource = GitaliskFileSource::new(gitalisk_repo.clone());

    let config = IndexingConfig {
        worker_threads: 1,
        max_file_size: 5_000_000,
        respect_gitignore: false,
    };

    let output_dir = local_repo.workspace_path.join("output");
    let output_path = output_dir.to_str().unwrap();

    let process_id = std::process::id();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let database_path: String = local_repo
        .workspace_path
        .join(format!("database_{}_{}.kz", process_id, timestamp))
        .to_str()
        .unwrap()
        .to_string();

    let _indexing_result = indexer
        .process_files_full_with_database(
            database,
            file_source.clone(),
            &config,
            output_path,
            &database_path,
        )
        .await
        .expect("Failed to process repository");

    // Ensure DB can be opened
    let _db = database
        .get_or_create_database(&database_path, None)
        .expect("Database should be accessible after setup completion");

    JavaReferenceTestSetup {
        _local_repo: local_repo,
        database_path,
    }
}

#[traced_test]
#[tokio::test]
async fn test_java_reference_resolution_main() {
    let database = Arc::new(KuzuDatabase::new());
    let setup = setup_java_reference_pipeline(&database).await;

    let database_instance = database
        .get_or_create_database(&setup.database_path, None)
        .expect("Failed to create database");
    let node_database_service = NodeDatabaseService::new(&database_instance);

    // Main.main -> Traceable
    let callers_to_traceable = node_database_service
        .find_calls_to_method("com.example.app.Traceable")
        .unwrap_or_default();
    assert!(
        callers_to_traceable
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should have a Traceable annotation"
    );

    // Main.main -> new Foo()
    let callers_to_foo = node_database_service
        .find_calls_to_method("com.example.app.Foo")
        .unwrap_or_default();

    assert!(
        callers_to_foo
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.Main")),
        "Main.Main should call Foo"
    );

    // Main.main -> this.myParameter.bar()
    let callers_to_foo_bar = node_database_service
        .find_calls_to_method("com.example.app.Foo.bar")
        .unwrap_or_default();
    assert!(
        callers_to_foo_bar
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call Foo.bar"
    );

    // Main.main -> Bar.baz (pattern variable)
    let callers_to_baz = node_database_service
        .find_calls_to_method("com.example.app.Bar.baz")
        .unwrap_or_default();
    assert!(
        callers_to_baz
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call Bar.baz"
    );

    // Main.main -> Executor.execute (method reference)
    let callers_to_execute = node_database_service
        .find_calls_to_method("com.example.app.Executor.execute")
        .unwrap_or_default();
    assert!(
        callers_to_execute
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call Executor.execute"
    );

    // Main.main -> Main.await
    let callers_to_await = node_database_service
        .find_calls_to_method("com.example.app.Main.await")
        .unwrap_or_default();
    assert!(
        callers_to_await
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call Main.await"
    );

    // Main.main -> Application.run (through super)
    let callers_to_application_run = node_database_service
        .find_calls_to_method("com.example.app.Application.run")
        .unwrap_or_default();
    assert!(
        callers_to_application_run
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call Application.run through super"
    );

    // Main.main -> Outer.make
    let callers_to_outer_make = node_database_service
        .find_calls_to_method("com.example.util.Outer.make")
        .unwrap_or_default();
    assert!(
        callers_to_outer_make
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call Outer.make via direct import resolution"
    );

    // Main.main -> Outer.outerMethod
    let callers_to_outer_outer_method = node_database_service
        .find_calls_to_method("com.example.util.Outer.outerMethod")
        .unwrap_or_default();
    assert!(
        callers_to_outer_outer_method
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call Outer.outerMethod via resolved variable type"
    );

    // Main.main -> Outer.Inner
    let callers_to_outer_inner = node_database_service
        .find_calls_to_method("com.example.util.Outer.Inner")
        .unwrap_or_default();
    assert!(
        callers_to_outer_inner
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call Outer.Inner"
    );

    // Main.main -> Outer.Inner.innerMethod
    let callers_to_inner_inner_method = node_database_service
        .find_calls_to_method("com.example.util.Outer.Inner.innerMethod")
        .unwrap_or_default();
    assert!(
        callers_to_inner_inner_method
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call Outer.Inner.innerMethod"
    );

    // Main.main -> Outer.Inner.innerStatic
    let callers_to_inner_inner_static = node_database_service
        .find_calls_to_method("com.example.util.Outer.Inner.innerStatic")
        .unwrap_or_default();
    assert!(
        callers_to_inner_inner_static
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call Outer.Inner.innerStatic"
    );

    // Main.main -> EnumClass.ENUM_VALUE_1.enumMethod1
    let callers_to_enum_value_1_enum_method_1 = node_database_service
        .find_calls_to_method("com.example.app.EnumClass.enumMethod1")
        .unwrap_or_default();
    assert!(
        callers_to_enum_value_1_enum_method_1
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call EnumClass.enumMethod1"
    );

    // Main.main -> EnumClass.ENUM_VALUE_2.enumMethod2
    let callers_to_enum_value_2_enum_method_2 = node_database_service
        .find_calls_to_method("com.example.app.EnumClass.enumMethod2")
        .unwrap_or_default();
    assert!(
        callers_to_enum_value_2_enum_method_2
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call EnumClass.enumMethod2"
    );
}

#[traced_test]
#[tokio::test]
async fn test_java_reference_resolution_to_imported_symbol() {
    let database = Arc::new(KuzuDatabase::new());
    let setup = setup_java_reference_pipeline(&database).await;

    let database_instance = database
        .get_or_create_database(&setup.database_path, None)
        .expect("Failed to create database");
    let node_database_service = NodeDatabaseService::new(&database_instance);

    // Main.main -> java.util.ArrayList
    let callers_to_array_list = node_database_service
        .find_calls_to_imported_symbol("java.util", "ArrayList")
        .unwrap_or_default();
    assert!(
        callers_to_array_list
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call ArrayList"
    );

    // Main.main -> java.util.List.of
    let callers_to_array_list_of = node_database_service
        .find_calls_to_imported_symbol("java.util", "List")
        .unwrap_or_default();
    assert!(
        callers_to_array_list_of
            .iter()
            .any(|c| c.ends_with("com.example.app.Main.main")),
        "Main.main should call List"
    );

    // Traceable -> java.lang.annotation.Retention
    let callers_to_retention = node_database_service
        .find_calls_to_imported_symbol("java.lang.annotation", "Retention")
        .unwrap_or_default();

    assert!(
        callers_to_retention
            .iter()
            .any(|c| c.ends_with("com.example.app.Traceable")),
        "Traceable should have a Retention annotation"
    );

    // Traceable -> java.lang.annotation.Target
    let callers_to_target = node_database_service
        .find_calls_to_imported_symbol("java.lang.annotation", "Target")
        .unwrap_or_default();
    assert!(
        callers_to_target
            .iter()
            .any(|c| c.ends_with("com.example.app.Traceable")),
        "Traceable should have a Retention annotation"
    );
}
