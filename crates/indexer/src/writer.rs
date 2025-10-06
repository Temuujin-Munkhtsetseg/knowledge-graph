use crate::analysis::types::{
    ConsolidatedRelationship, DefinitionNode, DirectoryNode, FileNode, GraphData,
    ImportedSymbolNode, RelationshipKind,
};
use crate::analysis::types::{get_relationships_for_pair, rels_by_kind};
use crate::mutation::utils::{GraphMapper, NodeIdGenerator};
use anyhow::{Context, Error, Result};
use arrow::{datatypes::Schema, record_batch::RecordBatch};
use database::schema::init::RELATIONSHIP_TABLES;
use database::schema::types::{
    ArrowBatchConverter, RelationshipTable, ToArrowBatch, ToArrowRelationshipBatch,
};
use parquet::{arrow::ArrowWriter, basic::Compression, file::properties::WriterProperties};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

/// Writer service for creating Parquet files from graph data
pub struct WriterService {
    output_directory: PathBuf,
}

/// Results of writing graph data to Parquet files
#[derive(Debug, Clone)]
pub struct WriterResult {
    pub files_written: Vec<WrittenFile>,
    pub total_directories: usize,
    pub total_files: usize,
    pub total_definitions: usize,
    pub total_imported_symbols: usize,
    pub total_directory_relationships: usize,
    pub total_file_definition_relationships: usize,
    pub total_file_imported_symbol_relationships: usize,
    pub total_definition_relationships: usize,
    pub total_definition_imported_symbol_relationships: usize,
    pub total_imported_symbol_relationships: usize,
    pub writing_duration: Duration,
}

/// Information about a written Parquet file
#[derive(Debug, Clone)]
pub struct WrittenFile {
    pub file_path: PathBuf,
    pub file_type: String,
    pub record_count: usize,
    pub file_size_bytes: u64,
}

impl WriterService {
    /// Create a new writer service
    pub fn new<P: AsRef<Path>>(output_directory: P) -> Result<Self> {
        let output_directory = output_directory.as_ref().to_path_buf();

        // Create output directory if it doesn't exist
        if !output_directory.exists() {
            std::fs::create_dir_all(&output_directory).with_context(|| {
                format!(
                    "Failed to create output directory: {}",
                    output_directory.display()
                )
            })?;
        }

        Ok(Self { output_directory })
    }

    pub fn flush_output_directory(&self) -> Result<bool, Error> {
        if let Ok(entries) = std::fs::read_dir(&self.output_directory) {
            for entry in entries.flatten() {
                let _ = std::fs::remove_file(entry.path());
            }
        }

        // Check if the output directory is empty
        if let Ok(entries) = std::fs::read_dir(&self.output_directory)
            && entries.flatten().count() == 0
        {
            return Ok(true);
        }
        Ok(false)
    }

    pub fn write_batch_to_parquet(
        &self,
        file_path: &Path,
        schema: Arc<Schema>,
        batch: &RecordBatch,
    ) -> Result<()> {
        // Write to parquet file
        let file = File::create(file_path)
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;

        let props = WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build();

        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
        writer.write(batch)?;
        writer.close()?;
        Ok(())
    }

