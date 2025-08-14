//! Data directory management for the workspace-manager crate
//!
//! This module handles the centralized data directory where all Knowledge Graph
//! workspace folder data is stored, including manifest files, Kuzu databases, and Parquet files.
//! The typical structure of the data directory would look like this:
//!
//! ```text
//! .gkg/
//! ├── gkg_workspace_folders/
//! │   ├── workspace_folder_1_hash/
//! │   │   ├── project_1_hash/
//! │   │   │   ├── database.kz
//! │   │   │   ├── parquet_files/
//! │   │   ├── project_2_hash/
//! │   │   │   ├── database.kz
//! │   │   │   ├── parquet_files/
//! │   ├── workspace_folder_2_hash/
//! │   │   ├── project_1_hash/
//! │   │   │   ├── database.kz
//! │   │   │   ├── parquet_files/
//! ├── gkg_manifest.json
//! ```

use crate::errors::{Result, WorkspaceManagerError};
use std::path::{Path, PathBuf};

const GKG_DATA_DIR_NAME: &str = ".gkg";
const GKG_WORKSPACE_FOLDERS_NAME: &str = "gkg_workspace_folders";
const GKG_MANIFEST_FILE_NAME: &str = "gkg_manifest.json";
const GKG_KUZU_DB_NAME: &str = "database.kz";
const GKG_PARQUET_FILES_NAME: &str = "parquet_files";

/// Manages the centralized data directory for the Knowledge Graph framework
#[derive(Debug, Clone)]
pub struct DataDirectory {
    pub root_path: PathBuf,
    pub workspace_folders_dir: PathBuf,
    pub manifest_path: PathBuf,
}

impl DataDirectory {
    pub fn new_system_default() -> Result<Self> {
        let root_path = Self::get_system_data_directory()?;
        Self::new(root_path)
    }

    pub fn new(root_path: PathBuf) -> Result<Self> {
        let workspace_folders_dir = root_path.join(GKG_WORKSPACE_FOLDERS_NAME);
        let manifest_path = root_path.join(GKG_MANIFEST_FILE_NAME);
        let data_dir = Self {
            root_path,
            workspace_folders_dir,
            manifest_path,
        };
        data_dir.ensure_directory_structure()?;
        Ok(data_dir)
    }

