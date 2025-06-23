use crate::analysis::{
    DefinitionNode, DefinitionRelationship, DirectoryNode, DirectoryRelationship,
    FileDefinitionRelationship, FileNode, GraphData,
};
use anyhow::{Context, Result};
use arrow::{
    array::{Int32Array, Int64Array, StringArray},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use parquet::{arrow::ArrowWriter, basic::Compression, file::properties::WriterProperties};
use parser_core::definitions::DefinitionTypeInfo;
use std::{
    collections::HashMap,
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
    pub total_directory_relationships: usize,
    pub total_file_definition_relationships: usize,
    pub total_definition_relationships: usize,
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

    /// Write graph data to Parquet files
    pub fn write_graph_data(&self, graph_data: &GraphData) -> Result<WriterResult> {
        let start_time = Instant::now();
        log::info!(
            "Starting to write graph data to Parquet files in directory: {}",
            self.output_directory.display()
        );

        let mut files_written = Vec::new();

        // Write directory nodes
        if !graph_data.directory_nodes.is_empty() {
            let file_path = self.output_directory.join("directories.parquet");
            let record_count = graph_data.directory_nodes.len();
            self.write_directory_nodes(&file_path, &graph_data.directory_nodes)?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "directories".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write file nodes
        if !graph_data.file_nodes.is_empty() {
            let file_path = self.output_directory.join("files.parquet");
            let record_count = graph_data.file_nodes.len();
            self.write_file_nodes(&file_path, &graph_data.file_nodes)?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "files".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write definition nodes
        if !graph_data.definition_nodes.is_empty() {
            let file_path = self.output_directory.join("definitions.parquet");
            let record_count =
                self.write_definition_nodes(&file_path, &graph_data.definition_nodes)?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "definitions".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write directory relationships (separated by type)
        if !graph_data.directory_relationships.is_empty() {
            // Separate relationships by type
            let dir_to_dir_rels: Vec<DirectoryRelationship> = graph_data
                .directory_relationships
                .iter()
                .filter(|r| r.relationship_type == "DIR_CONTAINS_DIR")
                .cloned()
                .collect();
            let dir_to_file_rels: Vec<DirectoryRelationship> = graph_data
                .directory_relationships
                .iter()
                .filter(|r| r.relationship_type == "DIR_CONTAINS_FILE")
                .cloned()
                .collect();

            // Write DIR_CONTAINS_DIR relationships
            if !dir_to_dir_rels.is_empty() {
                let file_path = self.output_directory.join("dir_contains_dir.parquet");
                let record_count = dir_to_dir_rels.len();
                self.write_directory_relationships(&file_path, &dir_to_dir_rels)?;

                files_written.push(WrittenFile {
                    file_path: file_path.clone(),
                    file_type: "dir_contains_dir".to_string(),
                    record_count,
                    file_size_bytes: self.get_file_size(&file_path)?,
                });
            }

            // Write DIR_CONTAINS_FILE relationships
            if !dir_to_file_rels.is_empty() {
                let file_path = self.output_directory.join("dir_contains_file.parquet");
                let record_count = dir_to_file_rels.len();
                self.write_directory_relationships(&file_path, &dir_to_file_rels)?;

                files_written.push(WrittenFile {
                    file_path: file_path.clone(),
                    file_type: "dir_contains_file".to_string(),
                    record_count,
                    file_size_bytes: self.get_file_size(&file_path)?,
                });
            }
        }

        // Write file-definition relationships
        if !graph_data.file_definition_relationships.is_empty() {
            let file_path = self
                .output_directory
                .join("file_definition_relationships.parquet");
            let record_count = graph_data.file_definition_relationships.len();
            self.write_file_definition_relationships(
                &file_path,
                &graph_data.file_definition_relationships,
            )?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "file_definition_relationships".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write definition relationships (separated by type)
        if !graph_data.definition_relationships.is_empty() {
            let mut rels_by_type: HashMap<String, Vec<DefinitionRelationship>> = HashMap::new();
            for rel in &graph_data.definition_relationships {
                rels_by_type
                    .entry(rel.relationship_type.clone())
                    .or_default()
                    .push(rel.clone());
            }

            for (rel_type, rels) in rels_by_type {
                let file_name = format!("{}.parquet", rel_type.to_lowercase());
                let file_path = self.output_directory.join(&file_name);
                let record_count = rels.len();

                // This specialized function writes only from/to columns
                self.write_definition_relationships_for_type(&file_path, &rels)?;

                files_written.push(WrittenFile {
                    file_path: file_path.clone(),
                    file_type: rel_type,
                    record_count,
                    file_size_bytes: self.get_file_size(&file_path)?,
                });
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
            total_directory_relationships: graph_data.directory_relationships.len(),
            total_file_definition_relationships: graph_data.file_definition_relationships.len(),
            total_definition_relationships: graph_data.definition_relationships.len(),
            writing_duration,
        })
    }

    /// Write directory nodes to a Parquet file
    fn write_directory_nodes(
        &self,
        file_path: &Path,
        directory_nodes: &[DirectoryNode],
    ) -> Result<()> {
        log::info!(
            "Writing {} directory nodes to Parquet: {}",
            directory_nodes.len(),
            file_path.display()
        );

        // Define Arrow schema for DirectoryNode
        let schema = Arc::new(Schema::new(vec![
            Field::new("path", DataType::Utf8, false),
            Field::new("absolute_path", DataType::Utf8, false),
            Field::new("repository_name", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        // Convert data to Arrow arrays
        let path_array = StringArray::from(
            directory_nodes
                .iter()
                .map(|n| n.path.as_str())
                .collect::<Vec<_>>(),
        );
        let absolute_path_array = StringArray::from(
            directory_nodes
                .iter()
                .map(|n| n.absolute_path.as_str())
                .collect::<Vec<_>>(),
        );
        let repository_name_array = StringArray::from(
            directory_nodes
                .iter()
                .map(|n| n.repository_name.as_str())
                .collect::<Vec<_>>(),
        );
        let name_array = StringArray::from(
            directory_nodes
                .iter()
                .map(|n| n.name.as_str())
                .collect::<Vec<_>>(),
        );

        // Create record batch
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(path_array),
                Arc::new(absolute_path_array),
                Arc::new(repository_name_array),
                Arc::new(name_array),
            ],
        )?;

        // Write to parquet file
        let file = File::create(file_path)
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;

        let props = WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build();

        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        log::info!(
            "✅ Successfully wrote {} directory nodes to Parquet",
            directory_nodes.len()
        );
        Ok(())
    }

    /// Write file nodes to a Parquet file
    fn write_file_nodes(&self, file_path: &Path, file_nodes: &[FileNode]) -> Result<()> {
        log::info!(
            "Writing {} file nodes to Parquet: {}",
            file_nodes.len(),
            file_path.display()
        );

        // Define Arrow schema for FileNode
        let schema = Arc::new(Schema::new(vec![
            Field::new("path", DataType::Utf8, false),
            Field::new("absolute_path", DataType::Utf8, false),
            Field::new("language", DataType::Utf8, false),
            Field::new("repository_name", DataType::Utf8, false),
            Field::new("extension", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        // Convert data to Arrow arrays
        let path_array = StringArray::from(
            file_nodes
                .iter()
                .map(|n| n.path.as_str())
                .collect::<Vec<_>>(),
        );
        let absolute_path_array = StringArray::from(
            file_nodes
                .iter()
                .map(|n| n.absolute_path.as_str())
                .collect::<Vec<_>>(),
        );
        let language_array = StringArray::from(
            file_nodes
                .iter()
                .map(|n| n.language.as_str())
                .collect::<Vec<_>>(),
        );
        let repository_name_array = StringArray::from(
            file_nodes
                .iter()
                .map(|n| n.repository_name.as_str())
                .collect::<Vec<_>>(),
        );
        let extension_array = StringArray::from(
            file_nodes
                .iter()
                .map(|n| n.extension.as_str())
                .collect::<Vec<_>>(),
        );
        let name_array = StringArray::from(
            file_nodes
                .iter()
                .map(|n| n.name.as_str())
                .collect::<Vec<_>>(),
        );

        // Create record batch
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(path_array),
                Arc::new(absolute_path_array),
                Arc::new(language_array),
                Arc::new(repository_name_array),
                Arc::new(extension_array),
                Arc::new(name_array),
            ],
        )?;

        // Write to parquet file
        let file = File::create(file_path)
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;

        let props = WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build();

        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        log::info!(
            "✅ Successfully wrote {} file nodes to Parquet",
            file_nodes.len()
        );
        Ok(())
    }

    /// Write definition nodes to a Parquet file
    fn write_definition_nodes(
        &self,
        file_path: &Path,
        definition_nodes: &[DefinitionNode],
    ) -> Result<usize> {
        log::info!(
            "Writing {} definition nodes to Parquet: {}",
            definition_nodes.len(),
            file_path.display()
        );

        // Create one record per definition using primary location
        let mut fqn_values = Vec::new();
        let mut name_values = Vec::new();
        let mut definition_type_values = Vec::new();
        let mut primary_file_path_values = Vec::new();
        let mut primary_start_byte_values = Vec::new();
        let mut primary_end_byte_values = Vec::new();
        let mut primary_line_number_values = Vec::new();
        let mut total_locations_values = Vec::new();

        for definition_node in definition_nodes {
            let total_locations = definition_node.file_locations.len() as i32;

            // Use primary (first) location for the record
            if let Some(primary_location) = definition_node.primary_location() {
                fqn_values.push(definition_node.fqn.as_str());
                name_values.push(definition_node.name.as_str());
                definition_type_values.push(definition_node.definition_type.as_str());
                primary_file_path_values.push(primary_location.file_path.as_str());
                primary_start_byte_values.push(primary_location.start_byte);
                primary_end_byte_values.push(primary_location.end_byte);
                primary_line_number_values.push(primary_location.line_number);
                total_locations_values.push(total_locations);
            } else {
                log::warn!(
                    "Definition '{}' has no locations, skipping",
                    definition_node.fqn
                );
            }
        }

        let total_records = fqn_values.len();
        log::info!(
            "Created {} definition records (one per unique FQN)",
            total_records
        );

        // Define Arrow schema matching the database schema
        let schema = Arc::new(Schema::new(vec![
            Field::new("fqn", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("definition_type", DataType::Utf8, false),
            Field::new("primary_file_path", DataType::Utf8, false),
            Field::new("primary_start_byte", DataType::Int64, false),
            Field::new("primary_end_byte", DataType::Int64, false),
            Field::new("primary_line_number", DataType::Int32, false),
            Field::new("total_locations", DataType::Int32, false),
        ]));

        // Convert data to Arrow arrays
        let fqn_array = StringArray::from(fqn_values);
        let name_array = StringArray::from(name_values);
        let definition_type_array = StringArray::from(definition_type_values);
        let primary_file_path_array = StringArray::from(primary_file_path_values);
        let primary_start_byte_array = Int64Array::from(primary_start_byte_values);
        let primary_end_byte_array = Int64Array::from(primary_end_byte_values);
        let primary_line_number_array = Int32Array::from(primary_line_number_values);
        let total_locations_array = Int32Array::from(total_locations_values);

        // Create record batch
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(fqn_array),
                Arc::new(name_array),
                Arc::new(definition_type_array),
                Arc::new(primary_file_path_array),
                Arc::new(primary_start_byte_array),
                Arc::new(primary_end_byte_array),
                Arc::new(primary_line_number_array),
                Arc::new(total_locations_array),
            ],
        )?;

        // Write to parquet file
        let file = File::create(file_path)
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;

        let props = WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build();

        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        log::info!(
            "✅ Successfully wrote {} definition records to Parquet",
            total_records
        );
        Ok(total_records)
    }

    /// Write directory relationships to a Parquet file
    fn write_directory_relationships(
        &self,
        file_path: &Path,
        relationships: &[DirectoryRelationship],
    ) -> Result<()> {
        log::info!(
            "Writing {} directory relationships to Parquet: {}",
            relationships.len(),
            file_path.display()
        );

        // Define Arrow schema for DirectoryRelationship
        let schema = Arc::new(Schema::new(vec![
            Field::new("from_path", DataType::Utf8, false),
            Field::new("to_path", DataType::Utf8, false),
        ]));

        // Convert data to Arrow arrays
        let from_path_array = StringArray::from(
            relationships
                .iter()
                .map(|r| r.from_path.as_str())
                .collect::<Vec<_>>(),
        );
        let to_path_array = StringArray::from(
            relationships
                .iter()
                .map(|r| r.to_path.as_str())
                .collect::<Vec<_>>(),
        );

        // Create record batch
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(from_path_array), Arc::new(to_path_array)],
        )?;

        // Write to parquet file
        let file = File::create(file_path)
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;

        let props = WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build();

        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        log::info!(
            "✅ Successfully wrote {} directory relationships to Parquet",
            relationships.len()
        );
        Ok(())
    }

    /// Write file-definition relationships to a Parquet file
    fn write_file_definition_relationships(
        &self,
        file_path: &Path,
        relationships: &[FileDefinitionRelationship],
    ) -> Result<()> {
        log::info!(
            "Writing {} file-definition relationships to Parquet: {}",
            relationships.len(),
            file_path.display()
        );

        // Define Arrow schema for FileDefinitionRelationship (edges table)
        let schema = Arc::new(Schema::new(vec![
            Field::new("file_path", DataType::Utf8, false),
            Field::new("definition_fqn", DataType::Utf8, false),
            Field::new("relationship_type", DataType::Utf8, false),
        ]));

        // Convert data to Arrow arrays
        let file_path_array = StringArray::from(
            relationships
                .iter()
                .map(|r| r.file_path.as_str())
                .collect::<Vec<_>>(),
        );
        let definition_fqn_array = StringArray::from(
            relationships
                .iter()
                .map(|r| r.definition_fqn.as_str())
                .collect::<Vec<_>>(),
        );
        let relationship_type_array = StringArray::from(
            relationships
                .iter()
                .map(|r| r.relationship_type.as_str())
                .collect::<Vec<_>>(),
        );

        // Create record batch
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(file_path_array),
                Arc::new(definition_fqn_array),
                Arc::new(relationship_type_array),
            ],
        )?;

        // Write to parquet file
        let file = File::create(file_path)
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;

        let props = WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build();

        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        log::info!(
            "✅ Successfully wrote {} file-definition relationships to Parquet",
            relationships.len()
        );
        Ok(())
    }

    /// Write definition relationships for a specific type to a Parquet file (from/to only)
    fn write_definition_relationships_for_type(
        &self,
        file_path: &Path,
        relationships: &[DefinitionRelationship],
    ) -> Result<()> {
        log::info!(
            "Writing {} definition relationships to Parquet: {}",
            relationships.len(),
            file_path.display()
        );

        // Define Arrow schema for DefinitionRelationship (from/to only)
        let schema = Arc::new(Schema::new(vec![
            Field::new("from_definition_fqn", DataType::Utf8, false),
            Field::new("to_definition_fqn", DataType::Utf8, false),
        ]));

        // Convert data to Arrow arrays
        let from_definition_fqn_array = StringArray::from(
            relationships
                .iter()
                .map(|r| r.from_definition_fqn.as_str())
                .collect::<Vec<_>>(),
        );
        let to_definition_fqn_array = StringArray::from(
            relationships
                .iter()
                .map(|r| r.to_definition_fqn.as_str())
                .collect::<Vec<_>>(),
        );

        // Create record batch
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(from_definition_fqn_array),
                Arc::new(to_definition_fqn_array),
            ],
        )?;

        // Write to parquet file
        let file = File::create(file_path)
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;

        let props = WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build();

        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        log::info!(
            "✅ Successfully wrote {} definition relationships to Parquet",
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
            "  • Directory relationships: {}\n",
            self.total_directory_relationships
        ));
        result.push_str(&format!(
            "  • File-definition relationships: {}\n",
            self.total_file_definition_relationships
        ));
        result.push_str(&format!(
            "  • Definition relationships: {}\n",
            self.total_definition_relationships
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
