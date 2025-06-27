use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// Status of a workspace folder or project
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Indexed,
    Indexing,
    Error,
    Pending,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Indexed => write!(f, "indexed"),
            Status::Indexing => write!(f, "indexing"),
            Status::Error => write!(f, "error"),
            Status::Pending => write!(f, "pending"),
        }
    }
}

impl Default for Status {
    fn default() -> Self {
        Self::Pending
    }
}

/// Metadata for a project within a workspace folder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Hash-based unique identifier for the project directory
    pub project_hash: String,
    /// When this project was last indexed
    pub last_indexed_at: Option<DateTime<Utc>>,
    /// Current status of the project
    pub status: Status,
    /// Error message if status is Error
    pub error_message: Option<String>,
}

impl ProjectMetadata {
    pub fn new(project_hash: String) -> Self {
        Self {
            project_hash,
            last_indexed_at: None,
            status: Status::default(),
            error_message: None,
        }
    }

    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    pub fn with_error(mut self, error_message: String) -> Self {
        self.status = Status::Error;
        self.error_message = Some(error_message);
        self
    }

    pub fn mark_indexed(mut self) -> Self {
        self.status = Status::Indexed;
        self.last_indexed_at = Some(Utc::now());
        self.error_message = None;
        self
    }

    pub fn mark_indexing(mut self) -> Self {
        self.status = Status::Indexing;
        self.error_message = None;
        self
    }
}

/// Metadata for a workspace folder containing multiple projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFolderMetadata {
    /// Hash-based unique identifier for workspace folder data directory
    pub data_directory_name: String,
    /// When this workspace folder was last indexed
    pub last_indexed_at: Option<DateTime<Utc>>,
    /// Current status of the workspace folder
    pub status: Status,
    /// Map of project paths to their metadata
    pub projects: HashMap<String, ProjectMetadata>,
}

impl WorkspaceFolderMetadata {
    pub fn new(data_directory_name: String) -> Self {
        Self {
            data_directory_name,
            last_indexed_at: None,
            status: Status::default(),
            projects: HashMap::with_capacity(8),
        }
    }

    pub fn add_project(&mut self, project_path: String, metadata: ProjectMetadata) {
        self.projects.insert(project_path, metadata);
    }

    pub fn get_project(&self, project_path: &str) -> Option<&ProjectMetadata> {
        self.projects.get(project_path)
    }

    pub fn get_project_mut(&mut self, project_path: &str) -> Option<&mut ProjectMetadata> {
        self.projects.get_mut(project_path)
    }

    pub fn remove_project(&mut self, project_path: &str) -> Option<ProjectMetadata> {
        self.projects.remove(project_path)
    }

    pub fn project_count(&self) -> usize {
        self.projects.len()
    }

    pub fn mark_indexed(&mut self) {
        self.status = Status::Indexed;
        self.last_indexed_at = Some(Utc::now());
    }

    pub fn update_status_from_projects(&mut self) {
        if self.projects.is_empty() {
            self.status = Status::Pending;
            return;
        }

        let has_error = self.projects.values().any(|p| p.status == Status::Error);
        let has_indexing = self.projects.values().any(|p| p.status == Status::Indexing);
        let all_indexed = self.projects.values().all(|p| p.status == Status::Indexed);

        self.status = if has_error {
            Status::Error
        } else if has_indexing {
            Status::Indexing
        } else if all_indexed {
            Status::Indexed
        } else {
            Status::Pending
        };
    }
}

/// Complete manifest structure representing all workspace folders and their projects
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    /// Map of workspace folder canonical paths to their metadata
    workspace_folders: HashMap<String, WorkspaceFolderMetadata>,
    /// Framework version used for migrations / updating gkg / etc.
    pub framework_version: String,
}

impl Manifest {
    pub fn new(framework_version: String) -> Self {
        Self {
            workspace_folders: HashMap::with_capacity(16),
            framework_version,
        }
    }

    pub fn add_workspace_folder(
        &mut self,
        workspace_path: String,
        metadata: WorkspaceFolderMetadata,
    ) {
        self.workspace_folders.insert(workspace_path, metadata);
    }

    pub fn get_workspace_folder(&self, workspace_path: &str) -> Option<&WorkspaceFolderMetadata> {
        self.workspace_folders.get(workspace_path)
    }

    pub fn get_workspace_folder_mut(
        &mut self,
        workspace_path: &str,
    ) -> Option<&mut WorkspaceFolderMetadata> {
        self.workspace_folders.get_mut(workspace_path)
    }

    pub fn remove_workspace_folder(
        &mut self,
        workspace_path: &str,
    ) -> Option<WorkspaceFolderMetadata> {
        self.workspace_folders.remove(workspace_path)
    }

    pub fn workspace_folder_count(&self) -> usize {
        self.workspace_folders.len()
    }

