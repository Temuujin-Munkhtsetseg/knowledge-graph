use crate::analysis::types::{
    DefinitionImportedSymbolRelationship, DefinitionLocation, DefinitionNode,
    DefinitionRelationship, DefinitionType, FileDefinitionRelationship,
    FileImportedSymbolRelationship, FqnType, ImportIdentifier, ImportType, ImportedSymbolLocation,
    ImportedSymbolNode,
};
use crate::parsing::processor::FileProcessingResult;
use database::graph::RelationshipType;
use parser_core::typescript::{
    fqn::typescript_fqn_to_string,
    types::{
        TypeScriptDefinitionInfo, TypeScriptDefinitionType, TypeScriptFqn,
        TypeScriptImportedSymbolInfo,
    },
};
use std::collections::HashMap;

// Handles Python-specific analysis operations
pub struct TypeScriptAnalyzer;

impl Default for TypeScriptAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeScriptAnalyzer {
    /// Create a new TypeScript analyzer
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
        if let Some(defs) = file_result.definitions.iter_typescript() {
            for definition in defs {
                if definition.definition_type == TypeScriptDefinitionType::Namespace {
                    continue;
                }

                if let Ok(Some((location, fqn))) =
                    self.create_definition_location(definition, relative_file_path)
                {
                    let fqn_string = typescript_fqn_to_string(&definition.fqn);
                    let definition_node = DefinitionNode::new(
                        fqn_string.clone(),
                        definition.name.clone(),
                        DefinitionType::TypeScript(definition.definition_type),
                        location.clone(),
                    );

                    // If top-level definition, add file-to-definition relationship
                    if definition.fqn.len() == 1 {
                        file_definition_relationships.push(FileDefinitionRelationship {
                            file_path: relative_file_path.to_string(),
                            definition_fqn: fqn_string.clone(),
                            relationship_type: RelationshipType::FileDefines,
                            definition_location: location.clone(),
                        });
                    }

                    definition_map.insert(
                        (fqn_string.clone(), relative_file_path.to_string()),
                        (definition_node, FqnType::TypeScript(fqn.clone())),
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
        if let Some(imported_symbols) = &file_result.imported_symbols {
            if let Some(imports) = imported_symbols.iter_typescript() {
                for imported_symbol in imports {
                    let location =
                        self.create_imported_symbol_location(imported_symbol, relative_file_path);
                    let identifier = self.create_imported_symbol_identifier(imported_symbol);
                    let scope_fqn_string = if let Some(ref scope) = imported_symbol.scope {
                        typescript_fqn_to_string(scope)
                    } else {
                        "".to_string()
                    };
                    let imported_symbol_node = ImportedSymbolNode::new(
                        ImportType::TypeScript(imported_symbol.import_type),
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
                        },
                    );
                }
            }

            // Handle definition-to-definition relationships
            if let Some(parent_fqn_string) = self.get_parent_fqn_string(child_fqn) {
                if let Some((parent_def, _)) =
                    definition_map.get(&(parent_fqn_string.clone(), child_file_path.to_string()))
                {
                    if let Some(relationship_type) = self.get_definition_relationship_type(
                        &parent_def.definition_type,
                        &child_def.definition_type,
                    ) {
                        definition_relationships.push(DefinitionRelationship {
                            from_file_path: parent_def.location.file_path.clone(),
                            to_file_path: child_def.location.file_path.clone(),
                            from_definition_fqn: parent_fqn_string,
                            to_definition_fqn: child_fqn_string.clone(),
                            from_location: parent_def.location.clone(),
                            to_location: child_def.location.clone(),
                            relationship_type,
                        });
                    }
                }
            }
        }
    }

    /// Create a definition location from a definition info
    fn create_definition_location(
        &self,
        definition: &TypeScriptDefinitionInfo,
        file_path: &str,
    ) -> Result<Option<(DefinitionLocation, TypeScriptFqn)>, String> {
        let location = DefinitionLocation {
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
        imported_symbol: &TypeScriptImportedSymbolInfo,
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
        imported_symbol: &TypeScriptImportedSymbolInfo,
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
            FqnType::TypeScript(typescript_fqn) => {
                let parts_len = typescript_fqn.len();
                if parts_len <= 1 {
                    return None;
                }

                // Build parent string directly without creating new Vec
                Some(
                    typescript_fqn[..parts_len - 1]
                        .iter()
                        .map(|part| part.node_name.as_str())
                        .collect::<Vec<_>>()
                        .join("::"),
                )
            }
            _ => None,
        }
    }

    fn simplify_definition_type(&self, definition_type: &DefinitionType) -> DefinitionType {
        match definition_type {
            DefinitionType::TypeScript(TypeScriptDefinitionType::NamedClassExpression) => {
                DefinitionType::TypeScript(TypeScriptDefinitionType::Class)
            }
            DefinitionType::TypeScript(TypeScriptDefinitionType::NamedFunctionExpression) => {
                DefinitionType::TypeScript(TypeScriptDefinitionType::Function)
            }
            DefinitionType::TypeScript(TypeScriptDefinitionType::NamedArrowFunction) => {
                DefinitionType::TypeScript(TypeScriptDefinitionType::Function)
            }
            DefinitionType::TypeScript(
                TypeScriptDefinitionType::NamedGeneratorFunctionExpression,
            ) => DefinitionType::TypeScript(TypeScriptDefinitionType::Function),
            DefinitionType::TypeScript(TypeScriptDefinitionType::NamedCallExpression) => {
                DefinitionType::TypeScript(TypeScriptDefinitionType::Function)
            }
            _ => *definition_type,
        }
    }
    /// Determine the relationship type between parent and child definitions using proper types
    fn get_definition_relationship_type(
        &self,
        parent_type: &DefinitionType,
        child_type: &DefinitionType,
    ) -> Option<RelationshipType> {
        use TypeScriptDefinitionType::*;

        let parent_type = self.simplify_definition_type(parent_type);
        let child_type = self.simplify_definition_type(child_type);

        match (parent_type, child_type) {
            (DefinitionType::TypeScript(Class), DefinitionType::TypeScript(Class)) => {
                Some(RelationshipType::ClassToClass)
            }
            (DefinitionType::TypeScript(Class), DefinitionType::TypeScript(Method)) => {
                Some(RelationshipType::ClassToMethod)
            }
            (DefinitionType::TypeScript(Method), DefinitionType::TypeScript(Class)) => {
                Some(RelationshipType::MethodToClass)
            }
            (DefinitionType::TypeScript(Method), DefinitionType::TypeScript(Function)) => {
                Some(RelationshipType::MethodToFunction)
            }
            (DefinitionType::TypeScript(Function), DefinitionType::TypeScript(Class)) => {
                Some(RelationshipType::FunctionToClass)
            }
            (DefinitionType::TypeScript(Function), DefinitionType::TypeScript(Function)) => {
                Some(RelationshipType::FunctionToFunction)
            }
            (DefinitionType::TypeScript(Interface), DefinitionType::TypeScript(Class)) => {
                Some(RelationshipType::InterfaceToClass)
            }
            (DefinitionType::TypeScript(Interface), DefinitionType::TypeScript(Function)) => {
                Some(RelationshipType::InterfaceToFunction)
            }
            _ => None, // Unknown or unsupported relationship
        }
    }
}
