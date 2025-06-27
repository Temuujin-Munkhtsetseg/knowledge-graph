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
//! use workspace_manager::WorkspaceManager;
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a workspace manager with system default data directory
//! let manager = WorkspaceManager::new_system_default()?;
//!
//! // Register a workspace folder
//! let workspace_folder_path = Path::new("/path/to/workspace");
//! let discovery_result = manager.register_workspace_folder(workspace_folder_path)?;
//!
//! println!("Found {} projects in workspace", discovery_result.projects_found.len());
//!
//! // List all registered projects
//! let projects = manager.list_all_projects();
//! for project in projects {
//!     println!("Project: {} (Status: {:?})", project.project_path, project.status);
//! }
//!
//! // Mark a project as being indexed
//! if let Some(project) = discovery_result.projects_found.first() {
//!     manager.mark_project_indexing(&discovery_result.workspace_folder_path, &project.project_path)?;
//!
//!     // Access Gitalisk repository for a project
//!     let project_info = manager.get_project_info(&discovery_result.workspace_folder_path, &project.project_path)
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
pub use workspace_manager::{DiscoveryResult, ProjectInfo, WorkspaceFolderInfo, WorkspaceManager};