    pub fn workspace_folder_paths(&self) -> Vec<&String> {
        self.workspace_folders.keys().collect()
    }

    pub fn workspace_folders(&self) -> &HashMap<String, WorkspaceFolderMetadata> {
        &self.workspace_folders
    }

    pub fn workspace_folders_mut(&mut self) -> &mut HashMap<String, WorkspaceFolderMetadata> {
        &mut self.workspace_folders
    }

    pub fn get_all_projects(&self) -> Vec<(&str, &str, &ProjectMetadata)> {
        let total_projects: usize = self
            .workspace_folders
            .values()
            .map(|w| w.project_count())
            .sum();
        let mut projects = Vec::with_capacity(total_projects);

        for (workspace_path, workspace_metadata) in &self.workspace_folders {
            for (project_path, project_metadata) in &workspace_metadata.projects {
                projects.push((
                    workspace_path.as_str(),
                    project_path.as_str(),
                    project_metadata,
                ));
            }
        }
        projects
    }

    pub fn find_project(&self, project_path: &str) -> Option<(&str, &ProjectMetadata)> {
        for (workspace_path, workspace_metadata) in &self.workspace_folders {
            if let Some(project_metadata) = workspace_metadata.get_project(project_path) {
                return Some((workspace_path, project_metadata));
            }
        }
        None
    }

    pub fn get_workspace_for_project(
        &self,
        project_path: &str,
    ) -> Option<&WorkspaceFolderMetadata> {
        self.workspace_folders
            .values()
            .find(|&workspace_metadata| workspace_metadata.projects.contains_key(project_path))
    }
}

/// Helper function to generate a stable hash for a path
pub fn generate_path_hash(path: &str) -> String {
    use sha2::{Digest, Sha256};
    let canonical_path = PathBuf::from(path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path));

    let mut hasher = Sha256::new();
    hasher.update(canonical_path.to_string_lossy().as_bytes());
    let hash_bytes = hasher.finalize();

    hex::encode(&hash_bytes[..8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_metadata_lifecycle() {
        let mut project = ProjectMetadata::new("test_hash".to_string());

        assert_eq!(project.status, Status::Pending);
        assert_eq!(project.project_hash, "test_hash");

        project = project.mark_indexing();
        assert_eq!(project.status, Status::Indexing);

        project = project.mark_indexed();
        assert_eq!(project.status, Status::Indexed);
        assert!(project.last_indexed_at.is_some());

        project = project.with_error("Test error".to_string());
        assert_eq!(project.status, Status::Error);
        assert_eq!(project.error_message, Some("Test error".to_string()));
    }

    #[test]
    fn test_workspace_folder_metadata() {
        let mut workspace = WorkspaceFolderMetadata::new("workspace_hash".to_string());

        let project1 = ProjectMetadata::new("project1_hash".to_string()).mark_indexed();
        let project2 =
            ProjectMetadata::new("project2_hash".to_string()).with_error("Test error".to_string());

        workspace.add_project("/path/to/project1".to_string(), project1);
        workspace.add_project("/path/to/project2".to_string(), project2);

        assert_eq!(workspace.project_count(), 2);
        assert!(workspace.get_project("/path/to/project1").is_some());
        assert!(workspace.get_project("/path/to/project2").is_some());

        workspace.update_status_from_projects();
        assert_eq!(workspace.status, Status::Error);

        workspace.remove_project("/path/to/project2");
        workspace.update_status_from_projects();
        assert_eq!(workspace.status, Status::Indexed);
    }

    #[test]
    fn test_manifest_operations() {
        let mut manifest = Manifest::new("0.1.0".to_string());

        let mut workspace = WorkspaceFolderMetadata::new("workspace_hash".to_string());
        let project = ProjectMetadata::new("project_hash".to_string());
        workspace.add_project("/path/to/project".to_string(), project);

        manifest.add_workspace_folder("/path/to/workspace".to_string(), workspace);

        assert_eq!(manifest.workspace_folder_count(), 1);
        assert!(
            manifest
                .get_workspace_folder("/path/to/workspace")
                .is_some()
        );

        let all_projects = manifest.get_all_projects();
        assert_eq!(all_projects.len(), 1);
        assert_eq!(all_projects[0].0, "/path/to/workspace");
        assert_eq!(all_projects[0].1, "/path/to/project");

        let found_project = manifest.find_project("/path/to/project");
        assert!(found_project.is_some());
        assert_eq!(found_project.unwrap().0, "/path/to/workspace");
    }

    #[test]
    fn test_generate_path_hash() {
        let hash1 = generate_path_hash("/path/to/workspace");
        let hash2 = generate_path_hash("/path/to/workspace");
        let hash3 = generate_path_hash("/different/path");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 16);
    }
}
