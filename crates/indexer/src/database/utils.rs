use std::collections::HashMap;

use database::graph::RelationshipTypeMapping;

use crate::analysis::types::GraphData;

/// Consolidated relationship data for efficient storage
#[derive(Debug, Clone, Default, Copy)]
pub struct ConsolidatedRelationship {
    pub source_id: Option<u32>,
    pub target_id: Option<u32>,
    pub relationship_type: u8,
}

/// Container for different types of consolidated relationships
#[derive(Default, Clone)]
pub struct ConsolidatedRelationships {
    pub directory_to_directory: Vec<ConsolidatedRelationship>,
    pub directory_to_file: Vec<ConsolidatedRelationship>,
    pub file_to_definition: Vec<ConsolidatedRelationship>,
    pub definition_to_definition: Vec<ConsolidatedRelationship>,
}

/// Node ID generator for assigning integer IDs to nodes
#[derive(Debug, Clone)]
pub struct NodeIdGenerator {
    /// Directory path to ID mapping
    directory_ids: HashMap<String, u32>,
    /// File path to ID mapping
    file_ids: HashMap<String, u32>,
    /// Definition FQN to ID mapping (TODO: add file path)
    definition_ids: HashMap<(String, String), u32>,
    /// Next available IDs for each type
    pub next_directory_id: u32,
    pub next_file_id: u32,
    pub next_definition_id: u32,
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

    /// Clear all ID mappings while preserving the next ID counters
    pub fn clear(&mut self) {
        self.directory_ids.clear();
        self.file_ids.clear();
        self.definition_ids.clear();
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

    pub fn get_or_assign_definition_id(&mut self, fqn: &str, file_path: &str) -> u32 {
        if let Some(&id) = self
            .definition_ids
            .get(&(fqn.to_string(), file_path.to_string()))
        {
            return id;
        }

        let id = self.next_definition_id;
        self.definition_ids
            .insert((fqn.to_string(), file_path.to_string()), id);
        self.next_definition_id += 1;
        id
    }

    pub fn get_directory_id(&self, path: &str) -> Option<u32> {
        self.directory_ids.get(path).copied()
    }

    pub fn get_file_id(&self, path: &str) -> Option<u32> {
        self.file_ids.get(path).copied()
    }

    pub fn get_definition_id(&self, fqn: &str, file_path: &str) -> Option<u32> {
        self.definition_ids
            .get(&(fqn.to_string(), file_path.to_string()))
            .copied()
    }
}

pub struct GraphMapper<'a> {
    pub graph_data: &'a GraphData,
    pub node_id_generator: &'a mut NodeIdGenerator,
    pub relationship_mapping: &'a mut RelationshipTypeMapping,
}

impl<'a> GraphMapper<'a> {
    /// Create a new writer service
    pub fn new(
        graph_data: &'a GraphData,
        node_id_generator: &'a mut NodeIdGenerator,
        relationship_mapping: &'a mut RelationshipTypeMapping,
    ) -> Self {
        Self {
            graph_data,
            node_id_generator,
            relationship_mapping,
        }
    }

    /// Pre-assign integer IDs to all nodes
    fn assign_node_ids(&mut self) {
        // Assign directory IDs
        for dir_node in &self.graph_data.directory_nodes {
            self.node_id_generator
                .get_or_assign_directory_id(&dir_node.path);
        }

        // Assign file IDs
        for file_node in &self.graph_data.file_nodes {
            self.node_id_generator
                .get_or_assign_file_id(&file_node.path);
        }

        // Assign definition IDs
        for def_node in &self.graph_data.definition_nodes {
            self.node_id_generator
                .get_or_assign_definition_id(&def_node.fqn, &def_node.location.file_path);
        }
    }

    /// Map the graph data to the integer IDs
    pub fn map_graph_data(&mut self) -> Result<ConsolidatedRelationships, anyhow::Error> {
        // Pre-assign IDs to all nodes
        self.assign_node_ids();

        // Write consolidated relationship tables
        Self::consolidate_relationships(
            self.graph_data,
            self.node_id_generator,
            self.relationship_mapping,
        )
    }

