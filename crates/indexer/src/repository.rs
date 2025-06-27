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
use crossbeam_channel::{bounded, Receiver, RecvTimeoutError, Sender};
use gitalisk_core::repository::gitalisk_repository::{
    CoreGitaliskRepository, FileInfo, IterFileOptions,
};
use log::{debug, error, info, warn};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
// Simplified imports - file processing is now handled by the File module

use crate::analysis::{AnalysisService, GraphData};
use crate::database::{DatabaseConfig, KuzuConnection, SchemaManager};
use crate::parsing::file::process_file_info;
use crate::stats::IndexingStats;
use crate::writer::{WriterResult, WriterService};

// Constants for channel capacity and threading
const DEFAULT_WORKER_THREADS: usize = 8;
const WORK_QUEUE_CAPACITY: usize = 10000; // Increased capacity
const RESULT_QUEUE_CAPACITY: usize = 10000; // Increased capacity
const PROGRESS_UPDATE_INTERVAL: usize = 50; // Update progress every 50 files
const RESULT_TIMEOUT_SECS: u64 = 30; // Timeout for result collection

/// Core repository structure that manages file discovery and parallel processing
#[derive(Debug, Clone)]
pub struct Repository {
    /// Gitalisk repository for file discovery and git operations
    pub gitalisk_repo: CoreGitaliskRepository,
    /// Repository name for identification
    pub name: String,
    /// Files discovered in the repository with pre-computed metadata
    pub files: Vec<FileInfo>,
}

/// Configuration for repository indexing
#[derive(Debug, Clone)]
pub struct IndexingConfig {
    /// Number of worker threads to use (0 means auto-detect)
    pub worker_threads: usize,
    /// Supported file extensions to process
    pub file_extensions: Vec<String>,
    /// Maximum file size to process (in bytes)
    pub max_file_size: usize,
    /// Whether to skip files in .gitignore
    pub respect_gitignore: bool,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            worker_threads: 0, // Auto-detect
            file_extensions: vec![
                "rb".to_string(),
                "py".to_string(),
                "js".to_string(),
                "ts".to_string(),
            ],
            max_file_size: 5_000_000, // 5MB
            respect_gitignore: true,
        }
    }
}

// Re-export from file module
pub use crate::parsing::file::{FileProcessingResult, ProcessingStats};

/// Aggregated results from repository indexing
pub struct RepositoryIndexingResult {
    pub repository_name: String,
    pub repository_path: String,
    pub file_results: Vec<FileProcessingResult>,
    pub total_files_processed: usize,
    pub total_files_skipped: usize,
    pub total_files_errored: usize,
    pub total_processing_time: Duration,
    pub errors: Vec<(String, String)>, // (file_path, error_message)
    pub graph_data: Option<GraphData>,
    pub writer_result: Option<WriterResult>,
    pub database_path: Option<String>,
    pub database_loaded: bool,
}

/// Internal messages for worker communication
enum WorkMessage {
    ProcessFile(FileInfo), // file_info with pre-computed metadata
}

/// Internal messages for result collection
enum ResultMessage {
    Success(FileProcessingResult),
    Skipped(String, String), // file_path, reason
    Error(String, String),   // file_path, error_message
}

impl Repository {
    /// Create a new repository from a gitalisk repository
    pub fn new(gitalisk_repo: CoreGitaliskRepository) -> Self {
        let name = gitalisk_repo
            .path
            .split('/')
            .next_back()
            .unwrap_or("unknown")
            .to_string();

        Self {
            gitalisk_repo,
            name,
            files: Vec::new(),
        }
    }

    /// Create a new repository with a custom name
    pub fn with_name(gitalisk_repo: CoreGitaliskRepository, name: String) -> Self {
        Self {
            gitalisk_repo,
            name,
            files: Vec::new(),
        }
    }

    /// Get the repository path
    pub fn path(&self) -> &str {
        &self.gitalisk_repo.path
    }

    /// Get the workspace path
    pub fn workspace_path(&self) -> &str {
        &self.gitalisk_repo.workspace_path
    }

