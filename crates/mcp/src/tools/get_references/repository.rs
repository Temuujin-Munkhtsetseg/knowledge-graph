use std::{path::Path, sync::Arc};

use database::querying::QueryingService;
use rmcp::model::ErrorCode;

use super::input::GetReferencesToolInput;

pub const DEFAULT_PAGE_SIZE: u64 = 50;

#[derive(Debug)]
pub struct ReferenceQueryResult {
    pub definition_name: String,
    pub definition_fqn: String,
    pub definition_type: String,
    pub definition_primary_file_path: String,
    pub definition_start_line: i64,
    pub definition_end_line: i64,
    pub reference_start_line: i64,
    pub reference_end_line: i64,
    pub reference_type: String,
}

pub struct GetReferencesRepository {
    querying_service: Arc<dyn QueryingService>,
}

impl GetReferencesRepository {
    pub fn new(querying_service: Arc<dyn QueryingService>) -> Self {
        Self { querying_service }
    }

    pub fn query_references(
        &self,
        input: GetReferencesToolInput,
    ) -> Result<Vec<ReferenceQueryResult>, rmcp::ErrorData> {
        let definition_references_query = "
            MATCH (s:DefinitionNode)<-[r:DEFINITION_RELATIONSHIPS]-(t:DefinitionNode)
            WHERE 
                s.name = $definition_name 
                AND s.primary_file_path = $definition_file_path 
                AND r.type in $reference_types
            RETURN 
                t.name as target_name, 
                t.fqn as target_fqn,
                t.definition_type as target_definition_type,
                t.primary_file_path as target_primary_file_path,
                t.start_line as target_start_line,
                t.end_line as target_end_line,
                r.source_start_line as reference_start_line,
                r.source_end_line as reference_end_line,
                r.type as reference_type
            SKIP $skip
            LIMIT $limit
        ";

        let mut params = serde_json::Map::new();
        params.insert(
            "definition_name".to_string(),
            serde_json::Value::String(input.definition_name),
        );
        params.insert(
            "definition_file_path".to_string(),
            serde_json::Value::String(input.relative_file_path),
        );
        params.insert(
            "reference_types".to_string(),
            serde_json::Value::Array(
                self.get_reference_relationship_type_ids()
                    .iter()
                    .map(|id| serde_json::Value::from(id.clone()))
                    .collect(),
            ),
        );
        params.insert(
            "limit".to_string(),
            serde_json::Value::Number(DEFAULT_PAGE_SIZE.into()),
        );
        params.insert(
            "skip".to_string(),
            serde_json::Value::Number(((input.page - 1) * DEFAULT_PAGE_SIZE).into()),
        );

        let mut defnition_references = self
            .querying_service
            .execute_query(
                input.database_path,
                definition_references_query.to_string(),
                params,
            )
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    format!("Could not execute definition references query: {e}."),
                    None,
                )
            })?;

        let mut results: Vec<ReferenceQueryResult> = Vec::new();
        while let Some(row) = defnition_references.next() {
            results.push(ReferenceQueryResult {
                definition_name: row.get_string_value(0).unwrap(), // target_name
                definition_fqn: row.get_string_value(1).unwrap(),  // target_fqn
                definition_type: row.get_string_value(2).unwrap(), // target_definition_type
                definition_primary_file_path: Path::new(&input.project_path)
                    .join(row.get_string_value(3).unwrap())
                    .to_string_lossy()
                    .to_string(), // target_primary_file_path
                definition_start_line: row.get_int_value(4).unwrap() + 1, // target_start_line, one-indexed
                definition_end_line: row.get_int_value(5).unwrap() + 1, // target_end_line, one-indexed
                reference_start_line: row.get_int_value(6).unwrap() + 1, // reference_start_line, one-indexed
                reference_end_line: row.get_int_value(7).unwrap() + 1, // reference_end_line, one-indexed
                reference_type: row.get_string_value(8).unwrap(),      // reference_type
            });
        }

        Ok(results)
    }

    fn get_reference_relationship_type_ids(&self) -> Vec<String> {
        use database::graph::RelationshipType;

        vec![
            RelationshipType::Calls.as_string(),
            RelationshipType::PropertyReference.as_string(),
            RelationshipType::AmbiguouslyCalls.as_string(),
        ]
    }
}
