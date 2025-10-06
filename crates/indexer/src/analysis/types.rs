use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use internment::ArcIntern;

use database::graph::RelationshipType;
use database::schema::types::{NodeFieldAccess, NodeTable};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RelationshipKind {
    DirectoryToDirectory,
    DirectoryToFile,
    FileToDefinition,
    FileToImportedSymbol,
    DefinitionToDefinition,
    DefinitionToImportedSymbol,
    ImportedSymbolToImportedSymbol,
    ImportedSymbolToDefinition,
    ImportedSymbolToFile,
    #[default]
    Empty,
}

impl RelationshipKind {
    pub fn as_str(&self) -> &str {
        match self {
            RelationshipKind::DirectoryToDirectory => "DIR_CONTAINS_DIR",
            RelationshipKind::DirectoryToFile => "DIR_CONTAINS_FILE",
            RelationshipKind::FileToDefinition => "FILE_DEFINES",
            RelationshipKind::FileToImportedSymbol => "FILE_IMPORTS",
            RelationshipKind::DefinitionToDefinition => "DEFINES_DEFINITION",
            RelationshipKind::DefinitionToImportedSymbol => "DEFINES_IMPORTED_SYMBOL",
            RelationshipKind::ImportedSymbolToImportedSymbol => {
                "IMPORTED_SYMBOL_TO_IMPORTED_SYMBOL"
            }
            RelationshipKind::ImportedSymbolToDefinition => "IMPORTED_SYMBOL_TO_DEFINITION",
            RelationshipKind::ImportedSymbolToFile => "IMPORTED_SYMBOL_TO_FILE",
            RelationshipKind::Empty => "EMPTY",
        }
    }
}

/// Consolidated relationship data for efficient storage
#[derive(Debug, Clone)]
pub struct ConsolidatedRelationship {
    pub kind: RelationshipKind,
    pub source_id: Option<u32>,
    pub target_id: Option<u32>,
    pub relationship_type: RelationshipType,
    pub source_path: Option<ArcIntern<String>>,
    pub target_path: Option<ArcIntern<String>>,
    pub source_range: ArcIntern<Range>,
    pub target_range: ArcIntern<Range>,
    /// Definition location for source node (used for ID lookup)
    pub source_definition_range: Option<ArcIntern<Range>>,
    /// Definition location for target node (used for ID lookup)  
    pub target_definition_range: Option<ArcIntern<Range>>,
}

impl Default for ConsolidatedRelationship {
    fn default() -> Self {
        Self {
            kind: RelationshipKind::Empty,
            source_id: None,
            target_id: None,
            relationship_type: RelationshipType::Empty,
            source_path: None,
            target_path: None,
            source_range: ArcIntern::new(Range::empty()),
            target_range: ArcIntern::new(Range::empty()),
            source_definition_range: None,
            target_definition_range: None,
        }
    }
}

impl ConsolidatedRelationship {
    pub fn dir_to_dir(from_path: String, to_path: String) -> Self {
        Self {
            source_path: Some(ArcIntern::new(from_path)),
            target_path: Some(ArcIntern::new(to_path)),
            kind: RelationshipKind::DirectoryToDirectory,
            ..Default::default()
        }
    }

    pub fn dir_to_file(from_path: String, to_path: String) -> Self {
        Self {
            source_path: Some(ArcIntern::new(from_path)),
            target_path: Some(ArcIntern::new(to_path)),
            kind: RelationshipKind::DirectoryToFile,
            ..Default::default()
        }
    }

    pub fn import_to_import(from_path: String, to_path: String) -> Self {
        Self {
            source_path: Some(ArcIntern::new(from_path)),
            target_path: Some(ArcIntern::new(to_path)),
            kind: RelationshipKind::ImportedSymbolToImportedSymbol,
            ..Default::default()
        }
    }

    pub fn import_to_definition(from_path: String, to_path: String) -> Self {
        Self {
            source_path: Some(ArcIntern::new(from_path)),
            target_path: Some(ArcIntern::new(to_path)),
            kind: RelationshipKind::ImportedSymbolToDefinition,
            ..Default::default()
        }
    }

    pub fn import_to_file(from_path: String, to_path: String) -> Self {
        Self {
            source_path: Some(ArcIntern::new(from_path)),
            target_path: Some(ArcIntern::new(to_path)),
            kind: RelationshipKind::ImportedSymbolToFile,
            ..Default::default()
        }
    }

    pub fn definition_to_definition(from_path: String, to_path: String) -> Self {
        Self {
            source_path: Some(ArcIntern::new(from_path)),
            target_path: Some(ArcIntern::new(to_path)),
            kind: RelationshipKind::DefinitionToDefinition,
            ..Default::default()
        }
    }

