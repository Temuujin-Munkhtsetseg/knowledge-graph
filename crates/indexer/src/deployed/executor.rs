use crate::indexer::{IndexingConfig, RepositoryIndexer, RepositoryIndexingResult};
use crate::project::source::PathFileSource;
use anyhow::Result;
use database::kuzu::database::KuzuDatabase;
use std::path::PathBuf;

pub struct DeployedIndexingExecutor {
    repository_path: PathBuf,
    database_path: PathBuf,
    parquet_path: PathBuf,
    config: IndexingConfig,
}

/// This is the executor for the deployed version of the indexer,
/// which is a statically linked binary to the GitLab Mesh (formerly GitLab Zoekt Indexer)
/// Golang binary that is used to index repositories alongside GitLab Rails.
/// See the design document for more details:
/// https://handbook.gitlab.com/handbook/engineering/architecture/design-documents/knowledge_graph/
///
/// The Golang Binary uses the `indexer-c-bindings` crate to interface with the Rust code.
/// See the Golang module (UPDATE ME) for the related code.
impl DeployedIndexingExecutor {
    pub fn new(
        repository_path: PathBuf,
        database_path: PathBuf,
        parquet_path: PathBuf,
        config: IndexingConfig,
    ) -> Self {
        Self {
            repository_path,
            database_path,
            parquet_path,
            config,
        }
    }

    pub fn execute(&self) -> Result<RepositoryIndexingResult, String> {
        let repo_name = std::path::Path::new(&self.repository_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();

        let indexer = RepositoryIndexer::new(
            repo_name,
            self.repository_path
                .to_str()
                .expect("Expected string")
                .to_string(),
        );
        let file_source = PathFileSource::from_path(self.repository_path.clone());
        let database = KuzuDatabase::new();

        indexer.process_files_full_with_database(
            &database,
            file_source,
            &self.config,
            self.parquet_path.to_str().unwrap(),
            Some(self.database_path.to_str().unwrap()),
        )
    }
}
