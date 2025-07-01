use kuzu::Value;

pub enum KuzuNodeType {
    DirectoryNode,
    FileNode,
    DefinitionNode,
}

impl KuzuNodeType {
    pub fn as_str(&self) -> &str {
        match self {
            KuzuNodeType::DirectoryNode => "DirectoryNode",
            KuzuNodeType::FileNode => "FileNode",
            KuzuNodeType::DefinitionNode => "DefinitionNode",
        }
    }
}

/// Result structure for definition node queries
#[derive(Debug, Clone)]
pub struct DefinitionNodeResult {
    pub id: u32,
    pub fqn: String,
    pub name: String,
    pub definition_type: String,
    pub primary_file_path: String,
    pub total_locations: i32,
}

/// Result structure for file node queries
#[derive(Debug, Clone)]
pub struct FileNodeResult {
    pub id: u32,
    pub path: String,
    pub language: String,
    pub name: String,
}

/// Result structure for relationship queries (legacy)
#[derive(Debug, Clone)]
pub struct RelationshipResult {
    pub source_id: u32,
    pub source_identifier: String,
    pub target_id: u32,
    pub target_identifier: String,
    pub relationship_type: u8,
}

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
}

/// Relationship counts structure
#[derive(Debug, Clone)]
pub struct RelationshipCounts {
    pub directory_relationships: u32,
    pub file_relationships: u32,
    pub definition_relationships: u32,
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_tables: usize,
    pub node_tables: usize,
    pub rel_tables: usize,
    pub total_nodes: usize,
    pub total_relationships: usize,
}

impl std::fmt::Display for DatabaseStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Database Stats: {} tables ({} node, {} rel), {} nodes, {} relationships",
            self.total_tables,
            self.node_tables,
            self.rel_tables,
            self.total_nodes,
            self.total_relationships
        )
    }
}

/// Kuzu node parsing structures to avoid repetitive expansion logic
#[derive(Debug, Clone)]
pub struct DefinitionNodeFromKuzu {
    pub id: u32,
    pub fqn: String,
    pub name: String,
    pub definition_type: String,
    pub primary_file_path: String,
    pub primary_start_byte: i64,
    pub primary_end_byte: i64,
    pub primary_line_number: i32,
    pub total_locations: i32,
}

impl DefinitionNodeFromKuzu {
    pub fn empty() -> Self {
        Self {
            id: 0,
            fqn: String::new(),
            name: String::new(),
            definition_type: String::new(),
            primary_file_path: String::new(),
            primary_start_byte: 0,
            primary_end_byte: 0,
            primary_line_number: 0,
            total_locations: 0,
        }
    }

    pub fn from_kuzu_node(node: &Value) -> Self {
        if let Value::Node(node_val) = node {
            let mut node = Self::empty();
            for (prop_name, prop_value) in node_val.get_properties().iter() {
                match prop_name.as_str() {
                    "id" => {
                        if let Value::UInt32(i) = prop_value {
                            node.id = *i
                        }
                    }
                    "fqn" | "name" | "definition_type" | "primary_file_path" => {
                        if let Value::String(s) = prop_value {
                            match prop_name.as_str() {
                                "fqn" => node.fqn = s.to_string(),
                                "name" => node.name = s.to_string(),
                                "definition_type" => node.definition_type = s.to_string(),
                                "primary_file_path" => node.primary_file_path = s.to_string(),
                                _ => (),
                            }
                        }
                    }
                    "primary_start_byte" | "primary_end_byte" => {
                        if let Value::Int64(i) = prop_value {
                            match prop_name.as_str() {
                                "primary_start_byte" => node.primary_start_byte = *i,
                                "primary_end_byte" => node.primary_end_byte = *i,
                                _ => (),
                            }
                        }
                    }
                    "primary_line_number" | "total_locations" => {
                        if let Value::Int32(i) = prop_value {
                            match prop_name.as_str() {
                                "primary_line_number" => node.primary_line_number = *i,
                                "total_locations" => node.total_locations = *i,
                                _ => (),
                            }
                        }
                    }
                    _ => (),
                }
            }
            node
        } else {
            Self::empty()
        }
    }

    pub fn invalid() -> bool {
        Self::empty().id == 0
    }
}

impl std::fmt::Display for DefinitionNodeFromKuzu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DefinitionNodeFromKuzu(id: {}, fqn: {}, name: {}, definition_type: {}, primary_file_path: {}, primary_start_byte: {}, primary_end_byte: {}, primary_line_number: {}, total_locations: {})",
            self.id,
            self.fqn,
            self.name,
            self.definition_type,
            self.primary_file_path,
            self.primary_start_byte,
            self.primary_end_byte,
            self.primary_line_number,
            self.total_locations
        )
    }
}

#[derive(Debug, Clone)]
pub struct FileNodeFromKuzu {
    pub id: u32,
    pub path: String,
    pub absolute_path: String,
    pub language: String,
    pub repository_name: String,
    pub extension: String,
    pub name: String,
}

