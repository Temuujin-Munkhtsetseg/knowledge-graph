use database::graph::RelationshipType;
use parser_core::java::{
    ast::java_fqn_to_string,
    types::{
        JavaDefinitionInfo, JavaDefinitionType, JavaExpression, JavaImportType,
        JavaImportedSymbolInfo,
    },
};

use crate::{
    analysis::{
        languages::java::{
            java_file::{JavaClass, JavaFile},
            utils::full_import_path,
        },
        types::{DefinitionNode, DefinitionRelationship, DefinitionType},
    },
    parsing::processor::References,
};

use rustc_hash::FxHashMap;

// Resolved expression to a FQN. Can be a type, a method or a class.
#[derive(Debug, Clone)]
pub(crate) struct Resolution {
    pub name: String,
    pub fqn: String,
}

pub(crate) struct ExpressionResolver {
    /// Relative file path -> file
    files: FxHashMap<String, JavaFile>,
    /// FQN -> relative file path
    declaration_files: FxHashMap<String, String>,
    /// FQN -> definition_node
    definition_nodes: FxHashMap<String, DefinitionNode>,
    /// Package name -> (class name -> file_path). This index works because all top-level classes are declared in the same file.
    package_class_index: FxHashMap<String, FxHashMap<String, String>>,
}

impl Default for ExpressionResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpressionResolver {
    pub fn new() -> Self {
        Self {
            files: FxHashMap::default(),
            declaration_files: FxHashMap::default(),
            definition_nodes: FxHashMap::default(),
            package_class_index: FxHashMap::default(),
        }
    }

    pub fn resolve_references(
        &mut self,
        file_path: &str,
        references: &References,
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) {
        if let Some(java_iterator) = references.iter_java() {
            for reference in java_iterator {
                let range = (
                    reference.range.byte_offset.0 as u64,
                    reference.range.byte_offset.1 as u64,
                );
                let expression = reference.metadata.clone();

                let scope = reference.scope.clone();
                if scope.is_none() {
                    continue;
                }

                let from_definition = match self
                    .definition_nodes
                    .get(&java_fqn_to_string(&scope.unwrap()))
                {
                    Some(definition) => definition,
                    None => continue,
                };

                if let Some(expression) = expression {
                    let mut resolved_calls = Vec::new();
                    self.resolve_expression(file_path, range, &expression, &mut resolved_calls);

                    for resolved_call in resolved_calls {
                        let to_definition = self.definition_nodes.get(&resolved_call.fqn);

                        if let Some(to_definition) = to_definition {
                            definition_relationships.push(DefinitionRelationship {
                                from_file_path: file_path.to_string(),
                                to_file_path: to_definition.location.file_path.clone(),
                                from_definition_fqn: from_definition.fqn.clone(),
                                to_definition_fqn: to_definition.fqn.clone(),
                                from_location: from_definition.location.clone(),
                                to_location: to_definition.location.clone(),
                                relationship_type: RelationshipType::Calls,
                            });
                        }
                    }
                }
            }
        }
    }

