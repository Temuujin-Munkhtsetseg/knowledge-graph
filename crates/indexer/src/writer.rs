use crate::analysis::{DefinitionNode, DirectoryNode, FileNode, GraphData};
use anyhow::{Context, Result};
use arrow::{
    array::{Int32Array, Int64Array, StringArray, UInt8Array, UInt32Array},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use parquet::{arrow::ArrowWriter, basic::Compression, file::properties::WriterProperties};
use parser_core::definitions::DefinitionTypeInfo;
use serde_json;
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

/// Relationship type mappings for efficient storage
#[derive(Debug, Clone)]
pub struct RelationshipTypeMapping {
    /// Map from relationship type string to integer ID
    type_to_id: HashMap<String, u8>,
    /// Map from integer ID to relationship type string
    id_to_type: HashMap<u8, String>,
    /// Next available ID
    next_id: u8,
}

impl Default for RelationshipTypeMapping {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationshipTypeMapping {
    pub fn new() -> Self {
        let mut mapping = Self {
            type_to_id: HashMap::new(),
            id_to_type: HashMap::new(),
            next_id: 1, // Start from 1, reserve 0 for unknown/default
        };

        // Pre-register known relationship types
        mapping.register_known_types();
        mapping
    }

    fn register_known_types(&mut self) {
        let known_types = vec![
            // Directory relationships
            "DIR_CONTAINS_DIR",
            "DIR_CONTAINS_FILE",
            // File relationships
            "FILE_DEFINES",
            // Definition relationships - Module
            "MODULE_TO_CLASS",
            "MODULE_TO_MODULE",
            "MODULE_TO_METHOD",
            "MODULE_TO_SINGLETON_METHOD",
            "MODULE_TO_CONSTANT",
            "MODULE_TO_LAMBDA",
            "MODULE_TO_PROC",
            // Definition relationships - Class
            "CLASS_TO_METHOD",
            "CLASS_TO_ATTRIBUTE",
            "CLASS_TO_CONSTANT",
            "CLASS_INHERITS_FROM",
            "CLASS_TO_SINGLETON_METHOD",
            "CLASS_TO_CLASS",
            "CLASS_TO_LAMBDA",
            "CLASS_TO_PROC",
            // Definition relationships - Method
            "METHOD_CALLS",
            "METHOD_TO_BLOCK",
            "SINGLETON_METHOD_TO_BLOCK",
        ];

        for rel_type in known_types {
            self.register_type(rel_type);
        }
    }

    pub fn register_type(&mut self, type_name: &str) -> u8 {
        if let Some(&id) = self.type_to_id.get(type_name) {
            return id;
        }

        let id = self.next_id;
        self.type_to_id.insert(type_name.to_string(), id);
        self.id_to_type.insert(id, type_name.to_string());
        self.next_id += 1;

        if self.next_id == 0 {
            panic!("Relationship type ID overflow! Consider using UINT16 instead of UINT8");
        }

        id
    }

    pub fn get_type_id(&self, type_name: &str) -> Option<u8> {
        self.type_to_id.get(type_name).copied()
    }

    pub fn get_type_name(&self, type_id: u8) -> Option<&String> {
        self.id_to_type.get(&type_id)
    }
}

/// Node ID generator for assigning integer IDs to nodes
#[derive(Debug, Clone)]
pub struct NodeIdGenerator {
    /// Directory path to ID mapping
    directory_ids: HashMap<String, u32>,
    /// File path to ID mapping
    file_ids: HashMap<String, u32>,
    /// Definition FQN to ID mapping
    definition_ids: HashMap<String, u32>,
    /// Next available IDs for each type
    next_directory_id: u32,
    next_file_id: u32,
    next_definition_id: u32,
}

impl Default for NodeIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeIdGenerator {
    pub fn new() -> Self {
        Self {
            directory_ids: HashMap::new(),
            file_ids: HashMap::new(),
            definition_ids: HashMap::new(),
            next_directory_id: 1,
            next_file_id: 1,
            next_definition_id: 1,
        }
    }

    pub fn get_or_assign_directory_id(&mut self, path: &str) -> u32 {
        if let Some(&id) = self.directory_ids.get(path) {
            return id;
        }

        let id = self.next_directory_id;
        self.directory_ids.insert(path.to_string(), id);
        self.next_directory_id += 1;
        id
    }

    pub fn get_or_assign_file_id(&mut self, path: &str) -> u32 {
        if let Some(&id) = self.file_ids.get(path) {
            return id;
        }

        let id = self.next_file_id;
        self.file_ids.insert(path.to_string(), id);
        self.next_file_id += 1;
        id
    }

    pub fn get_or_assign_definition_id(&mut self, fqn: &str) -> u32 {
        if let Some(&id) = self.definition_ids.get(fqn) {
            return id;
        }

        let id = self.next_definition_id;
        self.definition_ids.insert(fqn.to_string(), id);
        self.next_definition_id += 1;
        id
    }

    pub fn get_directory_id(&self, path: &str) -> Option<u32> {
        self.directory_ids.get(path).copied()
    }

    pub fn get_file_id(&self, path: &str) -> Option<u32> {
        self.file_ids.get(path).copied()
    }

    pub fn get_definition_id(&self, fqn: &str) -> Option<u32> {
        self.definition_ids.get(fqn).copied()
    }
}

/// Consolidated relationship data for efficient storage
#[derive(Debug, Clone)]
pub struct ConsolidatedRelationship {
    pub source_id: u32,
    pub target_id: u32,
    pub relationship_type: u8,
}

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

    /// Write graph data to Parquet files with consolidated relationship schema
    pub fn write_graph_data(&self, graph_data: &GraphData) -> Result<WriterResult> {
        let start_time = Instant::now();
        log::info!(
            "Starting to write graph data to Parquet files in directory: {}",
            self.output_directory.display()
        );

        let mut files_written = Vec::new();
        let mut node_id_generator = NodeIdGenerator::new();
        let mut relationship_mapping = RelationshipTypeMapping::new();

        // Pre-assign IDs to all nodes
        self.assign_node_ids(&mut node_id_generator, graph_data)?;

        // Write node tables with integer IDs
        if !graph_data.directory_nodes.is_empty() {
            let file_path = self.output_directory.join("directories.parquet");
            let record_count = graph_data.directory_nodes.len();
            self.write_directory_nodes_with_ids(
                &file_path,
                &graph_data.directory_nodes,
                &node_id_generator,
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
            self.write_file_nodes_with_ids(&file_path, &graph_data.file_nodes, &node_id_generator)?;

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
                &node_id_generator,
            )?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "definitions".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write consolidated relationship tables
        let (dir_rels, file_rels, def_rels) = self.consolidate_relationships(
            graph_data,
            &node_id_generator,
            &mut relationship_mapping,
        )?;

        // Write directory relationships (DIR_CONTAINS_DIR + DIR_CONTAINS_FILE)
        if !dir_rels.is_empty() {
            let file_path = self
                .output_directory
                .join("directory_relationships.parquet");
            let record_count = dir_rels.len();
            self.write_consolidated_relationships(&file_path, &dir_rels)?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "directory_relationships".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write file relationships (FILE_DEFINES)
        if !file_rels.is_empty() {
            let file_path = self.output_directory.join("file_relationships.parquet");
            let record_count = file_rels.len();
            self.write_consolidated_relationships(&file_path, &file_rels)?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "file_relationships".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write definition relationships (all MODULE_TO_*, CLASS_TO_*, METHOD_*)
        if !def_rels.is_empty() {
            let file_path = self
                .output_directory
                .join("definition_relationships.parquet");
            let record_count = def_rels.len();
            self.write_consolidated_relationships(&file_path, &def_rels)?;

            files_written.push(WrittenFile {
                file_path: file_path.clone(),
                file_type: "definition_relationships".to_string(),
                record_count,
                file_size_bytes: self.get_file_size(&file_path)?,
            });
        }

        // Write relationship type mapping for reference
        self.write_relationship_mapping(&relationship_mapping)?;

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
            total_directory_relationships: dir_rels.len(),
            total_file_definition_relationships: file_rels.len(),
            total_definition_relationships: def_rels.len(),
            writing_duration,
        })
    }

    /// Pre-assign integer IDs to all nodes
    fn assign_node_ids(
        &self,
        id_generator: &mut NodeIdGenerator,
        graph_data: &GraphData,
    ) -> Result<()> {
        // Assign directory IDs
        for dir_node in &graph_data.directory_nodes {
            id_generator.get_or_assign_directory_id(&dir_node.path);
        }

        // Assign file IDs
        for file_node in &graph_data.file_nodes {
            id_generator.get_or_assign_file_id(&file_node.path);
        }

        // Assign definition IDs
        for def_node in &graph_data.definition_nodes {
            id_generator.get_or_assign_definition_id(&def_node.fqn);
        }

        Ok(())
    }

    /// Consolidate all relationships into three categories with integer IDs and types
    fn consolidate_relationships(
        &self,
        graph_data: &GraphData,
        id_generator: &NodeIdGenerator,
        relationship_mapping: &mut RelationshipTypeMapping,
    ) -> Result<(
        Vec<ConsolidatedRelationship>,
        Vec<ConsolidatedRelationship>,
        Vec<ConsolidatedRelationship>,
    )> {
        let mut directory_relationships = Vec::new();
        let mut file_relationships = Vec::new();
        let mut definition_relationships = Vec::new();

        // Process directory relationships
        for dir_rel in &graph_data.directory_relationships {
            let source_id = id_generator
                .get_directory_id(&dir_rel.from_path)
                .ok_or_else(|| {
                    anyhow::anyhow!("Source directory ID not found: {}", dir_rel.from_path)
                })?;

            let relationship_type = relationship_mapping.register_type(&dir_rel.relationship_type);

            if dir_rel.relationship_type == "DIR_CONTAINS_DIR" {
                let target_id =
                    id_generator
                        .get_directory_id(&dir_rel.to_path)
                        .ok_or_else(|| {
                            anyhow::anyhow!("Target directory ID not found: {}", dir_rel.to_path)
                        })?;

                directory_relationships.push(ConsolidatedRelationship {
                    source_id,
                    target_id,
                    relationship_type,
                });
            } else if dir_rel.relationship_type == "DIR_CONTAINS_FILE" {
                let target_id = id_generator.get_file_id(&dir_rel.to_path).ok_or_else(|| {
                    anyhow::anyhow!("Target file ID not found: {}", dir_rel.to_path)
                })?;

                directory_relationships.push(ConsolidatedRelationship {
                    source_id,
                    target_id,
                    relationship_type,
                });
            }
        }

        // Process file-definition relationships
        for file_rel in &graph_data.file_definition_relationships {
            let source_id = id_generator
                .get_file_id(&file_rel.file_path)
                .ok_or_else(|| {
                    anyhow::anyhow!("Source file ID not found: {}", file_rel.file_path)
                })?;
            let target_id = id_generator
                .get_definition_id(&file_rel.definition_fqn)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Target definition ID not found: {}",
                        file_rel.definition_fqn
                    )
                })?;
            let relationship_type = relationship_mapping.register_type(&file_rel.relationship_type);

            file_relationships.push(ConsolidatedRelationship {
                source_id,
                target_id,
                relationship_type,
            });
        }

        // Process definition relationships
        for def_rel in &graph_data.definition_relationships {
            let source_id = id_generator
                .get_definition_id(&def_rel.from_definition_fqn)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Source definition ID not found: {}",
                        def_rel.from_definition_fqn
                    )
                })?;
            let target_id = id_generator
                .get_definition_id(&def_rel.to_definition_fqn)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Target definition ID not found: {}",
                        def_rel.to_definition_fqn
                    )
                })?;
            let relationship_type = relationship_mapping.register_type(&def_rel.relationship_type);

            definition_relationships.push(ConsolidatedRelationship {
                source_id,
                target_id,
                relationship_type,
            });
        }

        Ok((
            directory_relationships,
            file_relationships,
            definition_relationships,
        ))
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
        let mut primary_line_number_values = Vec::new();
        let mut total_locations_values = Vec::new();

        for definition_node in definition_nodes {
            let total_locations = definition_node.file_locations.len() as i32;

            // Use primary (first) location for the record
            if let Some(primary_location) = definition_node.primary_location() {
                id_values.push(
                    id_generator
                        .get_definition_id(&definition_node.fqn)
                        .unwrap(),
                );
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
            Field::new("primary_line_number", DataType::Int32, false),
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
        let primary_line_number_array = Int32Array::from(primary_line_number_values);
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

        log::info!("✅ Successfully wrote {total_records} definition records to Parquet");
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

    /// Write relationship type mapping for reference
    fn write_relationship_mapping(&self, mapping: &RelationshipTypeMapping) -> Result<()> {
        let file_path = self.output_directory.join("relationship_types.json");

        let mapping_data: HashMap<u8, String> = mapping.id_to_type.clone();
        let json_data = serde_json::to_string_pretty(&mapping_data)
            .context("Failed to serialize relationship mapping")?;

        std::fs::write(&file_path, json_data).with_context(|| {
            format!(
                "Failed to write relationship mapping: {}",
                file_path.display()
            )
        })?;

        log::info!(
            "✅ Successfully wrote relationship type mapping with {} types",
            mapping_data.len()
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