    /// Write graph data to Parquet files with consolidated relationship schema
    pub fn write_graph_data(
        &self,
        graph_data: &mut GraphData,
        node_id_generator: &mut NodeIdGenerator,
    ) -> Result<WriterResult> {
        let start_time = Instant::now();
        log::info!(
            "Starting to write graph data to Parquet files in directory: {}",
            self.output_directory.display()
        );

        let mut files_written = Vec::new();

        let mut graph_mapper = GraphMapper::new(graph_data, node_id_generator);

        // Pre-assign IDs to all nodes
        graph_mapper.assign_node_ids();

        // Consolidate relationships with assigned IDs
        graph_mapper.assign_relationship_ids()?;

        // WRITE ALL NODES to PARQUET
        let batches = [
            (
                &database::schema::init::DIRECTORY_TABLE,
                ArrowBatchConverter::to_record_batch(
                    &graph_data.directory_nodes,
                    &database::schema::init::DIRECTORY_TABLE,
                    |n: &DirectoryNode| node_id_generator.get_directory_id(&n.path).unwrap_or(0),
                ),
            ),
            (
                &database::schema::init::FILE_TABLE,
                ArrowBatchConverter::to_record_batch(
                    &graph_data.file_nodes,
                    &database::schema::init::FILE_TABLE,
                    |n: &FileNode| node_id_generator.get_file_id(&n.path).unwrap_or(0),
                ),
            ),
            (
                &database::schema::init::DEFINITION_TABLE,
                ArrowBatchConverter::to_record_batch(
                    &graph_data.definition_nodes,
                    &database::schema::init::DEFINITION_TABLE,
                    |n: &DefinitionNode| {
                        node_id_generator
                            .get_definition_id(
                                &n.file_path,
                                n.range.byte_offset.0,
                                n.range.byte_offset.1,
                            )
                            .unwrap_or(0)
                    },
                ),
            ),
            (
                &database::schema::init::IMPORTED_SYMBOL_TABLE,
                ArrowBatchConverter::to_record_batch(
                    &graph_data.imported_symbol_nodes,
                    &database::schema::init::IMPORTED_SYMBOL_TABLE,
                    |n: &ImportedSymbolNode| {
                        node_id_generator
                            .get_imported_symbol_id(
                                &n.location.file_path,
                                n.location.start_byte as usize,
                                n.location.end_byte as usize,
                            )
                            .unwrap_or(0)
                    },
                ),
            ),
        ];

        for (table, batch) in batches {
            let file_path = self.output_directory.join(table.parquet_filename);
            log::info!(
                "Writing {} nodes to Parquet: {}",
                table.name,
                file_path.display()
            );
            match batch {
                Ok(batch) => {
                    if batch.num_rows() == 0 {
                        log::warn!("No nodes to write for {}", table.name);
                        continue;
                    }
                    self.write_batch_to_parquet(&file_path, table.to_arrow_schema(), &batch)?;
                    log::info!(
                        "✅ Successfully wrote {} {} nodes to Parquet",
                        batch.num_rows(),
                        table.name
                    );
                    let file_type =
                        match table.parquet_filename.to_string().strip_suffix(".parquet") {
                            Some(s) => s.to_string(),
                            None => table.parquet_filename.to_string(),
                        };
                    files_written.push(WrittenFile {
                        file_path: file_path.clone(),
                        file_type,
                        record_count: batch.num_rows(),
                        file_size_bytes: self.get_file_size(&file_path)?,
                    });
                }
                Err(e) => {
                    log::error!(
                        "Error converting {} nodes to Arrow batch: {}",
                        table.name,
                        e
                    );
                }
            }
        }

        for table in RELATIONSHIP_TABLES.iter() {
            for (from, to) in table.from_to_pairs {
                let (filename, relationships) =
                    get_relationships_for_pair(&graph_data.relationships, from, to);
                if let Some(filename) = &filename {
                    let file_path = self.output_directory.join(filename);
                    if relationships.is_empty() {
                        continue;
                    }
                    self.write_consolidated_relationships(&file_path, &relationships, table)?;
                    files_written.push(WrittenFile {
                        file_path: file_path.clone(),
                        file_type: filename.clone(),
                        record_count: relationships.len(),
                        file_size_bytes: self.get_file_size(&file_path)?,
                    });
                }
            }
        }

        let writing_duration = start_time.elapsed();

        log::info!(
            "✅ Parquet writing completed in {:?}. Files written: {}",
            writing_duration,
            files_written.len()
        );

        Ok(WriterResult {
            files_written,
            total_directories: graph_data.directory_nodes.len(),
            total_files: graph_data.file_nodes.len(),
            total_definitions: graph_data.definition_nodes.len(),
            total_imported_symbols: graph_data.imported_symbol_nodes.len(),
            total_directory_relationships: rels_by_kind(
                &graph_data.relationships,
                RelationshipKind::DirectoryToDirectory,
            )
            .len()
                + rels_by_kind(&graph_data.relationships, RelationshipKind::DirectoryToFile).len(),
            total_file_definition_relationships: rels_by_kind(
                &graph_data.relationships,
                RelationshipKind::FileToDefinition,
            )
            .len(),
            total_file_imported_symbol_relationships: rels_by_kind(
                &graph_data.relationships,
                RelationshipKind::FileToImportedSymbol,
            )
            .len(),
            total_definition_relationships: rels_by_kind(
                &graph_data.relationships,
                RelationshipKind::DefinitionToDefinition,
            )
            .len(),
            total_definition_imported_symbol_relationships: rels_by_kind(
                &graph_data.relationships,
                RelationshipKind::DefinitionToImportedSymbol,
            )
            .len(),
            total_imported_symbol_relationships: rels_by_kind(
                &graph_data.relationships,
                RelationshipKind::ImportedSymbolToDefinition,
            )
            .len()
                + rels_by_kind(
                    &graph_data.relationships,
                    RelationshipKind::ImportedSymbolToImportedSymbol,
                )
                .len()
                + rels_by_kind(
                    &graph_data.relationships,
                    RelationshipKind::ImportedSymbolToFile,
                )
                .len(),
            writing_duration,
        })
    }