    pub fn file_to_definition(from_path: String, to_path: String) -> Self {
        Self {
            source_path: Some(ArcIntern::new(from_path)),
            target_path: Some(ArcIntern::new(to_path)),
            kind: RelationshipKind::FileToDefinition,
            ..Default::default()
        }
    }

    pub fn file_to_imported_symbol(from_path: String, to_path: String) -> Self {
        Self {
            source_path: Some(ArcIntern::new(from_path)),
            target_path: Some(ArcIntern::new(to_path)),
            kind: RelationshipKind::FileToImportedSymbol,
            ..Default::default()
        }
    }

    pub fn definition_to_imported_symbol(from_path: String, to_path: String) -> Self {
        Self {
            source_path: Some(ArcIntern::new(from_path)),
            target_path: Some(ArcIntern::new(to_path)),
            kind: RelationshipKind::DefinitionToImportedSymbol,
            ..Default::default()
        }
    }
}

pub fn rels_by_kind(
    relationships: &[ConsolidatedRelationship],
    kind: RelationshipKind,
) -> Vec<ConsolidatedRelationship> {
    relationships
        .iter()
        .filter(|rel| rel.kind == kind)
        .cloned()
        .collect()
}

pub fn get_relationships_for_pair(
    relationships: &[ConsolidatedRelationship],
    from_table: &NodeTable,
    to_table: &NodeTable,
) -> (Option<String>, Vec<ConsolidatedRelationship>) {
    let filename = from_table.relationship_filename(to_table);
    match (from_table.name, to_table.name) {
        ("DirectoryNode", "DirectoryNode") => (
            Some(filename),
            rels_by_kind(relationships, RelationshipKind::DirectoryToDirectory),
        ),
        ("DirectoryNode", "FileNode") => (
            Some(filename),
            rels_by_kind(relationships, RelationshipKind::DirectoryToFile),
        ),
        ("FileNode", "DefinitionNode") => (
            Some(filename),
            rels_by_kind(relationships, RelationshipKind::FileToDefinition),
        ),
        ("FileNode", "ImportedSymbolNode") => (
            Some(filename),
            rels_by_kind(relationships, RelationshipKind::FileToImportedSymbol),
        ),
        ("DefinitionNode", "DefinitionNode") => (
            Some(filename),
            rels_by_kind(relationships, RelationshipKind::DefinitionToDefinition),
        ),
        ("DefinitionNode", "ImportedSymbolNode") => (
            Some(filename),
            rels_by_kind(relationships, RelationshipKind::DefinitionToImportedSymbol),
        ),
        ("ImportedSymbolNode", "ImportedSymbolNode") => (
            Some(filename),
            rels_by_kind(
                relationships,
                RelationshipKind::ImportedSymbolToImportedSymbol,
            ),
        ),
        ("ImportedSymbolNode", "DefinitionNode") => (
            Some(filename),
            rels_by_kind(relationships, RelationshipKind::ImportedSymbolToDefinition),
        ),
        ("ImportedSymbolNode", "FileNode") => (
            Some(filename),
            rels_by_kind(relationships, RelationshipKind::ImportedSymbolToFile),
        ),
        _ => (None, vec![]),
    }
}

impl NodeFieldAccess for ConsolidatedRelationship {
    fn get_u32_field(&self, field_name: &str) -> Option<u32> {
        match field_name {
            "source_id" => self.source_id,
            "target_id" => self.target_id,
            _ => None,
        }
    }

    fn get_string_field(&self, field_name: &str) -> Option<String> {
        match field_name {
            "type" => Some(self.relationship_type.as_string()),
            "source_path" => self.source_path.as_ref().map(|p| p.as_ref().clone()),
            "target_path" => self.target_path.as_ref().map(|p| p.as_ref().clone()),
            _ => None,
        }
    }

    fn get_i64_field(&self, field_name: &str) -> Option<i64> {
        match field_name {
            "source_start_byte" => Some(self.source_range.byte_offset.0 as i64),
            "source_end_byte" => Some(self.source_range.byte_offset.1 as i64),
            _ => None,
        }
    }

    fn get_i32_field(&self, field_name: &str) -> Option<i32> {
        match field_name {
            "source_start_line" => Some(self.source_range.start.line as i32),
            "source_end_line" => Some(self.source_range.end.line as i32),
            "source_start_col" => Some(self.source_range.start.column as i32),
            "source_end_col" => Some(self.source_range.end.column as i32),
            _ => None,
        }
    }
}

