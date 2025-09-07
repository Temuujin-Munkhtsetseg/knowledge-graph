use crate::analysis::types::{
    DefinitionNode, DirectoryNode, FileNode, GraphData, ImportedSymbolNode,
};
use crate::mutation::types::ConsolidatedRelationship;
use crate::mutation::utils::{GraphMapper, NodeIdGenerator};
use anyhow::{Context, Error, Result};
use arrow::{
    array::{Int32Array, Int64Array, UInt8Array, UInt32Array},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use database::graph::RelationshipTypeMapping;
use database::schema::init::RELATIONSHIP_TABLES;
use database::schema::types::{ArrowBatchConverter, ToArrowBatch};
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
        graph_data: &GraphData,
        node_id_generator: &mut NodeIdGenerator,
    ) -> Result<WriterResult> {
        let start_time = Instant::now();
        log::info!(
            "Starting to write graph data to Parquet files in directory: {}",
            self.output_directory.display()
        );

        let mut files_written = Vec::new();
        let mut relationship_mapping = RelationshipTypeMapping::new();

        let mut graph_mapper =
            GraphMapper::new(graph_data, node_id_generator, &mut relationship_mapping);
        let relationships = graph_mapper.map_graph_data()?;

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
                        let range = n.location.to_range();
                        node_id_generator
                            .get_definition_id(&n.location.file_path, &range)
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
                            .get_imported_symbol_id(&n.location)
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

        // WRITE ALL RELATIONSHIPS to PARQUET
        for table in RELATIONSHIP_TABLES.iter() {
            for (from_to, filename) in table.get_parquet_filenames_from_pairs() {
                let (from, to) = from_to;
                let relationships = match (from.name, to.name) {
                    // Write directory-to-directory relationships
                    ("DirectoryNode", "DirectoryNode") => {
                        (&relationships.directory_to_directory, &filename)
                    }
                    // Write directory-to-file relationships
                    ("DirectoryNode", "FileNode") => (&relationships.directory_to_file, &filename),
                    // Write file-to-definition relationships (FILE_DEFINES)
                    ("FileNode", "DefinitionNode") => {
                        (&relationships.file_to_definition, &filename)
                    }
                    // Write file-to-imported-symbol relationships (FILE_IMPORTS)
                    ("FileNode", "ImportedSymbolNode") => {
                        (&relationships.file_to_imported_symbol, &filename)
                    }
                    // Write definition-to-definition relationships (all MODULE_TO_*, CLASS_TO_*, METHOD_*)
                    ("DefinitionNode", "DefinitionNode") => {
                        (&relationships.definition_to_definition, &filename)
                    }
                    // Write definition-to-imported-symbol relationships (all DEFINES_IMPORTED_SYMBOL)
                    ("DefinitionNode", "ImportedSymbolNode") => {
                        (&relationships.definition_to_imported_symbol, &filename)
                    }
                    _ => (&Vec::new(), &filename),
                };

                if !relationships.0.is_empty() {
                    let file_path = self.output_directory.join(filename.clone());
                    // DIRECTORY_RELATIONSHIPS keeps minimal schema; others have extended optional location columns
                    let with_location = table.name != "DIRECTORY_RELATIONSHIPS";
                    self.write_consolidated_relationships(
                        &file_path,
                        relationships.0,
                        with_location,
                    )?;
                    files_written.push(WrittenFile {
                        file_path: file_path.clone(),
                        file_type: filename.clone(),
                        record_count: relationships.0.len(),
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
            total_directory_relationships: relationships.directory_to_directory.len()
                + relationships.directory_to_file.len(),
            total_file_definition_relationships: relationships.file_to_definition.len(),
            total_file_imported_symbol_relationships: relationships.file_to_imported_symbol.len(),
            total_definition_relationships: relationships.definition_to_definition.len(),
            total_definition_imported_symbol_relationships: relationships
                .definition_to_imported_symbol
                .len(),
            writing_duration,
        })
    }

    /// Write consolidated relationships to a Parquet file
    fn write_consolidated_relationships(
        &self,
        file_path: &Path,
        relationships: &[ConsolidatedRelationship],
        with_location: bool,
    ) -> Result<()> {
        log::info!(
            "Writing {} consolidated relationships to Parquet: {} (with_location={})",
            relationships.len(),
            file_path.display(),
            with_location
        );
        // Define Arrow schema for consolidated relationships
        let schema = if with_location {
            Arc::new(Schema::new(vec![
                Field::new("source_id", DataType::UInt32, false),
                Field::new("target_id", DataType::UInt32, false),
                Field::new("type", DataType::UInt8, false),
                Field::new("source_start_byte", DataType::Int64, true),
                Field::new("source_end_byte", DataType::Int64, true),
                Field::new("source_start_line", DataType::Int32, true),
                Field::new("source_end_line", DataType::Int32, true),
                Field::new("source_start_col", DataType::Int32, true),
                Field::new("source_end_col", DataType::Int32, true),
            ]))
        } else {
            Arc::new(Schema::new(vec![
                Field::new("source_id", DataType::UInt32, false),
                Field::new("target_id", DataType::UInt32, false),
                Field::new("type", DataType::UInt8, false),
            ]))
        };

        // Convert data to Arrow arrays
        let source_id_array = UInt32Array::from(
            relationships
                .iter()
                .map(|r| r.source_id)
                .collect::<Vec<_>>(),
        );
        let target_id_array = UInt32Array::from(
            relationships
                .iter()
                .map(|r| r.target_id)
                .collect::<Vec<_>>(),
        );
        let type_array = UInt8Array::from(
            relationships
                .iter()
                .map(|r| r.relationship_type)
                .collect::<Vec<_>>(),
        );

        let batch = if with_location {
            let start_byte_array = Int64Array::from(
                relationships
                    .iter()
                    .map(|r| r.start_byte.map(|v| v as i64))
                    .collect::<Vec<_>>(),
            );
            let end_byte_array = Int64Array::from(
                relationships
                    .iter()
                    .map(|r| r.end_byte.map(|v| v as i64))
                    .collect::<Vec<_>>(),
            );
            let start_line_array = Int32Array::from(
                relationships
                    .iter()
                    .map(|r| r.start_line.map(|v| v as i32))
                    .collect::<Vec<_>>(),
            );
            let end_line_array = Int32Array::from(
                relationships
                    .iter()
                    .map(|r| r.end_line.map(|v| v as i32))
                    .collect::<Vec<_>>(),
            );
            let start_col_array = Int32Array::from(
                relationships
                    .iter()
                    .map(|r| r.start_column.map(|v| v as i32))
                    .collect::<Vec<_>>(),
            );
            let end_col_array = Int32Array::from(
                relationships
                    .iter()
                    .map(|r| r.end_column.map(|v| v as i32))
                    .collect::<Vec<_>>(),
            );

            RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(source_id_array),
                    Arc::new(target_id_array),
                    Arc::new(type_array),
                    Arc::new(start_byte_array),
                    Arc::new(end_byte_array),
                    Arc::new(start_line_array),
                    Arc::new(end_line_array),
                    Arc::new(start_col_array),
                    Arc::new(end_col_array),
                ],
            )?
        } else {
            RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(source_id_array),
                    Arc::new(target_id_array),
                    Arc::new(type_array),
                ],
            )?
        };

        self.write_batch_to_parquet(file_path, schema, &batch)?;

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
            "  • Definition-imported-symbol relationships: {}\n",
            self.total_definition_imported_symbol_relationships
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
