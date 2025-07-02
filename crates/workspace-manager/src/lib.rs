//! # Workspace Manager
//!
//! A stateful project management system for the GitLab Knowledge Graph framework.
//!
//! This crate provides:
//! - Workspace folder registration and lifecycle management
//! - Centralized data directory management  
//! - State persistence for indexed workspaces
//! - Project metadata and status tracking
//! - Direct access to Gitalisk repositories for projects
//! - For more information on `gitalisk`, see the [Gitalisk](https://gitlab.com/gitlab-org/rust/gitalisk) crate
//!
//! ## Usage
//!
//! ### Simple Usage with Factory Methods
//!
//! ```rust,no_run
//! use workspace_manager::{WorkspaceManager, Status};
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a workspace manager with system default data directory
//! let manager = WorkspaceManager::new_system_default()?;
//!
//! // Register a workspace folder
//! let workspace_folder_path = Path::new("/path/to/workspace");
//! let workspace_info = manager.register_workspace_folder(workspace_folder_path)?;
//!
//! println!("Found {} projects in workspace", workspace_info.project_count);
//!
//! // List all registered projects
//! let projects = manager.list_all_projects();
//! for project in projects {
//!     println!("Project: {} (Status: {:?})", project.project_path, project.status);
//! }
//!
//! // Get projects in the workspace and mark one as being indexed
//! let workspace_projects = manager.list_projects_in_workspace(&workspace_info.workspace_folder_path);
//! if let Some(project) = workspace_projects.first() {
//!     let error_message = None;
//!     manager.update_project_indexing_status(&workspace_info.workspace_folder_path, &project.project_path, Status::Indexing, error_message)?;
//!
//!     // Access Gitalisk repository for a project
//!     let project_info = manager.get_project_info(&workspace_info.workspace_folder_path, &project.project_path)
//!         .ok_or("Project not found")?;
//!     println!("Repository Branch: {}", project_info.repository.get_current_branch().unwrap_or_else(|_| "unknown".to_string()));
//! }
//!
//! # Ok(())
//! # }
//! ```
//!
//! ### Advanced Usage with Dependency Injection
//!
//! ```rust,no_run
//! use workspace_manager::{WorkspaceManager, DataDirectory, LocalStateService};
//! use std::path::PathBuf;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create dependencies with custom configuration
//! let data_directory = DataDirectory::new(PathBuf::from("/custom/data/path"))?;
//! let state_service = LocalStateService::new(&data_directory.manifest_path, "0.1.0".to_string())?;
//!
//! // Create workspace manager with dependency injection
//! let manager = WorkspaceManager::new(data_directory, state_service);
//!
//! // Use manager as normal...
//! # Ok(())
//! # }
//! ```

pub mod data_directory;
pub mod errors;
pub mod manifest;
pub mod state_service;
pub mod workspace_manager;

// Re-export main types for easier access
pub use data_directory::{DataDirectory, WorkspaceFolderDataDirectoryInfo, format_bytes};
pub use errors::{Result, WorkspaceManagerError};
pub use manifest::{
    Manifest, ProjectMetadata, Status, WorkspaceFolderMetadata, generate_path_hash,
};
pub use state_service::LocalStateService;
pub use workspace_manager::{ProjectInfo, WorkspaceFolderInfo, WorkspaceManager};
