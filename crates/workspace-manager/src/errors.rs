//! Error types for the workspace-manager crate

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for workspace manager operations
pub type Result<T> = std::result::Result<T, WorkspaceManagerError>;

/// Comprehensive error types for workspace management operations
#[derive(Error, Debug)]
pub enum WorkspaceManagerError {
    /// IO operations failed
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization failed
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Failed to create data directory
    #[error("Failed to create data directory: {path:?}")]
    DataDirectoryCreationFailed { path: PathBuf },

    /// Failed to determine system data directory
    #[error("Failed to determine system data directory")]
    SystemDataDirectoryNotFound,
}