    pub fn get_system_data_directory() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|data_dir| data_dir.join(GKG_DATA_DIR_NAME))
            .ok_or(WorkspaceManagerError::SystemDataDirectoryNotFound)
    }

    pub fn workspace_folder_data_directory(&self, workspace_folder_name: &str) -> PathBuf {
        self.workspace_folders_dir.join(workspace_folder_name)
    }

    pub fn project_directory(&self, workspace_folder_name: &str, project_name: &str) -> PathBuf {
        self.workspace_folder_data_directory(workspace_folder_name)
            .join(project_name)
    }

    pub fn project_database_path(
        &self,
        workspace_folder_name: &str,
        project_name: &str,
    ) -> PathBuf {
        self.project_directory(workspace_folder_name, project_name)
            .join(GKG_KUZU_DB_NAME)
    }

    pub fn project_parquet_directory(
        &self,
        workspace_folder_name: &str,
        project_name: &str,
    ) -> PathBuf {
        self.project_directory(workspace_folder_name, project_name)
            .join(GKG_PARQUET_FILES_NAME)
    }

    pub fn ensure_directory_structure(&self) -> Result<()> {
        if !self.root_path.exists() {
            std::fs::create_dir_all(&self.root_path).map_err(|_| {
                WorkspaceManagerError::DataDirectoryCreationFailed {
                    path: self.root_path.clone(),
                }
            })?;
            log::info!("Created data directory: {}", self.root_path.display());
        }

        if !self.workspace_folders_dir.exists() {
            std::fs::create_dir_all(&self.workspace_folders_dir).map_err(|_| {
                WorkspaceManagerError::DataDirectoryCreationFailed {
                    path: self.workspace_folders_dir.to_path_buf(),
                }
            })?;
            log::debug!(
                "Created workspace folders directory: {}",
                self.workspace_folders_dir.display()
            );
        }

        Ok(())
    }

    pub fn ensure_workspace_folder_directory(&self, data_directory_name: &str) -> Result<()> {
        let workspace_folder_dir = self.workspace_folder_data_directory(data_directory_name);

        if !workspace_folder_dir.exists() {
            std::fs::create_dir_all(&workspace_folder_dir).map_err(|_| {
                WorkspaceManagerError::DataDirectoryCreationFailed {
                    path: workspace_folder_dir.clone(),
                }
            })?;
            log::debug!(
                "Created workspace folder directory: {}",
                workspace_folder_dir.display()
            );
        }

        Ok(())
    }

    pub fn ensure_project_directory(
        &self,
        workspace_folder_name: &str,
        project_name: &str,
    ) -> Result<()> {
        self.ensure_workspace_folder_directory(workspace_folder_name)?;

        let project_dir = self.project_directory(workspace_folder_name, project_name);
        if !project_dir.exists() {
            std::fs::create_dir_all(&project_dir).map_err(|_| {
                WorkspaceManagerError::DataDirectoryCreationFailed {
                    path: project_dir.clone(),
                }
            })?;
            log::debug!("Created project directory: {}", project_dir.display());
        }

        let parquet_dir = self.project_parquet_directory(workspace_folder_name, project_name);
        if !parquet_dir.exists() {
            std::fs::create_dir_all(&parquet_dir).map_err(|_| {
                WorkspaceManagerError::DataDirectoryCreationFailed {
                    path: parquet_dir.clone(),
                }
            })?;
        }

        Ok(())
    }

    pub fn remove_workspace_folder_directory(&self, data_directory_name: &str) -> Result<()> {
        let workspace_folder_dir = self.workspace_folder_data_directory(data_directory_name);

        if workspace_folder_dir.exists() {
            std::fs::remove_dir_all(&workspace_folder_dir)?;
            log::info!(
                "Removed workspace folder directory: {}",
                workspace_folder_dir.display()
            );
        }

        Ok(())
    }

    pub fn remove_project_directory(
        &self,
        workspace_folder_name: &str,
        project_name: &str,
    ) -> Result<()> {
        let project_dir = self.project_directory(workspace_folder_name, project_name);

        if project_dir.exists() {
            std::fs::remove_dir_all(&project_dir)?;
            log::info!("Removed project directory: {}", project_dir.display());
        }

        Ok(())
    }

    pub fn get_workspace_folder_directory_size(&self, data_directory_name: &str) -> Result<u64> {
        let workspace_folder_dir = self.workspace_folder_data_directory(data_directory_name);
        Self::calculate_directory_size(&workspace_folder_dir)
    }

    pub fn get_project_directory_size(
        &self,
        workspace_folder_name: &str,
        project_name: &str,
    ) -> Result<u64> {
        let project_dir = self.project_directory(workspace_folder_name, project_name);
        Self::calculate_directory_size(&project_dir)
    }

    // Note: kuzu_db typically saves its "database" as a directory with the same name as the database.
    fn calculate_directory_size(dir: &Path) -> Result<u64> {
        if !dir.exists() {
            return Ok(0);
        }

        use ignore::WalkBuilder;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU64, Ordering};

        let total_size = Arc::new(AtomicU64::new(0));
        let walker = WalkBuilder::new(dir)
            .standard_filters(false)
            .build_parallel();

        let size_clone = Arc::clone(&total_size);
        walker.run(|| {
            let size_ref = Arc::clone(&size_clone);
            Box::new(move |entry| match entry {
                Ok(dir_entry) => {
                    if let Ok(metadata) = dir_entry.metadata()
                        && metadata.is_file()
                    {
                        size_ref.fetch_add(metadata.len(), Ordering::Relaxed);
                    }
                    ignore::WalkState::Continue
                }
                Err(_) => ignore::WalkState::Continue,
            })
        });

        Ok(total_size.load(Ordering::Relaxed))
    }

    pub fn list_workspace_folder_directories(&self) -> Result<Vec<String>> {
        let workspace_folder_dir = &self.workspace_folders_dir;

        if !workspace_folder_dir.exists() {
            return Ok(Vec::with_capacity(16));
        }

        let mut workspace_folder_dirs = Vec::with_capacity(16);

        for entry in std::fs::read_dir(workspace_folder_dir)? {
            let entry = entry?;
            if entry.metadata()?.is_dir()
                && let Some(dir_name) = entry.file_name().to_str()
            {
                workspace_folder_dirs.push(dir_name.to_string());
            }
        }

        workspace_folder_dirs.sort();
        Ok(workspace_folder_dirs)
    }

    pub fn list_project_directories(&self, workspace_folder_name: &str) -> Result<Vec<String>> {
        let workspace_folder_dir = self.workspace_folder_data_directory(workspace_folder_name);

        if !workspace_folder_dir.exists() {
            return Ok(Vec::with_capacity(8));
        }

        let mut project_dirs = Vec::with_capacity(8);

        for entry in std::fs::read_dir(workspace_folder_dir)? {
            let entry = entry?;
            if entry.metadata()?.is_dir()
                && let Some(dir_name) = entry.file_name().to_str()
            {
                project_dirs.push(dir_name.to_string());
            }
        }

        project_dirs.sort();
        Ok(project_dirs)
    }

    pub fn count_workspace_folder_directories(&self) -> Result<usize> {
        let workspace_folder_dir = &self.workspace_folders_dir;

        if !workspace_folder_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        for entry in std::fs::read_dir(workspace_folder_dir)? {
            let entry = entry?;
            if entry.metadata()?.is_dir() {
                count += 1;
            }
        }

        Ok(count)
    }

    pub fn count_project_directories(&self, workspace_folder_name: &str) -> Result<usize> {
        let workspace_folder_dir = self.workspace_folder_data_directory(workspace_folder_name);

        if !workspace_folder_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        for entry in std::fs::read_dir(workspace_folder_dir)? {
            let entry = entry?;
            if entry.metadata()?.is_dir() {
                count += 1;
            }
        }

        Ok(count)
    }

    pub fn get_info(&self) -> Result<WorkspaceFolderDataDirectoryInfo> {
        let total_size = Self::calculate_directory_size(&self.root_path)?;
        let workspace_folder_directories = self.list_workspace_folder_directories()?;
        let workspace_folder_count = workspace_folder_directories.len();
        let manifest_exists = self.manifest_path.exists();

        Ok(WorkspaceFolderDataDirectoryInfo {
            root_path: self.root_path.clone(),
            total_size,
            workspace_folder_count,
            workspace_folder_directories,
            manifest_exists,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceFolderDataDirectoryInfo {
    pub root_path: PathBuf,
    pub total_size: u64,
    pub workspace_folder_count: usize,
    pub workspace_folder_directories: Vec<String>,
    pub manifest_exists: bool,
}

impl WorkspaceFolderDataDirectoryInfo {
    pub fn format_total_size(&self) -> String {
        format_bytes(self.total_size)
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;

    if bytes == 0 {
        return "0 B".to_string();
    }

    if bytes < THRESHOLD {
        return format!("{bytes} B");
    }

    let mut unit_index = 0;
    let mut temp_bytes = bytes;
    while temp_bytes >= THRESHOLD && unit_index < UNITS.len() - 1 {
        temp_bytes /= THRESHOLD;
        unit_index += 1;
    }

    let divisor = THRESHOLD.pow(unit_index as u32);
    let size = bytes as f64 / divisor as f64;
    format!("{:.1} {}", size, UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_data_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = DataDirectory::new(temp_dir.path().to_path_buf()).unwrap();

        assert!(data_dir.root_path.exists());
        assert!(data_dir.workspace_folders_dir.exists());
        assert!(data_dir.manifest_path.parent().unwrap().exists());
    }

    #[test]
    fn test_new_system_default() {
        match DataDirectory::new_system_default() {
            Ok(data_dir) => {
                assert!(data_dir.root_path.exists());
                assert!(data_dir.workspace_folders_dir.exists());

                if let Err(e) = std::fs::remove_dir_all(data_dir.root_path) {
                    eprintln!("Warning: Failed to cleanup test data directory: {e}");
                }
            }
            Err(WorkspaceManagerError::SystemDataDirectoryNotFound) => {
                panic!("System data directory not found");
            }
            Err(e) => {
                panic!("Unexpected error from new_system_default: {e:?}");
            }
        }
    }

    #[test]
    fn test_project_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = DataDirectory::new(temp_dir.path().to_path_buf()).unwrap();

        let workspace_folder_name = "test-workspace-folder-name";
        let project_name = "test-project-name";
        data_dir
            .ensure_project_directory(workspace_folder_name, project_name)
            .unwrap();

        assert!(
            data_dir
                .workspace_folder_data_directory(workspace_folder_name)
                .exists()
        );
        assert!(
            data_dir
                .project_directory(workspace_folder_name, project_name)
                .exists()
        );
        assert!(
            data_dir
                .project_parquet_directory(workspace_folder_name, project_name)
                .exists()
        );
    }

    #[test]
    fn test_path_getters() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = DataDirectory::new(temp_dir.path().to_path_buf()).unwrap();

        let workspace_name = "test-workspace";
        let project_name = "test-project";

        assert_eq!(data_dir.root_path, temp_dir.path());

        let expected_manifest = temp_dir.path().join(GKG_MANIFEST_FILE_NAME);
        assert_eq!(data_dir.manifest_path, expected_manifest);

        let expected_workspace_folders = temp_dir.path().join(GKG_WORKSPACE_FOLDERS_NAME);
        assert_eq!(data_dir.workspace_folders_dir, expected_workspace_folders);

        let expected_workspace_dir = expected_workspace_folders.join(workspace_name);
        assert_eq!(
            data_dir.workspace_folder_data_directory(workspace_name),
            expected_workspace_dir
        );

        let expected_project_dir = expected_workspace_dir.join(project_name);
        assert_eq!(
            data_dir.project_directory(workspace_name, project_name),
            expected_project_dir
        );

        let expected_db_path = expected_project_dir.join(GKG_KUZU_DB_NAME);
        assert_eq!(
            data_dir.project_database_path(workspace_name, project_name),
            expected_db_path
        );

        let expected_parquet_path = expected_project_dir.join(GKG_PARQUET_FILES_NAME);
        assert_eq!(
            data_dir.project_parquet_directory(workspace_name, project_name),
            expected_parquet_path
        );
    }

    #[test]
    fn test_remove_workspace_folder_directory() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = DataDirectory::new(temp_dir.path().to_path_buf()).unwrap();

        let workspace_name = "test-workspace-to-remove";

        data_dir
            .ensure_workspace_folder_directory(workspace_name)
            .unwrap();
        assert!(
            data_dir
                .workspace_folder_data_directory(workspace_name)
                .exists()
        );

        data_dir
            .remove_workspace_folder_directory(workspace_name)
            .unwrap();
        assert!(
            !data_dir
                .workspace_folder_data_directory(workspace_name)
                .exists()
        );

        data_dir
            .remove_workspace_folder_directory("non-existent")
            .unwrap();
    }

    #[test]
    fn test_list_workspace_folder_directories() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = DataDirectory::new(temp_dir.path().to_path_buf()).unwrap();

        let dirs = data_dir.list_workspace_folder_directories().unwrap();
        assert!(dirs.is_empty());

        let workspace_names = vec!["workspace-z", "workspace-a", "workspace-m"];
        for name in &workspace_names {
            data_dir.ensure_workspace_folder_directory(name).unwrap();
        }

        let dirs = data_dir.list_workspace_folder_directories().unwrap();
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs, vec!["workspace-a", "workspace-m", "workspace-z"]);
    }

    #[test]
    fn test_directory_size_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = DataDirectory::new(temp_dir.path().to_path_buf()).unwrap();

        let workspace_name = "test-workspace-size";
        data_dir
            .ensure_workspace_folder_directory(workspace_name)
            .unwrap();

        let initial_size = data_dir
            .get_workspace_folder_directory_size(workspace_name)
            .unwrap();

        let workspace_dir = data_dir.workspace_folder_data_directory(workspace_name);
        let test_file = workspace_dir.join("test.txt");
        fs::write(&test_file, "Hello, World!").unwrap();

        let new_size = data_dir
            .get_workspace_folder_directory_size(workspace_name)
            .unwrap();
        assert!(new_size > initial_size);

        let non_existent_size = data_dir
            .get_workspace_folder_directory_size("non-existent")
            .unwrap();
        assert_eq!(non_existent_size, 0);
    }

    #[test]
    fn test_nested_directory_size_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = DataDirectory::new(temp_dir.path().to_path_buf()).unwrap();

        let workspace_name = "test-nested-size";
        data_dir
            .ensure_workspace_folder_directory(workspace_name)
            .unwrap();

        let workspace_dir = data_dir.workspace_folder_data_directory(workspace_name);

        let nested_dir = workspace_dir.join("nested").join("deep");
        fs::create_dir_all(&nested_dir).unwrap();

        fs::write(workspace_dir.join("root.txt"), "root").unwrap();
        fs::write(nested_dir.join("deep.txt"), "deep file content").unwrap();

        let total_size = data_dir
            .get_workspace_folder_directory_size(workspace_name)
            .unwrap();
        assert!(total_size >= 1);
    }

    #[test]
    fn test_workspace_folder_names_with_special_characters() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = DataDirectory::new(temp_dir.path().to_path_buf()).unwrap();

        let workspace_names = vec![
            "workspace-with-dashes",
            "workspace_with_underscores",
            "workspace123with456numbers",
            "WorkspaceWithCamelCase",
        ];

        for name in &workspace_names {
            data_dir.ensure_workspace_folder_directory(name).unwrap();
            assert!(data_dir.workspace_folder_data_directory(name).exists());
        }

        let dirs = data_dir.list_workspace_folder_directories().unwrap();
        assert_eq!(dirs.len(), workspace_names.len());

        for name in workspace_names {
            assert!(dirs.contains(&name.to_string()));
        }
    }

    #[test]
    fn test_project_operations() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = DataDirectory::new(temp_dir.path().to_path_buf()).unwrap();

        let workspace_name = "test-workspace";
        let project1 = "project-1";
        let project2 = "project-2";

        data_dir
            .ensure_project_directory(workspace_name, project1)
            .unwrap();
        data_dir
            .ensure_project_directory(workspace_name, project2)
            .unwrap();

        let projects = data_dir.list_project_directories(workspace_name).unwrap();
        assert_eq!(projects.len(), 2);
        assert_eq!(projects, vec!["project-1", "project-2"]);

        let count = data_dir.count_project_directories(workspace_name).unwrap();
        assert_eq!(count, 2);

        data_dir
            .remove_project_directory(workspace_name, project1)
            .unwrap();
        let projects = data_dir.list_project_directories(workspace_name).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects, vec!["project-2"]);

        let project_size = data_dir
            .get_project_directory_size(workspace_name, project2)
            .unwrap();
        assert_eq!(project_size, 0);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_bytes(1024_u64.pow(4)), "1.0 TB");
    }
}
