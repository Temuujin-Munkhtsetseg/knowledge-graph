// ██╗  ██╗███╗   ██╗ ██████╗ ██╗    ██╗██╗     ███████╗██████╗  ██████╗ ███████╗
// ██║ ██╔╝████╗  ██║██╔═══██╗██║    ██║██║     ██╔════╝██╔══██╗██╔════╝ ██╔════╝
// █████╔╝ ██╔██╗ ██║██║   ██║██║ █╗ ██║██║     █████╗  ██║  ██║██║  ███╗█████╗
// ██╔═██╗ ██║╚██╗██║██║   ██║██║███╗██║██║     ██╔══╝  ██║  ██║██║   ██║██╔══╝
// ██║  ██╗██║ ╚████║╚██████╔╝╚███╔███╔╝███████╗███████╗██████╔╝╚██████╔╝███████╗
// ╚═╝  ╚═╝╚═╝  ╚═══╝ ╚═════╝  ╚══╝╚══╝ ╚══════╝╚══════╝╚═════╝  ╚═════╝ ╚══════╝
//
//  ██████╗ ██████╗  █████╗ ██████╗ ██╗  ██╗
// ██╔════╝ ██╔══██╗██╔══██╗██╔══██╗██║  ██║
// ██║  ███╗██████╔╝███████║██████╔╝███████║
// ██║   ██║██╔══██╗██╔══██║██╔═══╝ ██╔══██║
// ╚██████╔╝██║  ██║██║  ██║██║     ██║  ██║
//  ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝     ╚═╝  ╚═╝
use database::kuzu::database::KuzuDatabase;
use database::schema::manager::SchemaManager;
use futures::stream::{self, StreamExt};
use gitalisk_core::repository::gitalisk_repository::FileInfo;
use log::{info, warn};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

// Simplified imports - file processing is now handled by the File module
use crate::analysis::{AnalysisService, types::GraphData};
use crate::mutation::changes::KuzuChanges;
use database::kuzu::config::DatabaseConfig;

use crate::parsing::processor::FileProcessor;
use crate::project::source::FileSource;
use crate::writer::{WriterResult, WriterService};

use crate::mutation::utils::NodeIdGenerator;
pub use crate::parsing::changes::{FileChanges, FileChangesPathType};
pub use crate::parsing::processor::{
    ErroredFile, FileProcessingResult, ProcessingStage, ProcessingStats, SkippedFile,
};
use crate::project::io::{ProcessingError, read_text_file};
use crate::project::source::ChangesFileSource;

type ParseFilesResult = (
    Vec<FileProcessingResult>,
    Vec<SkippedFile>,
    Vec<ErroredFile>,
    Vec<(String, String)>,
);

// Removed legacy worker task struct in favor of pipelined processing

#[derive(Debug)]
enum IndexingProcessingResult {
    Success(FileProcessingResult),
    Skipped(SkippedFile),
    Error(ErroredFile),
}

#[derive(Debug, Clone, Copy)]
pub struct IndexingConfig {
    pub worker_threads: usize,
    pub max_file_size: usize,
    pub respect_gitignore: bool,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            worker_threads: 0,
            max_file_size: 5_000_000,
            respect_gitignore: true,
        }
    }
}

pub struct RepositoryIndexingResult {
    pub total_processing_time: Duration,
    pub repository_name: String,
    pub repository_path: String,
    pub file_results: Vec<FileProcessingResult>,
    pub skipped_files: Vec<SkippedFile>,
    pub errored_files: Vec<ErroredFile>,
    pub errors: Vec<(String, String)>, // Kept for backward compatibility
    pub graph_data: Option<GraphData>,
    pub writer_result: Option<WriterResult>,
    pub database_path: Option<String>,
    pub database_loaded: bool,
}

pub struct RepositoryReindexingResult {
    pub total_processing_time: Duration,
    pub repository_name: String,
    pub repository_path: String,
    pub file_results: Vec<FileProcessingResult>,
    pub skipped_files: Vec<SkippedFile>,
    pub errored_files: Vec<ErroredFile>,
    pub errors: Vec<(String, String)>, // Kept for backward compatibility
    pub graph_data: Option<GraphData>,
    pub writer_result: Option<WriterResult>,
    pub database_path: Option<String>,
    pub database_loaded: bool,
}