    // Resolve an expression and returns the resolved type.
    pub fn resolve_expression(
        &self,
        file_path: &str,
        range: (u64, u64),
        expression: &JavaExpression,
        resolved_calls: &mut Vec<Resolution>,
    ) -> Option<Resolution> {
        match expression {
            JavaExpression::Identifier { name } => {
                self.resolve_identifier_expression(file_path, range, name)
            }
            JavaExpression::FieldAccess { target, member } => {
                let target = self.resolve_expression(file_path, range, target, resolved_calls);

                if let Some(target) = target {
                    return self.resolve_field_access(&target, member);
                }

                None
            }
            JavaExpression::MemberMethodCall { target, member } => {
                let target = self.resolve_expression(file_path, range, target, resolved_calls);

                if let Some(target) = target {
                    return self.resolve_method_call(&target, member, resolved_calls);
                }

                None
            }
            JavaExpression::MethodCall { name } => {
                self.resolve_class_method_call(file_path, range, name, resolved_calls)
            }
            JavaExpression::MethodReference { target, member } => {
                let target = self.resolve_expression(file_path, range, target, resolved_calls)?;
                self.resolve_method_call(&target, member, resolved_calls)
            }
            JavaExpression::Index { target } => {
                self.resolve_expression(file_path, range, target, resolved_calls)
            }
            JavaExpression::ObjectCreation { target } => {
                self.resolve_constructor_call(file_path, &target.name, resolved_calls)
            }
            JavaExpression::ArrayCreation { target } => {
                self.resolve_constructor_call(file_path, &target.name, resolved_calls)
            }
            JavaExpression::ArrayItem { target } => {
                self.resolve_expression(file_path, range, target, resolved_calls)
            }
            JavaExpression::This => self.resolve_this_reference(file_path, range),
            JavaExpression::Super => self.resolve_super_reference(file_path, range),
            JavaExpression::ArrayAccess { target } => {
                self.resolve_expression(file_path, range, target, resolved_calls)
            }
            JavaExpression::Annotation { name } => {
                if let Some(resolution) = self.resolve_type(file_path, name) {
                    resolved_calls.push(Resolution {
                        name: resolution.name.clone(),
                        fqn: resolution.fqn.clone(),
                    });
                    return Some(resolution);
                }

                None
            }
            JavaExpression::Literal => None,
        }
    }

    pub fn resolve_method_call(
        &self,
        target: &Resolution,
        member: &str,
        resolved_calls: &mut Vec<Resolution>,
    ) -> Option<Resolution> {
        let relative_path = self.declaration_files.get(target.fqn.as_str())?;
        let file = self.files.get(relative_path)?;

        let class = file.classes.get(target.name.as_str())?;
        self.resolve_method_in_class_hierarchy(class, member, file, resolved_calls)
    }

    fn resolve_method_in_class_hierarchy(
        &self,
        class: &JavaClass,
        member: &str,
        file: &JavaFile,
        resolved_calls: &mut Vec<Resolution>,
    ) -> Option<Resolution> {
        // Look for method in current class
        let method_fqn = format!("{}.{}", java_fqn_to_string(&class.fqn), member);
        if let Some(method) = file.methods.get(&method_fqn) {
            resolved_calls.push(Resolution {
                name: method.name.clone(),
                fqn: method_fqn,
            });

            if let Some(resolution) =
                self.resolve_type(file.file_path.as_str(), &method.return_type)
            {
                return Some(resolution);
            }
        }

        // Then check all super types recursively
        for super_type in class.super_types.iter() {
            if let Some(super_class) = self.resolve_type(file.file_path.as_str(), super_type) {
                let super_class_definition_file = match self.declaration_files.get(&super_class.fqn)
                {
                    Some(file_path) => self.files.get(file_path).unwrap(),
                    None => continue,
                };

                let super_class_definition_class =
                    match super_class_definition_file.classes.get(&super_class.name) {
                        Some(class) => class,
                        None => continue,
                    };

                if let Some(result) = self.resolve_method_in_class_hierarchy(
                    super_class_definition_class,
                    member,
                    super_class_definition_file,
                    resolved_calls,
                ) {
                    return Some(result);
                }
                continue;
            }
        }

        None
    }

    fn resolve_class_method_call(
        &self,
        file_path: &str,
        range: (u64, u64),
        name: &str,
        resolved_calls: &mut Vec<Resolution>,
    ) -> Option<Resolution> {
        // Find the enclosing class to look for the method
        let file = self.files.get(file_path)?;
        let class = file.get_class_at_offset(range.0)?;

        // Look for method in current class and its hierarchy
        self.resolve_method_in_class_hierarchy(&Box::new(class.clone()), name, file, resolved_calls)
    }

    fn resolve_this_reference(&self, file_path: &str, range: (u64, u64)) -> Option<Resolution> {
        let file = self.files.get(file_path)?;
        let class = file.get_class_at_offset(range.0)?;

        Some(Resolution {
            name: class.name.clone(),
            fqn: java_fqn_to_string(&class.fqn),
        })
    }

    fn resolve_super_reference(&self, file_path: &str, range: (u64, u64)) -> Option<Resolution> {
        let file = self.files.get(file_path)?;
        let class = file.get_class_at_offset(range.0)?;

        // Return the first super type (in Java, there's only one direct superclass)
        let super_type = class.super_types.iter().next()?;

        // Try to resolve the super type
        self.resolve_type(file_path, super_type)
    }

