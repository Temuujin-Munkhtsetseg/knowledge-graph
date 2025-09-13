use std::{path::Path, sync::Arc};

use database::querying::QueryingService;
use rmcp::model::ErrorCode;

use super::input::{DefinitionRequest, ReadDefinitionsToolInput};

#[derive(Debug)]
pub struct DefinitionQueryResult {
    pub name: String,
    pub fqn: String,
    pub definition_type: String,
    pub primary_file_path: String,
    pub start_line: i64,
    pub end_line: i64,
    pub request_index: usize, // To track which request this result belongs to
}

pub struct ReadDefinitionsRepository {
    querying_service: Arc<dyn QueryingService>,
}

impl ReadDefinitionsRepository {
    pub fn new(querying_service: Arc<dyn QueryingService>) -> Self {
        Self { querying_service }
    }

    pub fn query_definitions(
        &self,
        input: ReadDefinitionsToolInput,
    ) -> Result<Vec<DefinitionQueryResult>, rmcp::ErrorData> {
        let mut all_results = Vec::new();

        for (request_index, request) in input.definition_requests.iter().enumerate() {
            let results = self.query_single_definition(request, request_index)?;
            all_results.extend(results);
        }

        Ok(all_results)
    }

    fn query_single_definition(
        &self,
        request: &DefinitionRequest,
        request_index: usize,
    ) -> Result<Vec<DefinitionQueryResult>, rmcp::ErrorData> {
        let definition_query = "
            MATCH (d:DefinitionNode)
            WHERE 
                d.name = $definition_name 
                AND d.primary_file_path = $definition_file_path
            RETURN 
                d.name as name,
                d.fqn as fqn,
                d.definition_type as definition_type,
                d.primary_file_path as primary_file_path,
                d.start_line as start_line,
                d.end_line as end_line
            ORDER BY d.start_line
        ";

        let mut params = serde_json::Map::new();
        params.insert(
            "definition_name".to_string(),
            serde_json::Value::String(request.name.clone()),
        );
        params.insert(
            "definition_file_path".to_string(),
            serde_json::Value::String(request.relative_file_path.clone()),
        );

        let mut query_result = self
            .querying_service
            .execute_query(
                request.database_path.clone(),
                definition_query.to_string(),
                params,
            )
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    format!(
                        "Could not execute definition query for '{}': {e}.",
                        request.name
                    ),
                    None,
                )
            })?;

        let mut results: Vec<DefinitionQueryResult> = Vec::new();
        while let Some(row) = query_result.next() {
            results.push(DefinitionQueryResult {
                name: row.get_string_value(0).unwrap(),            // name
                fqn: row.get_string_value(1).unwrap(),             // fqn
                definition_type: row.get_string_value(2).unwrap(), // definition_type
                primary_file_path: Path::new(&request.project_path)
                    .join(row.get_string_value(3).unwrap())
                    .to_string_lossy()
                    .to_string(), // primary_file_path (convert to absolute)
                start_line: row.get_int_value(4).unwrap() + 1,     // start_line, one-indexed
                end_line: row.get_int_value(5).unwrap() + 1,       // end_line, one-indexed
                request_index,
            });
        }

        Ok(results)
    }
}
