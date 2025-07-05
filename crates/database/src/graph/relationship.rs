use std::collections::HashMap;

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

pub enum RelationshipType {
    // Directory relationships
    DirContainsDir,
    DirContainsFile,
    // File relationships
    FileDefines,
    // Definition relationships - Module
    ModuleToModule,
    ModuleToClass,
    ModuleToMethod,
    ModuleToSingletonMethod,
    ModuleToLambda,
    ModuleToProc,
    // Definition relationships - Class
    ClassToMethod,
    ClassToSingletonMethod,
    ClassToClass,
    ClassToLambda,
    ClassToProc,
}

const RELATIONSHIP_TYPES: [&str; 14] = [
    "DIR_CONTAINS_DIR",
    "DIR_CONTAINS_FILE",
    "FILE_DEFINES",
    "MODULE_TO_MODULE",
    "MODULE_TO_CLASS",
    "MODULE_TO_METHOD",
    "MODULE_TO_SINGLETON_METHOD",
    "MODULE_TO_LAMBDA",
    "MODULE_TO_PROC",
    "CLASS_TO_METHOD",
    "CLASS_TO_SINGLETON_METHOD",
    "CLASS_TO_CLASS",
    "CLASS_TO_LAMBDA",
    "CLASS_TO_PROC",
];

impl RelationshipType {
    pub fn as_str(&self) -> &str {
        match self {
            RelationshipType::DirContainsDir => "DIR_CONTAINS_DIR",
            RelationshipType::DirContainsFile => "DIR_CONTAINS_FILE",
            RelationshipType::FileDefines => "FILE_DEFINES",
            RelationshipType::ModuleToModule => "MODULE_TO_MODULE",
            RelationshipType::ModuleToClass => "MODULE_TO_CLASS",
            RelationshipType::ModuleToMethod => "MODULE_TO_METHOD",
            RelationshipType::ModuleToSingletonMethod => "MODULE_TO_SINGLETON_METHOD",
            RelationshipType::ModuleToLambda => "MODULE_TO_LAMBDA",
            RelationshipType::ModuleToProc => "MODULE_TO_PROC",
            RelationshipType::ClassToMethod => "CLASS_TO_METHOD",
            RelationshipType::ClassToSingletonMethod => "CLASS_TO_SINGLETON_METHOD",
            RelationshipType::ClassToClass => "CLASS_TO_CLASS",
            RelationshipType::ClassToLambda => "CLASS_TO_LAMBDA",
            RelationshipType::ClassToProc => "CLASS_TO_PROC",
        }
    }

    pub fn all_types() -> Vec<&'static str> {
        RELATIONSHIP_TYPES.to_vec()
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
        for rel_type in RelationshipType::all_types() {
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

    pub fn get_type_id(&self, type_name: RelationshipType) -> u8 {
        self.type_to_id.get(type_name.as_str()).copied().unwrap()
    }

    pub fn get_type_name(&self, type_id: u8) -> &String {
        self.id_to_type.get(&type_id).unwrap()
    }

    pub fn get_all_types(&self) -> Vec<&String> {
        self.id_to_type.values().collect()
    }

    pub fn get_all_mappings(&self) -> HashMap<String, u8> {
        self.type_to_id.clone()
    }
}
