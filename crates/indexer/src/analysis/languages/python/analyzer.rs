use crate::analysis::languages::python::interfile::get_possible_symbol_locations;
use crate::analysis::types::{
    DefinitionImportedSymbolRelationship, DefinitionNode, DefinitionRelationship, DefinitionType,
    FileDefinitionRelationship, FileImportedSymbolRelationship, FqnType, ImportIdentifier,
    ImportType, ImportedSymbolLocation, ImportedSymbolNode, OptimizedFileTree, SourceLocation,
};
use crate::parsing::processor::{FileProcessingResult, References};
use database::graph::RelationshipType;
use parser_core::python::types::PythonImportType;
use parser_core::python::types::PythonReferenceInfo;
use parser_core::python::{
    fqn::python_fqn_to_string,
    types::{
        PythonDefinitionInfo, PythonDefinitionType, PythonFqn, PythonImportedSymbolInfo,
        PythonTargetResolution,
    },
};
use parser_core::references::ReferenceTarget;
use std::collections::{HashMap, HashSet};

/// Represents the result of resolving an imported symbol
#[derive(Debug, Clone)]
enum ResolvedTarget {
    ImportedSymbol(ImportedSymbolNode),
    Definition(DefinitionNode),
}

// Handles Python-specific analysis operations
pub struct PythonAnalyzer;

impl Default for PythonAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonAnalyzer {
    /// Create a new Python analyzer
    pub fn new() -> Self {
        Self
    }

    /// Process definitions from a file result and update the definitions map
    pub fn process_definitions(
        &self,
        file_result: &FileProcessingResult,
        relative_file_path: &str,
        definition_map: &mut HashMap<(String, String), (DefinitionNode, FqnType)>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
    ) {
        if let Some(defs) = file_result.definitions.iter_python() {
            for definition in defs {
                if let Ok(Some((location, fqn))) =
                    self.create_definition_location(definition, relative_file_path)
                {
                    let fqn_string = python_fqn_to_string(&fqn);
                    let definition_node = DefinitionNode::new(
                        fqn_string.clone(),
                        definition.name.clone(),
                        DefinitionType::Python(definition.definition_type),
                        location.clone(),
                    );

                    if self.is_top_level_definition(&fqn) {
                        file_definition_relationships.push(FileDefinitionRelationship {
                            file_path: relative_file_path.to_string(),
                            definition_fqn: fqn_string.clone(),
                            relationship_type: RelationshipType::FileDefines,
                            definition_location: location.clone(),
                        });
                    }

                    definition_map.insert(
                        (fqn_string.clone(), relative_file_path.to_string()),
                        (definition_node, FqnType::Python(fqn)),
                    );
                }
            }
        }
    }

