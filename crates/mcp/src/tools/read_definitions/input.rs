use rmcp::model::{ErrorCode, JsonObject};
use std::{path::PathBuf, sync::Arc};
use workspace_manager::WorkspaceManager;

use crate::tools::{types::KnowledgeGraphToolInput, utils::resolve_paths};

use super::constants::{DEFINITIONS_FIELD, FILE_PATH_FIELD, NAMES_FIELD};

#[derive(Debug, Clone)]
pub struct DefinitionRequest {
    pub name: String,
    pub database_path: PathBuf,
    pub project_path: PathBuf,
    pub relative_file_path: String,
    pub absolute_file_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ReadDefinitionsToolInput {
    pub definition_requests: Vec<DefinitionRequest>,
}

impl ReadDefinitionsToolInput {
    pub fn new(
        object: JsonObject,
        workspace_manager: &Arc<WorkspaceManager>,
    ) -> Result<Self, rmcp::ErrorData> {
        let input = KnowledgeGraphToolInput { params: object };

        // Get the definitions array
        let definitions_array = input
            .params
            .get(DEFINITIONS_FIELD)
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    "Missing or invalid 'definitions' array.".to_string(),
                    None,
                )
            })?;

        if definitions_array.is_empty() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "Definitions array cannot be empty.".to_string(),
                None,
            ));
        }

        let mut definition_requests = Vec::new();
        for (index, def_value) in definitions_array.iter().enumerate() {
            let def_object = def_value.as_object().ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("Definition at index {index} must be an object."),
                    None,
                )
            })?;

            let names_array = def_object
                .get(NAMES_FIELD)
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        format!("Missing or invalid 'names' array in definition at index {index}."),
                        None,
                    )
                })?;

            if names_array.is_empty() {
                return Err(rmcp::ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("Names array cannot be empty at index {index}."),
                    None,
                ));
            }

            let input_file_path = def_object
                .get(FILE_PATH_FIELD)
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        format!(
                            "Missing or invalid 'file_path' field in definition at index {index}."
                        ),
                        None,
                    )
                })?
                .to_string();

            let (absolute_file_path, project_info, relative_file_path) =
                resolve_paths(workspace_manager, &input_file_path)?;

            // Create a DefinitionRequest for each name in the names array
            for (name_index, name_value) in names_array.iter().enumerate() {
                let name = name_value.as_str().ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        format!(
                            "Name at index {name_index} in names array at definition index {index} must be a string."
                        ),
                        None,
                    )
                })?.to_string();

                if name.is_empty() {
                    return Err(rmcp::ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        format!(
                            "Definition name cannot be empty at name index {name_index} in definition index {index}."
                        ),
                        None,
                    ));
                }

                definition_requests.push(DefinitionRequest {
                    name,
                    database_path: project_info.database_path.clone(),
                    project_path: project_info.project_path.clone().into(),
                    relative_file_path: relative_file_path.clone(),
                    absolute_file_path: absolute_file_path.clone(),
                });
            }
        }

        Ok(Self {
            definition_requests,
        })
    }
}