    pub fn resolve_field_access(&self, target: &Resolution, member: &str) -> Option<Resolution> {
        let relative_path = self.declaration_files.get(target.fqn.as_str())?;
        let file = self.files.get(relative_path)?;

        if let Some(class) = file.classes.get(member) {
            return Some(Resolution {
                name: class.name.clone(),
                fqn: java_fqn_to_string(&class.fqn),
            });
        }

        if let Some(constants) = file.enum_constants_by_enum.get(target.name.as_str())
            && constants.contains(member)
        {
            return Some(Resolution {
                name: target.name.clone(),
                fqn: target.fqn.clone(),
            });
        }

        let class = file.classes.get(target.name.as_str())?;
        self.resolve_field_in_class_hierarchy(class, member, file)
    }

    fn resolve_field_in_class_hierarchy(
        &self,
        class: &JavaClass,
        member: &str,
        file: &JavaFile,
    ) -> Option<Resolution> {
        // Check in current class first
        let scope = file.get_scope_by_fqn(&class.fqn)?;
        if let Some(binding) = scope.definition_map.unique_definitions.get(member) {
            // A field is always typed in Java
            if let Some(binding_type) = &binding.java_type {
                return self.resolve_type(file.file_path.as_str(), binding_type);
            }
        }

        // Then check all super types recursively
        for super_type in class.super_types.iter() {
            // First check if super type is in the same file
            if let Some(super_class) = self.resolve_type(file.file_path.as_str(), super_type) {
                let super_class_definition_file = match self.declaration_files.get(&super_class.fqn)
                {
                    Some(file_path) => self.files.get(file_path).unwrap(),
                    None => continue,
                };

                let super_class_definition_class =
                    match super_class_definition_file.classes.get(&super_class.name) {
                        Some(class) => class,
                        None => continue,
                    };

                if let Some(result) = self.resolve_field_in_class_hierarchy(
                    super_class_definition_class,
                    member,
                    super_class_definition_file,
                ) {
                    return Some(result);
                }
                continue;
            }
        }

        None
    }

    pub fn resolve_identifier_expression(
        &self,
        file_path: &str,
        range: (u64, u64),
        name: &str,
    ) -> Option<Resolution> {
        let file = self.files.get(file_path).unwrap();

        // Quickly look up if the identifier is a class name the imported symbols
        if let Some(import_path) = file.imported_symbols.get(name) {
            if let Some(imported_file_path) = self.declaration_files.get(import_path) {
                // If the imported symbol is a class, resolve to the class.
                let imported_file = self.files.get(imported_file_path).unwrap();
                if let Some(class) = imported_file.classes.get(name) {
                    return Some(Resolution {
                        name: class.name.clone(),
                        fqn: java_fqn_to_string(&class.fqn),
                    });
                }

                // If the imported symbol is an enum constant, resolve to its parent enum type.
                if let Some(def_node) = self.definition_nodes.get(import_path)
                    && matches!(
                        def_node.definition_type,
                        DefinitionType::Java(JavaDefinitionType::EnumConstant)
                    )
                {
                    let parent_fqn = import_path
                        .rsplit_once('.')
                        .map(|(left, _)| left)
                        .unwrap_or(import_path);

                    let parent_name = parent_fqn.rsplit('.').next().unwrap_or(parent_fqn);

                    return Some(Resolution {
                        name: parent_name.to_string(),
                        fqn: parent_fqn.to_string(),
                    });
                }
            } else {
                return None; // This means the import is not in the indexed code. We can't resolve it.
            }
        }

        // Quickly look up the file wildward imports
        for import_path in file.wildcard_imports.iter() {
            if let Some(imported_file_path) = self
                .package_class_index
                .get(import_path)
                .and_then(|map| map.get(name))
                && let Some(imported_file) = self.files.get(imported_file_path)
                && let Some(class) = imported_file.classes.get(name)
            {
                return Some(Resolution {
                    name: class.name.clone(),
                    fqn: java_fqn_to_string(&class.fqn),
                });
            }
        }

        // Quickly check the class index to validate if the identifier is a class name
        if let Some(class_file_path) = self
            .package_class_index
            .get(&file.package_name)
            .and_then(|map| map.get(name))
            && let Some(class_file) = self.files.get(class_file_path)
            && let Some(class) = class_file.classes.get(name)
        {
            return Some(Resolution {
                name: class.name.clone(),
                fqn: java_fqn_to_string(&class.fqn),
            });
        }

        self.resolve_identifier_type(file_path, range, name)
    }

