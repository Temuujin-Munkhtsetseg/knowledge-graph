pub mod files;
pub mod ruby;

use crate::parsing::file::FileProcessingResult;
use parser_core::{definitions::DefinitionTypeInfo, ruby::types::RubyDefinitionType};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

// Re-export the sub-module functionality
pub use files::FileSystemAnalyzer;
pub use ruby::RubyAnalyzer;

/// Structured graph data ready for writing to Parquet files
#[derive(Debug, Clone)]
pub struct GraphData {
    /// Directory nodes to be written to directories.parquet
    pub directory_nodes: Vec<DirectoryNode>,
    /// File nodes to be written to files.parquet
    pub file_nodes: Vec<FileNode>,
    /// Definition nodes to be written to definitions.parquet  
    pub definition_nodes: Vec<DefinitionNode>,
    /// Directory relationships to be written to directory_relationships.parquet
    pub directory_relationships: Vec<DirectoryRelationship>,
    /// File-to-definition relationships to be written to file_definition_relationships.parquet
    pub file_definition_relationships: Vec<FileDefinitionRelationship>,
    /// Definition-to-definition relationships to be written to definition_relationships.parquet
    pub definition_relationships: Vec<DefinitionRelationship>,
}

/// Represents a directory node in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryNode {
    /// Primary key: relative path from repository root
    pub path: String,
    /// Absolute path on filesystem
    pub absolute_path: String,
    /// Repository name
    pub repository_name: String,
    /// Directory name (last component of path)
    pub name: String,
}

/// Represents a file node in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    /// Primary key: relative path from repository root
    pub path: String,
    /// Absolute path on filesystem
    pub absolute_path: String,
    /// Programming language detected
    pub language: String,
    /// Repository name
    pub repository_name: String,
    /// File extension
    pub extension: String,
    /// File name (last component of path)
    pub name: String,
}

/// Represents a single location where a definition is found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionLocation {
    /// File path where this definition location is found
    pub file_path: String,
    /// Start byte position in the file
    pub start_byte: i64,
    /// End byte position in the file  
    pub end_byte: i64,
    /// Line number where definition starts
    pub line_number: i32,
}

/// Represents a definition node in the graph (can span multiple files for Ruby modules/classes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionNode {
    /// Primary key: fully qualified name
    pub fqn: String,
    /// Simple name of the definition
    pub name: String,
    /// Type of definition (Class, Module, Method, etc.)
    pub definition_type: RubyDefinitionType,
    /// All file locations where this definition is found (for Ruby module reopening)
    pub file_locations: Vec<DefinitionLocation>,
}

impl DefinitionNode {
    /// Create a new DefinitionNode with a single location
    pub fn new(
        fqn: String,
        name: String,
        definition_type: RubyDefinitionType,
        location: DefinitionLocation,
    ) -> Self {
        Self {
            fqn,
            name,
            definition_type,
            file_locations: vec![location],
        }
    }

    /// Add a new file location to this definition (for Ruby module reopening)
    pub fn add_location(&mut self, location: DefinitionLocation) {
        self.file_locations.push(location);
    }

    /// Get the primary (first) file location for this definition
    pub fn primary_location(&self) -> Option<&DefinitionLocation> {
        self.file_locations.first()
    }
}

/// Represents a relationship between directories and files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryRelationship {
    /// Source path (directory path)
    pub from_path: String,
    /// Target path (directory or file path)
    pub to_path: String,
    /// Type of relationship ("DIR_CONTAINS_DIR" or "DIR_CONTAINS_FILE")
    pub relationship_type: String,
}

/// Represents a relationship between a file and a definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDefinitionRelationship {
    /// File path (foreign key to FileNode.path)
    pub file_path: String,
    /// Definition FQN (foreign key to DefinitionNode.fqn)
    pub definition_fqn: String,
    /// Type of relationship (always "DEFINES" for now)
    pub relationship_type: String,
}

/// Represents a hierarchical relationship between definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionRelationship {
    /// Parent definition FQN (foreign key to DefinitionNode.fqn)
    pub from_definition_fqn: String,
    /// Child definition FQN (foreign key to DefinitionNode.fqn)
    pub to_definition_fqn: String,
    /// Type of relationship (e.g., "MODULE_TO_CLASS", "CLASS_TO_METHOD", etc.)
    pub relationship_type: String,
}

/// Analysis service that orchestrates the transformation of parsing results into graph data
pub struct AnalysisService {
    repository_name: String,
    repository_path: String,
    filesystem_analyzer: FileSystemAnalyzer,
    ruby_analyzer: RubyAnalyzer,
}

impl AnalysisService {
    /// Create a new analysis service
    pub fn new(repository_name: String, repository_path: String) -> Self {
        let filesystem_analyzer =
            FileSystemAnalyzer::new(repository_name.clone(), repository_path.clone());
        let ruby_analyzer = RubyAnalyzer::new();

        Self {
            repository_name,
            repository_path,
            filesystem_analyzer,
            ruby_analyzer,
        }
    }

