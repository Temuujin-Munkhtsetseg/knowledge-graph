use database::graph::RelationshipType;
use parser_core::{
    definitions::DefinitionTypeInfo,
    kotlin::types::{KotlinDefinitionType, KotlinFqn},
    python::types::{PythonDefinitionType, PythonFqn},
    ruby::{fqn::RubyFqn, types::RubyDefinitionType},
};
use serde::{Deserialize, Serialize};

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
    /// Relative path from repository root
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
    /// Relative path from repository root
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

/// Represents a language-specific definition type (e.g. class, module, method, etc.)
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DefinitionType {
    Ruby(RubyDefinitionType),
    Python(PythonDefinitionType),
    Kotlin(KotlinDefinitionType),
    Unsupported(),
}

impl DefinitionType {
    pub fn as_str(&self) -> &str {
        match self {
            DefinitionType::Ruby(ruby_type) => ruby_type.as_str(),
            DefinitionType::Python(python_type) => python_type.as_str(),
            DefinitionType::Kotlin(kotlin_type) => kotlin_type.as_str(),
            DefinitionType::Unsupported() => "unsupported",
        }
    }
}

/// Represents a language-specific FQN type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FqnType {
    Ruby(RubyFqn),
    Python(PythonFqn),
    Kotlin(KotlinFqn),
}

/// Represents a definition node in the graph (can span multiple files for Ruby modules/classes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionNode {
    /// Fully qualified name (unique identifier)
    pub fqn: String,
    /// Simple name of the definition
    pub name: String,
    /// Type of definition
    pub definition_type: DefinitionType,
    /// File location of the definition
    pub location: DefinitionLocation,
}

impl DefinitionNode {
    /// Create a new DefinitionNode
    pub fn new(
        fqn: String,
        name: String,
        definition_type: DefinitionType,
        location: DefinitionLocation,
    ) -> Self {
        Self {
            fqn,
            name,
            definition_type,
            location,
        }
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
    pub relationship_type: RelationshipType,
}

/// Represents a relationship between a file and a definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDefinitionRelationship {
    /// File path (foreign key to FileNode.path)
    pub file_path: String,
    /// Definition FQN (foreign key to DefinitionNode.fqn)
    pub definition_fqn: String,
    /// Type of relationship (always "DEFINES" for now)
    pub relationship_type: RelationshipType,
}

/// Represents a hierarchical relationship between definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionRelationship {
    /// Parent definition file path (foreign key to FileNode.path)
    pub from_file_path: String,
    /// Child definition file path (foreign key to FileNode.path)
    pub to_file_path: String,
    /// Parent definition FQN (foreign key to DefinitionNode.fqn)
    pub from_definition_fqn: String,
    /// Child definition FQN (foreign key to DefinitionNode.fqn)
    pub to_definition_fqn: String,
    /// Type of relationship (e.g., "MODULE_TO_CLASS", "CLASS_TO_METHOD", etc.)
    pub relationship_type: RelationshipType,
}
