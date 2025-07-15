use std::collections::HashSet;
use std::path::PathBuf;

use crate::indexer::IndexingConfig;
use crate::parsing::changes::FileChanges;
use crate::project::file_info::FileInfo;
use ignore::WalkBuilder;
use parser_core::parser::get_supported_extensions;
use std::sync::{Arc, Mutex};

// File source implementations to support different deployment scenarios:
//
// 1. Desktop Use Cases (CLI, Language Server, IDE integration):
//    - PathFileSource: Used when we have direct filesystem access and can enumerate files locally
//    - GitaliskFileSource: Used for local git repository operations and workspace management
//    - Supports real-time incremental indexing as users edit files
//    - Optimized for low-latency, interactive use cases
//
// 2. Server-Side Use Cases (GitLab Zoekt integration):
//    - Server-side workers will typically receive file content directly from Gitaly
//    - May use PathFileSource with pre-enumerated file lists from the server infrastructure
//    - Focuses on bulk indexing operations for repository-wide analysis
//    - Integrates with existing GitLab search infrastructure (Zoekt nodes)
//
// The FileSource trait provides a unified interface that allows the core indexing logic
// to remain agnostic to the specific file discovery mechanism being used.

pub trait FileSource {
    type Error: std::fmt::Display + Send + Sync + 'static;

    fn get_files(&self, config: &IndexingConfig) -> Result<Vec<FileInfo>, Self::Error>;
}

pub struct PathFileSource {
    pub files: Vec<FileInfo>,
    pub supported_extensions: HashSet<String>,
}

impl PathFileSource {
    pub fn new(files: Vec<FileInfo>) -> Self {
        let supported_extensions: HashSet<String> = get_supported_extensions()
            .iter()
            .map(|ext| ext.to_string())
            .collect();
        Self {
            files,
            supported_extensions,
        }
    }

    pub fn from_path(path: PathBuf) -> Self {
        // This is duplicate code that also exists in `::new`. But needed now to filter the files
        let supported_extensions: HashSet<String> = get_supported_extensions()
            .iter()
            .map(|ext| ext.to_string())
            .collect();

        let files = Arc::new(Mutex::new(Vec::new()));

        WalkBuilder::new(&path)
            .hidden(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .ignore(false)
            .parents(false)
            .build_parallel()
            .run(|| {
                let files: Arc<Mutex<Vec<FileInfo>>> = Arc::clone(&files);
                let supported_extensions = supported_extensions.clone();

                Box::new(move |result| {
                    if let Ok(entry) = result {
                        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                            let file_info = FileInfo::from_path(entry.path().to_path_buf());
                            if should_process_file_info(&file_info, &supported_extensions) {
                                files.lock().unwrap().push(file_info);
                            }
                        }
                    }
                    ignore::WalkState::Continue
                })
            });

        Self::new(files.lock().unwrap().clone())
    }
}

impl FileSource for PathFileSource {
    type Error = &'static str;

    fn get_files(&self, _config: &IndexingConfig) -> Result<Vec<FileInfo>, Self::Error> {
        let filtered_files = self
            .files
            .iter()
            .filter(|file_info| should_process_file_info(file_info, &self.supported_extensions))
            .cloned()
            .collect();
        Ok(filtered_files)
    }
}

#[derive(Debug, Clone)]
pub struct GitaliskFileSource {
    pub repository: gitalisk_core::repository::gitalisk_repository::CoreGitaliskRepository,
    pub supported_extensions: HashSet<String>,
}

impl GitaliskFileSource {
    pub fn new(
        repository: gitalisk_core::repository::gitalisk_repository::CoreGitaliskRepository,
    ) -> Self {
        let supported_extensions: HashSet<String> = get_supported_extensions()
            .iter()
            .map(|ext| ext.to_string())
            .collect();

        Self {
            repository,
            supported_extensions,
        }
    }
}

impl FileSource for GitaliskFileSource {
    type Error = std::io::Error;

    fn get_files(&self, config: &IndexingConfig) -> Result<Vec<FileInfo>, Self::Error> {
        let gitalisk_files = self.repository.get_repo_files(
            gitalisk_core::repository::gitalisk_repository::IterFileOptions {
                include_ignored: !config.respect_gitignore,
                include_hidden: false,
            },
        )?;

        let filtered_files = gitalisk_files
            .into_iter()
            .filter(|file_info| should_process_file_info(file_info, &self.supported_extensions))
            .collect();

        Ok(filtered_files)
    }
}

pub struct ChangesFileSource {
    pub changes: FileChanges,
    pub repository_path: String,
    pub supported_extensions: HashSet<String>,
}

impl ChangesFileSource {
    pub fn new(changes: &FileChanges, repository_path: String) -> Self {
        let supported_extensions: HashSet<String> = get_supported_extensions()
            .iter()
            .map(|ext| ext.to_string())
            .collect();

        Self {
            changes: changes.clone(),
            repository_path,
            supported_extensions,
        }
    }
}

impl FileSource for ChangesFileSource {
    type Error = std::io::Error;

    fn get_files(&self, _config: &IndexingConfig) -> Result<Vec<FileInfo>, Self::Error> {
        let mut files = Vec::new();

        // Convert changed files to FileInfo
        for file_path in &self.changes.changed_files {
            let path = PathBuf::from(&self.repository_path).join(file_path);
            let file_info = FileInfo::from_path(path);
            if should_process_file_info(&file_info, &self.supported_extensions) {
                files.push(file_info);
            }
        }

        Ok(files)
    }
}

// TODO: refactor this so that we have a cleaner architecture on
// parsing detection, language detection, indexer language management, etc.
fn should_process_file_info(file_info: &FileInfo, supported_extensions: &HashSet<String>) -> bool {
    let extension = file_info.extension();
    supported_extensions.contains(extension)
}
