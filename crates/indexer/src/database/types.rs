use database::kuzu::types::{DefinitionNodeFromKuzu, DirectoryNodeFromKuzu, FileNodeFromKuzu};

/// File-Definition relationship with full nodes
#[derive(Debug, Clone)]
pub struct FileDefinitionRelationship {
    pub file_node: FileNodeFromKuzu,
    pub definition_node: DefinitionNodeFromKuzu,
    pub relationship_type: u8,
}

/// Definition-Definition relationship with full nodes
#[derive(Debug, Clone)]
pub struct DefinitionDefinitionRelationship {
    pub from_definition: DefinitionNodeFromKuzu,
    pub to_definition: DefinitionNodeFromKuzu,
    pub relationship_type: u8,
}

/// Directory relationship with full nodes (target can be file or directory)
#[derive(Debug, Clone)]
pub enum DirectoryRelationshipTarget {
    File(FileNodeFromKuzu),
    Directory(DirectoryNodeFromKuzu),
}

#[derive(Debug, Clone)]
pub struct DirectoryRelationship {
    pub directory_node: DirectoryNodeFromKuzu,
    pub target: DirectoryRelationshipTarget,
    pub relationship_type: u8,
}

/// Node counts structure
#[derive(Debug, Clone)]
pub struct NodeCounts {
    pub directory_count: u32,
    pub file_count: u32,
    pub definition_count: u32,
    pub imported_symbol_count: u32,
}

/// Relationship counts structure
#[derive(Debug, Clone)]
pub struct RelationshipCounts {
    pub directory_relationships: u32,
    pub file_relationships: u32,
    pub definition_relationships: u32,
}