/// Structured graph data ready for writing to Parquet files
#[derive(Debug)]
pub struct GraphData {
    /// Directory nodes to be written to directories.parquet
    pub directory_nodes: Vec<DirectoryNode>,
    /// File nodes to be written to files.parquet
    pub file_nodes: Vec<FileNode>,
    /// Definition nodes to be written to definitions.parquet  
    pub definition_nodes: Vec<DefinitionNode>,
    /// Imported symbol nodes to be written to imported_symbols.parquet
    pub imported_symbol_nodes: Vec<ImportedSymbolNode>,
    /// Relationships to be written to parquet files based on their kind
    pub relationships: Vec<ConsolidatedRelationship>,
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
    pub range: Range,
    // File location of the definition
    pub file_path: String,
}

impl DefinitionNode {
    /// Create a new DefinitionNode
    pub fn new(
        fqn: String,
        name: String,
        definition_type: DefinitionType,
        range: Range,
        file_path: String,
    ) -> Self {
        Self {
            fqn,
            name,
            definition_type,
            range,
            file_path,
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
            "primary_file_path" => Some(self.file_path.clone()),
            _ => None,
        }
    }

    fn get_i32_field(&self, field_name: &str) -> Option<i32> {
        match field_name {
            "start_line" => Some(self.range.start.line as i32),
            "end_line" => Some(self.range.end.line as i32),
            "start_col" => Some(self.range.start.column as i32),
            "end_col" => Some(self.range.end.column as i32),
            "total_locations" => Some(1), // Default to 1 for single location
            _ => None,
        }
    }

    fn get_i64_field(&self, field_name: &str) -> Option<i64> {
        match field_name {
            "primary_start_byte" => Some(self.range.byte_offset.0 as i64),
            "primary_end_byte" => Some(self.range.byte_offset.1 as i64),
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
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
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

impl ImportedSymbolLocation {
    pub fn range(&self) -> Range {
        let start_pos = Position::new(self.start_line as usize, self.start_col as usize);
        let end_pos = Position::new(self.end_line as usize, self.end_col as usize);
        Range::new(
            start_pos,
            end_pos,
            (self.start_byte as usize, self.end_byte as usize),
        )
    }
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

/// Optimized file tree structure for fast lookups
#[derive(Debug, Clone)]
pub struct OptimizedFileTree {
    /// File paths
    normalized_files: HashMap<String, String>, // Normalized file path -> Original file path
    /// Precomputed root directories
    root_dirs: HashSet<PathBuf>,
    /// Directory structure for efficient path operations
    dirs: HashSet<PathBuf>,
}

impl OptimizedFileTree {
    pub fn new<'a>(files: impl Iterator<Item = &'a String>) -> Self {
        let mut dirs = HashSet::new();
        let mut normalized_files = HashMap::new();

        // Precompute normalized files and directory structure
        for file_path in files {
            normalized_files.insert(file_path.to_lowercase(), file_path.clone());

            let path = Path::new(&file_path);
            if let Some(parent) = path.parent() {
                dirs.insert(parent.to_path_buf());
            }
        }

        // Precompute root directories
        let root_dirs = Self::compute_root_dirs(&normalized_files, &dirs);

        Self {
            normalized_files,
            root_dirs,
            dirs,
        }
    }

    fn compute_root_dirs(
        files: &HashMap<String, String>,
        dirs: &HashSet<PathBuf>,
    ) -> HashSet<PathBuf> {
        let mut root_dirs = HashSet::new();

        // Find the most common root directory (shortest path)
        if let Some(common_root) = dirs.iter().min_by_key(|p| p.as_os_str().len()) {
            root_dirs.insert(common_root.clone());
        }

        // Look for directories that might be package roots (contain __init__.py)
        for (file_path, norm_file_path) in files {
            if norm_file_path.ends_with("__init__.py") {
                let path = Path::new(file_path);
                if let Some(package_dir) = path.parent()
                    && let Some(package_parent) = package_dir.parent()
                {
                    root_dirs.insert(package_parent.to_path_buf());
                }
            }
        }

        root_dirs
    }

    /// Get the original file path if it exists (case-insensitive)
    pub fn get_denormalized_file(&self, norm_file_path: &str) -> Option<&String> {
        self.normalized_files.get(norm_file_path)
    }

    /// Get root directories
    pub fn get_root_dirs(&self) -> &HashSet<PathBuf> {
        &self.root_dirs
    }

    /// Get all directories
    pub fn get_dirs(&self) -> &HashSet<PathBuf> {
        &self.dirs
    }
}
