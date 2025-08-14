use crate::data_directory::DataDirectory;
use crate::errors::{Result, WorkspaceManagerError};
use crate::manifest::{ProjectMetadata, Status, WorkspaceFolderMetadata, generate_path_hash};
use crate::state_service::LocalStateService;
use gitalisk_core::repository::gitalisk_repository::CoreGitaliskRepository;
use gitalisk_core::workspace_folder::gitalisk_workspace::CoreGitaliskWorkspaceFolder;
use log::info;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Current framework version for tracking compatibility
const FRAMEWORK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Main workspace management service that orchestrates all workspace and project operations
#[derive(Clone)]
pub struct WorkspaceManager {
    data_directory: DataDirectory,
    state_service: LocalStateService,
    gitalisk_workspaces: Arc<RwLock<HashMap<String, Arc<CoreGitaliskWorkspaceFolder>>>>,
}

/// Information about a registered workspace folder
#[derive(Clone)]
pub struct WorkspaceFolderInfo {
    pub workspace_folder_path: String,
    pub data_directory_name: String,
    pub status: Status,
    pub last_indexed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub project_count: usize,
    pub gitalisk_workspace: Option<Arc<CoreGitaliskWorkspaceFolder>>,
}

// TODO: make CoreGitaliskWorkspaceFolder implement Debug
impl std::fmt::Debug for WorkspaceFolderInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceFolderInfo")
            .field("workspace_folder_path", &self.workspace_folder_path)
            .field("data_directory_name", &self.data_directory_name)
            .field("status", &self.status)
            .field("last_indexed_at", &self.last_indexed_at)
            .field("project_count", &self.project_count)
            .field(
                "gitalisk_workspace",
                &self
                    .gitalisk_workspace
                    .as_ref()
                    .map(|_| "Arc<CoreGitaliskWorkspaceFolder>"),
            )
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub project_path: String,
    pub workspace_folder_path: String,
    pub project_hash: String,
    pub status: Status,
    pub last_indexed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
    pub database_path: PathBuf,
    pub parquet_directory: PathBuf,
    pub repository: CoreGitaliskRepository,
}

impl WorkspaceManager {
    /// Create a new WorkspaceManager with the provided dependencies
    ///
    /// This is the primary constructor that accepts pre-configured dependencies,
    /// making it ideal for testing and advanced use cases.
    pub fn new(data_directory: DataDirectory, state_service: LocalStateService) -> Self {
        Self {
            data_directory,
            state_service,
            gitalisk_workspaces: Arc::new(RwLock::new(HashMap::with_capacity(16))),
        }
    }

    /// Create a new WorkspaceManager with system default data directory
    ///
    /// This is a convenience factory method that automatically configures
    /// the dependencies using system defaults.
    pub fn new_system_default() -> Result<Self> {
        let data_directory = DataDirectory::new_system_default()?;
        let state_service =
            LocalStateService::new(&data_directory.manifest_path, FRAMEWORK_VERSION.to_string())?;

        Ok(Self::new(data_directory, state_service))
    }

    /// Create a new WorkspaceManager with custom data directory
    ///
    /// This is a convenience factory method that automatically configures
    /// the dependencies using the provided data directory path.
    pub fn new_with_directory(data_directory_path: PathBuf) -> Result<Self> {
        let data_directory = DataDirectory::new(data_directory_path)?;
        let state_service =
            LocalStateService::new(&data_directory.manifest_path, FRAMEWORK_VERSION.to_string())?;

        Ok(Self::new(data_directory, state_service))
    }

    fn register_project_internal(
        &self,
        workspace_folder_path: &str,
        workspace_metadata: &WorkspaceFolderMetadata,
        project_path: String,
        project_hash: String,
        project_metadata: &ProjectMetadata,
        repository: CoreGitaliskRepository,
    ) -> Result<ProjectInfo> {
        self.data_directory
            .ensure_project_directory(&workspace_metadata.data_directory_name, &project_hash)?;

        let database_path = self
            .data_directory
            .project_database_path(&workspace_metadata.data_directory_name, &project_hash);
        let parquet_directory = self
            .data_directory
            .project_parquet_directory(&workspace_metadata.data_directory_name, &project_hash);

        Ok(ProjectInfo {
            project_path,
            workspace_folder_path: workspace_folder_path.to_string(),
            project_hash,
            status: project_metadata.status.clone(),
            last_indexed_at: project_metadata.last_indexed_at,
            error_message: project_metadata.error_message.clone(),
            database_path,
            parquet_directory,
            repository,
        })
    }

