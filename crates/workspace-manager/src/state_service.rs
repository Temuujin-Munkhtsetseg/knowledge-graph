use crate::errors::{Result, WorkspaceManagerError};
use crate::manifest::Manifest;
use log::{debug, info, warn};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Service for managing the local state manifest JSON file
/// Provides thread-safe access to the manifest with optimized I/O operations
#[derive(Debug)]
pub struct LocalStateService {
    manifest_path: PathBuf,
    manifest: Arc<RwLock<Manifest>>,
}

impl LocalStateService {
    /// Create a new LocalStateService with the given manifest file path
    pub fn new(manifest_path: impl Into<PathBuf>, framework_version: String) -> Result<Self> {
        let manifest_path = manifest_path.into();
        let service = Self {
            manifest_path,
            manifest: Arc::new(RwLock::new(Manifest::new(framework_version))),
        };

        if service.manifest_path.exists() {
            service.load_manifest()?;
        } else {
            if let Some(parent) = service.manifest_path.parent() {
                fs::create_dir_all(parent).map_err(WorkspaceManagerError::Io)?;
            }
            service.save_manifest()?;
        }

        Ok(service)
    }

    fn load_manifest(&self) -> Result<()> {
        debug!("Loading manifest from: {}", self.manifest_path.display());

        let content = fs::read_to_string(&self.manifest_path)?;
        let loaded_manifest: Manifest = serde_json::from_str(&content)?;

        {
            let mut manifest = self.manifest.write().unwrap();
            *manifest = loaded_manifest;
        }

        info!(
            "Loaded manifest with {} workspace folders",
            self.get_workspace_folder_count()
        );
        Ok(())
    }

    fn save_manifest(&self) -> Result<()> {
        debug!("Saving manifest to: {}", self.manifest_path.display());

        let content = {
            let manifest = self.manifest.read().unwrap();
            serde_json::to_string_pretty(&*manifest)?
        };

        let temp_path = self.manifest_path.with_extension("tmp");
        fs::write(&temp_path, content)?;
        fs::rename(&temp_path, &self.manifest_path)?;

        debug!("Manifest saved successfully");
        Ok(())
    }

