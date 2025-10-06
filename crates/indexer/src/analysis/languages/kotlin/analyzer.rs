use std::collections::HashMap;

use parser_core::kotlin::{
    ast::kotlin_fqn_to_string,
    types::{KotlinDefinitionType, KotlinFqn, KotlinFqnPartType, KotlinImportedSymbolInfo},
};

use crate::{
    analysis::{
        languages::kotlin::{
            expression_resolver::KotlinExpressionResolver, utils::full_import_path,
        },
        types::{
            ConsolidatedRelationship, DefinitionNode, DefinitionType, FqnType, ImportIdentifier,
            ImportType, ImportedSymbolLocation, ImportedSymbolNode,
        },
    },
    parsing::processor::{FileProcessingResult, References},
};
use database::graph::RelationshipType;
use internment::ArcIntern;
use parser_core::utils::Range;

#[derive(Default)]
pub struct KotlinAnalyzer {
    expression_resolver: KotlinExpressionResolver,
}

impl KotlinAnalyzer {
    pub fn new() -> Self {
        Self {
            expression_resolver: KotlinExpressionResolver::default(),
        }
    }

    pub fn process_definitions(
        &mut self,
        file_result: &FileProcessingResult,
        relative_file_path: &str,
        definition_map: &mut HashMap<(String, String), (DefinitionNode, FqnType)>,
        relationships: &mut Vec<ConsolidatedRelationship>,
    ) {
        if let Some(defs) = file_result.definitions.iter_kotlin() {
            for definition in defs {
                if matches!(definition.definition_type, KotlinDefinitionType::Package) {
                    self.expression_resolver
                        .add_file(definition.name.clone(), relative_file_path.to_string());
                    continue;
                }

                let fqn_string = kotlin_fqn_to_string(&definition.fqn);
                let definition_node = DefinitionNode::new(
                    fqn_string.clone(),
                    definition.name.clone(),
                    DefinitionType::Kotlin(definition.definition_type),
                    definition.range,
                    relative_file_path.to_string(),
                );

                self.expression_resolver.add_definition(
                    relative_file_path.to_string(),
                    definition.clone(),
                    definition_node.clone(),
                );

                if definition.definition_type == KotlinDefinitionType::Parameter
                    || definition.definition_type == KotlinDefinitionType::LocalVariable
                {
                    continue;
                }

                if self.is_top_level_definition(&definition.fqn) {
                    let mut relationship = ConsolidatedRelationship::file_to_definition(
                        relative_file_path.to_string(),
                        relative_file_path.to_string(),
                    );
                    relationship.relationship_type = RelationshipType::FileDefines;
                    relationship.source_range = ArcIntern::new(Range::empty());
                    relationship.target_range = ArcIntern::new(definition.range);
                    relationships.push(relationship);
                }

                let key = (fqn_string.clone(), relative_file_path.to_string());
                definition_map.insert(
                    key,
                    (
                        definition_node.clone(),
                        FqnType::Kotlin(definition.fqn.clone()),
                    ),
                );
            }
        }
    }

    /// Process imported symbols from a file result and update the import map
    pub fn process_imports(
        &mut self,
        file_result: &FileProcessingResult,
        relative_file_path: &str,
        imported_symbol_map: &mut HashMap<(String, String), Vec<ImportedSymbolNode>>,
        relationships: &mut Vec<ConsolidatedRelationship>,
    ) {
        if let Some(imported_symbols) = &file_result.imported_symbols
            && let Some(imports) = imported_symbols.iter_kotlin()
        {
            for imported_symbol in imports {
                let location =
                    self.create_imported_symbol_location(imported_symbol, relative_file_path);
                let identifier = self.create_imported_symbol_identifier(imported_symbol);

                let imported_symbol_node = ImportedSymbolNode::new(
                    ImportType::Kotlin(imported_symbol.import_type),
                    imported_symbol.import_path.clone(),
                    identifier,
                    location.clone(),
                );

                let (_, full_import_path) = full_import_path(&imported_symbol_node);
                imported_symbol_map.insert(
                    (full_import_path, relative_file_path.to_string()),
                    vec![imported_symbol_node.clone()],
                );

                let mut relationship = ConsolidatedRelationship::file_to_imported_symbol(
                    relative_file_path.to_string(),
                    location.file_path.clone(),
                );
                relationship.relationship_type = RelationshipType::FileImports;
                relationship.source_range = ArcIntern::new(Range::empty());
                relationship.target_range = ArcIntern::new(location.range());
                relationships.push(relationship);

                self.expression_resolver
                    .add_import(relative_file_path.to_string(), &imported_symbol_node);
            }
        }
    }