    pub fn register_workspace_folder(
        &self,
        workspace_folder_path: &Path,
    ) -> Result<WorkspaceFolderInfo> {
        let canonical_workspace_folder_path = workspace_folder_path
            .canonicalize()
            .map_err(WorkspaceManagerError::Io)?;
        let workspace_folder_path_str = canonical_workspace_folder_path
            .to_string_lossy()
            .to_string();

        info!("Discovering workspace: {workspace_folder_path_str}");

        let gitalisk_workspace = Arc::new(CoreGitaliskWorkspaceFolder::new(
            workspace_folder_path_str.clone(),
        ));
        let stats = gitalisk_workspace
            .index_repositories()
            .map_err(|e| WorkspaceManagerError::Io(std::io::Error::other(e)))?;

        info!(
            "Found {} repositories with {} files in workspace",
            stats.repo_count, stats.file_count
        );

        let repositories = gitalisk_workspace.get_repositories();
        let mut projects_found = Vec::with_capacity(repositories.len());

        let workspace_hash = generate_path_hash(&workspace_folder_path_str);
        let mut workspace_metadata = WorkspaceFolderMetadata::new(workspace_hash.clone());

        for repository in &repositories {
            let project_path = repository.path.clone();
            let project_hash = generate_path_hash(&project_path);
            let project_metadata = ProjectMetadata::new(project_hash.clone());

            workspace_metadata.add_project(project_path.clone(), project_metadata);
        }

        workspace_metadata.update_status_from_projects();

        self.state_service.add_workspace_folder(
            workspace_folder_path_str.clone(),
            workspace_metadata.clone(),
        )?;

        self.data_directory
            .ensure_workspace_folder_directory(&workspace_hash)?;

        for repository in repositories {
            let project_path = repository.path.clone();
            let project_hash = generate_path_hash(&project_path);
            let project_metadata = workspace_metadata.projects.get(&project_path).unwrap();

            let project_info = self.register_project_internal(
                &workspace_folder_path_str,
                &workspace_metadata,
                project_path,
                project_hash,
                project_metadata,
                repository,
            )?;
            projects_found.push(project_info);
        }

        {
            let mut workspaces = self.gitalisk_workspaces.write().unwrap();
            workspaces.insert(
                workspace_folder_path_str.clone(),
                gitalisk_workspace.clone(),
            );
        }

        Ok(WorkspaceFolderInfo {
            workspace_folder_path: workspace_folder_path_str,
            data_directory_name: workspace_metadata.data_directory_name.clone(),
            status: workspace_metadata.status.clone(),
            last_indexed_at: workspace_metadata.last_indexed_at,
            project_count: workspace_metadata.project_count(),
            gitalisk_workspace: Some(gitalisk_workspace),
        })
    }

