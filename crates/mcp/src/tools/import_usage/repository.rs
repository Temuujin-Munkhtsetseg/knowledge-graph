use std::path::PathBuf;
use std::sync::Arc;

use database::querying::{ImportUsageQueryOptions, QueryLibrary, QueryingService};
use rmcp::model::ErrorCode;

#[derive(Debug, Clone)]
pub struct ImportHit {
    pub file_path: String,
    pub import_path: String,
    pub name: String,
    pub alias: String,
    pub start_line: i64,
    pub end_line: i64,
}

#[derive(Debug, Clone)]
pub struct ReferenceHit {
    pub file_path: String,
    pub start_line: i64,
    pub end_line: i64,
    pub fqn: String,
    pub def_start_line: i64,
}

pub struct ImportUsageRepository {
    querying_service: Arc<dyn QueryingService>,
}

impl ImportUsageRepository {
    pub fn new(querying_service: Arc<dyn QueryingService>) -> Self {
        Self { querying_service }
    }

    pub fn find_imports_and_references_combined(
        &self,
        database_path: PathBuf,
        import_paths: Vec<String>,
        names: Vec<String>,
        aliases: Vec<String>,
    ) -> Result<(Vec<ImportHit>, Vec<ReferenceHit>), rmcp::ErrorData> {
        use database::graph::{RelationshipType, RelationshipTypeMapping};
        let mapping = RelationshipTypeMapping::new();
        let calls_type_id = mapping.get_type_id(RelationshipType::Calls) as i64;
        let ambiguous_calls_type_id =
            mapping.get_type_id(RelationshipType::AmbiguouslyCalls) as i64;

        let mut params = serde_json::Map::new();
        let lowercased: Vec<serde_json::Value> = import_paths
            .into_iter()
            .map(|s| serde_json::Value::String(s.to_lowercase()))
            .collect();
        params.insert("paths_lc".to_string(), serde_json::Value::Array(lowercased));
        params.insert(
            "calls_type_id".to_string(),
            serde_json::Value::Number(calls_type_id.into()),
        );
        params.insert(
            "ambiguous_calls_type_id".to_string(),
            serde_json::Value::Number(ambiguous_calls_type_id.into()),
        );
        params.insert("limit".to_string(), serde_json::Value::Number(500.into()));

        // Determine if we need to filter by name/alias
        let has_names = names.iter().any(|n| !n.is_empty());
        let has_aliases = aliases.iter().any(|a| !a.is_empty());

        let q = QueryLibrary::get_import_usage(ImportUsageQueryOptions {
            include_name: has_names,
            include_alias: has_aliases,
        });

        if has_names {
            // Use the first non-empty name for filtering
            if let Some(name) = names.iter().find(|n| !n.is_empty()) {
                params.insert(
                    "import_name".to_string(),
                    serde_json::Value::String(name.clone()),
                );
            }
        }

        if has_aliases {
            // Use the first non-empty alias for filtering
            if let Some(alias) = aliases.iter().find(|a| !a.is_empty()) {
                params.insert(
                    "import_alias".to_string(),
                    serde_json::Value::String(alias.clone()),
                );
            }
        }

        let mut result = self
            .querying_service
            .execute_query(database_path, q.query, params)
            .map_err(|e| rmcp::ErrorData::new(ErrorCode::INVALID_REQUEST, e.to_string(), None))?;

        let mut import_hits = Vec::new();
        let mut reference_hits = Vec::new();

        while let Some(row) = result.next() {
            let file_path = row.get_string_value(0).unwrap_or_default();

            // Extract import info
            let import_hit = ImportHit {
                file_path: file_path.clone(),
                import_path: row.get_string_value(1).unwrap_or_default(),
                name: row.get_string_value(2).unwrap_or_default(),
                alias: row.get_string_value(3).unwrap_or_default(),
                start_line: row.get_int_value(4).unwrap_or_default() + 1,
                end_line: row.get_int_value(5).unwrap_or_default() + 1,
            };
            import_hits.push(import_hit);

            // Extract reference info if it exists (OPTIONAL MATCH may return nulls)
            if let Ok(ref_file) = row.get_string_value(6)
                && !ref_file.is_empty()
            {
                let reference_hit = ReferenceHit {
                    file_path: ref_file,
                    start_line: row.get_int_value(7).unwrap_or_default() + 1,
                    end_line: row.get_int_value(8).unwrap_or_default() + 1,
                    fqn: row.get_string_value(9).unwrap_or_default(),
                    def_start_line: row.get_int_value(10).unwrap_or_default() + 1,
                };
                reference_hits.push(reference_hit);
            }
        }

        Ok((import_hits, reference_hits))
    }
}
