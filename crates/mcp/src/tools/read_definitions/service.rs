use std::sync::Arc;

use database::querying::QueryingService;
use tokio::time::{Duration, timeout};

use crate::tools::file_reader_utils::read_file_chunks;
use crate::tools::read_definitions::input::ReadDefinitionsToolInput;
use crate::tools::read_definitions::output::{
    ReadDefinitionsToolDefinitionOutput, ReadDefinitionsToolOutput,
};
use crate::tools::read_definitions::repository::ReadDefinitionsRepository;

const FILE_READ_TIMEOUT_SECONDS: u64 = 10;

pub struct ReadDefinitionsService {
    repository: ReadDefinitionsRepository,
}

impl ReadDefinitionsService {
    pub fn new(querying_service: Arc<dyn QueryingService>) -> Self {
        Self {
            repository: ReadDefinitionsRepository::new(querying_service),
        }
    }

    pub async fn read_definitions(
        &self,
        input: ReadDefinitionsToolInput,
    ) -> Result<ReadDefinitionsToolOutput, rmcp::ErrorData> {
        let results = self.repository.query_definitions(input.clone())?;

        if results.is_empty() {
            return Ok(ReadDefinitionsToolOutput::empty(self.get_system_message(
                input,
                vec![],
                0,
                0,
                0,
            )));
        }

        let mut file_chunks = Vec::new();
        for result in &results {
            file_chunks.push((
                result.primary_file_path.clone(),
                result.start_line as usize,
                result.end_line as usize,
            ));
        }

        let file_contents = self.read_definition_chunks(file_chunks).await;

        let mut file_read_errors = Vec::new();
        let mut definitions = Vec::new();
        let mut total_with_body = 0;
        let mut total_with_errors = 0;

        for (index, result) in results.iter().enumerate() {
            let definition_body = match file_contents.get(index) {
                Some(Ok(content)) => {
                    total_with_body += 1;
                    Some(content.trim().to_string())
                }
                Some(Err(_)) => {
                    total_with_errors += 1;
                    file_read_errors.push(result.primary_file_path.clone());
                    None
                }
                None => {
                    total_with_errors += 1;
                    file_read_errors.push(result.primary_file_path.clone());
                    None
                }
            };

            definitions.push(ReadDefinitionsToolDefinitionOutput {
                name: result.name.clone(),
                fqn: result.fqn.clone(),
                definition_type: result.definition_type.clone(),
                location: format!(
                    "{}:L{}-{}",
                    result.primary_file_path, result.start_line, result.end_line
                ),
                definition_body: definition_body.unwrap_or_default(),
            });
        }

        Ok(ReadDefinitionsToolOutput {
            definitions,
            system_message: self.get_system_message(
                input,
                file_read_errors,
                results.len(),
                total_with_body,
                total_with_errors,
            ),
        })
    }

    async fn read_definition_chunks(
        &self,
        file_chunks: Vec<(String, usize, usize)>,
    ) -> Vec<std::io::Result<String>> {
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
                        format!("Failed to read file chunks: {e}."),
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
    }

    fn get_system_message(
        &self,
        input: ReadDefinitionsToolInput,
        file_read_errors: Vec<String>,
        total_found: usize,
        total_with_body: usize,
        total_with_errors: usize,
    ) -> String {
        let mut message = String::new();

        // Document failed file reads
        for (index, file_read_error) in file_read_errors.iter().enumerate() {
            if index == 0 {
                message.push_str("Failed to read some definition bodies:");
            }
            message.push_str(&format!("\n- {file_read_error}."));
            if index == file_read_errors.len() - 1 {
                message.push_str(
                    "\nPerhaps some files were deleted, moved or changed since the last indexing.",
                );
                message.push_str("\nIf the missing definition bodies are important, use the `index_project` tool to re-index the project and try again.\n");
            }
        }

        // Document results summary
        let total_requested = input.definition_requests.len();
        message.push_str(&format!(
            "Processed {total_requested} definition requests, found {total_found} definitions.\n"
        ));

        if total_found > 0 {
            message.push_str(&format!(
                "Successfully read {total_with_body} definition bodies, {total_with_errors} had errors.\n"
            ));

            message.push_str("\nDecision Framework:\n");
            message.push_str("  - If your current task is to understand specific definitions, you can use the returned definition bodies directly.\n");
            message.push_str("  - If you need to find references to these definitions, use the `get_references` tool with the definition names and file paths.\n");
            message.push_str("  - If you need to find related definitions or explore the codebase further, use the `search_codebase_definitions` tool.\n");
        } else {
            message.push_str("No definitions were found for the requested names and file paths.\n");

            message.push_str("\nDecision Framework:\n");
            message.push_str("  - Verify that the definition names and file paths are correct and exact matches.\n");
            message.push_str("  - Use the `search_codebase_definitions` tool to find definitions with similar names.\n");
            message.push_str("  - If you know the definitions exist, use the `index_project` tool to re-index the project and try again.\n");
            message.push_str("  - If you know the definitions exist, and the indexing is up to date, you can stop using the Knowledge Graph for the missing definitions.\n");
        }

        message
    }
}