#[derive(Debug)]
pub enum AnalyzeAndWriteErrors {
    FailedToLoadDatabase(String),
    FailedToAnalyze(String),
    FailedToWrite(String),
}

#[derive(Debug)]
pub enum FatalIndexingError {
    FailedToGetFiles(String),
    FailedToProcessFiles(String),
    FailedToAnalyze(AnalyzeAndWriteErrors),
    FailedToWrite(AnalyzeAndWriteErrors),
    FailedToLoadDatabase(AnalyzeAndWriteErrors),
    FailedToSyncChanges(String),
}

impl std::fmt::Display for FatalIndexingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FatalIndexingError::FailedToGetFiles(msg) => write!(f, "Failed to get files: {msg}"),
            FatalIndexingError::FailedToProcessFiles(msg) => {
                write!(f, "Failed to process files: {msg}")
            }
            FatalIndexingError::FailedToAnalyze(err) => write!(f, "Failed to analyze: {err:?}"),
            FatalIndexingError::FailedToWrite(err) => write!(f, "Failed to write: {err:?}"),
            FatalIndexingError::FailedToLoadDatabase(err) => {
                write!(f, "Failed to load database: {err:?}")
            }
            FatalIndexingError::FailedToSyncChanges(msg) => {
                write!(f, "Failed to sync changes: {msg}")
            }
        }
    }
}

pub struct RepositoryIndexer {
    pub name: String,
    pub path: String,
}

impl RepositoryIndexer {
    pub fn new(name: String, path: String) -> Self {
        Self { name, path }
    }