    pub fn resolve_identifier_type(
        &self,
        file_path: &str,
        range: (u64, u64),
        name: &str,
    ) -> Option<Resolution> {
        let file = self.files.get(file_path).unwrap();
        let file_scope = file.get_scope_at_offset(range.0);

        // Look up through the scope hierarchy to find the correct binding
        let mut current_scope = file_scope;
        while let Some(scope) = current_scope {
            // Check unique definitions first
            if let Some(binding) = scope.definition_map.unique_definitions.get(name) {
                // Resolve binding type
                if let Some(binding_type) = &binding.java_type {
                    return self.resolve_type(file_path, binding_type);
                } else if let Some(init) = &binding.init {
                    return self.resolve_expression(file_path, range, init, &mut Vec::new());
                }
            }

            // Then check duplicated definitions
            if let Some(bindings) = scope.definition_map.duplicated_definitions.get(name) {
                for binding in bindings {
                    if binding.range.0 <= range.0 && binding.range.1 >= range.1 {
                        if let Some(binding_type) = &binding.java_type {
                            return self.resolve_type(file_path, binding_type);
                        } else if let Some(init) = &binding.init {
                            return self.resolve_expression(
                                file_path,
                                range,
                                init,
                                &mut Vec::new(),
                            );
                        }
                    }
                }
            }

            // Move up to parent scope
            if let Some(parent_scope_name) = file.scope_hierarchy.get(&scope.fqn) {
                current_scope = file.scopes.get(parent_scope_name);
            } else {
                current_scope = None;
            }
        }

        None
    }

    pub fn resolve_constructor_call(
        &self,
        file_path: &str,
        type_name: &str,
        resolved_calls: &mut Vec<Resolution>,
    ) -> Option<Resolution> {
        if let Some(java_type) = self.resolve_type(file_path, type_name) {
            let file = self.files.get(file_path)?;

            let constructor_resolution = Resolution {
                name: java_type.name.clone(),
                fqn: format!("{}.{}", java_type.fqn, java_type.name),
            };

            let class_resolution = Resolution {
                name: java_type.name,
                fqn: java_type.fqn,
            };

            if file.methods.contains_key(&constructor_resolution.fqn) {
                resolved_calls.push(constructor_resolution.clone());
            } else {
                resolved_calls.push(class_resolution.clone());
            }

            return Some(class_resolution);
        }

        None
    }

    pub fn resolve_type(&self, file_path: &str, type_name: &str) -> Option<Resolution> {
        // if type name first letter is a lowercase, it's a FQN.
        if let Some(first_letter) = type_name.chars().next()
            && first_letter.is_lowercase()
        {
            return self.resolve_fully_qualified_name(type_name);
        }

        // if type name first letter is a uppercase, it's a class name
        self.resolve_class_name(file_path, type_name)
    }

    // ex: java.util.List
    fn resolve_fully_qualified_name(&self, type_name: &str) -> Option<Resolution> {
        if let Some(definition) = self.definition_nodes.get(type_name) {
            return Some(Resolution {
                name: definition.name.clone(),
                fqn: definition.fqn.clone(),
            });
        }

        None
    }

