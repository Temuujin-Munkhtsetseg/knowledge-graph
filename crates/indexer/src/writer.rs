use crate::analysis::types::{
    DefinitionNode, DirectoryNode, FileNode, GraphData, ImportedSymbolNode,
};
use crate::database::utils::{ConsolidatedRelationship, GraphMapper, NodeIdGenerator};
use anyhow::{Context, Error, Result};
use arrow::{
    array::{Int32Array, Int64Array, StringArray, UInt8Array, UInt32Array},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use database::graph::RelationshipTypeMapping;
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
        if let Ok(entries) = std::fs::read_dir(&self.output_directory) {
            if entries.flatten().count() == 0 {
                return Ok(true);
            }
        }
        Ok(false)
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

        // Write node tables with integer IDs
        if !graph_data.directory_nodes.is_empty() {
            let file_path = self.output_directory.join("directories.parquet");
            let record_count = graph_data.directory_nodes.len();
            self.write_directory_nodes_with_ids(
                &file_path,
                &graph_data.directory_nodes,
                node_id_generator,
            )?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "directories".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        if !graph_data.file_nodes.is_empty() {
            let file_path = self.output_directory.join("files.parquet");
            let record_count = graph_data.file_nodes.len();
            self.write_file_nodes_with_ids(&file_path, &graph_data.file_nodes, node_id_generator)?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "files".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        if !graph_data.definition_nodes.is_empty() {
            let file_path = self.output_directory.join("definitions.parquet");
            let record_count = self.write_definition_nodes_with_ids(
                &file_path,
                &graph_data.definition_nodes,
                node_id_generator,
            )?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "definitions".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        if !graph_data.imported_symbol_nodes.is_empty() {
            let file_path = self.output_directory.join("imported_symbols.parquet");
            let record_count = self.write_imported_symbol_nodes_with_ids(
                &file_path,
                &graph_data.imported_symbol_nodes,
                node_id_generator,
            )?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "imported_symbols".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write directory-to-directory relationships
        if !relationships.directory_to_directory.is_empty() {
            let file_path = self
                .output_directory
                .join("directory_to_directory_relationships.parquet");
            let record_count = relationships.directory_to_directory.len();
            self.write_consolidated_relationships(
                &file_path,
                &relationships.directory_to_directory,
            )?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "directory_to_directory_relationships".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write directory-to-file relationships
        if !relationships.directory_to_file.is_empty() {
            let file_path = self
                .output_directory
                .join("directory_to_file_relationships.parquet");
            let record_count = relationships.directory_to_file.len();
            self.write_consolidated_relationships(&file_path, &relationships.directory_to_file)?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "directory_to_file_relationships".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write file-to-definition relationships (FILE_DEFINES)
        if !relationships.file_to_definition.is_empty() {
            let file_path = self
                .output_directory
                .join("file_to_definition_relationships.parquet");
            let record_count = relationships.file_to_definition.len();
            self.write_consolidated_relationships(&file_path, &relationships.file_to_definition)?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "file_to_definition_relationships".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write file-to-imported-symbol relationships (FILE_IMPORTS)
        if !relationships.file_to_imported_symbol.is_empty() {
            let file_path = self
                .output_directory
                .join("file_to_imported_symbol_relationships.parquet");
            let record_count = relationships.file_to_imported_symbol.len();
            self.write_consolidated_relationships(
                &file_path,
                &relationships.file_to_imported_symbol,
            )?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "file_to_imported_symbol_relationships".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write definition relationships (all MODULE_TO_*, CLASS_TO_*, METHOD_*)
        if !relationships.definition_to_definition.is_empty() {
            let file_path = self
                .output_directory
                .join("definition_to_definition_relationships.parquet");
            let record_count = relationships.definition_to_definition.len();
            self.write_consolidated_relationships(
                &file_path,
                &relationships.definition_to_definition,
            )?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "definition_to_definition_relationships".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write definition-to-imported-symbol relationships (all DEFINES_IMPORTED_SYMBOL)
        if !relationships.definition_to_imported_symbol.is_empty() {
            let file_path = self
                .output_directory
                .join("definition_to_imported_symbol_relationships.parquet");
            let record_count = relationships.definition_to_imported_symbol.len();
            self.write_consolidated_relationships(
                &file_path,
                &relationships.definition_to_imported_symbol,
            )?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "definition_to_imported_symbol_relationships".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
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

    /// Write directory nodes with integer IDs
    fn write_directory_nodes_with_ids(
        &self,
        file_path: &Path,
        directory_nodes: &[DirectoryNode],
        id_generator: &NodeIdGenerator,
    ) -> Result<()> {
        log::info!(
            "Writing {} directory nodes with IDs to Parquet: {}",
            directory_nodes.len(),
            file_path.display()
        );

        // Define Arrow schema for DirectoryNode with ID
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::UInt32, false),
            Field::new("path", DataType::Utf8, false),
            Field::new("absolute_path", DataType::Utf8, false),
            Field::new("repository_name", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        // Convert data to Arrow arrays
        let id_array = UInt32Array::from(
            directory_nodes
                .iter()
                .map(|n| id_generator.get_directory_id(&n.path).unwrap())
                .collect::<Vec<_>>(),
        );
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
                Arc::new(id_array),
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
            "✅ Successfully wrote {} directory nodes with IDs to Parquet",
            directory_nodes.len()
        );
        Ok(())
    }

    /// Write file nodes with integer IDs
    fn write_file_nodes_with_ids(
        &self,
        file_path: &Path,
        file_nodes: &[FileNode],
        id_generator: &NodeIdGenerator,
    ) -> Result<()> {
        log::info!(
            "Writing {} file nodes with IDs to Parquet: {}",
            file_nodes.len(),
            file_path.display()
        );

        // Define Arrow schema for FileNode with ID
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::UInt32, false),
            Field::new("path", DataType::Utf8, false),
            Field::new("absolute_path", DataType::Utf8, false),
            Field::new("language", DataType::Utf8, false),
            Field::new("repository_name", DataType::Utf8, false),
            Field::new("extension", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        // Convert data to Arrow arrays
        let id_array = UInt32Array::from(
            file_nodes
                .iter()
                .map(|n| id_generator.get_file_id(&n.path).unwrap())
                .collect::<Vec<_>>(),
        );
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
                Arc::new(id_array),
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
            "✅ Successfully wrote {} file nodes with IDs to Parquet",
            file_nodes.len()
        );
        Ok(())
    }

    /// Write definition nodes with integer IDs
    fn write_definition_nodes_with_ids(
        &self,
        file_path: &Path,
        definition_nodes: &[DefinitionNode],
        id_generator: &NodeIdGenerator,
    ) -> Result<usize> {
        log::info!(
            "Writing {} definition nodes with IDs to Parquet: {}",
            definition_nodes.len(),
            file_path.display()
        );

        // Create one record per definition using primary location
        let mut id_values = Vec::new();
        let mut fqn_values = Vec::new();
        let mut name_values = Vec::new();
        let mut definition_type_values = Vec::new();
        let mut primary_file_path_values = Vec::new();
        let mut primary_start_byte_values = Vec::new();
        let mut primary_end_byte_values = Vec::new();
        let mut start_line_values = Vec::new();
        let mut end_line_values = Vec::new();
        let mut start_col_values = Vec::new();
        let mut end_col_values = Vec::new();
        let mut total_locations_values = Vec::new();

        for definition_node in definition_nodes {
            let location = definition_node.location.clone();
            id_values.push(
                id_generator
                    .get_definition_id(&location.file_path, &location.to_range())
                    .unwrap(),
            );
            fqn_values.push(definition_node.fqn.as_str());
            name_values.push(definition_node.name.as_str());
            definition_type_values.push(definition_node.definition_type.as_str());
            primary_file_path_values.push(location.file_path.clone());
            primary_start_byte_values.push(location.start_byte);
            primary_end_byte_values.push(location.end_byte);
            start_line_values.push(location.start_line);
            end_line_values.push(location.end_line);
            start_col_values.push(location.start_col);
            end_col_values.push(location.end_col);
            total_locations_values.push(1);
        }

        let total_records = fqn_values.len();
        log::info!("Created {total_records} definition records (one per unique FQN)");

        // Define Arrow schema matching the database schema with ID
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::UInt32, false),
            Field::new("fqn", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("definition_type", DataType::Utf8, false),
            Field::new("primary_file_path", DataType::Utf8, false),
            Field::new("primary_start_byte", DataType::Int64, false),
            Field::new("primary_end_byte", DataType::Int64, false),
            Field::new("start_line", DataType::Int32, false),
            Field::new("end_line", DataType::Int32, false),
            Field::new("start_col", DataType::Int32, false),
            Field::new("end_col", DataType::Int32, false),
            Field::new("total_locations", DataType::Int32, false),
        ]));

        // Convert data to Arrow arrays
        let id_array = UInt32Array::from(id_values);
        let fqn_array = StringArray::from(fqn_values);
        let name_array = StringArray::from(name_values);
        let definition_type_array = StringArray::from(definition_type_values);
        let primary_file_path_array = StringArray::from(primary_file_path_values);
        let primary_start_byte_array = Int64Array::from(primary_start_byte_values);
        let primary_end_byte_array = Int64Array::from(primary_end_byte_values);
        let start_line_array = Int32Array::from(start_line_values);
        let end_line_array = Int32Array::from(end_line_values);
        let start_col_array = Int32Array::from(start_col_values);
        let end_col_array = Int32Array::from(end_col_values);
        let total_locations_array = Int32Array::from(total_locations_values);

        // Create record batch
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id_array),
                Arc::new(fqn_array),
                Arc::new(name_array),
                Arc::new(definition_type_array),
                Arc::new(primary_file_path_array),
                Arc::new(primary_start_byte_array),
                Arc::new(primary_end_byte_array),
                Arc::new(start_line_array),
                Arc::new(end_line_array),
                Arc::new(start_col_array),
                Arc::new(end_col_array),
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

        log::info!("✅ Successfully wrote {total_records} definition records to Parquet");
        Ok(total_records)
    }

    /// Write imported symbol nodes with integer IDs
    fn write_imported_symbol_nodes_with_ids(
        &self,
        file_path: &Path,
        imported_symbol_nodes: &[ImportedSymbolNode],
        id_generator: &NodeIdGenerator,
    ) -> Result<usize> {
        log::info!(
            "Writing {} imported symbol nodes with IDs to Parquet: {}",
            imported_symbol_nodes.len(),
            file_path.display()
        );

        // Create one record per imported symbol
        let mut id_values = Vec::new();
        let mut import_type_values = Vec::new();
        let mut import_path_values = Vec::new();
        let mut name_values = Vec::new();
        let mut alias_values = Vec::new();
        let mut file_path_values = Vec::new();
        let mut start_byte_values = Vec::new();
        let mut end_byte_values = Vec::new();
        let mut start_line_values = Vec::new();
        let mut end_line_values = Vec::new();
        let mut start_col_values = Vec::new();
        let mut end_col_values = Vec::new();

        for imported_symbol_node in imported_symbol_nodes {
            let location = imported_symbol_node.location.clone();
            let identifier = &imported_symbol_node.identifier;

            id_values.push(id_generator.get_imported_symbol_id(&location).unwrap());
            import_type_values.push(imported_symbol_node.import_type.as_str());
            import_path_values.push(imported_symbol_node.import_path.clone());
            file_path_values.push(location.file_path.clone());
            start_byte_values.push(location.start_byte);
            end_byte_values.push(location.end_byte);
            start_line_values.push(location.start_line);
            end_line_values.push(location.end_line);
            start_col_values.push(location.start_col);
            end_col_values.push(location.end_col);

            if !identifier.is_some() {
                name_values.push(None);
                alias_values.push(None);
            } else {
                name_values.push(Some(identifier.as_ref().unwrap().name.clone()));
                alias_values.push(identifier.as_ref().unwrap().alias.clone());
            }
        }

        let total_records = id_values.len();
        log::info!("Created {total_records} imported symbol records");

        // Define Arrow schema matching the database schema with ID
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::UInt32, false),
            Field::new("import_type", DataType::Utf8, false),
            Field::new("import_path", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("alias", DataType::Utf8, true),
            Field::new("file_path", DataType::Utf8, false),
            Field::new("start_byte", DataType::Int64, false),
            Field::new("end_byte", DataType::Int64, false),
            Field::new("start_line", DataType::Int32, false),
            Field::new("end_line", DataType::Int32, false),
            Field::new("start_col", DataType::Int32, false),
            Field::new("end_col", DataType::Int32, false),
        ]));

        // Convert data to Arrow arrays
        let id_array = UInt32Array::from(id_values);
        let import_type_array = StringArray::from(import_type_values);
        let import_path_array = StringArray::from(import_path_values);
        let name_array = StringArray::from(name_values);
        let alias_array = StringArray::from(alias_values);
        let file_path_array = StringArray::from(file_path_values);
        let start_byte_array = Int64Array::from(start_byte_values);
        let end_byte_array = Int64Array::from(end_byte_values);
        let start_line_array = Int32Array::from(start_line_values);
        let end_line_array = Int32Array::from(end_line_values);
        let start_col_array = Int32Array::from(start_col_values);
        let end_col_array = Int32Array::from(end_col_values);

        // Create record batch
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id_array),
                Arc::new(import_type_array),
                Arc::new(import_path_array),
                Arc::new(name_array),
                Arc::new(alias_array),
                Arc::new(file_path_array),
                Arc::new(start_byte_array),
                Arc::new(end_byte_array),
                Arc::new(start_line_array),
                Arc::new(end_line_array),
                Arc::new(start_col_array),
                Arc::new(end_col_array),
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

        log::info!("✅ Successfully wrote {total_records} imported symbol records to Parquet");
        Ok(total_records)
    }

    /// Write consolidated relationships to a Parquet file
    fn write_consolidated_relationships(
        &self,
        file_path: &Path,
        relationships: &[ConsolidatedRelationship],
    ) -> Result<()> {
        log::info!(
            "Writing {} consolidated relationships to Parquet: {}",
            relationships.len(),
            file_path.display()
        );

        // Define Arrow schema for consolidated relationships
        let schema = Arc::new(Schema::new(vec![
            Field::new("source_id", DataType::UInt32, false),
            Field::new("target_id", DataType::UInt32, false),
            Field::new("type", DataType::UInt8, false),
        ]));

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

        // Create record batch
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(source_id_array),
                Arc::new(target_id_array),
                Arc::new(type_array),
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
