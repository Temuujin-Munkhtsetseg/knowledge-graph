use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use database::graph::{RelationshipType, RelationshipTypeMapping};
use database::kuzu::connection::KuzuConnection;
use database::kuzu::database::KuzuDatabase;
use rmcp::model::ErrorCode;
use workspace_manager::WorkspaceManager;

use super::input::GetDefinitionInput;
use super::output::{Definition, DefinitionInfo, GetDefinitionOutput, ImportedSymbolInfo};
use super::repository::{self, RawHit};
use crate::tools::file_reader_utils::{find_matching_line_numbers, read_file_chunks};
use crate::tools::utils;

pub struct GetDefinitionService {
    database: Arc<KuzuDatabase>,
    workspace_manager: Arc<WorkspaceManager>,
}

impl GetDefinitionService {
    pub fn new(database: Arc<KuzuDatabase>, workspace_manager: Arc<WorkspaceManager>) -> Self {
        Self {
            database,
            workspace_manager,
        }
    }

    pub async fn get_definition(
        &self,
        input: GetDefinitionInput,
    ) -> Result<GetDefinitionOutput, rmcp::ErrorData> {
        let (abs_path, project_info, relative_file_path) =
            utils::resolve_paths(&self.workspace_manager, &input.file_path)?;

        let matching_lines = find_matching_line_numbers(abs_path.to_str().unwrap(), &input.line)
            .await
            .map_err(|e| rmcp::ErrorData::new(ErrorCode::INVALID_REQUEST, e.to_string(), None))?;

        if matching_lines.is_empty() {
            return Ok(GetDefinitionOutput {
                definitions: vec![],
                system_message: None,
            });
        }

        let abs_path_str = abs_path.to_str().unwrap().to_string();
        let line_chunks: Vec<(String, usize, usize)> = matching_lines
            .iter()
            .map(|l| (abs_path_str.clone(), *l, *l))
            .collect();
        let line_results = if line_chunks.is_empty() {
            Vec::new()
        } else {
            read_file_chunks(line_chunks)
                .await
                .map_err(|e| rmcp::ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
        };

        let (calls_type_id, ambiguous_calls_type_id) = {
            let mapping = RelationshipTypeMapping::new();
            (
                mapping.get_type_id(RelationshipType::Calls) as i64,
                mapping.get_type_id(RelationshipType::AmbiguouslyCalls) as i64,
            )
        };

        let database = self
            .database
            .get_or_create_database(&project_info.database_path.to_string_lossy(), None)
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Failed to get database for workspace".to_string(),
                    None,
                )
            })?;

        let raw_hits = {
            let conn = KuzuConnection::new(&database).map_err(|e| {
                rmcp::ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
            })?;
            let mut raw_hits: Vec<RawHit> = Vec::new();
            let multiple_line_matches = matching_lines.len() > 1;
            let mut multiple_symbol_occurrences = false;

            for (idx, line_1) in matching_lines.iter().enumerate() {
                let db_line = (*line_1 as i64) - 1;
                let actual_line = match line_results.get(idx) {
                    Some(Ok(s)) => s.as_str(),
                    _ => continue,
                };
                let ranges = find_symbol_col_ranges(actual_line, &input.symbol_name);
                if ranges.len() > 1 {
                    multiple_symbol_occurrences = true;
                }

                for (start_col, end_col) in ranges {
                    let hits = repository::find_definitions(
                        &conn,
                        &relative_file_path,
                        db_line,
                        start_col,
                        end_col,
                        calls_type_id,
                        ambiguous_calls_type_id,
                    )?;
                    for hit in hits {
                        if hit.target_type == "Definition" {
                            if hit.name.eq_ignore_ascii_case(&input.symbol_name) {
                                raw_hits.push(hit);
                            }
                        } else {
                            raw_hits.push(hit);
                        }
                    }
                }
            }
            (raw_hits, multiple_line_matches, multiple_symbol_occurrences)
        };

        let (raw_hits, multiple_line_matches, multiple_symbol_occurrences) = raw_hits;

        let selected_hits = select_best_hits(raw_hits);

        let mut definitions = Vec::new();
        let mut chunks_input = Vec::new();
        for hit in selected_hits {
            let abs_target_path = Path::new(&project_info.project_path)
                .join(&hit.path)
                .to_string_lossy()
                .to_string();
            let start_line_1 = (hit.start_line_db + 1).max(1) as usize;
            let end_line_1 = (hit.end_line_db + 1).max(hit.start_line_db + 1) as usize;
            chunks_input.push((abs_target_path.clone(), start_line_1, end_line_1));

            let is_ambiguous = hit.rel_type_id == ambiguous_calls_type_id;

            if hit.target_type == "Definition" {
                definitions.push(Definition::Definition(DefinitionInfo {
                    id: hit.id,
                    name: hit.name,
                    fqn: hit.fqn,
                    primary_file_path: hit.path,
                    absolute_file_path: abs_target_path,
                    start_line: hit.start_line_db,
                    end_line: hit.end_line_db,
                    rel_start_col: hit.rel_start_col,
                    rel_end_col: hit.rel_end_col,
                    is_ambiguous,
                    code: None,
                    code_error: None,
                }));
            } else if hit.target_type == "ImportedSymbol" {
                definitions.push(Definition::ImportedSymbol(ImportedSymbolInfo {
                    id: hit.id,
                    name: hit.name,
                    fqn: hit.fqn,
                    primary_file_path: hit.path,
                    absolute_file_path: abs_target_path,
                    start_line: hit.start_line_db,
                    end_line: hit.end_line_db,
                    rel_start_col: hit.rel_start_col,
                    rel_end_col: hit.rel_end_col,
                    is_ambiguous,
                    code: None,
                    code_error: None,
                }));
            }
        }

        let chunks_results = if chunks_input.is_empty() {
            Vec::new()
        } else {
            read_file_chunks(chunks_input)
                .await
                .map_err(|e| rmcp::ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
        };

        let enriched_definitions = definitions
            .into_iter()
            .enumerate()
            .map(|(i, def)| {
                let (code, code_error) = if let Some(code_res) = chunks_results.get(i) {
                    match code_res {
                        Ok(code) => (Some(code.clone()), None),
                        Err(err) => (None, Some(err.to_string())),
                    }
                } else {
                    (None, None)
                };

                match def {
                    Definition::Definition(mut info) => {
                        info.code = code;
                        info.code_error = code_error;
                        Definition::Definition(info)
                    }
                    Definition::ImportedSymbol(mut info) => {
                        info.code = code;
                        info.code_error = code_error;
                        Definition::ImportedSymbol(info)
                    }
                }
            })
            .collect();

        let mut system_message = None;
        if multiple_line_matches || multiple_symbol_occurrences {
            let mut parts = Vec::new();
            if multiple_line_matches {
                parts.push(format!(
                    "Multiple lines matched this code ({}).",
                    matching_lines.len()
                ));
            }
            if multiple_symbol_occurrences {
                parts.push("Multiple occurrences of the symbol on the line.".to_string());
            }
            system_message = Some(parts.join(" "));
        }

        Ok(GetDefinitionOutput {
            definitions: enriched_definitions,
            system_message,
        })
    }
}

