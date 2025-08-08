use gitalisk_core::repository::gitalisk_repository::{FileStatusInfo, StatusCode};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct FileChanges {
    pub changed_files: HashSet<String>,
    pub deleted_files: HashSet<String>,
    pub changed_dirs: HashSet<String>,
    pub deleted_dirs: HashSet<String>,
}

#[derive(Debug, Clone)]
pub enum FileChangesPathType {
    ChangedFiles,
    DeletedFiles,
    ChangedDirs,
    DeletedDirs,
}

// HELPERS

// Convert a path to a relative path
fn to_relative_path(path: &str, repo_path: &Path) -> String {
    Path::new(path)
        .strip_prefix(repo_path)
        .unwrap_or(Path::new(path))
        .to_string_lossy()
        .to_string()
}

// Check if a path is a directory
fn is_dir(path: &str) -> bool {
    path.ends_with('/')
        || path
            .split('/')
            .next_back()
            .is_some_and(|s| !s.contains('.'))
}

impl FileChanges {
    pub fn from_git_status(git_status: Vec<FileStatusInfo>) -> Self {
        let mut changed_files = HashSet::new();
        let mut deleted_files = HashSet::new();
        let mut changed_dirs = HashSet::new();
        let mut deleted_dirs = HashSet::new();

        for status in git_status {
            let path = status.path;
            let is_dir = is_dir(&path);

            if status.status.index == StatusCode::Deleted
                || status.status.worktree == StatusCode::Deleted
            {
                match is_dir {
                    true => deleted_dirs.insert(path),
                    false => deleted_files.insert(path),
                };
            } else if status.status.index == StatusCode::Added
                || status.status.worktree == StatusCode::Added
                || status.status.index == StatusCode::Modified
                || status.status.worktree == StatusCode::Modified
            {
                match is_dir {
                    true => changed_dirs.insert(path),
                    false => changed_files.insert(path),
                };
            }
        }

        Self {
            changed_files,
            deleted_files,
            changed_dirs,
            deleted_dirs,
        }
    }

    pub fn from_watched_files(watched_files_paths: Vec<String>) -> Self {
        let mut changed_files = HashSet::new();
        let mut deleted_files = HashSet::new();
        let mut changed_dirs = HashSet::new();
        let mut deleted_dirs = HashSet::new();

        // Check file/directory validity via os
        for path_str in &watched_files_paths {
            let path = Path::new(path_str);
            if path.exists() {
                if path.is_file() {
                    changed_files.insert(path_str.to_string());
                } else if path.is_dir() {
                    changed_dirs.insert(path_str.to_string());
                }
            } else if is_dir(path_str) {
                deleted_dirs.insert(path_str.to_string());
            } else {
                deleted_files.insert(path_str.to_string());
            }
        }

        Self {
            changed_files,
            deleted_files,
            changed_dirs,
            deleted_dirs,
        }
    }

    pub fn has_changes(&self) -> bool {
        !self.changed_files.is_empty()
            || !self.deleted_files.is_empty()
            || !self.changed_dirs.is_empty()
            || !self.deleted_dirs.is_empty()
    }

    pub fn pretty_print(&self) {
        tracing::info!("Changed files: {:?}", self.changed_files.len());
        tracing::info!("Deleted files: {:?}", self.deleted_files.len());
        tracing::info!("Changed dirs: {:?}", self.changed_dirs.len());
        tracing::info!("Deleted dirs: {:?}", self.deleted_dirs.len());

        tracing::info!("\nChanged files:");
        for file in &self.changed_files {
            tracing::info!("  {file}");
        }

        tracing::info!("Deleted files:");
        for file in &self.deleted_files {
            tracing::info!("  {file}");
        }

        tracing::info!("Changed dirs:");
        for dir in &self.changed_dirs {
            tracing::info!("  {dir}");
        }

        tracing::info!("Deleted dirs:");
        for dir in &self.deleted_dirs {
            tracing::info!("  {dir}");
        }
    }

    pub fn get_rel_paths(&self, path_type: FileChangesPathType, repo_path: &str) -> Vec<String> {
        let repo_path = Path::new(repo_path);
        let paths = match path_type {
            FileChangesPathType::ChangedFiles => &self.changed_files,
            FileChangesPathType::DeletedFiles => &self.deleted_files,
            FileChangesPathType::ChangedDirs => &self.changed_dirs,
            FileChangesPathType::DeletedDirs => &self.deleted_dirs,
        };

        paths
            .iter()
            .map(|p| to_relative_path(p, repo_path))
            .collect()
    }
}
