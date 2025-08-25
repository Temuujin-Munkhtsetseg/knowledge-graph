use parser_core::java::types::{JavaImportType, JavaImportedSymbolInfo};

/// Returns the name of the imported symbol and the full import path.
pub fn full_import_path(import: &JavaImportedSymbolInfo) -> (String, String) {
    let name = match import.import_type {
        JavaImportType::Import => import
            .identifier
            .as_ref()
            .map(|i| i.name.clone())
            .unwrap_or_default(),
        JavaImportType::StaticImport => import
            .identifier
            .as_ref()
            .map(|i| i.name.clone())
            .unwrap_or_default(),
        _ => return (String::new(), String::new()),
    };

    (name.clone(), format!("{}.{}", import.import_path, name))
}