impl FileNodeFromKuzu {
    pub fn empty() -> Self {
        Self {
            id: 0,
            path: String::new(),
            absolute_path: String::new(),
            language: String::new(),
            repository_name: String::new(),
            extension: String::new(),
            name: String::new(),
        }
    }

    pub fn from_kuzu_node(node: &Value) -> Self {
        if let Value::Node(node_val) = node {
            let mut node = Self::empty();
            for (prop_name, prop_value) in node_val.get_properties().iter() {
                match prop_name.as_str() {
                    "id" => {
                        if let Value::UInt32(i) = prop_value {
                            node.id = *i
                        }
                    }
                    "path" | "absolute_path" | "language" | "repository_name" | "extension"
                    | "name" => {
                        if let Value::String(s) = prop_value {
                            match prop_name.as_str() {
                                "path" => node.path = s.to_string(),
                                "absolute_path" => node.absolute_path = s.to_string(),
                                "language" => node.language = s.to_string(),
                                "repository_name" => node.repository_name = s.to_string(),
                                "extension" => node.extension = s.to_string(),
                                "name" => node.name = s.to_string(),
                                _ => (),
                            }
                        }
                    }
                    _ => (),
                }
            }
            node
        } else {
            Self::empty()
        }
    }

    pub fn invalid() -> bool {
        Self::empty().id == 0
    }
}

impl std::fmt::Display for FileNodeFromKuzu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FileNodeFromKuzu(id: {}, path: {}, absolute_path: {}, language: {}, repository_name: {}, extension: {}, name: {})",
            self.id,
            self.path,
            self.absolute_path,
            self.language,
            self.repository_name,
            self.extension,
            self.name
        )
    }
}

#[derive(Debug, Clone)]
pub struct DirectoryNodeFromKuzu {
    pub id: u32,
    pub path: String,
    pub absolute_path: String,
    pub repository_name: String,
    pub name: String,
}

impl DirectoryNodeFromKuzu {
    pub fn empty() -> Self {
        Self {
            id: 0,
            path: String::new(),
            absolute_path: String::new(),
            repository_name: String::new(),
            name: String::new(),
        }
    }

    pub fn from_kuzu_node(node: &Value) -> Self {
        if let Value::Node(node_val) = node {
            let mut node = Self::empty();
            for (prop_name, prop_value) in node_val.get_properties().iter() {
                match prop_name.as_str() {
                    "id" => {
                        if let Value::UInt32(i) = prop_value {
                            node.id = *i
                        }
                    }
                    "path" | "absolute_path" | "repository_name" | "name" => {
                        if let Value::String(s) = prop_value {
                            match prop_name.as_str() {
                                "path" => node.path = s.to_string(),
                                "absolute_path" => node.absolute_path = s.to_string(),
                                "repository_name" => node.repository_name = s.to_string(),
                                "name" => node.name = s.to_string(),
                                _ => (),
                            }
                        }
                    }
                    _ => (),
                }
            }
            node
        } else {
            Self::empty()
        }
    }

    pub fn invalid() -> bool {
        Self::empty().id == 0
    }
}

impl std::fmt::Display for DirectoryNodeFromKuzu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DirectoryNodeFromKuzu(id: {}, path: {}, absolute_path: {}, repository_name: {}, name: {})",
            self.id, self.path, self.absolute_path, self.repository_name, self.name
        )
    }
}

/// Trait to determine if a value needs to be quoted in SQL
pub trait QuoteEscape {
    fn needs_quotes(&self) -> bool;
}

macro_rules! impl_quote_escape {
    ($($t:ty: $v:expr),*) => {
        $(
            impl QuoteEscape for $t {
                fn needs_quotes(&self) -> bool { $v }
            }
        )*
    }
}

impl_quote_escape!(
    // Strings need quotes
    String: true, &str: true,
    // Numeric types don't need quotes
    i8: false, i16: false, i32: false, i64: false, i128: false, isize: false,
    u8: false, u16: false, u32: false, u64: false, u128: false, usize: false,
    f32: false, f64: false
);

pub trait FromKuzuNode: Sized {
    fn from_kuzu_node(node: &Value) -> Self;
    fn name() -> &'static str;
}

impl FromKuzuNode for DefinitionNodeFromKuzu {
    fn from_kuzu_node(node: &Value) -> Self {
        Self::from_kuzu_node(node)
    }

    fn name() -> &'static str {
        KuzuNodeType::DefinitionNode.as_str()
    }
}

impl FromKuzuNode for FileNodeFromKuzu {
    fn from_kuzu_node(node: &Value) -> Self {
        Self::from_kuzu_node(node)
    }

    fn name() -> &'static str {
        KuzuNodeType::FileNode.as_str()
    }
}

impl FromKuzuNode for DirectoryNodeFromKuzu {
    fn from_kuzu_node(node: &Value) -> Self {
        Self::from_kuzu_node(node)
    }

    fn name() -> &'static str {
        KuzuNodeType::DirectoryNode.as_str()
    }
}