    pub fn with_manifest<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Manifest) -> R,
    {
        let manifest = self.manifest.read().unwrap();
        f(&manifest)
    }

    pub fn with_manifest_mut<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Manifest) -> R,
    {
        let result = {
            let mut manifest = self.manifest.write().unwrap();
            f(&mut manifest)
        };

        self.save_manifest()?;
        Ok(result)
    }

    pub fn get_workspace_folder_count(&self) -> usize {
        self.with_manifest(|manifest| manifest.workspace_folder_count())
    }

    pub fn get_workspace_folder_paths(&self) -> Vec<String> {
        self.with_manifest(|manifest| {
            manifest
                .workspace_folder_paths()
                .into_iter()
                .cloned()
                .collect()
        })
    }

    pub fn has_workspace_folder(&self, workspace_path: &str) -> bool {
        self.with_manifest(|manifest| manifest.get_workspace_folder(workspace_path).is_some())
    }

    pub fn get_workspace_folder(
        &self,
        workspace_path: &str,
    ) -> Option<crate::manifest::WorkspaceFolderMetadata> {
        self.with_manifest(|manifest| manifest.get_workspace_folder(workspace_path).cloned())
    }

    pub fn add_workspace_folder(
        &self,
        workspace_path: String,
        metadata: crate::manifest::WorkspaceFolderMetadata,
    ) -> Result<()> {
        self.with_manifest_mut(|manifest| {
            manifest.add_workspace_folder(workspace_path, metadata);
        })
    }

    pub fn remove_workspace_folder(
        &self,
        workspace_path: &str,
    ) -> Result<Option<crate::manifest::WorkspaceFolderMetadata>> {
        self.with_manifest_mut(|manifest| manifest.remove_workspace_folder(workspace_path))
    }

    pub fn add_project(
        &self,
        workspace_path: &str,
        project_path: String,
        metadata: crate::manifest::ProjectMetadata,
    ) -> Result<()> {
        self.with_manifest_mut(|manifest| {
            if let Some(workspace_metadata) = manifest.get_workspace_folder_mut(workspace_path) {
                workspace_metadata.add_project(project_path, metadata);
                workspace_metadata.update_status_from_projects();
            }
        })
    }

    pub fn remove_project(
        &self,
        workspace_path: &str,
        project_path: &str,
    ) -> Result<Option<crate::manifest::ProjectMetadata>> {
        self.with_manifest_mut(|manifest| {
            if let Some(workspace_metadata) = manifest.get_workspace_folder_mut(workspace_path) {
                let result = workspace_metadata.remove_project(project_path);
                workspace_metadata.update_status_from_projects();
                result
            } else {
                None
            }
        })
    }

    pub fn get_project(
        &self,
        workspace_path: &str,
        project_path: &str,
    ) -> Option<crate::manifest::ProjectMetadata> {
        self.with_manifest(|manifest| {
            manifest
                .get_workspace_folder(workspace_path)
                .and_then(|workspace| workspace.get_project(project_path))
                .cloned()
        })
    }

    pub fn update_project<F>(&self, workspace_path: &str, project_path: &str, f: F) -> Result<bool>
    where
        F: FnOnce(&mut crate::manifest::ProjectMetadata),
    {
        self.with_manifest_mut(|manifest| {
            if let Some(workspace_metadata) = manifest.get_workspace_folder_mut(workspace_path) {
                if let Some(project_metadata) = workspace_metadata.get_project_mut(project_path) {
                    f(project_metadata);
                    workspace_metadata.update_status_from_projects();
                    return true;
                }
            }
            false
        })
    }

    pub fn get_all_projects(&self) -> Vec<(String, String, crate::manifest::ProjectMetadata)> {
        self.with_manifest(|manifest| {
            let workspace_folders = manifest.workspace_folders();
            let total_projects: usize = workspace_folders.values().map(|w| w.project_count()).sum();
            let mut projects = Vec::with_capacity(total_projects);

            for (workspace_path, workspace_metadata) in workspace_folders {
                for (project_path, project_metadata) in &workspace_metadata.projects {
                    projects.push((
                        workspace_path.clone(),
                        project_path.clone(),
                        project_metadata.clone(),
                    ));
                }
            }

            projects
        })
    }

    pub fn find_project(
        &self,
        project_path: &str,
    ) -> Option<(String, crate::manifest::ProjectMetadata)> {
        self.with_manifest(|manifest| {
            manifest
                .find_project(project_path)
                .map(|(workspace_path, project_metadata)| {
                    (workspace_path.to_string(), project_metadata.clone())
                })
        })
    }

    pub fn reload(&self) -> Result<()> {
        if self.manifest_path.exists() {
            self.load_manifest()
        } else {
            warn!(
                "Manifest file not found during reload: {}",
                self.manifest_path.display()
            );
            Ok(())
        }
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn manifest_exists(&self) -> bool {
        self.manifest_path.exists()
    }

    pub fn create_backup(&self) -> Result<PathBuf> {
        let backup_path = self.manifest_path.with_extension("backup");
        fs::copy(&self.manifest_path, &backup_path)?;
        info!("Created manifest backup at: {}", backup_path.display());
        Ok(backup_path)
    }

    pub fn restore_from_backup(&self, backup_path: &Path) -> Result<()> {
        if !backup_path.exists() {
            return Err(WorkspaceManagerError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Backup file not found: {}", backup_path.display()),
            )));
        }

        fs::copy(backup_path, &self.manifest_path)?;
        self.load_manifest()?;
        info!("Restored manifest from backup: {}", backup_path.display());
        Ok(())
    }
}

impl Clone for LocalStateService {
    fn clone(&self) -> Self {
        Self {
            manifest_path: self.manifest_path.clone(),
            manifest: Arc::clone(&self.manifest),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ProjectMetadata, Status, WorkspaceFolderMetadata};
    use tempfile::TempDir;

    #[test]
    fn test_local_state_service_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("test_manifest.json");

        let service = LocalStateService::new(manifest_path.clone(), "0.1.0".to_string()).unwrap();

        assert!(manifest_path.exists());
        assert_eq!(service.get_workspace_folder_count(), 0);
    }

    #[test]
    fn test_workspace_folder_operations() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("test_manifest.json");
        let service = LocalStateService::new(manifest_path, "0.1.0".to_string()).unwrap();