    /// Process imported symbols from a file result and update the import map
    pub fn process_imports(
        &self,
        file_result: &FileProcessingResult,
        relative_file_path: &str,
        imported_symbol_map: &mut HashMap<(String, String), Vec<ImportedSymbolNode>>,
        file_import_relationships: &mut Vec<FileImportedSymbolRelationship>,
    ) {
        if let Some(imported_symbols) = &file_result.imported_symbols
            && let Some(imports) = imported_symbols.iter_python()
        {
            for imported_symbol in imports {
                let location =
                    self.create_imported_symbol_location(imported_symbol, relative_file_path);
                let identifier = self.create_imported_symbol_identifier(imported_symbol);
                let scope_fqn_string = if let Some(ref scope) = imported_symbol.scope {
                    python_fqn_to_string(scope)
                } else {
                    "".to_string()
                };
                let imported_symbol_node = ImportedSymbolNode::new(
                    ImportType::Python(imported_symbol.import_type),
                    imported_symbol.import_path.clone(),
                    identifier,
                    location.clone(),
                );

                if let Some(imported_symbol_nodes) = imported_symbol_map
                    .get_mut(&(scope_fqn_string.clone(), relative_file_path.to_string()))
                {
                    imported_symbol_nodes.push(imported_symbol_node);
                } else {
                    imported_symbol_map.insert(
                        (scope_fqn_string.clone(), relative_file_path.to_string()),
                        vec![imported_symbol_node],
                    );
                }

                file_import_relationships.push(FileImportedSymbolRelationship {
                    file_path: relative_file_path.to_string(),
                    import_location: location.clone(),
                    relationship_type: RelationshipType::FileImports,
                });
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process_references(
        &self,
        file_references: &Option<References>,
        relative_file_path: &str,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
        definition_imported_symbol_relationships: &mut Vec<DefinitionImportedSymbolRelationship>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
        file_import_relationships: &mut Vec<FileImportedSymbolRelationship>,
        imported_symbol_to_imported_symbols: &HashMap<
            ImportedSymbolLocation,
            Vec<ImportedSymbolNode>,
        >,
        imported_symbol_to_definitions: &HashMap<ImportedSymbolLocation, Vec<DefinitionNode>>,
        imported_symbol_to_files: &HashMap<ImportedSymbolLocation, Vec<String>>,
    ) {
        let file_path = relative_file_path.to_string();
        if let Some(references) = file_references
            && let Some(references) = references.iter_python()
        {
            for reference in references {
                let source_definition = if let Some(scope) = reference.scope.as_ref() {
                    let fqn_string = python_fqn_to_string(scope);
                    definition_map
                        .get(&(fqn_string, file_path.clone()))
                        .map(|map_value| map_value.0.clone())
                } else {
                    None
                };

                match &reference.target {
                    ReferenceTarget::Resolved(resolved_target) => {
                        self.process_resolved_target(
                            resolved_target,
                            &file_path,
                            reference,
                            &source_definition,
                            definition_map,
                            definition_relationships,
                            definition_imported_symbol_relationships,
                            file_definition_relationships,
                            file_import_relationships,
                            imported_symbol_to_imported_symbols,
                            imported_symbol_to_definitions,
                            imported_symbol_to_files,
                            false,
                        );
                    }
                    ReferenceTarget::Ambiguous(possible_targets) => {
                        for possible_target in possible_targets {
                            self.process_resolved_target(
                                possible_target,
                                &file_path,
                                reference,
                                &source_definition,
                                definition_map,
                                definition_relationships,
                                definition_imported_symbol_relationships,
                                file_definition_relationships,
                                file_import_relationships,
                                imported_symbol_to_imported_symbols,
                                imported_symbol_to_definitions,
                                imported_symbol_to_files,
                                true,
                            );
                        }
                    }
                    ReferenceTarget::Unresolved() => {
                        // TODO: Handle references to symbols brought in via wildcard imports
                        continue;
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process_resolved_target(
        &self,
        resolved_target: &PythonTargetResolution,
        file_path: &str,
        reference: &PythonReferenceInfo,
        source_definition: &Option<DefinitionNode>,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
        definition_imported_symbol_relationships: &mut Vec<DefinitionImportedSymbolRelationship>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
        file_import_relationships: &mut Vec<FileImportedSymbolRelationship>,
        imported_symbol_to_imported_symbols: &HashMap<
            ImportedSymbolLocation,
            Vec<ImportedSymbolNode>,
        >,
        imported_symbol_to_definitions: &HashMap<ImportedSymbolLocation, Vec<DefinitionNode>>,
        imported_symbol_to_files: &HashMap<ImportedSymbolLocation, Vec<String>>,
        is_ambiguous: bool,
    ) {
        match resolved_target {
            PythonTargetResolution::Definition(target_def_info) => {
                let target_def_node = definition_map
                    .get(&(
                        python_fqn_to_string(&target_def_info.fqn),
                        file_path.to_owned(),
                    ))
                    .map(|map_value| map_value.0.clone());
                if let Some(target_def_node) = target_def_node {
                    self.add_definition_reference_relationship(
                        file_path,
                        reference,
                        source_definition,
                        &target_def_node,
                        definition_relationships,
                        file_definition_relationships,
                        is_ambiguous,
                    );
                }
            }
            PythonTargetResolution::ImportedSymbol(target_import_info) => {
                let mut results = Vec::new();
                let mut visited = HashSet::new();

                fn resolve_recursive(
                    current_location: ImportedSymbolLocation,
                    imported_symbol_to_imported_symbols: &HashMap<
                        ImportedSymbolLocation,
                        Vec<ImportedSymbolNode>,
                    >,
                    imported_symbol_to_definitions: &HashMap<
                        ImportedSymbolLocation,
                        Vec<DefinitionNode>,
                    >,
                    imported_symbol_to_files: &HashMap<ImportedSymbolLocation, Vec<String>>,
                    results: &mut Vec<ResolvedTarget>,
                    visited: &mut HashSet<ImportedSymbolLocation>,
                ) {
                    // Prevent infinite recursion
                    if visited.contains(&current_location) {
                        return;
                    }
                    visited.insert(current_location.clone());

                    // Check imported_symbol_to_imported_symbols hashmap
                    if let Some(matched_imported_symbols) =
                        imported_symbol_to_imported_symbols.get(&current_location)
                    {
                        for matched_imported_symbol in matched_imported_symbols {
                            // Check if this is a terminal imported symbol (no further resolution)
                            let is_terminal = !imported_symbol_to_imported_symbols
                                .contains_key(&matched_imported_symbol.location)
                                && !imported_symbol_to_definitions
                                    .contains_key(&matched_imported_symbol.location)
                                && !imported_symbol_to_files
                                    .contains_key(&matched_imported_symbol.location);

                            if is_terminal {
                                results.push(ResolvedTarget::ImportedSymbol(
                                    matched_imported_symbol.clone(),
                                ));
                            } else {
                                // Keep recursing
                                resolve_recursive(
                                    matched_imported_symbol.location.clone(),
                                    imported_symbol_to_imported_symbols,
                                    imported_symbol_to_definitions,
                                    imported_symbol_to_files,
                                    results,
                                    visited,
                                );
                            }
                        }
                    }

                    // Check imported_symbol_to_definitions hashmap
                    if let Some(matched_definitions) =
                        imported_symbol_to_definitions.get(&current_location)
                    {
                        for matched_definition in matched_definitions {
                            results.push(ResolvedTarget::Definition(matched_definition.clone()));
                        }
                    }

                    // Check imported_symbol_to_files hashmap
                    if imported_symbol_to_files.contains_key(&current_location) {
                        // Ignore and terminate search as this case is only possible for wildcard imports or partial resolutions
                        todo!();
                    }
                }

                let imported_symbol_location =
                    self.create_imported_symbol_location(target_import_info, file_path);
                resolve_recursive(
                    imported_symbol_location.clone(),
                    imported_symbol_to_imported_symbols,
                    imported_symbol_to_definitions,
                    imported_symbol_to_files,
                    &mut results,
                    &mut visited,
                );

                // Create relationships based on resolved targets
                let is_ambiguous = results.len() > 1 || is_ambiguous;
                for resolved_target in results {
                    match resolved_target {
                        ResolvedTarget::ImportedSymbol(target_imported_symbol_node) => {
                            self.add_imported_symbol_reference_relationship(
                                file_path,
                                reference,
                                source_definition,
                                &target_imported_symbol_node,
                                definition_imported_symbol_relationships,
                                file_import_relationships,
                                is_ambiguous,
                            );
                        }
                        ResolvedTarget::Definition(target_definition_node) => self
                            .add_definition_reference_relationship(
                                file_path,
                                reference,
                                source_definition,
                                &target_definition_node,
                                definition_relationships,
                                file_definition_relationships,
                                is_ambiguous,
                            ),
                    }
                }
            }
            PythonTargetResolution::PartialResolution(_symbol_chain) => {
                // TODO
            }
        }
    }

    pub fn resolve_imported_symbols(
        &self,
        imported_symbol_map: &HashMap<(String, String), Vec<ImportedSymbolNode>>,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        file_tree: &OptimizedFileTree,
        imported_symbol_to_imported_symbols: &mut HashMap<
            ImportedSymbolLocation,
            Vec<ImportedSymbolNode>,
        >,
        imported_symbol_to_definitions: &mut HashMap<ImportedSymbolLocation, Vec<DefinitionNode>>,
        imported_symbol_to_files: &mut HashMap<ImportedSymbolLocation, Vec<String>>,
    ) {
        for ((_imported_symbol_fqn_string, _imported_symbol_file_path), imported_symbol_nodes) in
            imported_symbol_map
        {
            for imported_symbol_node in imported_symbol_nodes {
                if let ImportType::Python(import_type) = imported_symbol_node.import_type {
                    let possible_files = get_possible_symbol_locations(
                        imported_symbol_node,
                        file_tree,
                        definition_map,
                    );

                    match import_type {
                        PythonImportType::FutureImport | PythonImportType::AliasedFutureImport => {}
                        PythonImportType::Import | PythonImportType::AliasedImport => {
                            // NOTE: For now, we are ignoring other possible files because it's very unlikely that there will
                            // be more than one
                            if let Some(possible_file) = possible_files.first() {
                                imported_symbol_to_files.insert(
                                    imported_symbol_node.location.clone(),
                                    vec![possible_file.clone()],
                                );
                            }
                        }
                        PythonImportType::WildcardImport
                        | PythonImportType::RelativeWildcardImport => {
                            // TODO: We should preserve all *possible* relationships instead of only the first. When we attempt to resolve
                            // unresolved or partial resolutions, we will need to explore all possible files for a symbol.
                            let first_possible_file = possible_files.first();
                            if let Some(first_possible_file) = first_possible_file {
                                imported_symbol_to_files.insert(
                                    imported_symbol_node.location.clone(),
                                    vec![first_possible_file.clone()],
                                );
                            }
                        }
                        // From imports (`from A import B`, `from A import B as C`, `from . import A`, `from . import *`)
                        _ => {
                            if let Some(name) = imported_symbol_node
                                .identifier
                                .as_ref()
                                .map(|identifier| identifier.name.clone())
                            {
                                let mut matched_definitions = vec![];
                                let mut matched_imported_symbols = vec![];
                                for possible_file in possible_files {
                                    // Get matching definition and imported symbol (if either exist)
                                    let matched_definition_node = definition_map
                                        .get(&(name.clone(), possible_file.clone()))
                                        .map(|(definition_node, _)| definition_node.clone());
                                    let matched_imported_symbol_node =
                                        if let Some(imported_symbol_nodes) = imported_symbol_map
                                            .get(&("".to_string(), possible_file.clone()))
                                        {
                                            imported_symbol_nodes
                                                .iter()
                                                .filter(|node| {
                                                    if let Some(identifier) = &node.identifier {
                                                        if let Some(alias) = &identifier.alias {
                                                            alias == &name
                                                        } else {
                                                            identifier.name == name
                                                        }
                                                    } else {
                                                        false
                                                    }
                                                })
                                                .max_by_key(|node| node.location.start_byte)
                                        } else {
                                            None
                                        };

                                    // Prefer the most recent symbol: imported symbol if it exists and is more recent, otherwise definition, otherwise imported symbol
                                    match (matched_definition_node, matched_imported_symbol_node) {
                                        (Some(def_node), Some(imp_node)) => {
                                            if imp_node.location.start_byte
                                                > def_node.location.start_byte
                                            {
                                                matched_imported_symbols.push(imp_node.clone());
                                            } else {
                                                matched_definitions.push(def_node);
                                            }
                                        }
                                        (Some(def_node), None) => {
                                            matched_definitions.push(def_node);
                                        }
                                        (None, Some(imp_node)) => {
                                            matched_imported_symbols.push(imp_node.clone());
                                        }
                                        (None, None) => {}
                                    }
                                }

                                imported_symbol_to_imported_symbols.insert(
                                    imported_symbol_node.location.clone(),
                                    matched_imported_symbols,
                                );
                                imported_symbol_to_definitions.insert(
                                    imported_symbol_node.location.clone(),
                                    matched_definitions,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Create definition-to-definition and definition-to-imported-symbol relationships using definitions map
    pub fn add_definition_relationships(
        &self,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        imported_symbol_map: &HashMap<(String, String), Vec<ImportedSymbolNode>>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
        definition_imported_symbol_relationships: &mut Vec<DefinitionImportedSymbolRelationship>,
    ) {
        for ((child_fqn_string, child_file_path), (child_def, child_fqn)) in definition_map {
            // Handle definition-to-imported-symbol relationships
            if let Some(imported_symbol_nodes) =
                imported_symbol_map.get(&(child_fqn_string.clone(), child_file_path.to_string()))
            {
                for imported_symbol in imported_symbol_nodes {
                    definition_imported_symbol_relationships.push(
                        DefinitionImportedSymbolRelationship {
                            file_path: child_file_path.clone(),
                            definition_fqn: child_fqn_string.clone(),
                            imported_symbol_location: imported_symbol.location.clone(),
                            relationship_type: RelationshipType::DefinesImportedSymbol,
                            definition_location: child_def.location.clone(),
                            // FIXME: add source location for Python imports
                            source_location: None,
                        },
                    );
                }
            }

            // Handle definition-to-definition relationships
            if let Some(parent_fqn_string) = self.get_parent_fqn_string(child_fqn)
                && let Some((parent_def, _)) =
                    definition_map.get(&(parent_fqn_string.clone(), child_file_path.to_string()))
                && let Some(relationship_type) = self.get_definition_relationship_type(
                    &parent_def.definition_type,
                    &child_def.definition_type,
                )
            {
                definition_relationships.push(DefinitionRelationship {
                    from_file_path: parent_def.location.file_path.clone(),
                    to_file_path: child_def.location.file_path.clone(),
                    from_definition_fqn: parent_fqn_string,
                    to_definition_fqn: child_fqn_string.clone(),
                    from_location: parent_def.location.clone(),
                    to_location: child_def.location.clone(),
                    relationship_type,
                    source_location: None,
                });
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add_definition_reference_relationship(
        &self,
        file_path: &str,
        reference: &PythonReferenceInfo,
        source_definition: &Option<DefinitionNode>,
        target_definition_node: &DefinitionNode,
        definition_relationships: &mut Vec<DefinitionRelationship>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
        is_ambiguous: bool,
    ) {
        if source_definition.is_none() {
            let relationship = FileDefinitionRelationship {
                file_path: file_path.to_string(),
                definition_fqn: target_definition_node.fqn.clone(),
                relationship_type: if is_ambiguous {
                    RelationshipType::AmbiguouslyCalls
                } else {
                    RelationshipType::Calls
                },
                definition_location: target_definition_node.location.clone(),
            };
            // TODO: Add source location
            file_definition_relationships.push(relationship);
        } else {
            let source_definition = source_definition.as_ref().unwrap();
            let relationship = DefinitionRelationship {
                from_file_path: source_definition.location.file_path.clone(),
                to_file_path: target_definition_node.location.file_path.clone(),
                from_definition_fqn: source_definition.fqn.clone(),
                to_definition_fqn: target_definition_node.fqn.clone(),
                from_location: source_definition.location.clone(),
                to_location: target_definition_node.location.clone(),
                relationship_type: if is_ambiguous {
                    RelationshipType::AmbiguouslyCalls
                } else {
                    RelationshipType::Calls
                },
                source_location: Some(SourceLocation {
                    file_path: file_path.to_string(),
                    start_byte: reference.range.byte_offset.0 as i64,
                    end_byte: reference.range.byte_offset.1 as i64,
                    start_line: reference.range.start.line as i32,
                    end_line: reference.range.end.line as i32,
                    start_col: reference.range.start.column as i32,
                    end_col: reference.range.end.column as i32,
                }),
            };
            definition_relationships.push(relationship);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add_imported_symbol_reference_relationship(
        &self,
        file_path: &str,
        reference: &PythonReferenceInfo,
        source_definition: &Option<DefinitionNode>,
        target_imported_symbol_node: &ImportedSymbolNode,
        definition_imported_symbol_relationships: &mut Vec<DefinitionImportedSymbolRelationship>,
        file_import_relationships: &mut Vec<FileImportedSymbolRelationship>,
        is_ambiguous: bool,
    ) {
        if source_definition.is_none() {
            let relationship = FileImportedSymbolRelationship {
                file_path: file_path.to_string(),
                import_location: target_imported_symbol_node.location.clone(),
                relationship_type: if is_ambiguous {
                    RelationshipType::AmbiguouslyCalls
                } else {
                    RelationshipType::Calls
                },
            };
            // TODO: Add source location
            file_import_relationships.push(relationship);
        } else {
            let source_definition = source_definition.as_ref().unwrap();
            let relationship = DefinitionImportedSymbolRelationship {
                file_path: file_path.to_string(),
                definition_fqn: source_definition.fqn.clone(),
                imported_symbol_location: target_imported_symbol_node.location.clone(),
                relationship_type: if is_ambiguous {
                    RelationshipType::AmbiguouslyCalls
                } else {
                    RelationshipType::Calls
                },
                definition_location: source_definition.location.clone(),
                source_location: Some(SourceLocation {
                    file_path: file_path.to_string(),
                    start_byte: reference.range.byte_offset.0 as i64,
                    end_byte: reference.range.byte_offset.1 as i64,
                    start_line: reference.range.start.line as i32,
                    end_line: reference.range.end.line as i32,
                    start_col: reference.range.start.column as i32,
                    end_col: reference.range.end.column as i32,
                }),
            };
            definition_imported_symbol_relationships.push(relationship);
        }
    }

    /// Create a definition location from a definition info
    fn create_definition_location(
        &self,
        definition: &PythonDefinitionInfo,
        file_path: &str,
    ) -> Result<Option<(SourceLocation, PythonFqn)>, String> {
        let location = SourceLocation {
            file_path: file_path.to_string(),
            start_byte: definition.range.byte_offset.0 as i64,
            end_byte: definition.range.byte_offset.1 as i64,
            start_line: definition.range.start.line as i32,
            end_line: definition.range.end.line as i32,
            start_col: definition.range.start.column as i32,
            end_col: definition.range.end.column as i32,
        };

        Ok(Some((location, definition.fqn.clone())))
    }

    /// Create an imported symbol location from an imported symbol info
    fn create_imported_symbol_location(
        &self,
        imported_symbol: &PythonImportedSymbolInfo,
        file_path: &str,
    ) -> ImportedSymbolLocation {
        ImportedSymbolLocation {
            file_path: file_path.to_string(),
            start_byte: imported_symbol.range.byte_offset.0 as i64,
            end_byte: imported_symbol.range.byte_offset.1 as i64,
            start_line: imported_symbol.range.start.line as i32,
            end_line: imported_symbol.range.end.line as i32,
            start_col: imported_symbol.range.start.column as i32,
            end_col: imported_symbol.range.end.column as i32,
        }
    }

    fn create_imported_symbol_identifier(
        &self,
        imported_symbol: &PythonImportedSymbolInfo,
    ) -> Option<ImportIdentifier> {
        if imported_symbol.identifier.is_some() {
            return Some(ImportIdentifier {
                name: imported_symbol.identifier.as_ref().unwrap().name.clone(),
                alias: imported_symbol.identifier.as_ref().unwrap().alias.clone(),
            });
        }

        None
    }

    /// Extract parent FQN string from a given FQN
    fn get_parent_fqn_string(&self, fqn: &FqnType) -> Option<String> {
        match fqn {
            FqnType::Python(python_fqn) => {
                let parts_len = python_fqn.parts.len();
                if parts_len <= 1 {
                    return None;
                }

                // Build parent string directly without creating new Vec
                Some(
                    python_fqn.parts[..parts_len - 1]
                        .iter()
                        .map(|part| part.node_name.replace('.', "#"))
                        .collect::<Vec<_>>()
                        .join("."),
                )
            }
            _ => None,
        }
    }

    fn simplify_definition_type(&self, definition_type: &DefinitionType) -> Option<DefinitionType> {
        use PythonDefinitionType::*;

        match definition_type {
            DefinitionType::Python(Class) | DefinitionType::Python(DecoratedClass) => {
                Some(DefinitionType::Python(Class))
            }
            DefinitionType::Python(Method)
            | DefinitionType::Python(AsyncMethod)
            | DefinitionType::Python(DecoratedMethod)
            | DefinitionType::Python(DecoratedAsyncMethod) => Some(DefinitionType::Python(Method)),
            DefinitionType::Python(Function)
            | DefinitionType::Python(AsyncFunction)
            | DefinitionType::Python(DecoratedFunction)
            | DefinitionType::Python(DecoratedAsyncFunction) => {
                Some(DefinitionType::Python(Function))
            }
            DefinitionType::Python(Lambda) => Some(DefinitionType::Python(Lambda)),
            _ => None,
        }
    }

    /// Determine the relationship type between parent and child definitions using proper types
    fn get_definition_relationship_type(
        &self,
        parent_type: &DefinitionType,
        child_type: &DefinitionType,
    ) -> Option<RelationshipType> {
        use PythonDefinitionType::*;

        let parent_type = self.simplify_definition_type(parent_type)?;
        let child_type = self.simplify_definition_type(child_type)?;

        match (parent_type, child_type) {
            (DefinitionType::Python(Class), DefinitionType::Python(Class)) => {
                Some(RelationshipType::ClassToClass)
            }
            (DefinitionType::Python(Class), DefinitionType::Python(Method)) => {
                Some(RelationshipType::ClassToMethod)
            }
            (DefinitionType::Python(Class), DefinitionType::Python(Lambda)) => {
                Some(RelationshipType::ClassToLambda)
            }
            (DefinitionType::Python(Method), DefinitionType::Python(Class)) => {
                Some(RelationshipType::MethodToClass)
            }
            (DefinitionType::Python(Method), DefinitionType::Python(Function)) => {
                Some(RelationshipType::MethodToFunction)
            }
            (DefinitionType::Python(Method), DefinitionType::Python(Lambda)) => {
                Some(RelationshipType::MethodToLambda)
            }
            (DefinitionType::Python(Function), DefinitionType::Python(Function)) => {
                Some(RelationshipType::FunctionToFunction)
            }
            (DefinitionType::Python(Function), DefinitionType::Python(Class)) => {
                Some(RelationshipType::FunctionToClass)
            }
            (DefinitionType::Python(Function), DefinitionType::Python(Lambda)) => {
                Some(RelationshipType::FunctionToLambda)
            }
            (DefinitionType::Python(Lambda), DefinitionType::Python(Lambda)) => {
                Some(RelationshipType::LambdaToLambda)
            }
            (DefinitionType::Python(Lambda), DefinitionType::Python(Class)) => {
                Some(RelationshipType::LambdaToClass)
            }
            (DefinitionType::Python(Lambda), DefinitionType::Python(Function)) => {
                Some(RelationshipType::LambdaToFunction)
            }
            _ => None, // Unknown or unsupported relationship
        }
    }

    fn is_top_level_definition(&self, fqn: &PythonFqn) -> bool {
        fqn.len() == 1
    }
}
