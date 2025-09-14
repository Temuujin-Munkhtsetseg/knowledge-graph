use std::collections::HashMap;
use std::sync::Arc;

use database::querying::QueryingService;
use tokio::time::{Duration, timeout};

use crate::tools::file_reader_utils::read_file_chunks;
use crate::tools::get_references::input::GetReferencesToolInput;
use crate::tools::get_references::output::{
    GetReferencesToolDefinitionOutput, GetReferencesToolOutput, GetReferencesToolReferenceOutput,
};
use crate::tools::get_references::repository::DEFAULT_PAGE_SIZE;
use crate::tools::get_references::repository::GetReferencesRepository;
use database::graph::RelationshipTypeMapping;

const FILE_READ_TIMEOUT_SECONDS: u64 = 10;
const SURROUNDING_LINES: i64 = 2;

pub struct GetReferencesService {
    repository: GetReferencesRepository,
}

impl GetReferencesService {
    pub fn new(querying_service: Arc<dyn QueryingService>) -> Self {
        Self {
            repository: GetReferencesRepository::new(querying_service),
        }
    }

    pub fn get_references(
        &self,
        input: GetReferencesToolInput,
    ) -> Result<GetReferencesToolOutput, rmcp::ErrorData> {
        let results = self.repository.query_references(input.clone())?;

        let total_results = results.len();
        let mut next_page = None;
        if (total_results as u64) >= DEFAULT_PAGE_SIZE {
            next_page = Some(input.page + 1);
        }

        if results.is_empty() {
            return Ok(GetReferencesToolOutput::empty(self.get_system_message(
                input,
                vec![],
                0,
                next_page,
            )));
        }

        // Group results by definition (the thing that references the target)
        let mut grouped_results: HashMap<String, Vec<_>> = HashMap::new();
        for result in results {
            let definition_key = format!(
                "{}:L{}-{}",
                result.definition_fqn, result.definition_start_line, result.definition_end_line
            );
            grouped_results
                .entry(definition_key)
                .or_default()
                .push(result);
        }

        // Prepare file chunks to read for all references
        let mut file_chunks = Vec::new();
        let mut chunk_indices = Vec::new(); // Track which chunk belongs to which result
        let mut current_index = 0;

        for group in grouped_results.values() {
            for item in group {
                let chunk_start_line =
                    (item.reference_start_line - SURROUNDING_LINES).max(item.definition_start_line);
                let chunk_end_line =
                    (item.reference_end_line + SURROUNDING_LINES).min(item.definition_end_line);

                file_chunks.push((
                    item.definition_primary_file_path.clone(),
                    chunk_start_line as usize,
                    chunk_end_line as usize,
                ));
                chunk_indices.push(current_index);
                current_index += 1;
            }
        }
        let file_contents = self.read_reference_chunks(file_chunks);

        // Build final results with content
        let mut file_read_errors = Vec::new();
        let mut definitions = Vec::new();
        let mut content_index = 0;

        for (_, group) in grouped_results {
            if let Some(first_item) = group.first() {
                let mut references = Vec::new();

                for item in &group {
                    let context = match file_contents.get(content_index) {
                        Some(Ok(content)) => content.trim().to_string(),
                        Some(Err(_)) => {
                            file_read_errors.push(item.definition_primary_file_path.clone());
                            "".to_string()
                        }
                        None => {
                            file_read_errors.push(item.definition_primary_file_path.clone());
                            "".to_string()
                        }
                    };

                    references.push(GetReferencesToolReferenceOutput {
                        reference_type: RelationshipTypeMapping::new()
                            .get_type_name(item.reference_type)
                            .to_string(),
                        location: format!(
                            "{}:L{}-{}",
                            item.definition_primary_file_path,
                            item.reference_start_line,
                            item.reference_end_line
                        ),
                        context,
                    });

                    content_index += 1;
                }

                definitions.push(GetReferencesToolDefinitionOutput {
                    name: first_item.definition_name.clone(),
                    location: format!(
                        "{}:L{}-{}",
                        first_item.definition_primary_file_path,
                        first_item.definition_start_line,
                        first_item.definition_end_line
                    ),
                    definition_type: first_item.definition_type.clone(),
                    fqn: first_item.definition_fqn.clone(),
                    references,
                });
            }
        }

        Ok(GetReferencesToolOutput {
            definitions,
            next_page,
            system_message: self.get_system_message(
                input,
                file_read_errors,
                total_results,
                next_page,
            ),
        })
    }

    fn read_reference_chunks(
        &self,
        file_chunks: Vec<(String, usize, usize)>,
    ) -> Vec<std::io::Result<String>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                match timeout(
                    Duration::from_secs(FILE_READ_TIMEOUT_SECONDS),
                    read_file_chunks(file_chunks.clone()),
                )
                .await
                {
                    Ok(Ok(results)) => results,
                    Ok(Err(e)) => file_chunks
                        .iter()
                        .map(|_| {
                            Err(std::io::Error::new(
                                e.kind(),
                                format!("Failed to read file chunks: {}.", e),
                            ))
                        })
                        .collect(),
                    Err(_) => file_chunks
                        .iter()
                        .map(|_| {
                            Err(std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "File reading operation timed out.",
                            ))
                        })
                        .collect(),
                }
            })
        })
    }

    fn get_system_message(
        &self,
        input: GetReferencesToolInput,
        file_read_errors: Vec<String>,
        total_results: usize,
        next_page: Option<u64>,
    ) -> String {
        let mut message = String::new();

        // Document failed file reads
        for (index, file_read_error) in file_read_errors.iter().enumerate() {
            if index == 0 {
                message.push_str("Failed to read some some files:");
            }
            message.push_str(&format!("\n- {}.", file_read_error));
            if index == file_read_errors.len() - 1 {
                message.push_str(
                    "\nPerhaps some files were deleted, moved or changed since the last indexing.",
                );
                message.push_str(&format!("\nIf the missing context is important, use the `index_project` tool to re-index the project {} and try again.\n", input.project_path.to_string_lossy()));
            }
        }

        // Document next page
        if let Some(next_page) = next_page {
            message.push_str(&format!("There are more results on page {} if more context is needed for the current task.\n", next_page));
        }

        // Document total results
        if total_results > 0 {
            message.push_str(&format!(
                "Found a total of {} references for the definition {} in the file {}.\n",
                total_results, input.definition_name, input.relative_file_path
            ));

            message.push_str("\nDecision Framework:\n");
            message.push_str("  - If your current task is to find all references to a definition, you can stop here.\n");
            message.push_str("  - If you're analyzing how a change might affect the codebase, use the `get_references` tool again to examine what references the symbols that point to your target definition.\n");
            message.push_str("  - If you need more background about a definition that references your target symbol, use the `search_codebase_definitions` tool to explore further.\n");
        } else {
            message.push_str(&format!(
                "No indexed references found for the definition {} in the file {}.\n",
                input.definition_name, input.relative_file_path
            ));

            message.push_str("\nDecision Framework:\n");
            message.push_str("  - If you know for sure that the definition is referenced somewhere, you can use the `index_project` tool to re-index the project and try again.\n");
            message.push_str("  - If you know for sure that the definition is referenced somewhere, and the indexing is up to date, you can stop using the Knowledge Graph for getting references for the requested symbol.\n");
        }

        message
    }
}
