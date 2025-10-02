use std::path::{Path, PathBuf};

use rmcp::model::{ErrorCode, JsonObject};
use serde_json::{Map, Value};

use database::querying::QueryingService;
use workspace_manager::WorkspaceManager;

use super::input::RepoMapInput;

pub struct RepoMapService<'a> {
    pub query_service: &'a dyn QueryingService,
    pub workspace_manager: &'a WorkspaceManager,
}

pub struct RepoMapDefinition {
    pub fqn: String,
    pub definition_type: String,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
}

impl<'a> RepoMapService<'a> {
    pub fn parse_input(&self, params: JsonObject) -> Result<RepoMapInput, rmcp::ErrorData> {
        RepoMapInput::try_from(params)
    }

    pub fn to_relative(project_root: &Path, absolute: &str) -> Option<String> {
        let abs = Path::new(absolute);
        abs.strip_prefix(project_root)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    }

    pub fn query_definitions(
        &self,
        database_path: PathBuf,
        relative_files: Vec<String>,
        page: u64,
        page_size: u64,
    ) -> Result<Vec<RepoMapDefinition>, rmcp::ErrorData> {
        let skip = (page - 1) * page_size;
        // Note that definitions at this time are not always directly associated with a file
        // so we need to use the primary_file_path to find them
        // TODO: explore if we should tie *all* definitions to files in the database
        let query = r#"
            MATCH (d:DefinitionNode)
            WHERE d.primary_file_path IN $relative_files
            RETURN 
                d.fqn as fqn,
                d.definition_type as definition_type,
                d.primary_file_path as file_path,
                d.start_line as start_line,
                d.end_line as end_line
            ORDER BY d.primary_file_path, d.start_line
            SKIP $skip
            LIMIT $limit
        "#;

        let mut params = Map::new();
        params.insert(
            "relative_files".to_string(),
            Value::Array(
                relative_files
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            ),
        );
        params.insert("skip".to_string(), Value::Number(skip.into()));
        params.insert("limit".to_string(), Value::Number(page_size.into()));

        let mut res = self
            .query_service
            .execute_query(database_path, query.to_string(), params)
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    format!("Database query failed: {e}."),
                    None,
                )
            })?;

        let mut rows: Vec<RepoMapDefinition> = Vec::with_capacity(page_size as usize);
        while let Some(row) = res.next() {
            let fqn = row.get_string_value(0).unwrap_or_default();
            let def_type = row.get_string_value(1).unwrap_or_default();
            let file_rel = row.get_string_value(2).unwrap_or_default();
            let start_line_0 = row.get_int_value(3).unwrap_or(0) as usize;
            let end_line_0 = row.get_int_value(4).unwrap_or(0) as usize;
            rows.push(RepoMapDefinition {
                fqn,
                definition_type: def_type,
                file_path: file_rel,
                start_line: start_line_0,
                end_line: end_line_0,
            });
        }
        Ok(rows)
    }
}