    pub fn process_references(
        &self,
        file_references: &References,
        relative_file_path: &str,
        relationships: &mut Vec<ConsolidatedRelationship>,
    ) {
        self.expression_resolver.resolve_expressions(
            relative_file_path,
            file_references,
            relationships,
        );
    }

    pub fn add_definition_relationships(
        &self,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        relationships: &mut Vec<ConsolidatedRelationship>,
    ) {
        for ((_, child_file_path), (child_def, child_fqn)) in definition_map {
            if let Some(parent_fqn) = self.get_parent_fqn_string(child_fqn)
                && let Some((parent_def, _)) =
                    definition_map.get(&(parent_fqn.clone(), child_file_path.to_string()))
                && let Some(relationship_type) = self.get_definition_relationship_type(
                    &parent_def.definition_type,
                    &child_def.definition_type,
                )
            {
                let mut relationship = ConsolidatedRelationship::definition_to_definition(
                    parent_def.file_path.clone(),
                    child_def.file_path.clone(),
                );
                relationship.relationship_type = relationship_type;
                relationship.source_range = ArcIntern::new(parent_def.range);
                relationship.target_range = ArcIntern::new(child_def.range);
                relationships.push(relationship);
            }
        }
    }

    fn get_parent_fqn_string(&self, fqn: &FqnType) -> Option<String> {
        match fqn {
            FqnType::Kotlin(kotlin_fqn) => {
                if kotlin_fqn.len() <= 1 {
                    return None;
                }

                let parent_parts: Vec<String> = kotlin_fqn[..kotlin_fqn.len() - 1]
                    .iter()
                    .map(|part| part.node_name.clone())
                    .collect();

                if parent_parts.is_empty() {
                    None
                } else {
                    Some(parent_parts.join("."))
                }
            }
            _ => None,
        }
    }