        let workspace_metadata = WorkspaceFolderMetadata::new("test_hash".to_string());
        let workspace_path = "/path/to/workspace".to_string();

        service
            .add_workspace_folder(workspace_path.clone(), workspace_metadata)
            .unwrap();
        assert_eq!(service.get_workspace_folder_count(), 1);
        assert!(service.has_workspace_folder(&workspace_path));

        let retrieved = service.get_workspace_folder(&workspace_path);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data_directory_name, "test_hash");

        let removed = service.remove_workspace_folder(&workspace_path).unwrap();
        assert!(removed.is_some());
        assert_eq!(service.get_workspace_folder_count(), 0);
    }

    #[test]
    fn test_project_operations() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("test_manifest.json");
        let service = LocalStateService::new(manifest_path, "0.1.0".to_string()).unwrap();

        let workspace_metadata = WorkspaceFolderMetadata::new("workspace_hash".to_string());
        let workspace_path = "/path/to/workspace".to_string();
        let project_path = "/path/to/project".to_string();

        service
            .add_workspace_folder(workspace_path.clone(), workspace_metadata)
            .unwrap();

        let project_metadata = ProjectMetadata::new("project_hash".to_string());
        service
            .add_project(&workspace_path, project_path.clone(), project_metadata)
            .unwrap();

        let retrieved = service.get_project(&workspace_path, &project_path);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().project_hash, "project_hash");

        let updated = service
            .update_project(&workspace_path, &project_path, |project| {
                project.status = Status::Indexed;
            })
            .unwrap();
        assert!(updated);

        let updated_project = service.get_project(&workspace_path, &project_path).unwrap();
        assert_eq!(updated_project.status, Status::Indexed);

        let removed = service
            .remove_project(&workspace_path, &project_path)
            .unwrap();
        assert!(removed.is_some());
        assert!(
            service
                .get_project(&workspace_path, &project_path)
                .is_none()
        );
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("test_manifest.json");

        {
            let service =
                LocalStateService::new(manifest_path.clone(), "0.1.0".to_string()).unwrap();
            let workspace_metadata = WorkspaceFolderMetadata::new("test_hash".to_string());
            service
                .add_workspace_folder("/test/workspace".to_string(), workspace_metadata)
                .unwrap();
        }

        {
            let service =
                LocalStateService::new(manifest_path.clone(), "0.1.0".to_string()).unwrap();
            assert_eq!(service.get_workspace_folder_count(), 1);
            assert!(service.has_workspace_folder("/test/workspace"));
        }

        {
            let service =
                LocalStateService::new(manifest_path.clone(), "0.1.0".to_string()).unwrap();
            assert_eq!(
                service.with_manifest(|manifest| manifest.framework_version.clone()),
                "0.1.0".to_string()
            );
        }
    }

    #[test]
    fn test_find_project() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("test_manifest.json");
        let service = LocalStateService::new(manifest_path, "0.1.0".to_string()).unwrap();

        let workspace_metadata = WorkspaceFolderMetadata::new("workspace_hash".to_string());
        let workspace_path = "/path/to/workspace".to_string();
        let project_path = "/path/to/project".to_string();

        service
            .add_workspace_folder(workspace_path.clone(), workspace_metadata)
            .unwrap();

        let project_metadata = ProjectMetadata::new("project_hash".to_string());
        service
            .add_project(&workspace_path, project_path.clone(), project_metadata)
            .unwrap();

        let found = service.find_project(&project_path);
        assert!(found.is_some());
        assert_eq!(found.unwrap().0, workspace_path);
    }

    #[test]
    fn test_backup_and_restore() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("test_manifest.json");
        let service = LocalStateService::new(manifest_path, "0.1.0".to_string()).unwrap();

        let workspace_metadata = WorkspaceFolderMetadata::new("test_hash".to_string());
        service
            .add_workspace_folder("/test/workspace".to_string(), workspace_metadata)
            .unwrap();

        let backup_path = service.create_backup().unwrap();
        assert!(backup_path.exists());

        service.remove_workspace_folder("/test/workspace").unwrap();
        assert_eq!(service.get_workspace_folder_count(), 0);

        service.restore_from_backup(&backup_path).unwrap();
        assert_eq!(service.get_workspace_folder_count(), 1);
        assert!(service.has_workspace_folder("/test/workspace"));
    }
}