    /// Analyze file processing results and transform them into graph data
    pub fn analyze_results(
        &self,
        file_results: &Vec<FileProcessingResult>,
    ) -> Result<GraphData, String> {
        let start_time = Instant::now();
        log::info!(
            "Starting analysis of {} file results for repository '{}' at '{}'",
            file_results.len(),
            self.repository_name,
            self.repository_path
        );

        let mut directory_nodes = Vec::new();
        let mut file_nodes = Vec::new();
        let mut directory_relationships = Vec::new();
        let mut file_definition_relationships = Vec::new();
        let mut definition_relationships = Vec::new();

        // Track created directories and merged definitions
        let mut created_directories = HashSet::new();
        let mut created_dir_relationships = HashSet::new();
        let mut merged_definitions = HashMap::new();

        for file_result in file_results {
            // Create directory nodes and relationships for this file's path
            self.filesystem_analyzer.create_directory_hierarchy(
                &file_result.file_path,
                &mut directory_nodes,
                &mut directory_relationships,
                &mut created_directories,
                &mut created_dir_relationships,
            )?;

            // Create file node
            let file_node = self.filesystem_analyzer.create_file_node(file_result)?;

            // Store the relative path before moving file_node
            let relative_file_path = file_node.path.clone();

            // Create directory-to-file relationship using the same relative path as the FileNode
            if let Some(parent_dir) = self
                .filesystem_analyzer
                .get_parent_directory(&file_result.file_path)
            {
                directory_relationships.push(DirectoryRelationship {
                    from_path: parent_dir,
                    to_path: relative_file_path.clone(),
                    relationship_type: "DIR_CONTAINS_FILE".to_string(),
                });
            }

            file_nodes.push(file_node);

            // Process definitions from this file (currently Ruby-specific)
            if file_result.is_supported {
                // Pass the relative path to ensure consistency with FileNode primary keys
                self.ruby_analyzer.process_definitions(
                    file_result,
                    &relative_file_path,
                    &mut merged_definitions,
                    &mut file_definition_relationships,
                )?;
            }
        }

        // Extract final definition nodes and create relationships
        let definition_nodes = self.ruby_analyzer.finalize_definitions_and_relationships(
            merged_definitions,
            &mut definition_relationships,
        );

        let analysis_time = start_time.elapsed();
        log::info!(
            "Analysis completed in {:?}: {} directories, {} files, {} definitions ({} total locations), {} total relationships",
            analysis_time,
            directory_nodes.len(),
            file_nodes.len(),
            definition_nodes.len(),
            definition_nodes.iter().map(|d| d.file_locations.len()).sum::<usize>(),
            directory_relationships.len() + file_definition_relationships.len() + definition_relationships.len()
        );

        Ok(GraphData {
            directory_nodes,
            file_nodes,
            definition_nodes,
            directory_relationships,
            file_definition_relationships,
            definition_relationships,
        })
    }
}

/// Analysis statistics
#[derive(Debug, Clone)]
pub struct AnalysisStats {
    pub total_directories_created: usize,
    pub total_files_analyzed: usize,
    pub total_definitions_created: usize,
    pub total_directory_relationships: usize,
    pub total_file_definition_relationships: usize,
    pub total_definition_relationships: usize,
    pub analysis_duration: Duration,
    pub files_by_language: HashMap<String, usize>,
    pub definitions_by_type: HashMap<String, usize>,
    pub relationships_by_type: HashMap<String, usize>,
}

impl AnalysisStats {
    /// Create analysis statistics from graph data
    pub fn from_graph_data(graph_data: &GraphData, analysis_duration: Duration) -> Self {
        let mut files_by_language = HashMap::new();
        let mut definitions_by_type = HashMap::new();
        let mut relationships_by_type = HashMap::new();

        // Count files by language
        for file_node in &graph_data.file_nodes {
            *files_by_language
                .entry(file_node.language.clone())
                .or_insert(0) += 1;
        }

        // Count definitions by type
        for definition_node in &graph_data.definition_nodes {
            *definitions_by_type
                .entry(definition_node.definition_type.as_str().to_string())
                .or_insert(0) += 1;
        }

        // Count relationships by type
        for rel in &graph_data.directory_relationships {
            *relationships_by_type
                .entry(rel.relationship_type.clone())
                .or_insert(0) += 1;
        }
        for rel in &graph_data.file_definition_relationships {
            *relationships_by_type
                .entry(rel.relationship_type.clone())
                .or_insert(0) += 1;
        }
        for rel in &graph_data.definition_relationships {
            *relationships_by_type
                .entry(rel.relationship_type.clone())
                .or_insert(0) += 1;
        }

        Self {
            total_directories_created: graph_data.directory_nodes.len(),
            total_files_analyzed: graph_data.file_nodes.len(),
            total_definitions_created: graph_data.definition_nodes.len(),
            total_directory_relationships: graph_data.directory_relationships.len(),
            total_file_definition_relationships: graph_data.file_definition_relationships.len(),
            total_definition_relationships: graph_data.definition_relationships.len(),
            analysis_duration,
            files_by_language,
            definitions_by_type,
            relationships_by_type,
        }
    }

    /// Format statistics as a readable string
    pub fn format_stats(&self) -> String {
        let mut result = String::new();
        result.push_str(&format!(
            "📊 Analysis Statistics (completed in {:?}):\n",
            self.analysis_duration
        ));
        result.push_str(&format!(
            "  • Directories created: {}\n",
            self.total_directories_created
        ));
        result.push_str(&format!(
            "  • Files analyzed: {}\n",
            self.total_files_analyzed
        ));
        result.push_str(&format!(
            "  • Definitions created: {}\n",
            self.total_definitions_created
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

        if !self.files_by_language.is_empty() {
            result.push_str("  • Files by language:\n");
            for (language, count) in &self.files_by_language {
                result.push_str(&format!("    - {}: {}\n", language, count));
            }
        }

        if !self.definitions_by_type.is_empty() {
            result.push_str("  • Definitions by type:\n");
            for (def_type, count) in &self.definitions_by_type {
                result.push_str(&format!("    - {}: {}\n", def_type, count));
            }
        }

        if !self.relationships_by_type.is_empty() {
            result.push_str("  • Relationships by type:\n");
            for (rel_type, count) in &self.relationships_by_type {
                result.push_str(&format!("    - {}: {}\n", rel_type, count));
            }
        }

        result
    }
}