    /// Write consolidated relationships to a Parquet file
    fn write_consolidated_relationships(
        &self,
        file_path: &Path,
        relationships: &[ConsolidatedRelationship],
        table: &RelationshipTable,
    ) -> Result<()> {
        log::info!(
            "Writing {} consolidated relationships to Parquet: {}",
            relationships.len(),
            file_path.display(),
        );

        let batch = ArrowBatchConverter::to_relationship_record_batch(relationships, table)
            .map_err(|e| anyhow::anyhow!("Failed to create Arrow batch: {}", e))?;

        self.write_batch_to_parquet(file_path, table.to_arrow_schema(), &batch)?;

        log::info!(
            "✅ Successfully wrote {} consolidated relationships to Parquet",
            relationships.len()
        );
        Ok(())
    }

    /// Get file size in bytes
    fn get_file_size(&self, file_path: &Path) -> Result<u64> {
        let metadata = std::fs::metadata(file_path)
            .with_context(|| format!("Failed to get metadata for file: {}", file_path.display()))?;
        Ok(metadata.len())
    }
}

impl WriterResult {
    /// Format the writer result as a readable string
    pub fn format_summary(&self) -> String {
        let mut result = String::new();
        result.push_str(&format!(
            "📦 Parquet Writer Summary (completed in {:?}):\n",
            self.writing_duration
        ));
        result.push_str(&format!(
            "  • Total files written: {}\n",
            self.files_written.len()
        ));
        result.push_str(&format!(
            "  • Directory nodes: {}\n",
            self.total_directories
        ));
        result.push_str(&format!("  • File nodes: {}\n", self.total_files));
        result.push_str(&format!(
            "  • Definition nodes: {}\n",
            self.total_definitions
        ));
        result.push_str(&format!(
            "  • Imported symbol nodes: {}\n",
            self.total_imported_symbols
        ));
        result.push_str(&format!(
            "  • Directory relationships: {}\n",
            self.total_directory_relationships
        ));
        result.push_str(&format!(
            "  • File-definition relationships: {}\n",
            self.total_file_definition_relationships
        ));
        result.push_str(&format!(
            "  • File-imported-symbol relationships: {}\n",
            self.total_file_imported_symbol_relationships
        ));
        result.push_str(&format!(
            "  • Definition-definition relationships: {}\n",
            self.total_definition_relationships
        ));
        result.push_str(&format!(
            "  • Imported symbol relationships: {}\n",
            self.total_imported_symbol_relationships
        ));

        if !self.files_written.is_empty() {
            result.push_str("  • Files created:\n");
            for written_file in &self.files_written {
                result.push_str(&format!(
                    "    - {} ({} records, {} bytes)\n",
                    written_file.file_path.display(),
                    written_file.record_count,
                    written_file.file_size_bytes
                ));
            }
        }

        result
    }
}
