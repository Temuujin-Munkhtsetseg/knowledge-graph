use database::graph::RelationshipType;
use parser_core::java::{
    ast::java_fqn_to_string,
    types::{JavaDefinitionInfo, JavaDefinitionType, JavaExpression, JavaImportType},
};

use crate::{
    analysis::{
        languages::java::{
            java_file::{JavaClass, JavaFile},
            utils::full_import_path,
        },
        types::{
            DefinitionImportedSymbolRelationship, DefinitionNode, DefinitionRelationship,
            DefinitionType, ImportType, ImportedSymbolNode, SourceLocation,
        },
    },
    parsing::processor::References,
};

use rustc_hash::FxHashMap;

#[derive(Default)]
pub(crate) struct Resolutions {
    definition_resolutions: Vec<DefinitionResolution>,
    import_resolutions: Vec<ImportedSymbolNode>,
}

pub(crate) enum ResolvedType {
    Definition(DefinitionResolution),
    Import(ImportedSymbolNode),
}

// Resolved expression to a FQN. Can be a type, a method or a class.
#[derive(Debug, Clone)]
pub(crate) struct DefinitionResolution {
    pub name: String,
    pub fqn: String,
}

pub(crate) struct ExpressionResolver {
    /// Relative file path -> file
    files: FxHashMap<String, JavaFile>,
    /// FQN -> definition_node
    definition_nodes: FxHashMap<String, DefinitionNode>,
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
            definition_nodes: FxHashMap::default(),
        }
    }

    pub fn resolve_references(
        &mut self,
        file_path: &str,
        references: &References,
        definition_relationships: &mut Vec<DefinitionRelationship>,
        definition_imported_symbol_relationships: &mut Vec<DefinitionImportedSymbolRelationship>,
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
                    let mut resolutions = Resolutions::default();
                    self.resolve_expression(file_path, range, &expression, &mut resolutions);

                    for resolved_definition in resolutions.definition_resolutions {
                        let to_definition = self.definition_nodes.get(&resolved_definition.fqn);

                        if let Some(to_definition) = to_definition {
                            definition_relationships.push(DefinitionRelationship {
                                from_file_path: file_path.to_string(),
                                to_file_path: to_definition.location.file_path.clone(),
                                from_definition_fqn: from_definition.fqn.clone(),
                                to_definition_fqn: to_definition.fqn.clone(),
                                from_location: from_definition.location.clone(),
                                to_location: to_definition.location.clone(),
                                relationship_type: RelationshipType::Calls,
                                source_location: Some(SourceLocation {
                                    file_path: file_path.to_string(),
                                    start_byte: reference.range.byte_offset.0 as i64,
                                    end_byte: reference.range.byte_offset.1 as i64,
                                    start_line: reference.range.start.line as i32,
                                    end_line: reference.range.end.line as i32,
                                    start_col: reference.range.start.column as i32,
                                    end_col: reference.range.end.column as i32,
                                }),
                            });
                        }
                    }

                    for resolved_import in resolutions.import_resolutions {
                        definition_imported_symbol_relationships.push(
                            DefinitionImportedSymbolRelationship {
                                file_path: file_path.to_string(),
                                definition_fqn: from_definition.fqn.clone(),
                                imported_symbol_location: resolved_import.location.clone(),
                                relationship_type: RelationshipType::Calls,
                                definition_location: from_definition.location.clone(),
                                source_location: Some(SourceLocation {
                                    file_path: file_path.to_string(),
                                    start_byte: reference.range.byte_offset.0 as i64,
                                    end_byte: reference.range.byte_offset.1 as i64,
                                    start_line: reference.range.start.line as i32,
                                    end_line: reference.range.end.line as i32,
                                    start_col: reference.range.start.column as i32,
                                    end_col: reference.range.end.column as i32,
                                }),
                            },
                        );
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
        resolutions: &mut Resolutions,
    ) -> Option<ResolvedType> {
        match expression {
            JavaExpression::Identifier { name } => {
                self.resolve_identifier_expression(file_path, range, name)
            }
            JavaExpression::FieldAccess { target, member } => {
                let target = self.resolve_expression(file_path, range, target, resolutions);

                if let Some(ResolvedType::Definition(target)) = target {
                    return self.resolve_field_access(&target, member);
                } else if let Some(ResolvedType::Import(import)) = target {
                    resolutions.import_resolutions.push(import);
                }

                None
            }
            JavaExpression::MemberMethodCall { target, member } => {
                let target = self.resolve_expression(file_path, range, target, resolutions);

                if let Some(ResolvedType::Definition(target)) = target {
                    return self.resolve_method_call(&target, member, resolutions);
                } else if let Some(ResolvedType::Import(import)) = target {
                    resolutions.import_resolutions.push(import);
                }

                None
            }
            JavaExpression::MethodCall { name } => {
                self.resolve_class_method_call(file_path, range, name, resolutions)
            }
            JavaExpression::MethodReference { target, member } => {
                let target = self.resolve_expression(file_path, range, target, resolutions);

                if let Some(ResolvedType::Definition(target)) = target {
                    return self.resolve_method_call(&target, member, resolutions);
                } else if let Some(ResolvedType::Import(import)) = target {
                    resolutions.import_resolutions.push(import);
                }

                None
            }
            JavaExpression::Index { target } => {
                self.resolve_expression(file_path, range, target, resolutions)
            }
            JavaExpression::ObjectCreation { target } => {
                self.resolve_constructor_call(file_path, &target.name, resolutions)
            }
            JavaExpression::ArrayCreation { target } => {
                self.resolve_constructor_call(file_path, &target.name, resolutions)
            }
            JavaExpression::ArrayItem { target } => {
                self.resolve_expression(file_path, range, target, resolutions)
            }
            JavaExpression::This => self.resolve_this_reference(file_path, range),
            JavaExpression::Super => self.resolve_super_reference(file_path, range),
            JavaExpression::ArrayAccess { target } => {
                self.resolve_expression(file_path, range, target, resolutions)
            }
            JavaExpression::Annotation { name } => {
                if let Some(resolution) = self.resolve_type(file_path, None, name) {
                    match resolution {
                        ResolvedType::Definition(definition) => {
                            resolutions.definition_resolutions.push(definition.clone());
                            return Some(ResolvedType::Definition(definition));
                        }
                        ResolvedType::Import(import) => {
                            resolutions.import_resolutions.push(import);
                            return None;
                        }
                    }
                }

                None
            }
            JavaExpression::Literal => None,
        }
    }

    pub fn resolve_method_call(
        &self,
        target: &DefinitionResolution,
        member: &str,
        resolutions: &mut Resolutions,
    ) -> Option<ResolvedType> {
        let relative_path = self.definition_nodes.get(target.fqn.as_str())?.file_path();
        let file = self.files.get(relative_path)?;

        let class = file.classes.get(target.fqn.as_str())?;
        self.resolve_method_in_class_hierarchy(class, member, file, resolutions)
    }

    fn resolve_method_in_class_hierarchy(
        &self,
        class: &JavaClass,
        member: &str,
        file: &JavaFile,
        resolutions: &mut Resolutions,
    ) -> Option<ResolvedType> {
        // Look for method in current class
        let method_fqn = format!("{}.{}", class.fqn, member);
        if let Some(method) = file.methods.get(&method_fqn) {
            resolutions
                .definition_resolutions
                .push(DefinitionResolution {
                    name: method.name.clone(),
                    fqn: method_fqn,
                });

            if let Some(resolution) = self.resolve_type(
                file.file_path.as_str(),
                Some(&class.fqn),
                &method.return_type,
            ) {
                return Some(resolution);
            }
        }

        // Then check all super types recursively
        for super_type in class.super_types.iter() {
            if let Some(ResolvedType::Definition(super_class)) =
                self.resolve_type(file.file_path.as_str(), Some(&class.fqn), super_type)
            {
                let super_class_definition_file = match self.definition_nodes.get(&super_class.fqn)
                {
                    Some(definition) => self.files.get(definition.file_path()).unwrap(),
                    None => continue,
                };

                let super_class_definition_class =
                    match super_class_definition_file.classes.get(&super_class.fqn) {
                        Some(class) => class,
                        None => continue,
                    };

                if let Some(result) = self.resolve_method_in_class_hierarchy(
                    super_class_definition_class,
                    member,
                    super_class_definition_file,
                    resolutions,
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
        resolutions: &mut Resolutions,
    ) -> Option<ResolvedType> {
        // Find the enclosing class to look for the method
        let file = self.files.get(file_path)?;
        let class = file.get_class_at_offset(range.0)?;

        // Look for method in current class and its hierarchy
        self.resolve_method_in_class_hierarchy(class, name, file, resolutions)
    }

    fn resolve_this_reference(&self, file_path: &str, range: (u64, u64)) -> Option<ResolvedType> {
        let file = self.files.get(file_path)?;
        let class = file.get_class_at_offset(range.0)?;

        Some(ResolvedType::Definition(DefinitionResolution {
            name: class.name.clone(),
            fqn: class.fqn.clone(),
        }))
    }

    fn resolve_super_reference(&self, file_path: &str, range: (u64, u64)) -> Option<ResolvedType> {
        let file = self.files.get(file_path)?;
        let class = file.get_class_at_offset(range.0)?;

        // Return the first super type (in Java, there's only one direct superclass)
        let super_type = class.super_types.iter().next()?;

        // Try to resolve the super type
        if let Some(ResolvedType::Definition(super_class)) =
            self.resolve_type(file_path, Some(&class.fqn), super_type)
        {
            return Some(ResolvedType::Definition(super_class));
        }

        None
    }

    pub fn resolve_field_access(
        &self,
        target: &DefinitionResolution,
        member: &str,
    ) -> Option<ResolvedType> {
        let relative_path = self.definition_nodes.get(target.fqn.as_str())?.file_path();
        let file = self.files.get(relative_path)?;

        let potential_class_fqn = format!("{}.{}", target.fqn, member);
        if let Some(class) = file.classes.get(&potential_class_fqn) {
            return Some(ResolvedType::Definition(DefinitionResolution {
                name: class.name.clone(),
                fqn: class.fqn.clone(),
            }));
        }

        if let Some(constants) = file.enum_constants_by_enum.get(&target.name)
            && constants.contains(member)
        {
            return Some(ResolvedType::Definition(DefinitionResolution {
                name: target.name.clone(),
                fqn: target.fqn.clone(),
            }));
        }

        let class = file.classes.get(&target.fqn)?;
        self.resolve_field_in_class_hierarchy(class, member, file)
    }

    fn resolve_field_in_class_hierarchy(
        &self,
        class: &JavaClass,
        member: &str,
        file: &JavaFile,
    ) -> Option<ResolvedType> {
        // Check in current class first
        let scope = file.scopes.get(&class.fqn)?;
        if let Some(binding) = scope.definition_map.unique_definitions.get(member) {
            // A field is always typed in Java
            if let Some(binding_type) = &binding.java_type
                && let Some(resolved_type) =
                    self.resolve_type(file.file_path.as_str(), Some(&class.fqn), binding_type)
            {
                return Some(resolved_type);
            }
        }

        // Then check all super types recursively
        for super_type in class.super_types.iter() {
            // First check if super type is in the same file
            if let Some(ResolvedType::Definition(super_class)) =
                self.resolve_type(file.file_path.as_str(), Some(&class.fqn), super_type)
            {
                let super_class_definition_file = match self.definition_nodes.get(&super_class.fqn)
                {
                    Some(definition) => self.files.get(definition.file_path()).unwrap(),
                    None => continue,
                };

                let super_class_definition_class =
                    match super_class_definition_file.classes.get(&super_class.fqn) {
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
    ) -> Option<ResolvedType> {
        let file = self.files.get(file_path).unwrap();

        // Look up if the identifier is a class name the imported symbols
        if let Some(import_path) = file.imported_symbols.get(name) {
            if let Some(imported_definition) = self.definition_nodes.get(import_path) {
                // If the imported symbol is a class, resolve to the class.
                let imported_file = self.files.get(imported_definition.file_path()).unwrap();
                if let Some(class) = imported_file.classes.get(&imported_definition.fqn) {
                    return Some(ResolvedType::Definition(DefinitionResolution {
                        name: class.name.clone(),
                        fqn: class.fqn.clone(),
                    }));
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

                    return Some(ResolvedType::Definition(DefinitionResolution {
                        name: parent_name.to_string(),
                        fqn: parent_fqn.to_string(),
                    }));
                }
            } else {
                if let Some(imported_symbol_node) = file.import_nodes.get(import_path) {
                    return Some(ResolvedType::Import(imported_symbol_node.clone()));
                }

                return None;
            }
        }

        // Look up the file wildward imports
        for import_path in file.wildcard_imports.iter() {
            let potential_fqn = format!("{}.{}", import_path, name);
            if let Some(imported_file_path) = self.definition_nodes.get(&potential_fqn)
                && let Some(imported_file) = self.files.get(imported_file_path.file_path())
                && let Some(class) = imported_file.classes.get(&imported_file_path.fqn)
            {
                return Some(ResolvedType::Definition(DefinitionResolution {
                    name: class.name.clone(),
                    fqn: class.fqn.clone(),
                }));
            }
        }

        // Quickly check the class index to validate if the identifier is a class name
        let potential_fqn = format!("{}.{}", file.package_name, name);
        if let Some(class_file_path) = self.definition_nodes.get(&potential_fqn)
            && let Some(class_file) = self.files.get(class_file_path.file_path())
            && let Some(class) = class_file.classes.get(&class_file_path.fqn)
        {
            return Some(ResolvedType::Definition(DefinitionResolution {
                name: class.name.clone(),
                fqn: class.fqn.clone(),
            }));
        }

        self.resolve_identifier_type(file_path, range, name)
    }

    pub fn resolve_identifier_type(
        &self,
        file_path: &str,
        range: (u64, u64),
        name: &str,
    ) -> Option<ResolvedType> {
        let file = self.files.get(file_path).unwrap();
        let file_scope = file.get_scope_at_offset(range.0);

        // Look up through the scope hierarchy to find the correct binding
        let mut current_scope = file_scope;
        while let Some(scope) = current_scope {
            // Check unique definitions first
            if let Some(binding) = scope.definition_map.unique_definitions.get(name) {
                // Resolve binding type
                if let Some(binding_type) = &binding.java_type {
                    if let Some(resolved_type) =
                        self.resolve_type(file_path, Some(&scope.fqn), binding_type)
                    {
                        return Some(resolved_type);
                    }
                } else if let Some(init) = &binding.init {
                    return self.resolve_expression(
                        file_path,
                        range,
                        init,
                        &mut Resolutions::default(),
                    );
                }
            }

            // Then check duplicated definitions
            if let Some(bindings) = scope.definition_map.duplicated_definitions.get(name) {
                for binding in bindings {
                    if binding.range.0 <= range.0 && binding.range.1 >= range.1 {
                        if let Some(binding_type) = &binding.java_type {
                            if let Some(resolved_type) =
                                self.resolve_type(file_path, Some(&scope.fqn), binding_type)
                            {
                                return Some(resolved_type);
                            }
                        } else if let Some(init) = &binding.init {
                            return self.resolve_expression(
                                file_path,
                                range,
                                init,
                                &mut Resolutions::default(),
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
        resolutions: &mut Resolutions,
    ) -> Option<ResolvedType> {
        match self.resolve_type(file_path, None, type_name) {
            Some(ResolvedType::Definition(java_type)) => {
                let file = self.files.get(file_path);

                let name = java_type.name.clone();
                let fqn = java_type.fqn.clone();

                let constructor_resolution = DefinitionResolution {
                    name: name.clone(),
                    fqn: format!("{}.{}", fqn, name),
                };

                let class_resolution = DefinitionResolution { name, fqn };

                if file.is_some()
                    && file
                        .unwrap()
                        .methods
                        .contains_key(&constructor_resolution.fqn)
                {
                    resolutions
                        .definition_resolutions
                        .push(constructor_resolution.clone());
                } else {
                    resolutions
                        .definition_resolutions
                        .push(class_resolution.clone());
                }

                Some(ResolvedType::Definition(java_type))
            }
            Some(ResolvedType::Import(import)) => {
                resolutions.import_resolutions.push(import);
                None
            }
            None => None,
        }
    }

    pub fn resolve_type(
        &self,
        file_path: &str,
        class_fqn: Option<&str>,
        type_name: &str,
    ) -> Option<ResolvedType> {
        // if type name first letter is a lowercase, it's a FQN.
        if let Some(first_letter) = type_name.chars().next()
            && first_letter.is_lowercase()
        {
            return self.resolve_fully_qualified_name(type_name);
        }

        // Attempt to resolve the type in the class hierarchy
        if let Some(class_fqn) = class_fqn {
            let file = self.files.get(file_path)?;

            let parts = type_name.split('.').collect::<Vec<&str>>();
            if let Some(part) = parts.first()
                && let Some(parent_scope) = file.scope_hierarchy.get(class_fqn)
            {
                let mut current_scope = file.scopes.get(parent_scope);
                while let Some(scope) = current_scope {
                    let potential_fqn = format!("{}.{}", scope.fqn, part);
                    if let Some(class) = file.classes.get(&potential_fqn) {
                        return Some(ResolvedType::Definition(DefinitionResolution {
                            name: class.name.clone(),
                            fqn: class.fqn.clone(),
                        }));
                    }

                    if let Some(parent_scope_fqn) = file.scope_hierarchy.get(&scope.fqn) {
                        current_scope = file.scopes.get(parent_scope_fqn);
                    } else {
                        current_scope = None;
                    }
                }
            }
        }

        // if type name first letter is a uppercase, it's a class name
        self.resolve_class_name(file_path, type_name)
    }

    // ex: java.util.List
    fn resolve_fully_qualified_name(&self, type_name: &str) -> Option<ResolvedType> {
        if let Some(definition) = self.definition_nodes.get(type_name) {
            return Some(ResolvedType::Definition(DefinitionResolution {
                name: definition.name.clone(),
                fqn: definition.fqn.clone(),
            }));
        }

        None
    }

    // ex: Map, Map.Entry, Map.Entry.Key
    fn resolve_class_name(&self, file_path: &str, type_name: &str) -> Option<ResolvedType> {
        // All sub classes are declared in the same file. We need to find the imported symbol that contains any part of the class name, than lookup the class in the file.
        let parts = type_name.split('.').collect::<Vec<&str>>();
        let file = self.files.get(file_path)?;

        // Let's find the file in which the class is declared
        let mut parent_symbol_file = None;
        if let Some(parent_symbol) = parts.clone().first() {
            let potential_fqn = format!("{}.{}", file.package_name, parent_symbol);
            if file.classes.contains_key(&potential_fqn) {
                parent_symbol_file = Some(file);
            }

            // Look at the imported symbols
            if let Some(import_path) = file.imported_symbols.get(*parent_symbol) {
                if let Some(imported_definition) = self.definition_nodes.get(import_path)
                    && let Some(file) = self.files.get(imported_definition.file_path())
                {
                    parent_symbol_file = Some(file);
                } else {
                    if let Some(imported_symbol_node) = file.import_nodes.get(import_path) {
                        return Some(ResolvedType::Import(imported_symbol_node.clone()));
                    }

                    return None;
                }
            }

            // Look at the wildward imports
            for import_path in file.wildcard_imports.iter() {
                if parent_symbol_file.is_some() {
                    break;
                }

                let potential_fqn = format!("{}.{}", import_path, parent_symbol);
                if let Some(definition) = self.definition_nodes.get(&potential_fqn) {
                    if let Some(file) = self.files.get(definition.file_path()) {
                        parent_symbol_file = Some(file);
                    }
                    break;
                }
            }

            // Look at all the files in the same package
            let potential_fqn = format!("{}.{}", file.package_name, parent_symbol);
            if let Some(file_path) = self.definition_nodes.get(&potential_fqn)
                && let Some(file) = self.files.get(file_path.file_path())
                && parent_symbol_file.is_none()
            {
                parent_symbol_file = Some(file);
            }
        }

        if let Some(parent_symbol_file) = parent_symbol_file {
            let potential_fqn = format!("{}.{}", parent_symbol_file.package_name, type_name);
            if let Some(class) = parent_symbol_file.classes.get(&potential_fqn) {
                return Some(ResolvedType::Definition(DefinitionResolution {
                    name: class.name.clone(),
                    fqn: class.fqn.clone(),
                }));
            }
        }

        None
    }

    pub fn add_file(&mut self, package_name: String, file_path: String) {
        if !self.files.contains_key(&file_path) {
            self.files.insert(
                file_path.clone(),
                JavaFile::new(package_name.clone(), file_path.clone()),
            );
        } else {
            self.files.get_mut(&file_path).unwrap().package_name = package_name.clone();
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
        match definition.definition_type {
            JavaDefinitionType::Class
            | JavaDefinitionType::Interface
            | JavaDefinitionType::Enum
            | JavaDefinitionType::Record
            | JavaDefinitionType::Annotation
            | JavaDefinitionType::EnumConstant
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

    pub fn add_import(&mut self, file_path: String, imported_symbol: &ImportedSymbolNode) {
        if !self.files.contains_key(&file_path) {
            self.files.insert(
                file_path.clone(),
                JavaFile::new_in_unknown_package(file_path.clone()),
            );
        }

        let file = self.files.get_mut(&file_path).unwrap();

        if matches!(
            imported_symbol.import_type,
            ImportType::Java(JavaImportType::WildcardImport)
        ) {
            file.wildcard_imports
                .insert(imported_symbol.import_path.clone());
            file.import_nodes
                .insert(imported_symbol.import_path.clone(), imported_symbol.clone());
        } else {
            let (name, import_path) = full_import_path(imported_symbol);
            file.imported_symbols.insert(name, import_path.clone());
            file.import_nodes
                .insert(import_path, imported_symbol.clone());
        }
    }
}
