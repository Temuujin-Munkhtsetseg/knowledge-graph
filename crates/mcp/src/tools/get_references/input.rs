use rmcp::model::{ErrorCode, JsonObject};
use std::{path::PathBuf, sync::Arc};
use workspace_manager::WorkspaceManager;

use crate::tools::{
    get_references::constants::{MIN_PAGE, PAGE_FIELD},
    types::KnowledgeGraphToolInput,
    utils::resolve_paths,
};

use super::constants::{DEFAULT_PAGE, DEFINITION_NAME_FIELD, FILE_PATH_FIELD};

#[derive(Debug, Clone)]
pub struct GetReferencesToolInput {
    pub definition_name: String,
    pub database_path: PathBuf,
    pub project_path: PathBuf,
    pub relative_file_path: String,
    pub absolute_file_path: PathBuf,
    pub page: u64,
}

impl GetReferencesToolInput {
    pub fn new(
        object: JsonObject,
        workspace_manager: &Arc<WorkspaceManager>,
    ) -> Result<Self, rmcp::ErrorData> {
        let input = KnowledgeGraphToolInput { params: object };
        let definition_name = input.get_string(DEFINITION_NAME_FIELD)?.to_string();
        if definition_name.is_empty() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "Definition name cannot be empty.".to_string(),
                None,
            ));
        }

        let input_file_path = input.get_string(FILE_PATH_FIELD)?.to_string();
        let (absolute_file_path, project_info, relative_file_path) =
            resolve_paths(workspace_manager, &input_file_path)?;

        let tool_input = Self {
            definition_name,
            database_path: project_info.database_path,
            project_path: project_info.project_path.into(),
            relative_file_path,
            absolute_file_path,
            page: input
                .get_u64_optional(PAGE_FIELD)
                .unwrap_or(DEFAULT_PAGE)
                .max(MIN_PAGE),
        };

        Ok(tool_input)
    }
}