    /// Consolidate all relationships into four categories with integer IDs and types
    fn consolidate_relationships(
        graph_data: &GraphData,
        id_generator: &mut NodeIdGenerator,
        relationship_mapping: &mut RelationshipTypeMapping,
    ) -> Result<ConsolidatedRelationships, anyhow::Error> {
        let mut relationships = ConsolidatedRelationships::default();
        let mut dir_not_found = 0;
        let mut file_not_found = 0;
        let mut def_not_found = 0;

        // Process directory relationships
        for dir_rel in &graph_data.directory_relationships {
            let Some(source_id) = id_generator.get_directory_id(&dir_rel.from_path) else {
                dir_not_found += 1;
                tracing::warn!(
                    "(DIR_CONTAINS_DIR) Source directory ID not found: Directory({})",
                    dir_rel.from_path
                );
                continue;
            };

            let relationship_type = relationship_mapping.register_type(&dir_rel.relationship_type);

            if dir_rel.relationship_type == "DIR_CONTAINS_DIR" {
                let Some(target_id) = id_generator.get_directory_id(&dir_rel.to_path) else {
                    dir_not_found += 1;
                    tracing::warn!(
                        "(DIR_CONTAINS_DIR) Target directory ID not found: Directory({})",
                        dir_rel.to_path
                    );
                    continue;
                };

                relationships
                    .directory_to_directory
                    .push(ConsolidatedRelationship {
                        source_id: Some(source_id),
                        target_id: Some(target_id),
                        relationship_type,
                    });
            } else if dir_rel.relationship_type == "DIR_CONTAINS_FILE" {
                let Some(target_id) = id_generator.get_file_id(&dir_rel.to_path) else {
                    file_not_found += 1;
                    tracing::warn!(
                        "(DIR_CONTAINS_FILE) Target file ID not found: File({})",
                        dir_rel.to_path
                    );
                    continue;
                };

                relationships
                    .directory_to_file
                    .push(ConsolidatedRelationship {
                        source_id: Some(source_id),
                        target_id: Some(target_id),
                        relationship_type,
                    });
            }
        }

        // Process file-definition relationships
        // For each relationship, we need to find the definition's primary location to get the correct ID
        for file_rel in &graph_data.file_definition_relationships {
            let Some(source_id) = id_generator.get_file_id(&file_rel.file_path) else {
                file_not_found += 1;
                tracing::warn!(
                    "(FILE_DEFINES) Source file ID not found: File({})",
                    file_rel.file_path
                );
                continue;
            };

            let Some(target_id) =
                id_generator.get_definition_id(&file_rel.definition_fqn, &file_rel.file_path)
            else {
                def_not_found += 1;
                tracing::warn!(
                    "(FILE_DEFINES) Target definition ID not found: FQN({}) File({})",
                    file_rel.definition_fqn,
                    file_rel.file_path,
                );
                continue;
            };
            let relationship_type = relationship_mapping.register_type(&file_rel.relationship_type);

            relationships
                .file_to_definition
                .push(ConsolidatedRelationship {
                    source_id: Some(source_id),
                    target_id: Some(target_id),
                    relationship_type,
                });
        }

        // Process definition relationships
        for def_rel in &graph_data.definition_relationships {
            let Some(source_id) = id_generator
                .get_definition_id(&def_rel.from_definition_fqn, &def_rel.from_file_path)
            else {
                def_not_found += 1;
                tracing::warn!(
                    "(DEFINITION_RELATIONSHIPS) Source definition ID not found: {} {}",
                    def_rel.from_definition_fqn,
                    def_rel.from_file_path,
                );
                continue;
            };

            let Some(target_id) =
                id_generator.get_definition_id(&def_rel.to_definition_fqn, &def_rel.to_file_path)
            else {
                def_not_found += 1;
                tracing::warn!(
                    "(DEFINITION_RELATIONSHIPS) Target definition ID not found: {} {}",
                    def_rel.to_definition_fqn,
                    def_rel.to_file_path,
                );
                continue;
            };

            let relationship_type = relationship_mapping.register_type(&def_rel.relationship_type);

            relationships
                .definition_to_definition
                .push(ConsolidatedRelationship {
                    source_id: Some(source_id),
                    target_id: Some(target_id),
                    relationship_type,
                });
        }

        tracing::info!(
            "Consolidated relationships: dir_not_found: {}, file_not_found: {}, def_not_found: {}",
            dir_not_found,
            file_not_found,
            def_not_found
        );

        Ok(relationships)
    }
}