    fn get_definition_relationship_type(
        &self,
        parent_type: &DefinitionType,
        child_type: &DefinitionType,
    ) -> Option<RelationshipType> {
        use KotlinDefinitionType::*;

        let parent_type = self.simplify_definition_type(parent_type)?;
        let child_type = self.simplify_definition_type(child_type)?;

        match (parent_type, child_type) {
            // Class relationships
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(Class)) => {
                Some(RelationshipType::ClassToClass)
            }
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(Interface)) => {
                Some(RelationshipType::ClassToInterface)
            }
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(Function)) => {
                Some(RelationshipType::ClassToMethod)
            }
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(Property)) => {
                Some(RelationshipType::ClassToProperty)
            }
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(Lambda)) => {
                Some(RelationshipType::ClassToLambda)
            }
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(Constructor)) => {
                Some(RelationshipType::ClassToConstructor)
            }
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(EnumEntry)) => {
                Some(RelationshipType::ClassToEnumEntry)
            }
            // Interface relationships
            (DefinitionType::Kotlin(Interface), DefinitionType::Kotlin(Interface)) => {
                Some(RelationshipType::InterfaceToInterface)
            }
            (DefinitionType::Kotlin(Interface), DefinitionType::Kotlin(Class)) => {
                Some(RelationshipType::InterfaceToClass)
            }
            (DefinitionType::Kotlin(Interface), DefinitionType::Kotlin(Function)) => {
                Some(RelationshipType::InterfaceToMethod)
            }
            (DefinitionType::Kotlin(Interface), DefinitionType::Kotlin(Property)) => {
                Some(RelationshipType::InterfaceToProperty)
            }
            (DefinitionType::Kotlin(Interface), DefinitionType::Kotlin(Lambda)) => {
                Some(RelationshipType::InterfaceToLambda)
            }
            // Function relationships
            (DefinitionType::Kotlin(Function), DefinitionType::Kotlin(Function)) => {
                Some(RelationshipType::MethodToMethod)
            }
            (DefinitionType::Kotlin(Function), DefinitionType::Kotlin(Property)) => {
                Some(RelationshipType::MethodToProperty)
            }
            (DefinitionType::Kotlin(Function), DefinitionType::Kotlin(Lambda)) => {
                Some(RelationshipType::MethodToLambda)
            }
            (DefinitionType::Kotlin(Function), DefinitionType::Kotlin(Class)) => {
                Some(RelationshipType::MethodToClass)
            }
            (DefinitionType::Kotlin(Function), DefinitionType::Kotlin(Interface)) => {
                Some(RelationshipType::MethodToInterface)
            }
            // Lambda relationships
            (DefinitionType::Kotlin(Lambda), DefinitionType::Kotlin(Lambda)) => {
                Some(RelationshipType::LambdaToLambda)
            }
            (DefinitionType::Kotlin(Lambda), DefinitionType::Kotlin(Class)) => {
                Some(RelationshipType::LambdaToClass)
            }
            (DefinitionType::Kotlin(Lambda), DefinitionType::Kotlin(Function)) => {
                Some(RelationshipType::LambdaToMethod)
            }
            (DefinitionType::Kotlin(Lambda), DefinitionType::Kotlin(Property)) => {
                Some(RelationshipType::LambdaToProperty)
            }
            (DefinitionType::Kotlin(Lambda), DefinitionType::Kotlin(Interface)) => {
                Some(RelationshipType::LambdaToInterface)
            }
            _ => None,
        }
    }

    fn simplify_definition_type(&self, definition_type: &DefinitionType) -> Option<DefinitionType> {
        use KotlinDefinitionType::*;

        match definition_type {
            DefinitionType::Kotlin(Class) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(DataClass) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(ValueClass) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(AnnotationClass) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(Object) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(CompanionObject) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(Enum) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(Interface) => Some(DefinitionType::Kotlin(Interface)),
            DefinitionType::Kotlin(EnumEntry) => Some(DefinitionType::Kotlin(EnumEntry)),
            DefinitionType::Kotlin(Constructor) => Some(DefinitionType::Kotlin(Constructor)),
            DefinitionType::Kotlin(Function) => Some(DefinitionType::Kotlin(Function)),
            DefinitionType::Kotlin(Property) => Some(DefinitionType::Kotlin(Property)),
            DefinitionType::Kotlin(Lambda) => Some(DefinitionType::Kotlin(Lambda)),
            _ => None,
        }
    }

    fn is_top_level_definition(&self, fqn: &KotlinFqn) -> bool {
        fqn.len() == 1 || (fqn.len() == 2 && fqn[0].node_type == KotlinFqnPartType::Package)
    }

    /// Create an imported symbol location from an imported symbol info
    fn create_imported_symbol_location(
        &self,
        imported_symbol: &KotlinImportedSymbolInfo,
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
        imported_symbol: &KotlinImportedSymbolInfo,
    ) -> Option<ImportIdentifier> {
        if imported_symbol.identifier.is_some() {
            return Some(ImportIdentifier {
                name: imported_symbol.identifier.as_ref().unwrap().name.clone(),
                alias: imported_symbol.identifier.as_ref().unwrap().alias.clone(),
            });
        }

        None
    }
}
