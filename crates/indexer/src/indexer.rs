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
use log::{debug, error, info, warn};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::analysis::{AnalysisService, GraphData};
use crate::database::{DatabaseConfig, KuzuConnection, SchemaManager};
use crate::parsing::processor::process_file_info;
use crate::project::file_info::FileInfo;
use crate::project::source::FileSource;
use crate::stats::IndexingStats;
use crate::writer::{WriterResult, WriterService};

use crate::database::utils::NodeIdGenerator;
pub use crate::parsing::processor::{FileProcessingResult, ProcessingStats};

const DEFAULT_WORKER_THREADS: usize = 8;
const WORK_QUEUE_CAPACITY: usize = 10000;
const RESULT_QUEUE_CAPACITY: usize = 10000;
const PROGRESS_UPDATE_INTERVAL: usize = 50;
const RESULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone)]
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
    pub repository_name: String,
    pub repository_path: String,
    pub file_results: Vec<FileProcessingResult>,
    pub total_files_processed: usize,
    pub total_files_skipped: usize,
    pub total_files_errored: usize,
    pub total_processing_time: Duration,
    pub errors: Vec<(String, String)>,
    pub graph_data: Option<GraphData>,
    pub writer_result: Option<WriterResult>,
    pub database_path: Option<String>,
    pub database_loaded: bool,
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

    pub fn index_files<F: FileSource>(
        &self,
        file_source: F,
        config: &IndexingConfig,
    ) -> Result<RepositoryIndexingResult, String> {
        let start_time = Instant::now();
        info!("Starting repository indexing for: {}", self.name);

        let files = file_source
            .get_files(config)
            .map_err(|e| format!("Failed to get files: {e}"))?;

        if files.is_empty() {
            warn!("No files to process in repository: {}", self.name);
            return Ok(RepositoryIndexingResult {
                repository_name: self.name.clone(),
                repository_path: self.path.clone(),
                file_results: Vec::new(),
                total_files_processed: 0,
                total_files_skipped: 0,
                total_files_errored: 0,
                total_processing_time: start_time.elapsed(),
                errors: Vec::new(),
                graph_data: None,
                writer_result: None,
                database_path: None,
                database_loaded: false,
            });
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

        let stats = Arc::new(IndexingStats::new());

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
        let mut total_files_processed = 0;
        let mut total_files_skipped = 0;
        let mut total_files_errored = 0;
        let mut errors = Vec::new();
        let mut messages_received = 0;

        // Use timeout-based result collection to prevent hanging
        loop {
            let timeout = Duration::from_secs(RESULT_TIMEOUT_SECS);
            match result_receiver.recv_timeout(timeout) {
                Ok(result_msg) => {
                    messages_received += 1;

                    match &result_msg {
                        ResultMessage::Success(file_result) => {
                            let file_extension =
                                IndexingStats::extract_extension(&file_result.file_path);
                            stats.increment_file_type_processed(&file_extension);
                            total_files_processed += 1;
                        }
                        ResultMessage::Skipped(file_path, _) => {
                            let file_extension = IndexingStats::extract_extension(file_path);
                            stats.increment_file_type_skipped(&file_extension);
                            total_files_skipped += 1;
                        }
                        ResultMessage::Error(file_path, _) => {
                            let file_extension = IndexingStats::extract_extension(file_path);
                            stats.increment_file_type_errored(&file_extension);
                            total_files_errored += 1;
                        }
                    }

                    if messages_received % PROGRESS_UPDATE_INTERVAL == 0 {
                        let progress_pct = stats.progress_percentage(total_files);
                        let completed = stats.total_files_processed();
                        info!(
                            "🔄 Progress: {:.1}% ({}/{} files) - {} processed, {} skipped, {} errors",
                            progress_pct,
                            completed,
                            total_files,
                            stats.files_processed(),
                            stats.files_skipped(),
                            stats.files_errored()
                        );
                    }

                    match result_msg {
                        ResultMessage::Success(file_result) => {
                            file_results.push(file_result);
                        }
                        ResultMessage::Skipped(file_path, reason) => {
                            debug!("Skipped {file_path}: {reason}");
                        }
                        ResultMessage::Error(file_path, error_msg) => {
                            errors.push((file_path.clone(), error_msg.clone()));
                            error!("Error processing {file_path}: {error_msg}");
                        }
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
        feeder_handle.join().map_err(|_| "Feeder thread panicked")?;

        info!(
            "Waiting for {} worker threads to complete...",
            worker_handles.len()
        );
        for (i, handle) in worker_handles.into_iter().enumerate() {
            handle
                .join()
                .map_err(|_| format!("Worker thread {i} panicked"))?;
        }

        let total_processing_time = start_time.elapsed();

        info!(
            "✅ Repository indexing completed for '{}' in {:?}",
            self.name, total_processing_time
        );
        info!(
            "📊 Final results: {:.1}% complete - {} processed, {} skipped, {} errors",
            stats.progress_percentage(total_files),
            stats.files_processed(),
            stats.files_skipped(),
            stats.files_errored()
        );

        let file_type_stats = stats.format_file_type_stats();
        if !file_type_stats.trim().is_empty() {
            info!("📁 File type breakdown:\n{file_type_stats}");
        }

        Ok(RepositoryIndexingResult {
            repository_name: self.name.clone(),
            repository_path: self.path.clone(),
            file_results,
            total_files_processed,
            total_files_skipped,
            total_files_errored,
            total_processing_time,
            errors,
            graph_data: None,
            writer_result: None,
            database_path: None,
            database_loaded: false,
        })
    }

    /// Analyze processed files, write graph data to Parquet files, and load into Kuzu database
    pub fn analyze_and_write_graph_data(
        &self,
        indexing_result: &mut RepositoryIndexingResult,
        output_directory: &str,
        database_path: Option<&str>,
    ) -> Result<(), String> {
        info!(
            "Starting analysis and writing phase for repository: {}",
            self.name
        );
        let start_time = Instant::now();

        let analysis_service = AnalysisService::new(self.name.clone(), self.path.clone());

        let graph_data = analysis_service
            .analyze_results(&indexing_result.file_results)
            .map_err(|e| format!("Analysis failed: {e}"))?;

        info!(
            "Analysis completed: {} files, {} definitions, {} relationships",
            graph_data.file_nodes.len(),
            graph_data.definition_nodes.len(),
            graph_data.file_definition_relationships.len()
        );

        let writer_service = WriterService::new(output_directory)
            .map_err(|e| format!("Failed to create writer service: {e}"))?;

        let mut node_id_generator = NodeIdGenerator::new();

        let writer_result = writer_service
            .write_graph_data(&graph_data, &mut node_id_generator)
            .map_err(|e| format!("Writing failed: {e}"))?;

        let analysis_duration = start_time.elapsed();
        info!(
            "✅ Analysis and writing completed in {:?}. Parquet files created: {}",
            analysis_duration,
            writer_result.files_written.len()
        );

        indexing_result.graph_data = Some(graph_data);
        indexing_result.writer_result = Some(writer_result);

        if let Some(db_path) = database_path {
            info!("Loading graph data into Kuzu database at: {db_path}");
            match self.load_into_database(output_directory, db_path) {
                Ok(_) => {
                    info!("✅ Successfully loaded graph data into Kuzu database");
                    indexing_result.database_path = Some(db_path.to_string());
                    indexing_result.database_loaded = true;
                }
                Err(e) => {
                    warn!("⚠️ Failed to load graph data into database: {e}");
                    indexing_result.database_path = Some(db_path.to_string());
                    indexing_result.database_loaded = false;
                }
            }
        }

        Ok(())
    }
    /// Full repository processing pipeline: discover, index, analyze, and write
    pub fn process_files_full<F: FileSource>(
        &self,
        file_source: F,
        config: &IndexingConfig,
        output_directory: &str,
    ) -> Result<RepositoryIndexingResult, String> {
        self.process_files_full_with_database(file_source, config, output_directory, None)
    }

    pub fn process_files_full_with_database<F: FileSource>(
        &self,
        file_source: F,
        config: &IndexingConfig,
        output_directory: &str,
        database_path: Option<&str>,
    ) -> Result<RepositoryIndexingResult, String> {
        let mut indexing_result = self.index_files(file_source, config)?;
        self.analyze_and_write_graph_data(&mut indexing_result, output_directory, database_path)?;
        Ok(indexing_result)
    }

    fn load_into_database(
        &self,
        parquet_directory: &str,
        database_path: &str,
    ) -> Result<(), String> {
        info!("Initializing Kuzu database and loading graph data...");

        let config = DatabaseConfig::new(database_path)
            .with_buffer_size(512 * 1024 * 1024)
            .with_compression(true);

        let database = KuzuConnection::create_database(config)
            .map_err(|e| format!("Failed to create database: {e:?}"))?;

        let connection_manager = KuzuConnection::new(&database, database_path.to_string())
            .map_err(|e| format!("Failed to create database connection: {e:?}"))?;

        let schema_manager = SchemaManager::new();
        schema_manager
            .initialize_schema(&connection_manager)
            .map_err(|e| format!("Failed to initialize database schema: {e:?}"))?;

        schema_manager
            .import_graph_data(&connection_manager, parquet_directory)
            .map_err(|e| format!("Failed to import graph data: {e:?}"))?;

        match schema_manager.get_schema_stats(&connection_manager) {
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
            format!("{repo_path}/{file_path}")
        };

        let metadata = std::fs::metadata(&full_path).map_err(|e| {
            ProcessingError::Error(file_path.clone(), format!("Failed to read metadata: {e}"))
        })?;

        if metadata.len() as usize > max_file_size {
            return Err(ProcessingError::Skipped(
                file_path.clone(),
                format!("File too large: {} bytes", metadata.len()),
            ));
        }

        let content = std::fs::read_to_string(&full_path).map_err(|e| {
            ProcessingError::Error(file_path.clone(), format!("Failed to read file: {e}"))
        })?;

        process_file_info(file_info, &content)
            .map_err(|e| ProcessingError::Error(file_path, e.to_string()))
    }
}

#[derive(Debug)]
enum ProcessingError {
    Skipped(String, String), // file_path, reason
    Error(String, String),   // file_path, error_message
}