    pub fn with_name(name: String, path: String) -> Self {
        Self { name, path }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    /// FIXME: SEPARATE THIS INTO A SEPARATE MODULE/EXECUTOR
    pub async fn index_files<F: FileSource>(
        &self,
        database: &KuzuDatabase,
        output_directory: &str,
        database_path: &str,
        file_source: F,
        config: &IndexingConfig,
    ) -> Result<RepositoryIndexingResult, FatalIndexingError> {
        let start_time = Instant::now();
        info!("Starting repository indexing for: {}", self.name);

        let files = file_source
            .get_files(config)
            .map_err(|e| FatalIndexingError::FailedToGetFiles(e.to_string()))?;

        let total_files = files.len();

        let (file_results, skipped_files, errored_files, errors) =
            self.parse_files(files, config).await?;

        let (graph_data, writer_result) = self.analyze_and_write_graph_data(
            database,
            &file_results,
            output_directory,
            database_path,
        )?;

        let file_results_len = file_results.len();
        let skipped_files_len = skipped_files.len();
        let errored_files_len = errored_files.len();

        let mut indexing_result = RepositoryIndexingResult {
            total_processing_time: start_time.elapsed(),
            repository_name: self.name.clone(),
            repository_path: self.path.clone(),
            file_results,
            skipped_files,
            errored_files,
            errors,
            graph_data: None,
            writer_result: None,
            database_path: None,
            database_loaded: false,
        };

        indexing_result.graph_data = Some(graph_data);
        indexing_result.writer_result = Some(writer_result);

        info!(
            "✅ Repository indexing completed for '{}' in {:?}",
            self.name, indexing_result.total_processing_time
        );
        info!(
            "📊 Final results: {:.1}% complete - {} processed, {} skipped, {} errors",
            (file_results_len as f64 / total_files as f64) * 100.0,
            file_results_len,
            skipped_files_len,
            errored_files_len
        );

        Ok(indexing_result)
    }

    pub async fn parse_files(
        &self,
        files: Vec<FileInfo>,
        config: &IndexingConfig,
    ) -> Result<ParseFilesResult, FatalIndexingError> {
        if files.is_empty() {
            return Ok((Vec::new(), Vec::new(), Vec::new(), Vec::new()));
        }

        let total_files = files.len();
        info!("Processing {total_files} files");

        // Calculate optimal worker count
        let num_cores = num_cpus::get();
        let worker_count = if config.worker_threads == 0 {
            std::cmp::max(num_cores, 4)
        } else {
            config.worker_threads
        };

        info!("Using {worker_count} CPU workers (spawn_blocking)");
        // FIXME: make this configurable in the future
        let io_concurrency = std::cmp::max(worker_count * 2, 8);
        let cpu_sem = Arc::new(Semaphore::new(worker_count));

        // Collect results
        let mut file_results = Vec::with_capacity(total_files);
        let mut skipped_files = Vec::new();
        let mut errored_files = Vec::new();
        let mut errors = Vec::new();

        let repo_path = self.path.clone();
        let max_file_size = config.max_file_size;
        let start_time = Instant::now();
        let mut last_progress = 0usize;

        // Stage 1: async read -> Stage 2: CPU parse (bounded)
        // TODO: investigate splitting the pipeline into multiple modules
        // and turning the entire execution pipeline into a streaming model.
        let pipeline = stream::iter(files.into_iter().map(|file_info| {
            let file_path_str = file_info.path.to_string_lossy().to_string();
            let full_path = if file_path_str.starts_with(&repo_path) {
                file_info.path.to_path_buf()
            } else {
                Path::new(&repo_path).join(&file_info.path)
            };
            (file_info, full_path)
        }))
        .map(move |(file_info, full_path)| async move {
            let content_res = read_text_file(&full_path, max_file_size).await;
            (file_info, content_res)
        })
        .buffer_unordered(io_concurrency)
        .map(|(file_info, content_res)| {
            let cpu_sem = Arc::clone(&cpu_sem);
            async move {
                match content_res {
                    Ok(content) => {
                        // Acquire CPU permit then parse in blocking pool
                        let _permit = cpu_sem.acquire_owned().await.expect("semaphore closed");
                        let file_path_for_error = file_info.path.to_string_lossy().to_string();
                        let fi_for_parse = file_info;

                        let parse_res = tokio_rayon::spawn(move || {
                            let processor = FileProcessor::from_file_info(fi_for_parse, &content);
                            processor.process()
                        })
                        .await;

                        match parse_res {
                            crate::parsing::processor::ProcessingResult::Success(file_result) => {
                                IndexingProcessingResult::Success(file_result)
                            }
                            crate::parsing::processor::ProcessingResult::Skipped(skipped) => {
                                IndexingProcessingResult::Skipped(skipped)
                            }
                            crate::parsing::processor::ProcessingResult::Error(errored) => {
                                IndexingProcessingResult::Error(ErroredFile {
                                    file_path: file_path_for_error,
                                    error_message: format!(
                                        "Task execution failed: {:?}",
                                        errored.error_message
                                    ),
                                    error_stage: ProcessingStage::Unknown,
                                })
                            }
                        }
                    }
                    Err(processing_error) => match processing_error {
                        ProcessingError::Skipped(file_path, reason) => {
                            IndexingProcessingResult::Skipped(SkippedFile {
                                file_path,
                                reason,
                                file_size: None,
                            })
                        }
                        ProcessingError::Error(file_path, error_msg) => {
                            IndexingProcessingResult::Error(ErroredFile {
                                file_path,
                                error_message: error_msg,
                                error_stage: ProcessingStage::FileSystem,
                            })
                        }
                    },
                }
            }
        })
        .buffer_unordered(worker_count);

        tokio::pin!(pipeline);
        while let Some(result) = pipeline.next().await {
            match result {
                IndexingProcessingResult::Success(file_result) => {
                    file_results.push(file_result);
                }
                IndexingProcessingResult::Skipped(skipped) => {
                    skipped_files.push(skipped);
                }
                IndexingProcessingResult::Error(errored) => {
                    errors.push((errored.file_path.clone(), errored.error_message.clone()));
                    errored_files.push(errored);
                }
            }

            let completed = file_results.len() + skipped_files.len() + errored_files.len();
            let progress = (completed * 100) / total_files;
            if progress >= last_progress + 10 && progress <= 100 {
                let elapsed = start_time.elapsed();
                let files_per_sec = completed as f64 / elapsed.as_secs_f64();
                info!(
                    "🔄 Progress: {}% ({}/{} files) - {:.1} files/sec - {} processed, {} skipped, {} errors",
                    progress,
                    completed,
                    total_files,
                    files_per_sec,
                    file_results.len(),
                    skipped_files.len(),
                    errored_files.len()
                );
                last_progress = progress;
            }
        }

        let final_completed = file_results.len() + skipped_files.len() + errored_files.len();
        info!(
            "✅ Pipelined processing completed: {} processed, {} skipped, {} errors ({} total)",
            file_results.len(),
            skipped_files.len(),
            errored_files.len(),
            final_completed
        );

        Ok((file_results, skipped_files, errored_files, errors))
    }

    fn get_files<F: FileSource>(
        &self,
        file_source: F,
        config: &IndexingConfig,
    ) -> Result<Vec<FileInfo>, FatalIndexingError> {
        file_source
            .get_files(config)
            .map_err(|e| FatalIndexingError::FailedToGetFiles(e.to_string()))
    }

    /// Analyze processed files, write graph data to Parquet files, and load into Kuzu database
    /// FIXME: SEPARATE THIS INTO A SEPARATE MODULE/EXECUTOR
    pub fn analyze_and_write_graph_data(
        &self,
        database: &KuzuDatabase,
        file_results: &Vec<FileProcessingResult>,
        output_directory: &str,
        database_path: &str,
    ) -> Result<(GraphData, WriterResult), FatalIndexingError> {
        info!(
            "Starting analysis and writing phase for repository: {}",
            self.name
        );
        let start_time = Instant::now();

        let mut analysis_service = AnalysisService::new(self.name.clone(), self.path.clone());

        let graph_data = analysis_service
            .analyze_results(file_results)
            .map_err(|e| {
                FatalIndexingError::FailedToAnalyze(AnalyzeAndWriteErrors::FailedToAnalyze(
                    e.to_string(),
                ))
            })?;

        info!(
            "Analysis completed: {} files, {} definitions, {} imported symbols, {} relationships",
            graph_data.file_nodes.len(),
            graph_data.definition_nodes.len(),
            graph_data.imported_symbol_nodes.len(),
            graph_data.directory_relationships.len()
                + graph_data.file_definition_relationships.len()
                + graph_data.file_imported_symbol_relationships.len()
                + graph_data.definition_relationships.len()
                + graph_data.definition_imported_symbol_relationships.len()
                + graph_data
                    .imported_symbol_imported_symbol_relationships
                    .len()
                + graph_data.imported_symbol_definition_relationships.len()
                + graph_data.imported_symbol_file_relationships.len()
        );

        let writer_service = WriterService::new(output_directory).map_err(|e| {
            FatalIndexingError::FailedToWrite(AnalyzeAndWriteErrors::FailedToWrite(e.to_string()))
        })?;

        let mut node_id_generator = NodeIdGenerator::new();

        let writer_result = writer_service
            .write_graph_data(&graph_data, &mut node_id_generator)
            .map_err(|e| {
                FatalIndexingError::FailedToWrite(AnalyzeAndWriteErrors::FailedToWrite(
                    e.to_string(),
                ))
            })?;

        let analysis_duration = start_time.elapsed();
        info!(
            "✅ Analysis and writing completed in {:?}. Parquet files created: {}",
            analysis_duration,
            writer_result.files_written.len()
        );

        info!("Loading graph data into Kuzu database at: {database_path}");
        self.load_into_database(database, output_directory, database_path)
            .map_err(|e| {
                FatalIndexingError::FailedToLoadDatabase(
                    AnalyzeAndWriteErrors::FailedToLoadDatabase(e.to_string()),
                )
            })?;

        Ok((graph_data, writer_result))
    }

    pub async fn process_files_full_with_database<F: FileSource>(
        &self,
        database: &KuzuDatabase,
        file_source: F,
        config: &IndexingConfig,
        output_directory: &str,
        database_path: &str,
    ) -> Result<RepositoryIndexingResult, FatalIndexingError> {
        let indexing_result = self
            .index_files(
                database,
                output_directory,
                database_path,
                file_source,
                config,
            )
            .await?;

        Ok(indexing_result)
    }

    pub async fn reindex_repository(
        &mut self,
        database: &KuzuDatabase,
        file_changes: FileChanges,
        config: &IndexingConfig,
        database_path: &str,
        output_path: &str,
    ) -> Result<RepositoryReindexingResult, FatalIndexingError> {
        let start_time = Instant::now();

        if !file_changes.has_changes() {
            warn!("No files to process in repository: {}", self.name);
            return Ok(RepositoryReindexingResult {
                total_processing_time: start_time.elapsed(),
                repository_name: self.name.clone(),
                repository_path: self.path.clone(),
                file_results: Vec::new(),
                skipped_files: Vec::new(),
                errored_files: Vec::new(),
                errors: Vec::new(),
                graph_data: None,
                writer_result: None,
                database_path: Some(database_path.to_string()),
                database_loaded: false,
            });
        }

        let database_instance = database.get_or_create_database(database_path, None);
        if database_instance.is_none() {
            return Err(FatalIndexingError::FailedToLoadDatabase(
                AnalyzeAndWriteErrors::FailedToLoadDatabase(format!(
                    "Failed to create database: {database_path}."
                )),
            ));
        } else {
            info!("Found database_instance for reindexing: {database_instance:?}");
        }
        let database_instance = database_instance.unwrap();

        let file_source = ChangesFileSource::new(&file_changes, self.path.clone());
        let files = self.get_files(file_source, config)?;

        let (file_results, skipped_files, errored_files, errors) =
            self.parse_files(files, config).await?;

        let mut analysis_service = AnalysisService::new(self.name.clone(), self.path.clone());

        let graph_data = analysis_service
            .analyze_results(&file_results)
            .map_err(|e| {
                FatalIndexingError::FailedToAnalyze(AnalyzeAndWriteErrors::FailedToAnalyze(
                    e.to_string(),
                ))
            })?;

        // Sync diff changes to kuzu
        let mut kuzu_syncer = KuzuChanges::new(
            &database_instance,
            file_changes,
            graph_data.clone(),
            &self.path,
            output_path,
        );

        kuzu_syncer
            .sync_changes()
            .map(|writer_result| RepositoryReindexingResult {
                total_processing_time: start_time.elapsed(),
                repository_name: self.name.clone(),
                repository_path: self.path.clone(),
                file_results,
                skipped_files,
                errored_files,
                errors,
                graph_data: None,
                writer_result: Some(writer_result),
                database_path: Some(database_path.to_string()),
                database_loaded: true,
            })
            .map_err(|e| FatalIndexingError::FailedToSyncChanges(e.to_string()))
    }

    /// Load Parquet data into Kuzu database
    /// FIXME: SEPARATE THIS INTO A SEPARATE MODULE/EXECUTOR
    fn load_into_database(
        &self,
        database: &KuzuDatabase,
        parquet_directory: &str,
        database_path: &str,
    ) -> Result<(), String> {
        info!("Initializing Kuzu database and loading graph data...");

        let config = DatabaseConfig::new(database_path)
            .with_buffer_size(512 * 1024 * 1024)
            .with_compression(true);

        let database_instance = database
            .force_new_database(database_path, Some(config))
            .ok_or(format!("Failed to create database: {database_path}."))?;

        let database_instance = database_instance;

        let schema_manager = SchemaManager::new(&database_instance);
        schema_manager
            .initialize_schema()
            .map_err(|e| format!("Failed to initialize database schema: {e:?}"))?;

        schema_manager
            .import_graph_data(parquet_directory)
            .map_err(|e| format!("Failed to import graph data: {e:?}"))?;

        match schema_manager.get_schema_stats() {
            Ok(stats) => {
                info!("Database loading completed successfully:");
                info!("{stats}");
            }
            Err(e) => {
                warn!("Failed to get database statistics: {e:?}");
            }
        }

        Ok(())
    }
}
