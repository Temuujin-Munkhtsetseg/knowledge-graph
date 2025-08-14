use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};

use crate::analysis::types::{GraphData, ImportedSymbolLocation};
use crate::mutation::types::{ConsolidatedRelationship, ConsolidatedRelationships};
use database::graph::RelationshipType;
use database::graph::RelationshipTypeMapping;
use parser_core::utils::Range;

/// Node ID generator for assigning integer IDs to nodes
#[derive(Debug, Clone)]
pub struct NodeIdGenerator {
    /// Directory path to ID mapping
    directory_ids: HashMap<String, u32>,
    /// File path to ID mapping
    file_ids: HashMap<String, u32>,
    /// Definition byte range to ID mapping
    definition_ids: HashMap<(String, usize, usize), u32>,
    /// Imported symbol byte range to ID mapping
    imported_symbol_ids: HashMap<(String, usize, usize), u32>,
    /// Next available IDs for each type
    pub next_directory_id: u32,
    pub next_file_id: u32,
    pub next_definition_id: u32,
    pub next_imported_symbol_id: u32,
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
            imported_symbol_ids: HashMap::new(),
            next_directory_id: 1,
            next_file_id: 1,
            next_definition_id: 1,
            next_imported_symbol_id: 1,
        }
    }

    /// Clear all ID mappings while preserving the next ID counters
    pub fn clear(&mut self) {
        self.directory_ids.clear();
        self.file_ids.clear();
        self.definition_ids.clear();
        self.imported_symbol_ids.clear();
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

    pub fn get_or_assign_definition_id(&mut self, file_path: &str, range: &Range) -> u32 {
        if let Some(&id) = self.definition_ids.get(&(
            file_path.to_string(),
            range.byte_offset.0,
            range.byte_offset.1,
        )) {
            return id;
        }

        let id = self.next_definition_id;
        self.definition_ids.insert(
            (
                file_path.to_string(),
                range.byte_offset.0,
                range.byte_offset.1,
            ),
            id,
        );
        self.next_definition_id += 1;
        id
    }

    pub fn get_or_assign_imported_symbol_id(&mut self, location: &ImportedSymbolLocation) -> u32 {
        if let Some(&id) = self.imported_symbol_ids.get(&(
            location.file_path.to_string(),
            location.start_byte as usize,
            location.end_byte as usize,
        )) {
            return id;
        }

        let id = self.next_imported_symbol_id;
        self.imported_symbol_ids.insert(
            (
                location.file_path.to_string(),
                location.start_byte as usize,
                location.end_byte as usize,
            ),
            id,
        );
        self.next_imported_symbol_id += 1;

        id
    }

    pub fn get_directory_id(&self, path: &str) -> Option<u32> {
        self.directory_ids.get(path).copied()
    }

    pub fn get_file_id(&self, path: &str) -> Option<u32> {
        self.file_ids.get(path).copied()
    }

    pub fn get_definition_id(&self, file_path: &str, range: &Range) -> Option<u32> {
        self.definition_ids
            .get(&(
                file_path.to_string(),
                range.byte_offset.0,
                range.byte_offset.1,
            ))
            .copied()
    }

    pub fn get_imported_symbol_id(&self, location: &ImportedSymbolLocation) -> Option<u32> {
        self.imported_symbol_ids
            .get(&(
                location.file_path.clone(),
                location.start_byte as usize,
                location.end_byte as usize,
            ))
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
            self.node_id_generator.get_or_assign_definition_id(
                &def_node.location.file_path,
                &def_node.location.to_range(),
            );
        }

        // Assign imported symbol IDs
        for imported_symbol_node in &self.graph_data.imported_symbol_nodes {
            self.node_id_generator
                .get_or_assign_imported_symbol_id(&imported_symbol_node.location);
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
        let mut import_not_found = 0;
        let mut calls_count = 0;
        let mut missing_source_fqns = HashSet::new();
        let mut missing_target_fqns = HashSet::new();

        // Process directory-to-directory and directory-to-file relationships
        for dir_rel in &graph_data.directory_relationships {
            let Some(source_id) = id_generator.get_directory_id(&dir_rel.from_path) else {
                dir_not_found += 1;
                warn!(
                    "(DIR_CONTAINS_DIR) Source directory ID not found: Directory({})",
                    dir_rel.from_path
                );
                continue;
            };

            let relationship_type = relationship_mapping.get_type_id(dir_rel.relationship_type);

            if dir_rel.relationship_type == RelationshipType::DirContainsDir {
                let Some(target_id) = id_generator.get_directory_id(&dir_rel.to_path) else {
                    dir_not_found += 1;
                    warn!(
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
            } else if dir_rel.relationship_type == RelationshipType::DirContainsFile {
                let Some(target_id) = id_generator.get_file_id(&dir_rel.to_path) else {
                    file_not_found += 1;
                    warn!(
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

        // Process file-to-definition relationships
        for file_rel in &graph_data.file_definition_relationships {
            if file_rel.relationship_type == RelationshipType::Calls {
                calls_count += 1;
            }

            let Some(source_id) = id_generator.get_file_id(&file_rel.file_path) else {
                file_not_found += 1;
                warn!(
                    "(FILE_DEFINES) Source file ID not found: File({})",
                    file_rel.file_path
                );
                continue;
            };

            let Some(target_id) = id_generator.get_definition_id(
                &file_rel.file_path,
                &file_rel.definition_location.to_range(),
            ) else {
                def_not_found += 1;
                warn!(
                    "(FILE_DEFINES) Target definition ID not found: FQN({}) File({})",
                    file_rel.definition_fqn, file_rel.file_path,
                );
                continue;
            };
            let relationship_type = relationship_mapping.get_type_id(file_rel.relationship_type);

            relationships
                .file_to_definition
                .push(ConsolidatedRelationship {
                    source_id: Some(source_id),
                    target_id: Some(target_id),
                    relationship_type,
                });
        }

        // Process file-to-imported-symbol relationships
        for file_rel in &graph_data.file_imported_symbol_relationships {
            let Some(source_id) = id_generator.get_file_id(&file_rel.file_path) else {
                file_not_found += 1;
                warn!(
                    "(FILE_IMPORTS) Source file ID not found: File({})",
                    file_rel.file_path
                );
                continue;
            };

            let Some(target_id) = id_generator.get_imported_symbol_id(&file_rel.import_location)
            else {
                import_not_found += 1;
                warn!(
                    "(FILE_IMPORTS) Target imported symbol ID not found: Location({:?}) File({})",
                    file_rel.import_location, file_rel.file_path,
                );
                continue;
            };
            let relationship_type = relationship_mapping.get_type_id(file_rel.relationship_type);

            relationships
                .file_to_imported_symbol
                .push(ConsolidatedRelationship {
                    source_id: Some(source_id),
                    target_id: Some(target_id),
                    relationship_type,
                });
        }

        // Process definition-to-definition relationships
        for def_rel in &graph_data.definition_relationships {
            if def_rel.relationship_type == RelationshipType::Calls {
                calls_count += 1;
            }

            let Some(source_id) = id_generator
                .get_definition_id(&def_rel.from_file_path, &def_rel.from_location.to_range())
            else {
                def_not_found += 1;
                missing_source_fqns.insert((
                    def_rel.from_definition_fqn.clone(),
                    def_rel.from_file_path.clone(),
                ));
                debug!(
                    "(DEFINITION_RELATIONSHIPS) Source definition ID not found: {} {}",
                    def_rel.from_definition_fqn, def_rel.from_file_path,
                );
                continue;
            };

            let Some(target_id) = id_generator
                .get_definition_id(&def_rel.to_file_path, &def_rel.to_location.to_range())
            else {
                def_not_found += 1;
                missing_target_fqns.insert((
                    def_rel.to_definition_fqn.clone(),
                    def_rel.to_file_path.clone(),
                ));
                debug!(
                    "(DEFINITION_RELATIONSHIPS) Target definition ID not found: {} {}",
                    def_rel.to_definition_fqn, def_rel.to_file_path,
                );
                continue;
            };

            let relationship_type = relationship_mapping.get_type_id(def_rel.relationship_type);

            relationships
                .definition_to_definition
                .push(ConsolidatedRelationship {
                    source_id: Some(source_id),
                    target_id: Some(target_id),
                    relationship_type,
                });
        }

        // Process definition-to-imported-symbol relationships
        for def_rel in &graph_data.definition_imported_symbol_relationships {
            let Some(source_id) = id_generator
                .get_definition_id(&def_rel.file_path, &def_rel.definition_location.to_range())
            else {
                def_not_found += 1;
                missing_source_fqns
                    .insert((def_rel.definition_fqn.clone(), def_rel.file_path.clone()));
                debug!(
                    "(DEFINES_IMPORTED_SYMBOL) Source definition ID not found: {} {}",
                    def_rel.definition_fqn, def_rel.file_path,
                );
                continue;
            };

            let Some(target_id) =
                id_generator.get_imported_symbol_id(&def_rel.imported_symbol_location)
            else {
                import_not_found += 1;
                warn!(
                    "(DEFINITION_IMPORTED_SYMBOL_RELATIONSHIPS) Target imported symbol ID not found: {:?}",
                    def_rel.imported_symbol_location,
                );
                continue;
            };

            let relationship_type = relationship_mapping.get_type_id(def_rel.relationship_type);

            relationships
                .definition_to_imported_symbol
                .push(ConsolidatedRelationship {
                    source_id: Some(source_id),
                    target_id: Some(target_id),
                    relationship_type,
                });
        }

        info!(
            "Consolidated relationships: dir_not_found: {}, file_not_found: {}, def_not_found: {}, import_not_found: {}",
            dir_not_found, file_not_found, def_not_found, import_not_found
        );
        info!("Consolidated calls count: {}", calls_count);

        // Show summary of missing definitions instead of individual warnings
        if !missing_source_fqns.is_empty() {
            warn!(
                "Missing source definitions: {} unique FQNs",
                missing_source_fqns.len()
            );
            for (fqn, _file) in missing_source_fqns.iter().take(5) {
                debug!("  Missing source: {}", fqn);
            }
            if missing_source_fqns.len() > 5 {
                debug!("  ... and {} more", missing_source_fqns.len() - 5);
            }
        }

        if !missing_target_fqns.is_empty() {
            warn!(
                "Missing target definitions: {} unique FQNs",
                missing_target_fqns.len()
            );
            for (fqn, _file) in missing_target_fqns.iter().take(5) {
                debug!("  Missing target: {}", fqn);
            }
            if missing_target_fqns.len() > 5 {
                debug!("  ... and {} more", missing_target_fqns.len() - 5);
            }
        }

        Ok(relationships)
    }
}
