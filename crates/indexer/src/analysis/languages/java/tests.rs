use gitalisk_core::repository::gitalisk_repository::CoreGitaliskRepository;
use gitalisk_core::repository::testing::local::LocalGitRepository;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use workspace_manager::WorkspaceManager;

use crate::indexer::{IndexingConfig, RepositoryIndexer};
use crate::project::source::GitaliskFileSource;
use database::kuzu::database::KuzuDatabase;

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

pub struct JavaReferenceTestSetup {
    pub workspace_manager: WorkspaceManager,
    pub local_repo: LocalGitRepository,
    pub database_path: String,
}

impl JavaReferenceTestSetup {
    pub fn cleanup(&self) {
        self.workspace_manager.clean().unwrap();
    }
}

pub async fn setup_java_reference_pipeline(database: &Arc<KuzuDatabase>) -> JavaReferenceTestSetup {
    let local_repo = init_java_references_repository();
    let repo_path_str = local_repo.path.to_str().unwrap();
    let workspace_path = local_repo.workspace_path.to_str().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let workspace_manager =
        WorkspaceManager::new_with_directory(temp_dir.path().to_path_buf()).unwrap();
    let workspace_folder = workspace_manager
        .register_workspace_folder(local_repo.workspace_path.as_path())
        .unwrap();
    let workspace_project = workspace_manager
        .register_project(
            &workspace_folder.workspace_folder_path,
            local_repo.path.to_str().unwrap(),
        )
        .unwrap();

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

    indexer
        .process_files_full_with_database(
            database,
            file_source.clone(),
            &config,
            output_path,
            workspace_project.database_path.to_str().unwrap(),
        )
        .await
        .expect("Failed to process repository");

    // Ensure DB can be opened
    database
        .get_or_create_database(workspace_project.database_path.to_str().unwrap(), None)
        .expect("Database should be accessible after setup completion");

    JavaReferenceTestSetup {
        workspace_manager,
        local_repo,
        database_path: workspace_project
            .database_path
            .to_str()
            .unwrap()
            .to_string(),
    }
}

#[cfg(test)]
mod integration_tests {
    use std::sync::Arc;

    use crate::analysis::languages::java::tests::setup_java_reference_pipeline;
    use database::kuzu::database::KuzuDatabase;
    use database::kuzu::service::NodeDatabaseService;

    use tracing_test::traced_test;

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

    #[traced_test]
    #[tokio::test]
    // Regression test for resolving a class in a package that contains two classes with the same name.
    async fn test_java_reference_resolution_same_class_name_in_same_package() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // ServerFilter.Filter -> ServerFilter
        let callers_to_filter = node_database_service
            .find_calls_to_method("com.example.filter.Filter.apply")
            .unwrap_or_default();
        assert!(
            callers_to_filter
                .iter()
                .any(|c| c.ends_with("com.example.filter.ServerFilter.Filter.apply")),
            "ServerFilter.Filter should call ServerFilter.apply"
        );
    }

    #[traced_test]
    #[tokio::test]
    async fn test_java_call_relationship_has_location() {
        use database::kuzu::connection::KuzuConnection;

        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let conn = KuzuConnection::new(&database_instance).expect("conn");

        // Assert exact expected lines for specific calls in fixtures
        let mapping = database::graph::RelationshipTypeMapping::new();
        let calls_id = mapping.get_type_id(database::graph::RelationshipType::Calls);

        // 1) com.example.app.Main.main -> await(() -> super.run()) on line 22 (0-based 21)
        let query = format!(
            "MATCH (source:DefinitionNode)-[r:DEFINITION_RELATIONSHIPS]->(target:DefinitionNode) \
         WHERE source.fqn = 'com.example.app.Main.main' AND target.fqn = 'com.example.app.Application.run' AND r.type = {} \
         RETURN r.source_start_line, r.source_end_line",
            calls_id
        );
        let result = conn.query(&query).expect("query ok");
        let rows: Vec<_> = result.into_iter().collect();
        assert!(!rows.is_empty(), "Expected Application.run call row");
        let row = &rows[0];
        let start_line = row
            .first()
            .and_then(|v| match v {
                kuzu::Value::Int32(x) => Some(*x),
                _ => None,
            })
            .expect("start_line");
        let end_line = row
            .get(1)
            .and_then(|v| match v {
                kuzu::Value::Int32(x) => Some(*x),
                _ => None,
            })
            .expect("end_line");
        assert_eq!(start_line, 21);
        assert_eq!(end_line, 21);

        // 2) com.example.app.Main.main -> Outer.make() on line 25 (0-based 24)
        let query = format!(
            "MATCH (source:DefinitionNode)-[r:DEFINITION_RELATIONSHIPS]->(target:DefinitionNode) \
         WHERE source.fqn = 'com.example.app.Main.main' AND target.fqn = 'com.example.util.Outer.make' AND r.type = {} \
         RETURN r.source_start_line, r.source_end_line",
            calls_id
        );
        let result = conn.query(&query).expect("query ok");
        let rows: Vec<_> = result.into_iter().collect();
        assert!(!rows.is_empty(), "Expected Outer.make call row");
        let row = &rows[0];
        let start_line = row
            .first()
            .and_then(|v| match v {
                kuzu::Value::Int32(x) => Some(*x),
                _ => None,
            })
            .expect("start_line");
        let end_line = row
            .get(1)
            .and_then(|v| match v {
                kuzu::Value::Int32(x) => Some(*x),
                _ => None,
            })
            .expect("end_line");
        assert_eq!(start_line, 24);
        assert_eq!(end_line, 24);

        // 3) com.example.app.Main.main -> new ArrayList<String>() (imported symbol java.util.ArrayList) on line 42 (0-based 41)
        let query = format!(
        "MATCH (source:DefinitionNode)-[r:DEFINITION_RELATIONSHIPS]->(target:ImportedSymbolNode) \
         WHERE 
            source.fqn = 'com.example.app.Main.main' 
            AND target.import_path = 'java.util' 
            AND target.name = 'ArrayList' 
            AND r.type = {} \
         RETURN r.source_start_line, r.source_end_line",
        calls_id
    );
        let result = conn.query(&query).expect("query ok");
        let rows: Vec<_> = result.into_iter().collect();
        assert!(!rows.is_empty(), "Expected ArrayList call row");
        let row = &rows[0];
        let start_line = row
            .first()
            .and_then(|v| match v {
                kuzu::Value::Int32(x) => Some(*x),
                _ => None,
            })
            .expect("start_line");
        let end_line = row
            .get(1)
            .and_then(|v| match v {
                kuzu::Value::Int32(x) => Some(*x),
                _ => None,
            })
            .expect("end_line");
        assert_eq!(start_line, 41);
        assert_eq!(end_line, 41);
    }

    #[traced_test]
    #[tokio::test]
    async fn test_java_reference_to_deep_nested_class() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // InnerInnerHelpers.innerDoHelp -> InnerHelpers.innerDoHelp
        let callers_to_inner_do_help = node_database_service
            .find_calls_to_method("com.example.helpers.Helpers.InnerHelpers.innerDoHelp")
            .unwrap_or_default();

        assert!(
            callers_to_inner_do_help
                .iter()
                .any(|c| c
                    .ends_with("com.example.helpers.Helpers.InnerInnerHelpers.innerInnerDoHelp")),
            "InnerInnerHelpers.innerDoHelp should call InnerHelpers.innerDoHelp"
        );
    }
}