    pub fn register_project(
        &self,
        workspace_folder_path: &str,
        project_path: &str,
    ) -> Result<ProjectInfo> {
        let canonical_project_path = PathBuf::from(project_path)
            .canonicalize()
            .map_err(WorkspaceManagerError::Io)?;
        let project_path_str = canonical_project_path.to_string_lossy().to_string();

        if !self
            .state_service
            .has_workspace_folder(workspace_folder_path)
        {
            return Err(WorkspaceManagerError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Workspace not found: {workspace_folder_path}"),
            )));
        }

        let project_hash = generate_path_hash(&project_path_str);
        let project_metadata = ProjectMetadata::new(project_hash.clone());

        self.state_service.add_project(
            workspace_folder_path,
            project_path_str.clone(),
            project_metadata.clone(),
        )?;

        let workspace_metadata = self
            .state_service
            .get_workspace_folder(workspace_folder_path)
            .ok_or_else(|| {
                WorkspaceManagerError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Workspace not found",
                ))
            })?;

        self.ensure_workspace_loaded(workspace_folder_path)?;
        let repository =
            self.get_repository_for_project(workspace_folder_path, &project_path_str)?;

        self.register_project_internal(
            workspace_folder_path,
            &workspace_metadata,
            project_path_str,
            project_hash,
            &project_metadata,
            repository,
        )
    }

    fn get_repository_for_project(
        &self,
        workspace_folder_path: &str,
        project_path: &str,
    ) -> Result<CoreGitaliskRepository> {
        let workspaces = self.gitalisk_workspaces.read().unwrap();
        let workspace = workspaces.get(workspace_folder_path).ok_or_else(|| {
            WorkspaceManagerError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Workspace not loaded",
            ))
        })?;

        workspace
            .get_repositories()
            .into_iter()
            .find(|repo| repo.path == project_path)
            .ok_or_else(|| {
                WorkspaceManagerError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Repository not found for project: {project_path}"),
                ))
            })
    }

    fn ensure_workspace_loaded(&self, workspace_folder_path: &str) -> Result<()> {
        {
            let workspaces = self.gitalisk_workspaces.read().unwrap();
            if workspaces.contains_key(workspace_folder_path) {
                return Ok(());
            }
        }

        let gitalisk_workspace = Arc::new(CoreGitaliskWorkspaceFolder::new(
            workspace_folder_path.to_string(),
        ));
        let _ = gitalisk_workspace
            .index_repositories()
            .map_err(|e| WorkspaceManagerError::Io(std::io::Error::other(e)))?;

        {
            let mut workspaces = self.gitalisk_workspaces.write().unwrap();
            workspaces.insert(workspace_folder_path.to_string(), gitalisk_workspace);
        }

        Ok(())
    }

    pub fn get_workspace_folder_info(
        &self,
        workspace_folder_path: &str,
    ) -> Option<WorkspaceFolderInfo> {
        self.state_service
            .get_workspace_folder(workspace_folder_path)
            .map(|metadata| WorkspaceFolderInfo {
                workspace_folder_path: workspace_folder_path.to_string(),
                data_directory_name: metadata.data_directory_name.clone(),
                status: metadata.status.clone(),
                last_indexed_at: metadata.last_indexed_at,
                project_count: metadata.project_count(),
                gitalisk_workspace: self
                    .gitalisk_workspaces
                    .read()
                    .unwrap()
                    .get(workspace_folder_path)
                    .cloned(),
            })
    }

    pub fn get_project_info(
        &self,
        workspace_folder_path: &str,
        project_path: &str,
    ) -> Option<ProjectInfo> {
        let workspace_metadata = self
            .state_service
            .get_workspace_folder(workspace_folder_path)?;
        let project_metadata = workspace_metadata.get_project(project_path)?;

        let _ = self.ensure_workspace_loaded(workspace_folder_path);
        let repository = self
            .get_repository_for_project(workspace_folder_path, project_path)
            .ok()?;

        self.register_project_internal(
            workspace_folder_path,
            &workspace_metadata,
            project_path.to_string(),
            project_metadata.project_hash.clone(),
            project_metadata,
            repository,
        )
        .ok()
    }

    pub fn get_project_for_path(&self, project_path: &str) -> Option<ProjectInfo> {
        if let Some((workspace_folder_path, _project_metadata)) =
            self.state_service.find_project(project_path)
        {
            self.get_project_info(&workspace_folder_path, project_path)
        } else {
            None
        }
    }

    pub fn list_workspace_folders(&self) -> Vec<WorkspaceFolderInfo> {
        self.state_service.with_manifest(|manifest| {
            manifest
                .workspace_folders()
                .iter()
                .map(|(workspace_folder_path, metadata)| WorkspaceFolderInfo {
                    workspace_folder_path: workspace_folder_path.clone(),
                    data_directory_name: metadata.data_directory_name.clone(),
                    status: metadata.status.clone(),
                    last_indexed_at: metadata.last_indexed_at,
                    project_count: metadata.project_count(),
                    gitalisk_workspace: self
                        .gitalisk_workspaces
                        .read()
                        .unwrap()
                        .get(workspace_folder_path)
                        .cloned(),
                })
                .collect()
        })
    }

    pub fn list_all_projects(&self) -> Vec<ProjectInfo> {
        let mut project_infos = Vec::new();

        self.state_service.with_manifest(|manifest| {
            let workspace_folders = manifest.workspace_folders();

            for (workspace_folder_path, workspace_metadata) in workspace_folders {
                let _ = self.ensure_workspace_loaded(workspace_folder_path);

                for (project_path, project_metadata) in &workspace_metadata.projects {
                    if let Ok(repository) =
                        self.get_repository_for_project(workspace_folder_path, project_path)
                        && let Ok(project_info) = self.register_project_internal(
                            workspace_folder_path,
                            workspace_metadata,
                            project_path.clone(),
                            project_metadata.project_hash.clone(),
                            project_metadata,
                            repository,
                        )
                    {
                        project_infos.push(project_info);
                    }
                }
            }
        });

        project_infos
    }

    pub fn list_projects_in_workspace(&self, workspace_folder_path: &str) -> Vec<ProjectInfo> {
        let workspace_metadata = match self
            .state_service
            .get_workspace_folder(workspace_folder_path)
        {
            Some(metadata) => metadata,
            None => return Vec::new(),
        };

        let mut project_infos = Vec::new();
        let _ = self.ensure_workspace_loaded(workspace_folder_path);

        for (project_path, project_metadata) in &workspace_metadata.projects {
            if let Ok(repository) =
                self.get_repository_for_project(workspace_folder_path, project_path)
                && let Ok(project_info) = self.register_project_internal(
                    workspace_folder_path,
                    &workspace_metadata,
                    project_path.clone(),
                    project_metadata.project_hash.clone(),
                    project_metadata,
                    repository,
                )
            {
                project_infos.push(project_info);
            }
        }

        project_infos
    }

    pub fn update_project_indexing_status(
        &self,
        workspace_folder_path: &str,
        project_path: &str,
        status: Status,
        status_error_message: Option<String>,
    ) -> Result<ProjectInfo> {
        self.state_service
            .update_project(workspace_folder_path, project_path, |project| {
                *project = project.clone().mark_status(status, status_error_message);
            })?;

        self.get_project_info(workspace_folder_path, project_path)
            .ok_or_else(|| {
                WorkspaceManagerError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Project not found",
                ))
            })
    }

    pub fn remove_workspace_folder(&self, workspace_folder_path: &str) -> Result<bool> {
        let workspace_metadata = match self
            .state_service
            .get_workspace_folder(workspace_folder_path)
        {
            Some(metadata) => metadata,
            None => return Ok(false),
        };

        self.data_directory
            .remove_workspace_folder_directory(&workspace_metadata.data_directory_name)?;

        let removed = self
            .state_service
            .remove_workspace_folder(workspace_folder_path)?;

        {
            let mut workspaces = self.gitalisk_workspaces.write().unwrap();
            workspaces.remove(workspace_folder_path);
        }

        if removed.is_some() {
            info!("Removed workspace folder: {workspace_folder_path}");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn remove_project(&self, workspace_folder_path: &str, project_path: &str) -> Result<bool> {
        let workspace_metadata = match self
            .state_service
            .get_workspace_folder(workspace_folder_path)
        {
            Some(metadata) => metadata,
            None => return Ok(false),
        };

        let project_metadata = match workspace_metadata.get_project(project_path) {
            Some(metadata) => metadata,
            None => return Ok(false),
        };

        self.data_directory.remove_project_directory(
            &workspace_metadata.data_directory_name,
            &project_metadata.project_hash,
        )?;

        let removed = self
            .state_service
            .remove_project(workspace_folder_path, project_path)?;

        self.update_workspace_folder_status(workspace_folder_path, None)?;

        if removed.is_some() {
            info!("Removed project: {project_path} from workspace: {workspace_folder_path}");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn update_workspace_folder_status(
        &self,
        workspace_folder_path: &str,
        status: Option<Status>,
    ) -> Result<WorkspaceFolderInfo> {
        self.state_service
            .update_workspace_folder(workspace_folder_path, |workspace_folder| {
                if let Some(status) = status {
                    workspace_folder.status = status;
                } else {
                    workspace_folder.update_status_from_projects();
                }
            })?;

        self.get_workspace_folder_info(workspace_folder_path)
            .ok_or_else(|| {
                WorkspaceManagerError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Workspace not found",
                ))
            })
    }

    pub fn get_workspace_folder_size(&self, workspace_folder_path: &str) -> Result<u64> {
        let workspace_metadata = self
            .state_service
            .get_workspace_folder(workspace_folder_path)
            .ok_or_else(|| {
                WorkspaceManagerError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Workspace not found",
                ))
            })?;

        self.data_directory
            .get_workspace_folder_directory_size(&workspace_metadata.data_directory_name)
    }

    pub fn get_project_size(&self, workspace_folder_path: &str, project_path: &str) -> Result<u64> {
        let workspace_metadata = self
            .state_service
            .get_workspace_folder(workspace_folder_path)
            .ok_or_else(|| {
                WorkspaceManagerError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Workspace not found",
                ))
            })?;

        let project_metadata = workspace_metadata
            .get_project(project_path)
            .ok_or_else(|| {
                WorkspaceManagerError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Project not found",
                ))
            })?;

        self.data_directory.get_project_directory_size(
            &workspace_metadata.data_directory_name,
            &project_metadata.project_hash,
        )
    }

    pub fn get_data_directory_info(
        &self,
    ) -> Result<crate::data_directory::WorkspaceFolderDataDirectoryInfo> {
        self.data_directory.get_info()
    }

    pub fn get_framework_version(&self) -> Result<String> {
        Ok(self
            .state_service
            .with_manifest(|manifest| manifest.framework_version.clone()))
    }

    pub fn get_or_register_workspace_folder(
        &self,
        workspace_folder_path: &Path,
    ) -> Result<WorkspaceFolderInfo> {
        let canonical_path = workspace_folder_path
            .canonicalize()
            .map_err(WorkspaceManagerError::Io)?
            .to_string_lossy()
            .to_string();

        if let Some(info) = self.get_workspace_folder_info(&canonical_path) {
            return Ok(info);
        }

        self.register_workspace_folder(workspace_folder_path)
    }

    pub fn clean(&self) -> Result<()> {
        let workspace_folders_dir = &self.data_directory.workspace_folders_dir;
        if workspace_folders_dir.exists() {
            fs::remove_dir_all(workspace_folders_dir)?;
            info!(
                "Removed workspace folders directory: {}",
                workspace_folders_dir.display()
            );
        }

        let manifest_path = self.state_service.manifest_path().to_path_buf();
        if manifest_path.exists() {
            fs::remove_file(&manifest_path)?;
            info!("Removed manifest file: {}", manifest_path.display());
        }

        // Clear the workspaces in case the function is called in stateful context (server)
        {
            let mut workspaces = self.gitalisk_workspaces.write().unwrap();
            workspaces.clear();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_git_repo(path: &Path) {
        fs::create_dir_all(path).unwrap();

        std::process::Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .unwrap();

        fs::write(path.join("README.md"), "# Test Repo").unwrap();
        fs::write(path.join("main.rb"), "puts 'Hello, World!'").unwrap();

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    #[test]
    fn test_workspace_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = WorkspaceManager::new_with_directory(temp_dir.path().to_path_buf()).unwrap();

        let info = manager.get_data_directory_info().unwrap();
        assert_eq!(info.workspace_folder_count, 0);
    }

    #[test]
    fn test_workspace_manager_with_dependencies() {
        let temp_dir = TempDir::new().unwrap();

        let data_directory = DataDirectory::new(temp_dir.path().to_path_buf()).unwrap();
        let state_service =
            LocalStateService::new(&data_directory.manifest_path, FRAMEWORK_VERSION.to_string())
                .unwrap();

        let manager = WorkspaceManager::new(data_directory, state_service);

        let info = manager.get_data_directory_info().unwrap();
        assert_eq!(info.workspace_folder_count, 0);
    }

    #[test]
    fn test_discover_and_register_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_folder_path = temp_dir.path().join("test_workspace");
        fs::create_dir_all(&workspace_folder_path).unwrap();

        let repo1_path = workspace_folder_path.join("repo1");
        let repo2_path = workspace_folder_path.join("repo2");
        create_test_git_repo(&repo1_path);
        create_test_git_repo(&repo2_path);

        let data_dir = TempDir::new().unwrap();
        let manager = WorkspaceManager::new_with_directory(data_dir.path().to_path_buf()).unwrap();

        let result = manager
            .register_workspace_folder(&workspace_folder_path)
            .unwrap();

        assert_eq!(result.project_count, 2);

        let projects = manager.list_projects_in_workspace(&result.workspace_folder_path);
        assert_eq!(projects.len(), 2);

        let workspace_info = manager.get_workspace_folder_info(&result.workspace_folder_path);
        assert!(workspace_info.is_some());
        assert_eq!(workspace_info.unwrap().project_count, 2);
    }

    #[test]
    fn test_project_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_folder_path = temp_dir.path().join("test_workspace");
        let project_path = workspace_folder_path.join("test_project");

        fs::create_dir_all(&workspace_folder_path).unwrap();
        create_test_git_repo(&project_path);

        let data_dir = TempDir::new().unwrap();
        let manager = WorkspaceManager::new_with_directory(data_dir.path().to_path_buf()).unwrap();

        let result = manager
            .register_workspace_folder(&workspace_folder_path)
            .unwrap();
        let workspace_folder_path_str = result.workspace_folder_path;

        let projects = manager.list_projects_in_workspace(&workspace_folder_path_str);
        assert!(!projects.is_empty());
        let project_path_str = projects[0].project_path.clone();

        let project_info = manager.get_project_info(&workspace_folder_path_str, &project_path_str);
        assert!(project_info.is_some());
        assert_eq!(project_info.as_ref().unwrap().status, Status::Pending);

        let updated_project = manager
            .update_project_indexing_status(
                &workspace_folder_path_str,
                &project_path_str,
                Status::Indexing,
                None,
            )
            .unwrap();
        assert_eq!(updated_project.status, Status::Indexing);

        let project_info = manager
            .get_project_info(&workspace_folder_path_str, &project_path_str)
            .unwrap();
        assert_eq!(project_info.status, Status::Indexing);

        let updated_project = manager
            .update_project_indexing_status(
                &workspace_folder_path_str,
                &project_path_str,
                Status::Indexed,
                None,
            )
            .unwrap();
        assert_eq!(updated_project.status, Status::Indexed);

        let project_info = manager
            .get_project_info(&workspace_folder_path_str, &project_path_str)
            .unwrap();
        assert_eq!(project_info.status, Status::Indexed);
        assert!(project_info.last_indexed_at.is_some());

        let updated_project = manager
            .update_project_indexing_status(
                &workspace_folder_path_str,
                &project_path_str,
                Status::Error,
                Some("Test error".to_string()),
            )
            .unwrap();
        assert_eq!(updated_project.status, Status::Error);
        assert_eq!(
            updated_project.error_message,
            Some("Test error".to_string())
        );

        let project_info = manager
            .get_project_info(&workspace_folder_path_str, &project_path_str)
            .unwrap();
        assert_eq!(project_info.status, Status::Error);
        assert_eq!(project_info.error_message, Some("Test error".to_string()));

        let framework_version = manager.get_framework_version().unwrap();
        assert_eq!(framework_version, FRAMEWORK_VERSION);
    }

    #[test]
    fn test_list_operations() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_folder_path = temp_dir.path().join("test_workspace");
        fs::create_dir_all(&workspace_folder_path).unwrap();

        let repo_path = workspace_folder_path.join("test_repo");
        create_test_git_repo(&repo_path);

        let data_dir = TempDir::new().unwrap();
        let manager = WorkspaceManager::new_with_directory(data_dir.path().to_path_buf()).unwrap();

        let result = manager
            .register_workspace_folder(&workspace_folder_path)
            .unwrap();

        let workspaces = manager.list_workspace_folders();
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].project_count, 1);

        let all_projects = manager.list_all_projects();
        assert_eq!(all_projects.len(), 1);

        let workspace_projects = manager.list_projects_in_workspace(&result.workspace_folder_path);
        assert_eq!(workspace_projects.len(), 1);
    }

    #[test]
    fn test_removal_operations() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_folder_path = temp_dir.path().join("test_workspace");
        fs::create_dir_all(&workspace_folder_path).unwrap();

        let repo_path = workspace_folder_path.join("test_repo");
        create_test_git_repo(&repo_path);

        let data_dir = TempDir::new().unwrap();
        let manager = WorkspaceManager::new_with_directory(data_dir.path().to_path_buf()).unwrap();

        let result = manager
            .register_workspace_folder(&workspace_folder_path)
            .unwrap();
        let workspace_folder_path_str = result.workspace_folder_path;

        let projects = manager.list_projects_in_workspace(&workspace_folder_path_str);
        assert!(!projects.is_empty());
        let project_path_str = projects[0].project_path.clone();

        let removed = manager
            .remove_project(&workspace_folder_path_str, &project_path_str)
            .unwrap();
        assert!(removed);

        let workspace_projects = manager.list_projects_in_workspace(&workspace_folder_path_str);
        assert_eq!(workspace_projects.len(), 0);

        let removed = manager
            .remove_workspace_folder(&workspace_folder_path_str)
            .unwrap();
        assert!(removed);

        let workspaces = manager.list_workspace_folders();
        assert_eq!(workspaces.len(), 0);
    }

    /// Test concurrent operations for tokio server thread safety
    /// Validates: concurrent reads/writes, workspace reloading, and data integrity
    #[test]
    fn test_concurrent_operations() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_folder_path = temp_dir.path().join("test_workspace");
        fs::create_dir_all(&workspace_folder_path).unwrap();

        let repo1_path = workspace_folder_path.join("repo1");
        let repo2_path = workspace_folder_path.join("repo2");
        create_test_git_repo(&repo1_path);
        create_test_git_repo(&repo2_path);

        let data_dir = TempDir::new().unwrap();
        let manager = WorkspaceManager::new_with_directory(data_dir.path().to_path_buf()).unwrap();

        let result = manager
            .register_workspace_folder(&workspace_folder_path)
            .unwrap();
        let workspace_path = result.workspace_folder_path.clone();
        let projects = manager.list_projects_in_workspace(&workspace_path);
        let project_paths: Vec<String> = projects.iter().map(|p| p.project_path.clone()).collect();

        // Test 1: Concurrent reads + writes (simulates tokio server load)
        let handles: Vec<std::thread::JoinHandle<()>> = (0..20)
            .map(|i| {
                let manager_clone = manager.clone();
                let workspace_path = workspace_path.clone();
                let project_path = project_paths[i % project_paths.len()].clone();

                std::thread::spawn(move || {
                    match i % 4 {
                        0 => {
                            // Read operations
                            let _project_info =
                                manager_clone.get_project_info(&workspace_path, &project_path);
                            let _all_projects = manager_clone.list_all_projects();
                        }
                        1 => {
                            // Status updates
                            let _ = manager_clone.update_project_indexing_status(
                                &workspace_path,
                                &project_path,
                                Status::Indexing,
                                None,
                            );
                        }
                        2 => {
                            // Repository access
                            if let Some(project) =
                                manager_clone.get_project_info(&workspace_path, &project_path)
                            {
                                let _ = project.repository.get_current_branch();
                            }
                        }
                        3 => {
                            // More read operations
                            let _ = manager_clone.get_workspace_folder_info(&workspace_path);
                            let _ = manager_clone.get_project_for_path(&project_path);
                        }
                        _ => unreachable!(),
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Test 2: Workspace reloading under concurrency
        {
            let mut workspaces = manager.gitalisk_workspaces.write().unwrap();
            workspaces.clear(); // Force reload
        }

        let handles: Vec<std::thread::JoinHandle<()>> = (0..10)
            .map(|_| {
                let manager_clone = manager.clone();
                let workspace_path = workspace_path.clone();
                let project_path = project_paths[0].clone();

                std::thread::spawn(move || {
                    // Should trigger safe workspace reloading
                    let _project_info =
                        manager_clone.get_project_info(&workspace_path, &project_path);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify data integrity after concurrent operations
        let final_projects = manager.list_all_projects();
        assert_eq!(final_projects.len(), 2);

        for project in &final_projects {
            assert!(!project.project_path.is_empty());
            let _ = project.repository.get_current_branch(); // Repository should be accessible
        }

        // Verify workspace is still loaded correctly
        let workspace_info = manager.get_workspace_folder_info(&workspace_path).unwrap();
        assert_eq!(workspace_info.project_count, 2);
    }

    #[test]
    fn test_workspace_status_aggregation_integration() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_folder_path = temp_dir.path().join("test_workspace");
        fs::create_dir_all(&workspace_folder_path).unwrap();

        let repo1_path = workspace_folder_path.join("repo1");
        let repo2_path = workspace_folder_path.join("repo2");
        create_test_git_repo(&repo1_path);
        create_test_git_repo(&repo2_path);

        let data_dir = TempDir::new().unwrap();
        let manager = WorkspaceManager::new_with_directory(data_dir.path().to_path_buf()).unwrap();

        // Initial registration - workspace should be pending with no timestamp
        let result = manager
            .register_workspace_folder(&workspace_folder_path)
            .unwrap();
        let workspace_path = result.workspace_folder_path.clone();
        let projects = manager.list_projects_in_workspace(&workspace_path);
        let project_paths: Vec<String> = projects.iter().map(|p| p.project_path.clone()).collect();

        // Verify initial state
        let workspace_info = manager.get_workspace_folder_info(&workspace_path).unwrap();
        assert_eq!(workspace_info.status, Status::Pending);
        assert!(workspace_info.last_indexed_at.is_none());

        // Mark first project as indexing
        manager
            .update_project_indexing_status(
                &workspace_path,
                &project_paths[0],
                Status::Indexing,
                None,
            )
            .unwrap();

        let workspace_info = manager.get_workspace_folder_info(&workspace_path).unwrap();
        assert_eq!(workspace_info.status, Status::Indexing);
        assert!(workspace_info.last_indexed_at.is_none());

        // Mark first project as indexed
        manager
            .update_project_indexing_status(
                &workspace_path,
                &project_paths[0],
                Status::Indexed,
                None,
            )
            .unwrap();

        let workspace_info = manager.get_workspace_folder_info(&workspace_path).unwrap();
        assert_eq!(workspace_info.status, Status::Pending); // Still pending because project2 is pending
        assert!(workspace_info.last_indexed_at.is_some()); // Should have timestamp from project1

        // Mark second project as indexed
        manager
            .update_project_indexing_status(
                &workspace_path,
                &project_paths[1],
                Status::Indexed,
                None,
            )
            .unwrap();

        let workspace_info = manager.get_workspace_folder_info(&workspace_path).unwrap();
        assert_eq!(workspace_info.status, Status::Indexed); // Now all indexed
        assert!(workspace_info.last_indexed_at.is_some()); // Should have latest timestamp

        // Mark one project as error
        manager
            .update_project_indexing_status(
                &workspace_path,
                &project_paths[0],
                Status::Error,
                Some("Test error".to_string()),
            )
            .unwrap();

        let workspace_info = manager.get_workspace_folder_info(&workspace_path).unwrap();
        assert_eq!(workspace_info.status, Status::Error); // Should be error now
        assert!(workspace_info.last_indexed_at.is_some()); // Should keep timestamp

        // Verify projects have correct individual states
        let project1 = manager
            .get_project_info(&workspace_path, &project_paths[0])
            .unwrap();
        assert_eq!(project1.status, Status::Error);
        assert_eq!(project1.error_message, Some("Test error".to_string()));

        let project2 = manager
            .get_project_info(&workspace_path, &project_paths[1])
            .unwrap();
        assert_eq!(project2.status, Status::Indexed);
        assert!(project2.last_indexed_at.is_some());
    }
}
