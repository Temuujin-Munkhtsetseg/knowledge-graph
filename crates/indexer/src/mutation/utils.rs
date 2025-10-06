use std::collections::HashMap;
use tracing::warn;

use crate::analysis::types::{GraphData, ImportedSymbolLocation, RelationshipKind};
use parser_core::utils::Range;

pub enum RelationshipIdType {
    Source,
    Target,
}

impl RelationshipIdType {
    pub fn as_str(&self) -> &str {
        match self {
            RelationshipIdType::Source => "SOURCE",
            RelationshipIdType::Target => "TARGET",
        }
    }
}

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

    pub fn get_definition_id(
        &self,
        file_path: &str,
        start_byte: usize,
        end_byte: usize,
    ) -> Option<u32> {
        self.definition_ids
            .get(&(file_path.to_string(), start_byte, end_byte))
            .copied()
    }

    pub fn get_imported_symbol_id(
        &self,
        file_path: &str,
        start_byte: usize,
        end_byte: usize,
    ) -> Option<u32> {
        self.imported_symbol_ids
            .get(&(file_path.to_string(), start_byte, end_byte))
            .copied()
    }
}

pub struct GraphMapper<'a> {
    pub graph_data: &'a mut GraphData,
    pub node_id_generator: &'a mut NodeIdGenerator,
}

impl<'a> GraphMapper<'a> {
    /// Create a new writer service
    pub fn new(graph_data: &'a mut GraphData, node_id_generator: &'a mut NodeIdGenerator) -> Self {
        Self {
            graph_data,
            node_id_generator,
        }
    }

    /// Pre-assign integer IDs to all nodes
    pub fn assign_node_ids(&mut self) {
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
                .get_or_assign_definition_id(&def_node.file_path, &def_node.range);
        }