    // ex: Map, Map.Entry, Map.Entry.Key
    fn resolve_class_name(&self, file_path: &str, type_name: &str) -> Option<Resolution> {
        // All sub classes are declared in the same file. We need to find the imported symbol that contains any part of the class name, than lookup the class in the file.
        let parts = type_name.split('.').collect::<Vec<&str>>();
        let file = self.files.get(file_path)?;

        // Let's find the file in which the class is declared
        let mut parent_symbol_file = None;
        if let Some(parent_symbol) = parts.clone().first() {
            if file.classes.contains_key(*parent_symbol) {
                parent_symbol_file = Some(file);
            }

            // Look at the imported symbols
            if let Some(import_path) = file.imported_symbols.get(*parent_symbol)
                && let Some(file_path) = self.declaration_files.get(import_path)
                && let Some(file) = self.files.get(file_path)
            {
                parent_symbol_file = Some(file);
            }

            // Look at the wildward imports
            for import_path in file.wildcard_imports.iter() {
                if let Some(file_path) = self
                    .package_class_index
                    .get(import_path)
                    .and_then(|map| map.get(parent_symbol.to_string().as_str()))
                {
                    if let Some(file) = self.files.get(file_path) {
                        parent_symbol_file = Some(file);
                    }
                    break;
                }
            }

            // Look at all the files in the same package
            if let Some(file_path) = self
                .package_class_index
                .get(&file.package_name)
                .and_then(|map| map.get(parent_symbol.to_string().as_str()))
                && let Some(file) = self.files.get(file_path)
            {
                parent_symbol_file = Some(file);
            }
        }

        if let Some(parent_symbol_file) = parent_symbol_file
            && let Some(class) = parent_symbol_file
                .classes
                .get(parts.last()?.to_string().as_str())
        {
            return Some(Resolution {
                name: class.name.clone(),
                fqn: java_fqn_to_string(&class.fqn),
            });
        }

        None
    }

    pub fn add_file(&mut self, package_name: String, file_path: String) {
        if !self.files.contains_key(&file_path) {
            self.files.insert(
                file_path.clone(),
                JavaFile::new(package_name.clone(), file_path.clone()),
            );

            if !self.package_class_index.contains_key(&package_name) {
                self.package_class_index
                    .insert(package_name.clone(), FxHashMap::default());
            }
        } else {
            self.files.get_mut(&file_path).unwrap().package_name = package_name.clone();

            // Backfill the package class index.
            let file = self.files.get_mut(&file_path).unwrap();
            for class in file.classes.values() {
                self.package_class_index
                    .entry(package_name.clone())
                    .or_default()
                    .insert(class.name.clone(), file_path.clone());
            }
        }
    }

    pub fn add_definition(
        &mut self,
        file_path: String,
        definition: JavaDefinitionInfo,
        definition_node: DefinitionNode,
    ) {
        if !self.files.contains_key(&file_path) {
            self.files.insert(
                file_path.clone(),
                JavaFile::new_in_unknown_package(file_path.clone()),
            );
        }

        let fqn = java_fqn_to_string(&definition.fqn);
        self.declaration_files
            .insert(fqn.clone(), file_path.clone());
        match definition.definition_type {
            JavaDefinitionType::Class
            | JavaDefinitionType::Interface
            | JavaDefinitionType::Enum
            | JavaDefinitionType::Record
            | JavaDefinitionType::Annotation => {
                self.definition_nodes.insert(fqn.clone(), definition_node);

                let file = self.files.get(&file_path).unwrap();
                if !file.package_name.is_empty() {
                    self.package_class_index
                        .entry(file.package_name.clone())
                        .or_default()
                        .insert(definition.name.clone(), file_path.clone());
                }
            }
            JavaDefinitionType::EnumConstant
            | JavaDefinitionType::AnnotationDeclaration
            | JavaDefinitionType::Method
            | JavaDefinitionType::Constructor => {
                self.definition_nodes.insert(fqn, definition_node);
            }
            _ => {}
        }

        self.files
            .get_mut(&file_path)
            .unwrap()
            .index_definition(&definition);
    }

    pub fn add_import(&mut self, file_path: String, imported_symbol: &JavaImportedSymbolInfo) {
        if !self.files.contains_key(&file_path) {
            self.files.insert(
                file_path.clone(),
                JavaFile::new_in_unknown_package(file_path.clone()),
            );
        }

        let file = self.files.get_mut(&file_path).unwrap();

        if matches!(imported_symbol.import_type, JavaImportType::WildcardImport) {
            file.wildcard_imports
                .insert(imported_symbol.import_path.clone());
        } else {
            let (name, import_path) = full_import_path(imported_symbol);
            file.imported_symbols.insert(name, import_path);
        }
    }
}