fn find_symbol_col_ranges(line: &str, symbol_name: &str) -> Vec<(i64, i64)> {
    if symbol_name.is_empty() || line.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut search_start = 0usize;
    while let Some(pos) = line[search_start..].find(symbol_name) {
        let start = search_start + pos;
        let end_inclusive = start + symbol_name.len() - 1;
        ranges.push((start as i64, end_inclusive as i64));
        search_start = end_inclusive + 1;
        if search_start >= line.len() {
            break;
        }
    }
    ranges
}

/// Selects the most relevant hits from a raw list of potential definitions.
///
/// This function applies deduplication and selection logic to refine the results:
/// - For `Definition` types, all unique definitions (deduplicated by ID) are selected.
/// - For `ImportedSymbol` types, it selects the best match for each unique symbol ID.
///   The "best" match is determined by the narrowest column range (`rel_end_col` - `rel_start_col`),
///   as this often represents the most specific import statement.
fn select_best_hits(raw_hits: Vec<RawHit>) -> Vec<RawHit> {
    let mut best_import_by_id: HashMap<String, RawHit> = HashMap::new();
    let mut definition_seen: HashSet<String> = HashSet::new();
    let mut selected: Vec<RawHit> = Vec::new();

    for hit in raw_hits.into_iter() {
        if hit.target_type == "Definition" {
            if definition_seen.insert(hit.id.clone()) {
                selected.push(hit);
            }
        } else if hit.target_type == "ImportedSymbol" {
            let width = hit.rel_end_col - hit.rel_start_col;
            if let Some(existing) = best_import_by_id.get(&hit.id) {
                let existing_width = existing.rel_end_col - existing.rel_start_col;
                if width < existing_width {
                    best_import_by_id.insert(hit.id.clone(), hit);
                }
            } else {
                best_import_by_id.insert(hit.id.clone(), hit);
            }
        }
    }

    for (_, hit) in best_import_by_id.into_iter() {
        selected.push(hit);
    }

    selected
}
