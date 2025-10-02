use rmcp::model::{ErrorCode, JsonObject};
use std::{path::PathBuf, sync::Arc};
use workspace_manager::WorkspaceManager;

use super::constants::{DEFAULT_PAGE, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};
use crate::tools::{types::KnowledgeGraphToolInput, utils::get_database_path};

#[derive(Debug, Clone)]
pub struct PackageCandidate {
    pub import_path: String,
    pub name: String,
    pub alias: String,
    pub relative_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImportUsageInput {
    pub database_path: PathBuf,
    pub project_absolute_path: String,
    pub packages: Vec<PackageCandidate>,
    pub page: u64,
    pub page_size: u64,
}

impl ImportUsageInput {
    pub fn new(
        object: JsonObject,
        workspace_manager: &Arc<WorkspaceManager>,
    ) -> Result<Self, rmcp::ErrorData> {
        let input = KnowledgeGraphToolInput { params: object };

        let project_absolute_path = input.get_string("project_absolute_path")?.to_string();
        let database_path = get_database_path(workspace_manager, &project_absolute_path)?;

        let packages_value = input
            .params
            .get("packages")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    "Missing or invalid 'packages' array.".to_string(),
                    None,
                )
            })?;

        if packages_value.is_empty() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "Packages array cannot be empty.".to_string(),
                None,
            ));
        }

        let mut packages = Vec::new();
        for (idx, p) in packages_value.iter().enumerate() {
            let obj = p.as_object().ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("Package at index {idx} must be an object."),
                    None,
                )
            })?;

            let import_path = obj
                .get("import_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        format!("Missing 'import_path' in package index {idx}."),
                        None,
                    )
                })?
                .to_string();
            if import_path.trim().is_empty() {
                return Err(rmcp::ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("Empty 'import_path' in package index {idx}."),
                    None,
                ));
            }

            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let alias = obj
                .get("alias")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let relative_paths: Vec<String> = obj
                .get("relative_paths")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();

            packages.push(PackageCandidate {
                import_path,
                name,
                alias,
                relative_paths,
            });
        }

        let page = input
            .get_u64_optional("page")
            .unwrap_or(DEFAULT_PAGE)
            .max(1);
        let page_size = input
            .get_u64_optional("page_size")
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .clamp(1, MAX_PAGE_SIZE);

        Ok(Self {
            database_path,
            project_absolute_path,
            packages,
            page,
            page_size,
        })
    }
}