    /// Discover files in the repository using gitalisk with pre-computed metadata
    pub fn discover_files(&mut self, config: &IndexingConfig) -> Result<(), String> {
        info!("Discovering files in repository: {}", self.name);
        let start_time = Instant::now();

        // Use gitalisk to find repository files with metadata
        let file_infos = self
            .gitalisk_repo
            .get_repo_files(IterFileOptions {
                include_ignored: false,
                include_hidden: false,
            })
            .map_err(|e| format!("Failed to discover files: {e}"))?;

        // Filter files based on configuration (using pre-computed metadata)
        self.files = file_infos
            .into_iter()
            .filter(|file_info| self.should_process_file_info(file_info, config))
            .collect();

        let discovery_time = start_time.elapsed();
        info!(
            "Discovered {} files in repository '{}' in {:?}",
            self.files.len(),
            self.name,
            discovery_time
        );

        Ok(())
    }

    /// Index the repository by processing all discovered files in parallel
    pub fn index_repository(
        &self,
        config: &IndexingConfig,
    ) -> Result<RepositoryIndexingResult, String> {
        let start_time = Instant::now();
        info!("Starting repository indexing for: {}", self.name);

        if self.files.is_empty() {
            warn!("No files to process in repository: {}", self.name);
            return Ok(RepositoryIndexingResult {
                repository_name: self.name.clone(),
                repository_path: self.gitalisk_repo.path.clone(),
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

        // Determine worker thread count
        let worker_count = if config.worker_threads == 0 {
            std::cmp::min(num_cpus::get(), DEFAULT_WORKER_THREADS)
        } else {
            config.worker_threads
        };

        let total_files = self.files.len();
        info!("Processing {total_files} files using {worker_count} worker threads");

        // Create stats tracker
        let stats = Arc::new(IndexingStats::new());

        // Create channels for work distribution and result collection
        let (work_sender, work_receiver) =
            bounded::<WorkMessage>(WORK_QUEUE_CAPACITY.max(total_files));
        let (result_sender, result_receiver) = bounded::<ResultMessage>(RESULT_QUEUE_CAPACITY);

        // Spawn feeder thread to distribute work using FileInfo
        let files_to_process: Vec<FileInfo> = self.files.clone();
        let feeder_work_sender = work_sender;
        let feeder_handle = thread::spawn(move || {
            info!(
                "Feeder thread started, distributing {} files",
                files_to_process.len()
            );
            for file_info in files_to_process {
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

        // Spawn worker threads
        let mut worker_handles = Vec::with_capacity(worker_count);
        for worker_id in 0..worker_count {
            let worker_receiver = work_receiver.clone();
            let worker_sender = result_sender.clone();
            let repo_path = self.gitalisk_repo.path.clone();
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

        // Drop the original channels from main thread
        drop(work_receiver);
        drop(result_sender);

        // Collect results with progress tracking
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

                    // Update stats using pre-computed file extensions (no re-parsing!)
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

                    // Log progress every PROGRESS_UPDATE_INTERVAL messages
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

                    // Process the result
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

                    // Check if we've processed all files
                    if messages_received >= total_files {
                        info!("All {total_files} files processed, finishing result collection");
                        break;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    warn!(
                        "Result collection timeout after {RESULT_TIMEOUT_SECS}s. Received {messages_received}/{total_files} messages. Checking if workers are still alive..."
                    );

                    // Check if any worker threads are still running
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

        // Wait for all threads to complete
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

        // Final progress report with file type breakdown
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

        // Log file type statistics
        let file_type_stats = stats.format_file_type_stats();
        if !file_type_stats.trim().is_empty() {
            info!("📁 File type breakdown:\n{file_type_stats}");
        }

        Ok(RepositoryIndexingResult {
            repository_name: self.name.clone(),
            repository_path: self.gitalisk_repo.path.clone(),
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

        // Create analysis service
        let analysis_service =
            AnalysisService::new(self.name.clone(), self.gitalisk_repo.path.clone());

        // Analyze the file processing results to create graph data
        let graph_data = analysis_service
            .analyze_results(&indexing_result.file_results)
            .map_err(|e| format!("Analysis failed: {e}"))?;

        info!(
            "Analysis completed: {} files, {} definitions, {} relationships",
            graph_data.file_nodes.len(),
            graph_data.definition_nodes.len(),
            graph_data.file_definition_relationships.len()
        );

        // Create writer service and write parquet files
        let writer_service = WriterService::new(output_directory)
            .map_err(|e| format!("Failed to create writer service: {e}"))?;

        let writer_result = writer_service
            .write_graph_data(&graph_data)
            .map_err(|e| format!("Writing failed: {e}"))?;

        let analysis_duration = start_time.elapsed();
        info!(
            "✅ Analysis and writing completed in {:?}. Parquet files created: {}",
            analysis_duration,
            writer_result.files_written.len()
        );

        // Update the indexing result with the analysis and writer results
        indexing_result.graph_data = Some(graph_data);
        indexing_result.writer_result = Some(writer_result);

        // Load data into Kuzu database if database path is provided
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
                    // Note: We don't return an error here to avoid breaking the entire pipeline
                    // The Parquet files are still successfully created
                }
            }
        }

        Ok(())
    }

    /// Full repository processing pipeline: discover, index, analyze, and write
    pub fn process_repository_full(
        &mut self,
        config: &IndexingConfig,
        output_directory: &str,
    ) -> Result<RepositoryIndexingResult, String> {
        self.process_repository_full_with_database(config, output_directory, None)
    }

    /// Full repository processing pipeline with optional database loading
    pub fn process_repository_full_with_database(
        &mut self,
        config: &IndexingConfig,
        output_directory: &str,
        database_path: Option<&str>,
    ) -> Result<RepositoryIndexingResult, String> {
        // Step 1: Discover files
        self.discover_files(config)?;

        // Step 2: Index repository (parse and process files)
        let mut indexing_result = self.index_repository(config)?;

        // Step 3: Analyze and write graph data to Parquet files and optionally load into database
        self.analyze_and_write_graph_data(&mut indexing_result, output_directory, database_path)?;

        Ok(indexing_result)
    }

    /// Load Parquet data into Kuzu database
    fn load_into_database(
        &self,
        parquet_directory: &str,
        database_path: &str,
    ) -> Result<(), String> {
        info!("Initializing Kuzu database and loading graph data...");

        // Create database configuration
        let config = DatabaseConfig::new(database_path)
            .with_buffer_size(512 * 1024 * 1024) // 512MB buffer
            .with_compression(true);

        // Create database
        let database = KuzuConnection::create_database(config)
            .map_err(|e| format!("Failed to create database: {e:?}"))?;

        // Create connection manager with reference to database
        let connection_manager = KuzuConnection::new(&database, database_path.to_string())
            .map_err(|e| format!("Failed to create database connection: {e:?}"))?;

        // Create schema manager and initialize schema
        let schema_manager = SchemaManager::new();
        schema_manager
            .initialize_schema(&connection_manager)
            .map_err(|e| format!("Failed to initialize database schema: {e:?}"))?;

        // Import graph data from Parquet files
        schema_manager
            .import_graph_data(&connection_manager, parquet_directory)
            .map_err(|e| format!("Failed to import graph data: {e:?}"))?;

        // Get and log database statistics
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

    /// Worker thread task for processing files with FileInfo
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

        // Construct full file path
        let full_path = if file_path.starts_with(repo_path) {
            file_path.clone()
        } else {
            format!("{repo_path}/{file_path}")
        };

        // Check file size
        let metadata = std::fs::metadata(&full_path).map_err(|e| {
            ProcessingError::Error(file_path.clone(), format!("Failed to read metadata: {e}"))
        })?;

        if metadata.len() as usize > max_file_size {
            return Err(ProcessingError::Skipped(
                file_path.clone(),
                format!("File too large: {} bytes", metadata.len()),
            ));
        }

        // Read file content
        let content = std::fs::read_to_string(&full_path).map_err(|e| {
            ProcessingError::Error(file_path.clone(), format!("Failed to read file: {e}"))
        })?;

        process_file_info(file_info, &content)
            .map_err(|e| ProcessingError::Error(file_path, e.to_string()))
    }

    fn should_process_file_info(&self, file_info: &FileInfo, config: &IndexingConfig) -> bool {
        let extension = file_info.extension();

        if !config.file_extensions.iter().any(|ext| ext == extension) {
            return false;
        }

        // Additional filtering could be added here
        // (e.g., gitignore patterns, hidden files, etc.)

        true
    }
}

/// Internal error types for file processing
#[derive(Debug)]
enum ProcessingError {
    Skipped(String, String), // file_path, reason
    Error(String, String),   // file_path, error_message
}
