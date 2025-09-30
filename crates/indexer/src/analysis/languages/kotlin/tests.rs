use gitalisk_core::repository::gitalisk_repository::CoreGitaliskRepository;
use gitalisk_core::repository::testing::local::LocalGitRepository;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use workspace_manager::WorkspaceManager;

use crate::indexer::{IndexingConfig, RepositoryIndexer};
use crate::project::source::GitaliskFileSource;
use database::kuzu::database::KuzuDatabase;

fn init_kotlin_references_repository() -> LocalGitRepository {
    let mut local_repo = LocalGitRepository::new(None);
    let fixtures_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/kotlin");
    local_repo.copy_dir(&fixtures_path);
    local_repo
        .add_all()
        .commit("Initial commit with Kotlin reference examples");
    local_repo
}

pub struct KotlinReferenceTestSetup {
    pub workspace_manager: WorkspaceManager,
    pub local_repo: LocalGitRepository,
    pub database_path: String,
}

impl KotlinReferenceTestSetup {
    pub fn cleanup(&self) {
        self.workspace_manager.clean().unwrap();
    }
}

pub async fn setup_kotlin_reference_pipeline(
    database: &Arc<KuzuDatabase>,
) -> KotlinReferenceTestSetup {
    let local_repo = init_kotlin_references_repository();
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
        "kotlin-references-test".to_string(),
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

    KotlinReferenceTestSetup {
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

    use crate::analysis::languages::kotlin::tests::setup_kotlin_reference_pipeline;
    use database::kuzu::database::KuzuDatabase;
    use database::kuzu::service::NodeDatabaseService;

    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn test_kotlin_reference_resolution_main_function_calls() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_kotlin_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // Main.main -> Foo() constructor
        let callers_to_foo_constructor = node_database_service
            .find_calls_to_method("com.example.foo.Foo")
            .unwrap_or_default();
        assert!(
            callers_to_foo_constructor
                .iter()
                .any(|c| c.ends_with("com.example.main")),
            "Main function should call Foo constructor"
        );

        // Main.main -> foo.foo() instance method
        let callers_to_foo_method = node_database_service
            .find_calls_to_method("com.example.foo.Foo.foo")
            .unwrap_or_default();
        assert!(
            callers_to_foo_method
                .iter()
                .any(|c| c.ends_with("com.example.main")),
            "Main function should call foo.foo() instance method"
        );

        // Main.main -> foo.companionFoo() companion method
        let callers_to_companion_foo = node_database_service
            .find_calls_to_method("com.example.foo.Foo.Companion.companionFoo")
            .unwrap_or_default();
        assert!(
            callers_to_companion_foo
                .iter()
                .any(|c| c.ends_with("com.example.main")),
            "Main function should call foo.companionFoo() companion method"
        );

        // Main.main -> foo.baz() interface method through inheritance
        let callers_to_baz_method = node_database_service
            .find_calls_to_method("com.example.foo.Baz.baz")
            .unwrap_or_default();
        assert!(
            callers_to_baz_method
                .iter()
                .any(|c| c.ends_with("com.example.main")),
            "Main function should call foo.baz() interface method through inheritance"
        );

        setup.cleanup();
    }

    #[traced_test]
    #[tokio::test]
    async fn test_kotlin_inheritance_and_super_calls() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_kotlin_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // Foo.foo -> super.bar() call to parent class method
        let callers_to_bar_method = node_database_service
            .find_calls_to_method("com.example.foo.Bar.bar")
            .unwrap_or_default();
        assert!(
            callers_to_bar_method
                .iter()
                .any(|c| c.ends_with("com.example.foo.Foo.foo")),
            "Foo.foo should call super.bar() method from parent class"
        );

        setup.cleanup();
    }

    #[traced_test]
    #[tokio::test]
    async fn test_kotlin_inner_class_calls() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_kotlin_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // Foo.foo & Foo.InnerFoo.innerFoo -> fooInFooBody() call to inner class method
        let callers_to_inner_foo = node_database_service
            .find_calls_to_method("com.example.foo.Foo.fooInFooBody")
            .unwrap_or_default();

        assert!(
            callers_to_inner_foo
                .iter()
                .any(|c| c.ends_with("com.example.foo.Foo.foo")),
            "Foo.foo should call fooInFooBody() method"
        );

        assert!(
            callers_to_inner_foo
                .iter()
                .any(|c| c.ends_with("com.example.foo.Foo.InnerFoo.innerFoo")),
            "Foo.InnerFoo.innerFoo should call fooInFooBody() method"
        );

        setup.cleanup();
    }

    #[traced_test]
    #[tokio::test]
    async fn test_kotlin_type_inference_from_when_expression() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_kotlin_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // When.whenTypeInference -> Person.getName()
        let callers_to_get_name = node_database_service
            .find_calls_to_method("com.example.entites.Person.getName")
            .unwrap_or_default();

        assert!(
            callers_to_get_name
                .iter()
                .any(|c| c.ends_with("com.example.when.whenTypeInference")),
            "When.whenTypeInference should call Person.getName()"
        );

        setup.cleanup();
    }

    #[traced_test]
    #[tokio::test]
    async fn test_kotlin_type_inference_from_if_expression() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_kotlin_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // If.ifTypeInference -> Person.getName()
        let callers_to_get_name = node_database_service
            .find_calls_to_method("com.example.entites.Person.getName")
            .unwrap_or_default();

        assert!(
            callers_to_get_name
                .iter()
                .any(|c| c.ends_with("com.example.if.usageOfIfTypeInference")),
            "If.ifTypeInference should call Person.getName()"
        );

        setup.cleanup();
    }

    #[traced_test]
    #[tokio::test]
    async fn test_kotlin_type_inference_from_try_catch() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_kotlin_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // Try.tryTypeInference -> Person.getName()
        let callers_to_get_name = node_database_service
            .find_calls_to_method("com.example.entites.Person.getName")
            .unwrap_or_default();

        assert!(
            callers_to_get_name
                .iter()
                .any(|c| c.ends_with("com.example.try.tryTypeInference")),
            "Try.tryTypeInference should call Person.getName()"
        );

        setup.cleanup();
    }

    #[traced_test]
    #[tokio::test]
    async fn test_kotlin_reference_resolution_logger_calls() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_kotlin_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // Main.main -> logger.info("Hello, World!")
        let callers_to_logger_info = node_database_service
            .find_calls_to_imported_symbol("org.slf4j", "Logger")
            .unwrap_or_default();

        assert!(
            callers_to_logger_info
                .iter()
                .any(|c| c.ends_with("com.example.main")),
            "Main.main should call logger.info(\"Hello, World!\")"
        );

        setup.cleanup();
    }

    #[traced_test]
    #[tokio::test]
    async fn test_kotlin_reference_resolution_to_nested_classes() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_kotlin_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // Parent.Child.GrandChild.greet()
        let callers_to_greet = node_database_service
            .find_calls_to_method("com.example.nestedclasses.Parent.Child.GrandChild.greet")
            .unwrap_or_default();

        assert!(
            callers_to_greet
                .iter()
                .any(|c| c.ends_with("com.example.nestedclasses.Parent.GrandChild.greet")),
            "Parent.GrandChild.greet should call Parent.Child.GrandChild.greet"
        );

        assert!(
            callers_to_greet
                .iter()
                .any(|c| c.ends_with("com.example.main")),
            "main should call Parent.Child.GrandChild.greet()"
        );

        // Parent.GrandChild.greet()
        let callers_to_greet_2 = node_database_service
            .find_calls_to_method("com.example.nestedclasses.Parent.GrandChild.greet")
            .unwrap_or_default();

        assert!(
            callers_to_greet_2
                .iter()
                .any(|c| c.ends_with("com.example.main")),
            "main should call Parent.Child.GrandChild.greet"
        );

        setup.cleanup();
    }

    #[traced_test]
    #[tokio::test]
    async fn test_kotlin_reference_resolution_inheritance_of_classs_of_same_name() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_kotlin_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // ServerFilter.Filter -> Filter
        let callers_to_filter_filter = node_database_service
            .find_calls_to_method("com.example.edgecases.filter.Filter.filter")
            .unwrap_or_default();

        assert!(
            callers_to_filter_filter
                .iter()
                .any(|c| c.ends_with("com.example.edgecases.filter.ServerFilter.filter")),
            "ServerFilter.Filter.filter should call Filter.filter"
        );

        setup.cleanup();
    }

    #[traced_test]
    #[tokio::test]
    async fn test_kotlin_reference_to_operator_functions() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_kotlin_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // AnimalList.Companion.of -> AnimalList.plus
        let callers_to_plus = node_database_service
            .find_calls_to_method("com.example.operator.AnimalList.plus")
            .unwrap_or_default();

        assert!(
            callers_to_plus
                .iter()
                .any(|c| c.ends_with("com.example.operator.AnimalList.Companion.of")),
            "AnimalList.of should call AnimalList.plus"
        );

        // AnimalList.Companion.of -> AnimalList.display
        let callers_to_display = node_database_service
            .find_calls_to_method("com.example.operator.AnimalList.display")
            .unwrap_or_default();

        assert!(
            callers_to_display
                .iter()
                .any(|c| c.ends_with("com.example.operator.AnimalList.Companion.of")),
            "AnimalList.of should call AnimalList.display"
        );

        setup.cleanup();
    }

    #[traced_test]
    #[tokio::test]
    async fn test_kotlin_reference_to_enum_constants() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_kotlin_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // Enum.ENUM_VALUE_1.enumMethod()
        let callers_to_enum_value_1_enum_method = node_database_service
            .find_calls_to_method("com.example.enums.Enum.enumMethod")
            .unwrap_or_default();

        assert!(
            callers_to_enum_value_1_enum_method
                .iter()
                .any(|c| c.ends_with("com.example.main")),
            "Main.main should call Enum.ENUM_VALUE_1.enumMethod()"
        );

        let callers_to_enum_value_2_enum_method_2 = node_database_service
            .find_calls_to_method("com.example.enums.Enum.enumMethod2")
            .unwrap_or_default();

        assert!(
            callers_to_enum_value_2_enum_method_2
                .iter()
                .any(|c| c.ends_with("com.example.main")),
            "Main.main should call Enum.ENUM_VALUE_2.enumMethod2()"
        );

        setup.cleanup();
    }

    #[traced_test]
    #[tokio::test]
    async fn test_kotlin_reference_to_extensions() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_kotlin_reference_pipeline(&database).await;

        let database_instance = database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");
        let node_database_service = NodeDatabaseService::new(&database_instance);

        // Functions

        // ExtendMe.printValue()
        let callers_to_print_value = node_database_service
            .find_calls_to_method("com.example.extensions.printValue")
            .unwrap_or_default();

        assert!(
            callers_to_print_value
                .iter()
                .any(|c| c.ends_with("com.example.extensions.callToExtensions")),
            "callToExtensions should call ExtendMe.printValue()"
        );

        // ExtendMe.reversed()
        let callers_to_reversed = node_database_service
            .find_calls_to_method("com.example.extensions.utils.reverse")
            .unwrap_or_default();

        assert!(
            callers_to_reversed
                .iter()
                .any(|c| c.ends_with("com.example.extensions.callToImportedExtensions")),
            "callToImportedExtensions should call ExtendMe.reversed()"
        );

        // Reference to method through extension properties

        // ExtendMeFromProperty.printValue()
        let callers_to_print_value_2 = node_database_service
            .find_calls_to_method("com.example.extensions.entities.ExtendMeFromProperty.printValue")
            .unwrap_or_default();

        assert!(
            callers_to_print_value_2
                .iter()
                .any(|c| c.ends_with("com.example.extensions.callToExtensions")),
            "callToExtensions should call ExtendMeFromProperty.printValue()"
        );

        // ExtendMe.printValue()
        assert!(
            callers_to_print_value
                .iter()
                .any(|c| c.ends_with("com.example.extensions.callToImportedExtensions")),
            "callToExtensions should call ExtendMe.printValue()"
        );

        // ExternalType.print()
        let callers_to_print = node_database_service
            .find_calls_to_method("com.example.extensions.imported.print")
            .unwrap_or_default();

        assert!(
            callers_to_print
                .iter()
                .any(|c| c.ends_with("com.example.extensions.imported.callToImported")),
            "callToImportedExtensions should call ExternalType.print()"
        );

        setup.cleanup();
    }
}
