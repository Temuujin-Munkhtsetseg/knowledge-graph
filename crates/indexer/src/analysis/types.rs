use database::graph::RelationshipType;
use database::schema::types::NodeFieldAccess;
use parser_core::{
    csharp::types::{CSharpDefinitionType, CSharpFqn, CSharpImportType},
    definitions::DefinitionTypeInfo,
    imports::ImportTypeInfo,
    java::types::{JavaDefinitionType, JavaFqn, JavaImportType},
    kotlin::types::{KotlinDefinitionType, KotlinFqn, KotlinImportType},
    python::types::{PythonDefinitionType, PythonFqn, PythonImportType},
    ruby::types::{RubyDefinitionType, RubyFqn},
    rust::types::{RustDefinitionType, RustFqn, RustImportType},
    typescript::types::{TypeScriptDefinitionType, TypeScriptFqn, TypeScriptImportType},
    utils::{Position, Range},
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

/// Implementation of NodeFieldAccess for DirectoryNode
impl NodeFieldAccess for DirectoryNode {
    fn get_string_field(&self, field_name: &str) -> Option<String> {
        match field_name {
            "path" => Some(self.path.clone()),
            "absolute_path" => Some(self.absolute_path.clone()),
            "repository_name" => Some(self.repository_name.clone()),
            "name" => Some(self.name.clone()),
            _ => None,
        }
    }

    fn get_i32_field(&self, _field_name: &str) -> Option<i32> {
        None // DirectoryNode has no i32 fields
    }

    fn get_id_field<F>(&self, field_name: &str, id_callback: F) -> Option<u32>
    where
        F: FnOnce(&Self) -> u32,
    {
        match field_name {
            "id" => Some(id_callback(self)),
            _ => None,
        }
    }
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

/// Implementation of NodeFieldAccess for FileNode
impl NodeFieldAccess for FileNode {
    fn get_string_field(&self, field_name: &str) -> Option<String> {
        match field_name {
            "path" => Some(self.path.clone()),
            "absolute_path" => Some(self.absolute_path.clone()),
            "language" => Some(self.language.clone()),
            "repository_name" => Some(self.repository_name.clone()),
            "extension" => Some(self.extension.clone()),
            "name" => Some(self.name.clone()),
            _ => None,
        }
    }

    fn get_id_field<F>(&self, field_name: &str, id_callback: F) -> Option<u32>
    where
        F: FnOnce(&Self) -> u32,
    {
        match field_name {
            "id" => Some(id_callback(self)),
            _ => None,
        }
    }
}

/// Represents a single location where a definition or reference call is found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    /// File path where this definition or reference call location is found
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

impl SourceLocation {
    pub fn to_range(&self) -> Range {
        Range::new(
            Position::new(self.start_line as usize, self.start_col as usize),
            Position::new(self.end_line as usize, self.end_col as usize),
            (self.start_byte as usize, self.end_byte as usize),
        )
    }
}

/// Represents a language-specific definition type (e.g. class, module, method, etc.)
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DefinitionType {
    Ruby(RubyDefinitionType),
    Python(PythonDefinitionType),
    Kotlin(KotlinDefinitionType),
    Java(JavaDefinitionType),
    CSharp(CSharpDefinitionType),
    TypeScript(TypeScriptDefinitionType),
    Rust(RustDefinitionType),
    Unsupported(),
}

impl DefinitionType {
    pub fn as_str(&self) -> &str {
        match self {
            DefinitionType::Ruby(ruby_type) => ruby_type.as_str(),
            DefinitionType::Python(python_type) => python_type.as_str(),
            DefinitionType::Kotlin(kotlin_type) => kotlin_type.as_str(),
            DefinitionType::Java(java_type) => java_type.as_str(),
            DefinitionType::CSharp(csharp_type) => csharp_type.as_str(),
            DefinitionType::TypeScript(typescript_type) => typescript_type.as_str(),
            DefinitionType::Rust(rust_type) => rust_type.as_str(),
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
    CSharp(CSharpFqn),
    TypeScript(TypeScriptFqn),
    Rust(RustFqn),
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
    pub location: SourceLocation,
}

impl DefinitionNode {
    /// Create a new DefinitionNode
    pub fn new(
        fqn: String,
        name: String,
        definition_type: DefinitionType,
        location: SourceLocation,
    ) -> Self {
        Self {
            fqn,
            name,
            definition_type,
            location,
        }
    }
}

/// Implementation of NodeFieldAccess for DefinitionNode
impl NodeFieldAccess for DefinitionNode {
    fn get_string_field(&self, field_name: &str) -> Option<String> {
        match field_name {
            "fqn" => Some(self.fqn.clone()),
            "name" => Some(self.name.clone()),
            "definition_type" => Some(self.definition_type.as_str().to_string()),
            "primary_file_path" => Some(self.location.file_path.clone()),
            _ => None,
        }
    }

    fn get_i32_field(&self, field_name: &str) -> Option<i32> {
        match field_name {
            "start_line" => Some(self.location.start_line),
            "end_line" => Some(self.location.end_line),
            "start_col" => Some(self.location.start_col),
            "end_col" => Some(self.location.end_col),
            "total_locations" => Some(1), // Default to 1 for single location
            _ => None,
        }
    }

    fn get_i64_field(&self, field_name: &str) -> Option<i64> {
        match field_name {
            "primary_start_byte" => Some(self.location.start_byte),
            "primary_end_byte" => Some(self.location.end_byte),
            _ => None,
        }
    }

    fn get_id_field<F>(&self, field_name: &str, id_callback: F) -> Option<u32>
    where
        F: FnOnce(&Self) -> u32,
    {
        match field_name {
            "id" => Some(id_callback(self)),
            _ => None,
        }
    }
}

/// Represents a single location where an imported symbol is found
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    Java(JavaImportType),
    Kotlin(KotlinImportType),
    Python(PythonImportType),
    CSharp(CSharpImportType),
    TypeScript(TypeScriptImportType),
    Rust(RustImportType),
}

impl ImportType {
    pub fn as_str(&self) -> &str {
        match self {
            ImportType::Java(java_type) => java_type.as_str(),
            ImportType::Kotlin(kotlin_type) => kotlin_type.as_str(),
            ImportType::Python(python_type) => python_type.as_str(),
            ImportType::CSharp(csharp_type) => csharp_type.as_str(),
            ImportType::TypeScript(typescript_type) => typescript_type.as_str(),
            ImportType::Rust(rust_type) => rust_type.as_str(),
        }
    }
}

/// Represents an identifier associated with an imported symbol
#[derive(Debug, Clone)]
pub struct ImportIdentifier {
    /// Original name, e.g. "foo" in `from module import foo as bar`
    pub name: String,
    /// Alias, e.g. "bar" in `from module import foo as bar`
    pub alias: Option<String>,
}

/// Represents an imported symbol node in the graph
#[derive(Debug, Clone)]
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

/// Implementation of NodeFieldAccess for ImportedSymbolNode
impl NodeFieldAccess for ImportedSymbolNode {
    fn get_string_field(&self, field_name: &str) -> Option<String> {
        match field_name {
            "import_type" => Some(self.import_type.as_str().to_string()),
            "import_path" => Some(self.import_path.clone()),
            "name" => self.identifier.as_ref().map(|id| id.name.clone()),
            "alias" => self.identifier.as_ref().and_then(|id| id.alias.clone()),
            "file_path" => Some(self.location.file_path.clone()),
            _ => None,
        }
    }

    fn get_i32_field(&self, field_name: &str) -> Option<i32> {
        match field_name {
            "start_line" => Some(self.location.start_line),
            "end_line" => Some(self.location.end_line),
            "start_col" => Some(self.location.start_col),
            "end_col" => Some(self.location.end_col),
            _ => None,
        }
    }

    fn get_i64_field(&self, field_name: &str) -> Option<i64> {
        match field_name {
            "start_byte" => Some(self.location.start_byte),
            "end_byte" => Some(self.location.end_byte),
            _ => None,
        }
    }

    fn get_id_field<F>(&self, field_name: &str, id_callback: F) -> Option<u32>
    where
        F: FnOnce(&Self) -> u32,
    {
        match field_name {
            "id" => Some(id_callback(self)),
            _ => None,
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
    /// Definition location (foreign key to DefinitionNode.location)
    pub definition_location: SourceLocation,
}

/// Represents a relationship between a file and an imported symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileImportedSymbolRelationship {
    /// File path (foreign key to FileNode.path)
    pub file_path: String,
    /// Imported symbol location (foreign key to ImportedSymbolNode.location)
    pub import_location: ImportedSymbolLocation,
    /// Type of relationship (import, or a reference)
    pub relationship_type: RelationshipType,
}

/// Represents a hierarchical relationship between definitions
#[derive(Debug, Clone)]
pub struct DefinitionRelationship {
    /// Parent definition file path (foreign key to FileNode.path)
    pub from_file_path: String, // TODO: Drop (we have from_location already)
    /// Child definition file path (foreign key to FileNode.path)
    pub to_file_path: String, // TODO: Drop (we have to_location already)
    /// Parent definition FQN (foreign key to DefinitionNode.fqn)
    pub from_definition_fqn: String,
    /// Child definition FQN (foreign key to DefinitionNode.fqn)
    pub to_definition_fqn: String,
    /// Parent definition location (foreign key to DefinitionNode.location)
    pub from_location: SourceLocation,
    /// Child definition location (foreign key to DefinitionNode.location)
    pub to_location: SourceLocation,
    /// Type of relationship (e.g., "MODULE_TO_CLASS", "CLASS_TO_METHOD", etc.)
    pub relationship_type: RelationshipType,
    /// Optional call-site/source location for reference edges (e.g., Calls)
    pub source_location: Option<SourceLocation>,
}

/// Represents a relationship between a definition and an imported symbol
/// (i.e. when an import is contained in the body of a definition)
#[derive(Debug, Clone)]
pub struct DefinitionImportedSymbolRelationship {
    /// File path that the definition and import that's contained (or referenced) in it
    /// (foreign key to FileNode.path)
    pub file_path: String,
    /// Definition FQN (foreign key to DefinitionNode.fqn)
    pub definition_fqn: String,
    /// Imported symbol location (foreign key to ImportedSymbolNode.location)
    pub imported_symbol_location: ImportedSymbolLocation,
    /// Type of relationship (either "DEFINES_IMPORTED_SYMBOL" or "CALLS_IMPORTED_SYMBOL" for now)
    pub relationship_type: RelationshipType,
    /// Definition location (foreign key to DefinitionNode.location)
    pub definition_location: SourceLocation,
}
