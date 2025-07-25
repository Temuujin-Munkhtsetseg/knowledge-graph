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
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, bounded};
use database::kuzu::database::KuzuDatabase;
use database::kuzu::schema::SchemaManager;
use gitalisk_core::repository::gitalisk_repository::FileInfo;
use log::{debug, error, info, warn};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

// Simplified imports - file processing is now handled by the File module
use crate::analysis::{AnalysisService, types::GraphData};
use crate::database::changes::KuzuChanges;
use database::kuzu::config::DatabaseConfig;

use crate::parsing::processor::{FileProcessor, ProcessingResult};
use crate::project::source::FileSource;
use crate::writer::{WriterResult, WriterService};

use crate::database::utils::NodeIdGenerator;
pub use crate::parsing::changes::{FileChanges, FileChangesPathType};
pub use crate::parsing::processor::{
    ErroredFile, FileProcessingResult, ProcessingStage, ProcessingStats, SkippedFile,
};
use crate::project::source::ChangesFileSource;

const DEFAULT_WORKER_THREADS: usize = 8;
const WORK_QUEUE_CAPACITY: usize = 10000;
const RESULT_QUEUE_CAPACITY: usize = 10000;
const RESULT_TIMEOUT_SECS: u64 = 30;

type ParseFilesResult = (
    Vec<FileProcessingResult>,
    Vec<SkippedFile>,
    Vec<ErroredFile>,
    Vec<(String, String)>,
);

#[derive(Debug)]
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

/// Internal messages for worker communication
enum WorkMessage {
    ProcessFile(FileInfo),
}

