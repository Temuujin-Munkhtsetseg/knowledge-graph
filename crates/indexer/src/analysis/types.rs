use database::graph::RelationshipType;
use parser_core::fqn::{FQNPart, Fqn};
use parser_core::{
    definitions::DefinitionTypeInfo,
    imports::ImportTypeInfo,
    java::types::{JavaDefinitionType, JavaFqn},
    kotlin::types::{KotlinDefinitionType, KotlinFqn},
    python::types::{PythonDefinitionType, PythonFqn, PythonImportType},
    ruby::{fqn::RubyFqn, types::RubyDefinitionType},
    typescript::types::{TypeScriptDefinitionType, TypeScriptImportType},
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
    /// Imported symbol nodes to be written to imported_symbols.parquet
    pub imported_symbol_nodes: Vec<ImportedSymbolNode>,
    /// Directory relationships to be written to directory_relationships.parquet
    pub directory_relationships: Vec<DirectoryRelationship>,
    /// File-to-imported-symbol relationships to be written to file_imported_symbol_relationships.parquet
    pub file_imported_symbol_relationships: Vec<FileImportedSymbolRelationship>,
    /// File-to-definition relationships to be written to file_definition_relationships.parquet
    pub file_definition_relationships: Vec<FileDefinitionRelationship>,
    /// Definition-to-definition relationships to be written to definition_relationships.parquet
    pub definition_relationships: Vec<DefinitionRelationship>,
    /// Definition-to-imported-symbol relationships to be written to definition_imported_symbol_relationships.parquet
    pub definition_imported_symbol_relationships: Vec<DefinitionImportedSymbolRelationship>,
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
    /// Start line number
    pub start_line: i32,
    /// End line number
    pub end_line: i32,
    /// Start column number
    pub start_col: i32,
    /// End column number
    pub end_col: i32,
}

/// Represents a language-specific definition type (e.g. class, module, method, etc.)
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DefinitionType {
    Ruby(RubyDefinitionType),
    Python(PythonDefinitionType),
    Kotlin(KotlinDefinitionType),
    Java(JavaDefinitionType),
    TypeScript(TypeScriptDefinitionType),
    Unsupported(),
}

impl DefinitionType {
    pub fn as_str(&self) -> &str {
        match self {
            DefinitionType::Ruby(ruby_type) => ruby_type.as_str(),
            DefinitionType::Python(python_type) => python_type.as_str(),
            DefinitionType::Kotlin(kotlin_type) => kotlin_type.as_str(),
            DefinitionType::Java(java_type) => java_type.as_str(),
            DefinitionType::TypeScript(typescript_type) => typescript_type.as_str(),
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
    Java(JavaFqn),
    TypeScript(Fqn<FQNPart>),
}

/// Represents a definition node in the graph
#[derive(Debug, Clone)]
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

/// Represents a single location where an imported symbol is found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedSymbolLocation {
    /// File path where this symbol was imported
    pub file_path: String,
    /// Start byte position in the file
    pub start_byte: i64,
    /// End byte position in the file  
    pub end_byte: i64,
    /// Start line number
    pub start_line: i32,
    /// End line number
    pub end_line: i32,
    /// Start column
    pub start_col: i32,
    /// End column
    pub end_col: i32,
}

/// Represents a language-specific import type
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ImportType {
    Python(PythonImportType),
    TypeScript(TypeScriptImportType),
}

impl ImportType {
    pub fn as_str(&self) -> &str {
        match self {
            ImportType::Python(python_type) => python_type.as_str(),
            ImportType::TypeScript(typescript_type) => typescript_type.as_str(),
        }
    }
}

/// Represents an identifier associated with an imported symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportIdentifier {
    /// Original name, e.g. "foo" in `from module import foo as bar`
    pub name: String,
    /// Alias, e.g. "bar" in `from module import foo as bar`
    pub alias: Option<String>,
}

/// Represents an imported symbol node in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedSymbolNode {
    /// Language-specific type of import (regular, from, aliased, wildcard, etc.)
    pub import_type: ImportType,
    /// The import path as specified in the source code
    /// e.g., "./my_module", "react", "../utils"
    pub import_path: String,
    /// Information about the imported identifier(s)
    /// None for side-effect imports like `import "./styles.css"`
    pub identifier: Option<ImportIdentifier>,
    /// Location of the enclosing import statement
    pub location: ImportedSymbolLocation,
}

impl ImportedSymbolNode {
    /// Create a new ImportedSymbolNode
    pub fn new(
        import_type: ImportType,
        import_path: String,
        identifier: Option<ImportIdentifier>,
        location: ImportedSymbolLocation,
    ) -> Self {
        Self {
            import_type,
            import_path,
            identifier,
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

/// Represents a relationship between a file and an imported symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileImportedSymbolRelationship {
    /// File path (foreign key to FileNode.path)
    pub file_path: String,
    /// Imported symbol location (foreign key to ImportedSymbolNode.location)
    pub import_location: ImportedSymbolLocation,
    /// Type of relationship (always "FILE_IMPORTS" for now)
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

/// Represents a relationship between a definition and an imported symbol
/// (i.e. when an import is contained in the body of a definition)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionImportedSymbolRelationship {
    /// File path that the definition and import are contained in
    /// (foreign key to FileNode.path)
    pub file_path: String,
    /// Definition FQN (foreign key to DefinitionNode.fqn)
    pub definition_fqn: String,
    /// Imported symbol location (foreign key to ImportedSymbolNode.location)
    pub imported_symbol_location: ImportedSymbolLocation,
    /// Type of relationship (always "DEFINES_IMPORTED_SYMBOL" for now)
    pub relationship_type: RelationshipType,
}
