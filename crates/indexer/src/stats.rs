use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Statistics tracking for the indexing process
#[derive(Debug)]
pub struct IndexingStats {
    files_discovered: AtomicUsize,
    files_processed: AtomicUsize,
    files_skipped: AtomicUsize,
    files_errored: AtomicUsize,
    repositories_processed: AtomicUsize,
    repositories_errored: AtomicUsize,
    definitions_found: AtomicUsize,
    references_found: AtomicUsize,
    // File type statistics (extension -> count)
    file_types_processed: Arc<Mutex<HashMap<String, usize>>>,
    file_types_skipped: Arc<Mutex<HashMap<String, usize>>>,
    file_types_errored: Arc<Mutex<HashMap<String, usize>>>,
}

impl IndexingStats {
    pub fn new() -> Self {
        Self {
            files_discovered: AtomicUsize::new(0),
            files_processed: AtomicUsize::new(0),
            files_skipped: AtomicUsize::new(0),
            files_errored: AtomicUsize::new(0),
            repositories_processed: AtomicUsize::new(0),
            repositories_errored: AtomicUsize::new(0),
            definitions_found: AtomicUsize::new(0),
            references_found: AtomicUsize::new(0),
            file_types_processed: Arc::new(Mutex::new(HashMap::new())),
            file_types_skipped: Arc::new(Mutex::new(HashMap::new())),
            file_types_errored: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // File statistics
    pub fn increment_files_discovered(&self) {
        self.files_discovered.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_files_processed(&self) {
        self.files_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_files_skipped(&self) {
        self.files_skipped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_files_errored(&self) {
        self.files_errored.fetch_add(1, Ordering::Relaxed);
    }

    // File type statistics
    pub fn increment_file_type_processed(&self, file_extension: &str) {
        self.increment_files_processed();
        let mut map = self.file_types_processed.lock().unwrap();
        *map.entry(file_extension.to_string()).or_insert(0) += 1;
    }

    pub fn increment_file_type_skipped(&self, file_extension: &str) {
        self.increment_files_skipped();
        let mut map = self.file_types_skipped.lock().unwrap();
        *map.entry(file_extension.to_string()).or_insert(0) += 1;
    }

    pub fn increment_file_type_errored(&self, file_extension: &str) {
        self.increment_files_errored();
        let mut map = self.file_types_errored.lock().unwrap();
        *map.entry(file_extension.to_string()).or_insert(0) += 1;
    }

    // Repository statistics
    pub fn increment_repositories_processed(&self) {
        self.repositories_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_repository_errors(&self) {
        self.repositories_errored.fetch_add(1, Ordering::Relaxed);
    }

    // Code entity statistics
    pub fn add_definitions(&self, count: usize) {
        self.definitions_found.fetch_add(count, Ordering::Relaxed);
    }

    pub fn add_references(&self, count: usize) {
        self.references_found.fetch_add(count, Ordering::Relaxed);
    }

    // Getters
    pub fn files_discovered(&self) -> usize {
        self.files_discovered.load(Ordering::Relaxed)
    }

    pub fn files_processed(&self) -> usize {
        self.files_processed.load(Ordering::Relaxed)
    }

    pub fn files_skipped(&self) -> usize {
        self.files_skipped.load(Ordering::Relaxed)
    }

    pub fn files_errored(&self) -> usize {
        self.files_errored.load(Ordering::Relaxed)
    }

    pub fn repositories_processed(&self) -> usize {
        self.repositories_processed.load(Ordering::Relaxed)
    }

    pub fn repositories_errored(&self) -> usize {
        self.repositories_errored.load(Ordering::Relaxed)
    }

    pub fn definitions_found(&self) -> usize {
        self.definitions_found.load(Ordering::Relaxed)
    }

    pub fn references_found(&self) -> usize {
        self.references_found.load(Ordering::Relaxed)
    }

    // File type getters
    pub fn file_types_processed(&self) -> HashMap<String, usize> {
        self.file_types_processed.lock().unwrap().clone()
    }

    pub fn file_types_skipped(&self) -> HashMap<String, usize> {
        self.file_types_skipped.lock().unwrap().clone()
    }

    pub fn file_types_errored(&self) -> HashMap<String, usize> {
        self.file_types_errored.lock().unwrap().clone()
    }

    // Progress calculation
    pub fn total_files_processed(&self) -> usize {
        self.files_processed() + self.files_skipped() + self.files_errored()
    }

    pub fn progress_percentage(&self, total_expected: usize) -> f64 {
        if total_expected == 0 {
            100.0
        } else {
            (self.total_files_processed() as f64 / total_expected as f64) * 100.0
        }
    }

    // Helper to extract file extension
    pub fn extract_extension(file_path: &str) -> String {
        std::path::Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    // Pretty print file type statistics
    pub fn format_file_type_stats(&self) -> String {
        let mut result = String::new();

        let processed = self.file_types_processed();
        let skipped = self.file_types_skipped();
        let errored = self.file_types_errored();

        // Collect all file types
        let mut all_types: HashSet<String> = HashSet::new();
        all_types.extend(processed.keys().cloned());
        all_types.extend(skipped.keys().cloned());
        all_types.extend(errored.keys().cloned());

        let mut types: Vec<String> = all_types.into_iter().collect();
        types.sort();

        for file_type in types {
            let proc_count = processed.get(&file_type).unwrap_or(&0);
            let skip_count = skipped.get(&file_type).unwrap_or(&0);
            let err_count = errored.get(&file_type).unwrap_or(&0);
            let total = proc_count + skip_count + err_count;

            if total > 0 {
                result.push_str(&format!(
                    "  • .{}: {} total (✅ {} processed, ⏭️ {} skipped, ❌ {} errors)\n",
                    file_type, total, proc_count, skip_count, err_count
                ));
            }
        }

        result
    }
}

impl Default for IndexingStats {
    fn default() -> Self {
        Self::new()
    }
}