enum ResultMessage {
    Success(FileProcessingResult),
    Skipped(String, String),
    Error(String, String),
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
    pub fn index_files<F: FileSource>(
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
            self.parse_files(files, config)?;

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

    /// FIXME: SEPARATE THIS INTO A SEPARATE MODULE/EXECUTOR
    pub fn parse_files(
        &self,
        files: Vec<FileInfo>,
        config: &IndexingConfig,
    ) -> Result<ParseFilesResult, FatalIndexingError> {
        if files.is_empty() {
            return Ok((Vec::new(), Vec::new(), Vec::new(), Vec::new()));
        }

        let num_cores: usize = num_cpus::get();
        let worker_count = if config.worker_threads == 0 {
            if files.len() < num_cores && !files.is_empty() {
                files.len()
            } else {
                std::cmp::max(num_cores, DEFAULT_WORKER_THREADS)
            }
        } else {
            config.worker_threads
        };

        let total_files = files.len();
        info!("Processing {total_files} files using {worker_count} worker threads");

        let (work_sender, work_receiver) =
            bounded::<WorkMessage>(WORK_QUEUE_CAPACITY.max(total_files));
        let (result_sender, result_receiver) = bounded::<ResultMessage>(RESULT_QUEUE_CAPACITY);

        let feeder_work_sender = work_sender;
        let feeder_handle = thread::spawn(move || {
            info!("Feeder thread started, distributing {} files", files.len());
            for file_info in files {
                if feeder_work_sender
                    .send(WorkMessage::ProcessFile(file_info))
                    .is_err()
                {
                    error!("Work channel closed unexpectedly");
                    break;
                }
            }
            drop(feeder_work_sender);
            info!("Feeder thread finished");
        });

        let mut worker_handles = Vec::with_capacity(worker_count);
        for worker_id in 0..worker_count {
            let worker_receiver = work_receiver.clone();
            let worker_sender = result_sender.clone();
            let repo_path = self.path.clone();
            let max_file_size = config.max_file_size;

            let handle = thread::spawn(move || {
                debug!("Worker {worker_id} started");
                Self::worker_task(
                    worker_id,
                    worker_receiver,
                    worker_sender,
                    repo_path,
                    max_file_size,
                );
                debug!("Worker {worker_id} finished");
            });
            worker_handles.push(handle);
        }

        drop(work_receiver);
        drop(result_sender);

        let mut file_results = Vec::new();
        let mut errors = Vec::new();
        let mut messages_received = 0;
        let mut last_reported_percentage: u8 = 0;

        let mut skipped_files = Vec::new();
        let mut errored_files = Vec::new();
        let mut processed_files = 0;

        // Use timeout-based result collection to prevent hanging
        loop {
            let timeout = Duration::from_secs(RESULT_TIMEOUT_SECS);
            match result_receiver.recv_timeout(timeout) {
                Ok(result_msg) => {
                    messages_received += 1;
                    match result_msg {
                        ResultMessage::Success(file_result) => {
                            processed_files += 1;

                            file_results.push(file_result);
                        }
                        ResultMessage::Skipped(file_path, reason) => {
                            skipped_files.push(SkippedFile {
                                file_path: file_path.clone(),
                                reason: reason.clone(),
                                file_size: None,
                            });
                            debug!("Skipped {file_path}: {reason}");
                        }
                        ResultMessage::Error(file_path, error_msg) => {
                            errored_files.push(ErroredFile {
                                file_path: file_path.clone(),
                                error_message: error_msg.clone(),
                                error_stage: ProcessingStage::Unknown,
                            });
                            errors.push((file_path.clone(), error_msg.clone()));
                            error!("Error processing {file_path}: {error_msg}");
                        }
                    }

                    // Log progress only on each 5% chunk
                    let progress_pct = (processed_files as f64 / total_files as f64) * 100.0;
                    let current_percentage_chunk = (progress_pct / 5.0) as u8 * 5;

                    if current_percentage_chunk > last_reported_percentage
                        && current_percentage_chunk <= 100
                    {
                        let completed = processed_files;
                        info!(
                            "🔄 Progress: {}% ({}/{} files) - {} processed, {} skipped, {} errors",
                            current_percentage_chunk,
                            completed,
                            total_files,
                            processed_files,
                            skipped_files.len(),
                            errored_files.len()
                        );
                        last_reported_percentage = current_percentage_chunk;
                    }

                    if messages_received >= total_files {
                        info!("All {total_files} files processed, finishing result collection");
                        break;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    warn!(
                        "Result collection timeout after {RESULT_TIMEOUT_SECS}s. Received {messages_received}/{total_files} messages. Checking if workers are still alive..."
                    );

                    let mut active_workers = 0;
                    for handle in &worker_handles {
                        if !handle.is_finished() {
                            active_workers += 1;
                        }
                    }

                    if active_workers == 0 {
                        warn!("No active workers remaining, breaking from result collection");
                        break;
                    } else {
                        info!("{active_workers} workers still active, continuing to wait...");
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    info!("Result channel closed, all workers finished");
                    break;
                }
            }
        }

        info!("Waiting for feeder thread to complete...");
        feeder_handle.join().map_err(|_| {
            FatalIndexingError::FailedToProcessFiles("Feeder thread panicked".to_string())
        })?;

        info!(
            "Waiting for {} worker threads to complete...",
            worker_handles.len()
        );
        for (i, handle) in worker_handles.into_iter().enumerate() {
            handle.join().map_err(|_| {
                FatalIndexingError::FailedToProcessFiles(format!("Worker thread {i} panicked"))
            })?;
        }

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

        let analysis_service = AnalysisService::new(self.name.clone(), self.path.clone());

        let graph_data = analysis_service
            .analyze_results(file_results)
            .map_err(|e| {
                FatalIndexingError::FailedToAnalyze(AnalyzeAndWriteErrors::FailedToAnalyze(
                    e.to_string(),
                ))
            })?;

        info!(
            "Analysis completed: {} files, {} definitions, {} relationships",
            graph_data.file_nodes.len(),
            graph_data.definition_nodes.len(),
            graph_data.file_definition_relationships.len()
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

    pub fn process_files_full_with_database<F: FileSource>(
        &self,
        database: &KuzuDatabase,
        file_source: F,
        config: &IndexingConfig,
        output_directory: &str,
        database_path: &str,
    ) -> Result<RepositoryIndexingResult, FatalIndexingError> {
        let indexing_result = self.index_files(
            database,
            output_directory,
            database_path,
            file_source,
            config,
        )?;

        Ok(indexing_result)
    }

    pub fn reindex_repository(
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
            self.parse_files(files, config)?;

        let analysis_service = AnalysisService::new(self.name.clone(), self.path.clone());

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

    fn worker_task(
        worker_id: usize,
        work_receiver: Receiver<WorkMessage>,
        result_sender: Sender<ResultMessage>,
        repo_path: String,
        max_file_size: usize,
    ) {
        while let Ok(work_msg) = work_receiver.recv() {
            match work_msg {
                WorkMessage::ProcessFile(file_info) => {
                    let result =
                        Self::process_single_file_info(file_info, &repo_path, max_file_size);

                    let result_msg = match result {
                        Ok(file_result) => ResultMessage::Success(file_result),
                        Err(ProcessingError::Skipped(file_path, reason)) => {
                            ResultMessage::Skipped(file_path, reason)
                        }
                        Err(ProcessingError::Error(file_path, error_msg)) => {
                            ResultMessage::Error(file_path, error_msg)
                        }
                    };

                    if result_sender.send(result_msg).is_err() {
                        error!("Worker {worker_id}: Result channel closed");
                        break;
                    }
                }
            }
        }
    }

    fn process_single_file_info(
        file_info: FileInfo,
        repo_path: &str,
        max_file_size: usize,
    ) -> Result<FileProcessingResult, ProcessingError> {
        let file_path = file_info.path.to_string_lossy().to_string();

        let full_path = if file_path.starts_with(repo_path) {
            file_path.clone()
        } else {
            Path::new(repo_path)
                .join(&file_path)
                .to_string_lossy()
                .to_string()
        };

        let metadata = std::fs::metadata(&full_path).map_err(|e| {
            ProcessingError::Error(file_path.clone(), format!("Failed to read metadata: {e}"))
        })?;

        // FIXME: MOVE THIS TO THE PROCESSOR
        if metadata.len() as usize > max_file_size {
            return Err(ProcessingError::Skipped(
                file_path.clone(),
                format!("File too large: {} bytes", metadata.len()),
            ));
        }

        let content = std::fs::read_to_string(&full_path).map_err(|e| {
            ProcessingError::Error(file_path.clone(), format!("Failed to read file: {e}"))
        })?;

        let file = FileProcessor::from_file_info(file_info, &content);

        match file.process() {
            ProcessingResult::Success(file_result) => Ok(file_result),
            ProcessingResult::Skipped(skipped) => {
                Err(ProcessingError::Skipped(skipped.file_path, skipped.reason))
            }
            ProcessingResult::Error(errored) => Err(ProcessingError::Error(
                errored.file_path,
                errored.error_message,
            )),
        }
    }
}

#[derive(Debug)]
enum ProcessingError {
    Skipped(String, String), // file_path, reason
    Error(String, String),   // file_path, error_message
}