        // Assign imported symbol IDs
        for imported_symbol_node in &self.graph_data.imported_symbol_nodes {
            self.node_id_generator
                .get_or_assign_imported_symbol_id(&imported_symbol_node.location);
        }
    }

    /// Consolidate all relationships into four categories with integer IDs and types
    pub fn assign_relationship_ids(&mut self) -> Result<(), anyhow::Error> {
        let mut dir_not_found = 0;
        let mut file_not_found = 0;
        let mut def_not_found = 0;
        let mut import_not_found = 0;
        // let calls_count = 0;
        // let mut missing_source_fqns = HashSet::new();
        // let mut missing_target_fqns = HashSet::new();

        // Process all relationships in a single iteration
        for rel in &mut self.graph_data.relationships {
            let Some(from_path) = rel.source_path.as_ref().map(|p| p.as_ref()) else {
                continue;
            };
            let Some(to_path) = rel.target_path.as_ref().map(|p| p.as_ref()) else {
                continue;
            };
            let kind_str = rel.kind.as_str();

            match rel.kind {
                RelationshipKind::DirectoryToDirectory => {
                    let source_id = self.node_id_generator.get_directory_id(from_path);
                    let target_id = self.node_id_generator.get_directory_id(to_path);
                    if source_id.is_none() {
                        dir_not_found += 1;
                        warn!(
                            "({}) Source directory ID not found: Directory({})",
                            kind_str, from_path
                        );
                        continue;
                    }
                    if target_id.is_none() {
                        dir_not_found += 1;
                        warn!(
                            "({}) Target directory ID not found: Directory({})",
                            kind_str, to_path
                        );
                        continue;
                    }
                    rel.source_id = source_id;
                    rel.target_id = target_id;
                }
                RelationshipKind::DirectoryToFile => {
                    let source_id = self.node_id_generator.get_directory_id(from_path);
                    let target_id = self.node_id_generator.get_file_id(to_path);
                    if source_id.is_none() {
                        dir_not_found += 1;
                        warn!(
                            "({}) Source directory ID not found: Directory({})",
                            kind_str, from_path
                        );
                        continue;
                    }
                    if target_id.is_none() {
                        file_not_found += 1;
                        warn!("({}) Target file ID not found: File({})", kind_str, to_path);
                        continue;
                    }
                    rel.source_id = source_id;
                    rel.target_id = target_id;
                }
                RelationshipKind::FileToDefinition => {
                    let source_id = self.node_id_generator.get_file_id(from_path);
                    let target_id = self.node_id_generator.get_definition_id(
                        to_path,
                        rel.target_range.byte_offset.0,
                        rel.target_range.byte_offset.1,
                    );
                    if source_id.is_none() {
                        file_not_found += 1;
                        warn!(
                            "({}) Source file ID not found: File({})",
                            kind_str, from_path
                        );
                        continue;
                    }
                    if target_id.is_none() {
                        def_not_found += 1;
                        warn!(
                            "({}) Target definition ID not found: byte_offset({},{}) File({})",
                            kind_str,
                            { rel.target_range.byte_offset.0 },
                            { rel.target_range.byte_offset.1 },
                            to_path
                        );
                        continue;
                    }
                    rel.source_id = source_id;
                    rel.target_id = target_id;
                }
                RelationshipKind::FileToImportedSymbol => {
                    let source_id = self.node_id_generator.get_file_id(from_path);
                    let target_id = self.node_id_generator.get_imported_symbol_id(
                        to_path,
                        rel.target_range.byte_offset.0,
                        rel.target_range.byte_offset.1,
                    );
                    if source_id.is_none() {
                        file_not_found += 1;
                        warn!(
                            "({}) Source file ID not found: File({})",
                            kind_str, from_path
                        );
                        continue;
                    }
                    if target_id.is_none() {
                        import_not_found += 1;
                        warn!(
                            "({}) Target imported symbol ID not found: byte_offset({},{}) File({})",
                            kind_str,
                            { rel.target_range.byte_offset.0 },
                            { rel.target_range.byte_offset.1 },
                            to_path
                        );
                        continue;
                    }
                    rel.source_id = source_id;
                    rel.target_id = target_id;
                }
                RelationshipKind::DefinitionToDefinition => {
                    let (source_start, source_end) =
                        if let Some(def_range) = &rel.source_definition_range {
                            (def_range.byte_offset.0, def_range.byte_offset.1)
                        } else {
                            (
                                rel.source_range.byte_offset.0,
                                rel.source_range.byte_offset.1,
                            )
                        };
                    let (target_start, target_end) =
                        if let Some(def_range) = &rel.target_definition_range {
                            (def_range.byte_offset.0, def_range.byte_offset.1)
                        } else {
                            (
                                rel.target_range.byte_offset.0,
                                rel.target_range.byte_offset.1,
                            )
                        };
                    let source_id = self.node_id_generator.get_definition_id(
                        from_path,
                        source_start,
                        source_end,
                    );
                    let target_id =
                        self.node_id_generator
                            .get_definition_id(to_path, target_start, target_end);
                    if source_id.is_none() {
                        def_not_found += 1;
                        warn!(
                            "({}) Source definition ID not found: byte_offset({},{}) File({})",
                            kind_str,
                            { rel.source_range.byte_offset.0 },
                            { rel.source_range.byte_offset.1 },
                            from_path
                        );
                        continue;
                    }
                    if target_id.is_none() {
                        def_not_found += 1;
                        warn!(
                            "({}) Target definition ID not found: byte_offset({},{}) File({})",
                            kind_str,
                            { rel.target_range.byte_offset.0 },
                            { rel.target_range.byte_offset.1 },
                            to_path
                        );
                        continue;
                    }
                    rel.source_id = source_id;
                    rel.target_id = target_id;
                }
                RelationshipKind::DefinitionToImportedSymbol => {
                    let (source_start, source_end) =
                        if let Some(def_range) = &rel.source_definition_range {
                            (def_range.byte_offset.0, def_range.byte_offset.1)
                        } else {
                            (
                                rel.source_range.byte_offset.0,
                                rel.source_range.byte_offset.1,
                            )
                        };
                    let (target_start, target_end) =
                        if let Some(def_range) = &rel.target_definition_range {
                            (def_range.byte_offset.0, def_range.byte_offset.1)
                        } else {
                            (
                                rel.target_range.byte_offset.0,
                                rel.target_range.byte_offset.1,
                            )
                        };
                    let source_id = self.node_id_generator.get_definition_id(
                        from_path,
                        source_start,
                        source_end,
                    );
                    let target_id = self.node_id_generator.get_imported_symbol_id(
                        to_path,
                        target_start,
                        target_end,
                    );
                    if source_id.is_none() {
                        def_not_found += 1;
                        warn!(
                            "({}) Source definition ID not found: byte_offset({},{}) File({})",
                            kind_str,
                            { rel.source_range.byte_offset.0 },
                            { rel.source_range.byte_offset.1 },
                            from_path
                        );
                        continue;
                    }
                    if target_id.is_none() {
                        import_not_found += 1;
                        warn!(
                            "({}) Target imported symbol ID not found: byte_offset({},{}) File({})",
                            kind_str,
                            { rel.target_range.byte_offset.0 },
                            { rel.target_range.byte_offset.1 },
                            to_path
                        );
                        continue;
                    }
                    rel.source_id = source_id;
                    rel.target_id = target_id;
                }
                RelationshipKind::ImportedSymbolToDefinition => {
                    let source_id = self.node_id_generator.get_imported_symbol_id(
                        from_path,
                        rel.source_range.byte_offset.0,
                        rel.source_range.byte_offset.1,
                    );
                    let target_id = self.node_id_generator.get_definition_id(
                        to_path,
                        rel.target_range.byte_offset.0,
                        rel.target_range.byte_offset.1,
                    );
                    if source_id.is_none() {
                        import_not_found += 1;
                        warn!(
                            "({}) Source imported symbol ID not found: byte_offset({},{}) File({})",
                            kind_str,
                            { rel.source_range.byte_offset.0 },
                            { rel.source_range.byte_offset.1 },
                            from_path
                        );
                        continue;
                    }
                    if target_id.is_none() {
                        def_not_found += 1;
                        warn!(
                            "({}) Target definition ID not found: byte_offset({},{}) File({})",
                            kind_str,
                            { rel.target_range.byte_offset.0 },
                            { rel.target_range.byte_offset.1 },
                            to_path
                        );
                        continue;
                    }
                    rel.source_id = source_id;
                    rel.target_id = target_id;
                }
                RelationshipKind::ImportedSymbolToImportedSymbol => {
                    let source_id = self.node_id_generator.get_imported_symbol_id(
                        from_path,
                        rel.source_range.byte_offset.0,
                        rel.source_range.byte_offset.1,
                    );
                    let target_id = self.node_id_generator.get_imported_symbol_id(
                        to_path,
                        rel.target_range.byte_offset.0,
                        rel.target_range.byte_offset.1,
                    );
                    if source_id.is_none() {
                        import_not_found += 1;
                        warn!(
                            "({}) Source imported symbol ID not found: byte_offset({},{}) File({})",
                            kind_str,
                            { rel.source_range.byte_offset.0 },
                            { rel.source_range.byte_offset.1 },
                            from_path
                        );
                        continue;
                    }
                    if target_id.is_none() {
                        import_not_found += 1;
                        warn!(
                            "({}) Target imported symbol ID not found: byte_offset({},{}) File({})",
                            kind_str,
                            { rel.target_range.byte_offset.0 },
                            { rel.target_range.byte_offset.1 },
                            to_path
                        );
                        continue;
                    }
                    rel.source_id = source_id;
                    rel.target_id = target_id;
                }
                RelationshipKind::ImportedSymbolToFile => {
                    let source_id = self.node_id_generator.get_imported_symbol_id(
                        from_path,
                        rel.source_range.byte_offset.0,
                        rel.source_range.byte_offset.1,
                    );
                    let target_id = self.node_id_generator.get_file_id(to_path);
                    if source_id.is_none() {
                        import_not_found += 1;
                        warn!(
                            "({}) Source imported symbol ID not found: byte_offset({},{}) File({})",
                            kind_str,
                            { rel.source_range.byte_offset.0 },
                            { rel.source_range.byte_offset.1 },
                            from_path
                        );
                        continue;
                    }
                    if target_id.is_none() {
                        file_not_found += 1;
                        warn!("({}) Target file ID not found: File({})", kind_str, to_path);
                        continue;
                    }
                    rel.source_id = source_id;
                    rel.target_id = target_id;
                }
                _ => {
                    continue;
                }
            }
        }

        // Delete all relationships with no source or target id
        self.graph_data
            .relationships
            .retain(|rel| rel.source_id.is_some() && rel.target_id.is_some());

        warn!(
            "Consolidated relationships: dir_not_found: {}, file_not_found: {}, def_not_found: {}, import_not_found: {}",
            dir_not_found, file_not_found, def_not_found, import_not_found
        );

        // NOTE: these are temporarily deprecated
        // info!("Consolidated calls count: {}", calls_count);

        // // Show summary of missing definitions instead of individual warnings
        // if !missing_source_fqns.is_empty() {
        //     warn!(
        //         "Missing source definitions: {} unique FQNs",
        //         missing_source_fqns.len()
        //     );
        //     for (fqn, _file) in missing_source_fqns.iter().take(5) {
        //         debug!("  Missing source: {}", fqn);
        //     }
        //     if missing_source_fqns.len() > 5 {
        //         debug!("  ... and {} more", missing_source_fqns.len() - 5);
        //     }
        // }

        // if !missing_target_fqns.is_empty() {
        //     warn!(
        //         "Missing target definitions: {} unique FQNs",
        //         missing_target_fqns.len()
        //     );
        //     for (fqn, _file) in missing_target_fqns.iter().take(5) {
        //         debug!("  Missing target: {}", fqn);
        //     }
        //     if missing_target_fqns.len() > 5 {
        //         debug!("  ... and {} more", missing_target_fqns.len() - 5);
        //     }
        // }

        // // Show summary of missing definitions instead of individual warnings
        // if !missing_source_fqns.is_empty() {
        //     warn!(
        //         "Missing source definitions: {} unique FQNs",
        //         missing_source_fqns.len()
        //     );
        //     for (fqn, _file) in missing_source_fqns.iter().take(5) {
        //         debug!("  Missing source: {}", fqn);
        //     }
        //     if missing_source_fqns.len() > 5 {
        //         debug!("  ... and {} more", missing_source_fqns.len() - 5);
        //     }
        // }

        // if !missing_target_fqns.is_empty() {
        //     warn!(
        //         "Missing target definitions: {} unique FQNs",
        //         missing_target_fqns.len()
        //     );
        //     for (fqn, _file) in missing_target_fqns.iter().take(5) {
        //         debug!("  Missing target: {}", fqn);
        //     }
        //     if missing_target_fqns.len() > 5 {
        //         debug!("  ... and {} more", missing_target_fqns.len() - 5);
        //     }
        // }

        Ok(())
    }
}
