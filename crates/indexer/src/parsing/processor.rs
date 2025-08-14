use crate::project::file_info::FileInfo;
use parser_core::{
    csharp::{
        analyzer::CSharpAnalyzer,
        types::{CSharpDefinitionInfo, CSharpImportedSymbolInfo},
    },
    definitions::DefinitionTypeInfo,
    java::{
        analyzer::JavaAnalyzer,
        types::{JavaDefinitionInfo, JavaImportedSymbolInfo},
    },
    kotlin::{
        analyzer::KotlinAnalyzer,
        types::{KotlinDefinitionInfo, KotlinImportedSymbolInfo},
    },
    parser::{
        GenericParser, LanguageParser, ParseResult, SupportedLanguage,
        detect_language_from_extension,
    },
    python::{
        analyzer::PythonAnalyzer,
        types::{PythonDefinitionInfo, PythonImportedSymbolInfo},
    },
    ruby::{analyzer::RubyAnalyzer, definitions::RubyDefinitionInfo},
    rules::{MatchWithNodes, RuleManager, run_rules},
    rust::{analyzer::RustAnalyzer, imports::RustImportedSymbolInfo, types::RustDefinitionInfo},
    typescript::{
        analyzer::TypeScriptAnalyzer,
        types::{TypeScriptDefinitionInfo, TypeScriptImportedSymbolInfo},
    },
};
use std::time::{Duration, Instant};

/// Represents a file that was skipped during processing
#[derive(Debug, Clone)]
pub struct SkippedFile {
    pub file_path: String,
    pub reason: String,
    pub file_size: Option<u64>,
}

/// Represents a file that encountered an error during processing
#[derive(Debug, Clone)]
pub struct ErroredFile {
    pub file_path: String,
    pub error_message: String,
    pub error_stage: ProcessingStage,
}

/// Represents the stage where processing failed
#[derive(Debug, Clone)]
pub enum ProcessingStage {
    FileSystem, // Failed to read file metadata or content
    Parsing,    // Failed during parsing/analysis
    Unknown,    // Unknown stage
}

/// Result of processing a file that can be success, skipped, or error
#[derive(Debug)]
pub enum ProcessingResult {
    Success(FileProcessingResult),
    Skipped(SkippedFile),
    Error(ErroredFile),
}

impl ProcessingResult {
    /// Check if the result is a success
    pub fn is_success(&self) -> bool {
        matches!(self, ProcessingResult::Success(_))
    }

    /// Check if the result is skipped
    pub fn is_skipped(&self) -> bool {
        matches!(self, ProcessingResult::Skipped(_))
    }

    /// Check if the result is an error
    pub fn is_error(&self) -> bool {
        matches!(self, ProcessingResult::Error(_))
    }

    /// Get the file path regardless of result type
    pub fn file_path(&self) -> &str {
        match self {
            ProcessingResult::Success(result) => &result.file_path,
            ProcessingResult::Skipped(skipped) => &skipped.file_path,
            ProcessingResult::Error(errored) => &errored.file_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileProcessor<'a> {
    pub path: String,
    pub content: &'a str,
    /// Pre-computed file extension to avoid duplicate parsing
    pub extension: String,
}

impl<'a> FileProcessor<'a> {
    /// Create a new File with the given path and content
    pub fn new(path: String, content: &'a str) -> Self {
        let extension = std::path::Path::new(&path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self {
            path,
            content,
            extension,
        }
    }

    /// Create a new File from FileInfo with pre-computed metadata
    pub fn from_file_info(file_info: FileInfo, content: &'a str) -> Self {
        Self {
            path: file_info.path.to_string_lossy().to_string(),
            content,
            extension: file_info
                .path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        }
    }

    /// Create a new File with empty content (for lazy loading)
    pub fn new_empty(path: String) -> Self {
        let extension = std::path::Path::new(&path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self {
            path,
            content: "",
            extension,
        }
    }

    /// Get the file path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the file content
    pub fn content(&self) -> &str {
        self.content
    }

    /// Get the file extension
    pub fn extension(&self) -> &str {
        &self.extension
    }

    /// Set the file content
    pub fn set_content(&mut self, content: &'a str) {
        self.content = content;
    }

    pub fn size(&self) -> u64 {
        self.content.len() as u64
    }

    /// Process the file and extract definitions using a language parser
    pub fn process(&self) -> ProcessingResult {
        let start_time = Instant::now();

        // 1. Detect language using pre-computed extension (avoids duplicate parsing)
        let language = match detect_language_from_extension(&self.extension) {
            Ok(lang) => lang,
            Err(e) => {
                return ProcessingResult::Error(ErroredFile {
                    file_path: self.path.clone(),
                    error_message: format!("Failed to detect language: {e}"),
                    error_stage: ProcessingStage::Parsing,
                });
            }
        };

        // Check if language is supported
        let is_supported = matches!(
            language,
            SupportedLanguage::Ruby
                | SupportedLanguage::Python
                | SupportedLanguage::Kotlin
                | SupportedLanguage::Java
                | SupportedLanguage::CSharp
                | SupportedLanguage::TypeScript
                | SupportedLanguage::Rust
        );
        if !is_supported {
            return ProcessingResult::Skipped(SkippedFile {
                file_path: self.path.clone(),
                reason: format!("Unsupported language: {language:?}"),
                file_size: Some(self.size()),
            });
        }

        // 2. Parse the file
        let parse_start = Instant::now();
        let parser = GenericParser::default_for_language(language);
        let parse_result = match parser.parse(self.content, Some(&self.path)) {
            Ok(result) => result,
            Err(e) => {
                return ProcessingResult::Error(ErroredFile {
                    file_path: self.path.clone(),
                    error_message: format!("Failed to parse: {e}"),
                    error_stage: ProcessingStage::Parsing,
                });
            }
        };
        let parse_time = parse_start.elapsed();

        // 3. Run rules to find matches
        let rules_start = Instant::now();
        let rule_manager = RuleManager::new(language);
        let matches = run_rules(&parse_result.ast, Some(&self.path), &rule_manager);
        let rules_time = rules_start.elapsed();

        // 4. Use language-specific analyzer to extract constructs
        let analysis_start = Instant::now();
        let (definitions, imports) = match self.analyze_file(language, &parse_result, &matches) {
            Ok(result) => result,
            Err(e) => {
                return ProcessingResult::Error(ErroredFile {
                    file_path: self.path.clone(),
                    error_message: format!("Failed to analyze: {e}"),
                    error_stage: ProcessingStage::Parsing,
                });
            }
        };
        let analysis_time = analysis_start.elapsed();

        let matches_count = matches.len();
        let definitions_count = definitions.count();
        let imported_symbols_count = imports.as_ref().map_or(0, |i| i.count());

        ProcessingResult::Success(FileProcessingResult {
            file_path: self.path.clone(),
            extension: self.extension.clone(),
            file_size: self.size(),
            language,
            definitions,
            imported_symbols: imports,
            stats: ProcessingStats {
                total_time: start_time.elapsed(),
                parse_time,
                rules_time,
                analysis_time,
                rule_matches: matches_count,
                definitions_count,
                imported_symbols_count,
            },
            is_supported: true,
        })
    }

    fn analyze_file(
        &self,
        language: SupportedLanguage,
        parse_result: &ParseResult,
        matches: &[MatchWithNodes],
    ) -> Result<(Definitions, Option<ImportedSymbols>), anyhow::Error> {
        match language {
            SupportedLanguage::Ruby => {
                let analyzer = RubyAnalyzer::new();
                match analyzer.analyze(matches, parse_result) {
                    Ok(analysis_result) => {
                        Ok((Definitions::Ruby(analysis_result.definitions), None))
                    }
                    Err(e) => Err(anyhow::anyhow!(
                        "Failed to analyze Ruby file '{}': {}",
                        self.path,
                        e
                    )),
                }
            }
            SupportedLanguage::Python => {
                let analyzer = PythonAnalyzer::new();
                match analyzer.analyze(matches, parse_result) {
                    Ok(analysis_result) => Ok((
                        Definitions::Python(analysis_result.definitions),
                        Some(ImportedSymbols::Python(analysis_result.imports)),
                    )),
                    Err(e) => Err(anyhow::anyhow!(
                        "Failed to analyze Python file '{}': {}",
                        self.path,
                        e
                    )),
                }
            }
            SupportedLanguage::Kotlin => {
                let analyzer = KotlinAnalyzer::new();
                match analyzer.analyze(parse_result) {
                    Ok(analysis_result) => Ok((
                        Definitions::Kotlin(analysis_result.definitions),
                        Some(ImportedSymbols::Kotlin(analysis_result.imports)),
                    )),
                    Err(e) => Err(anyhow::anyhow!(
                        "Failed to analyze Kotlin file '{}': {}",
                        self.path,
                        e
                    )),
                }
            }
            SupportedLanguage::Java => {
                let analyzer = JavaAnalyzer::new();
                match analyzer.analyze(parse_result) {
                    Ok(analysis_result) => Ok((
                        Definitions::Java(analysis_result.definitions),
                        Some(ImportedSymbols::Java(analysis_result.imports)),
                    )),
                    Err(e) => Err(anyhow::anyhow!(
                        "Failed to analyze Java file '{}': {}",
                        self.path,
                        e
                    )),
                }
            }
            SupportedLanguage::CSharp => {
                let analyzer = CSharpAnalyzer::new();
                match analyzer.analyze(parse_result) {
                    Ok(analysis_result) => Ok((
                        Definitions::CSharp(analysis_result.definitions),
                        Some(ImportedSymbols::CSharp(analysis_result.imports)),
                    )),
                    Err(e) => Err(anyhow::anyhow!(
                        "Failed to analyze CSharp file '{}': {}",
                        self.path,
                        e
                    )),
                }
            }
            SupportedLanguage::TypeScript => {
                let analyzer = TypeScriptAnalyzer::new();
                match analyzer.analyze(parse_result) {
                    Ok(analysis_result) => Ok((
                        Definitions::TypeScript(analysis_result.definitions),
                        Some(ImportedSymbols::TypeScript(analysis_result.imports)),
                    )),
                    Err(e) => Err(anyhow::anyhow!(
                        "Failed to analyze TypeScript file '{}': {}",
                        self.path,
                        e
                    )),
                }
            }
            SupportedLanguage::Rust => {
                let analyzer = RustAnalyzer::new();
                match analyzer.analyze(matches, parse_result) {
                    Ok(analysis_result) => Ok((
                        Definitions::Rust(analysis_result.definitions),
                        Some(ImportedSymbols::Rust(analysis_result.imports)),
                    )),
                    Err(e) => Err(anyhow::anyhow!(
                        "Failed to analyze Rust file '{}': {}",
                        self.path,
                        e
                    )),
                }
            }
        }
    }
}

/// Enum to hold definitions based on language
#[derive(Clone, Debug)]
pub enum Definitions {
    Ruby(Vec<RubyDefinitionInfo>),
    Python(Vec<PythonDefinitionInfo>),
    Kotlin(Vec<KotlinDefinitionInfo>),
    Java(Vec<JavaDefinitionInfo>),
    CSharp(Vec<CSharpDefinitionInfo>),
    TypeScript(Vec<TypeScriptDefinitionInfo>),
    Rust(Vec<RustDefinitionInfo>),
}

impl Definitions {
    /// Get the count of definitions regardless of type
    pub fn count(&self) -> usize {
        match self {
            Definitions::Ruby(defs) => defs.len(),
            Definitions::Python(defs) => defs.len(),
            Definitions::Kotlin(defs) => defs.len(),
            Definitions::Java(defs) => defs.len(),
            Definitions::CSharp(defs) => defs.len(),
            Definitions::TypeScript(defs) => defs.len(),
            Definitions::Rust(defs) => defs.len(),
        }
    }

    /// Check if there are any definitions
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Get an iterator over definition type strings using the proper DefinitionTypeInfo trait
    pub fn iter_definition_types(&self) -> Box<dyn Iterator<Item = String> + '_> {
        match self {
            Definitions::Ruby(defs) => Box::new(
                defs.iter()
                    .map(|def| def.definition_type.as_str().to_string()),
            ),
            Definitions::Python(defs) => Box::new(
                defs.iter()
                    .map(|def| def.definition_type.as_str().to_string()),
            ),
            Definitions::Kotlin(defs) => Box::new(
                defs.iter()
                    .map(|def| def.definition_type.as_str().to_string()),
            ),
            Definitions::Java(defs) => Box::new(
                defs.iter()
                    .map(|def| def.definition_type.as_str().to_string()),
            ),
            Definitions::CSharp(defs) => Box::new(
                defs.iter()
                    .map(|def| def.definition_type.as_str().to_string()),
            ),
            Definitions::TypeScript(defs) => Box::new(
                defs.iter()
                    .map(|def| def.definition_type.as_str().to_string()),
            ),
            Definitions::Rust(defs) => Box::new(
                defs.iter()
                    .map(|def| def.definition_type.as_str().to_string()),
            ),
        }
    }

    pub fn iter_python(&self) -> Option<impl Iterator<Item = &PythonDefinitionInfo>> {
        match self {
            Definitions::Python(defs) => Some(defs.iter()),
            _ => None,
        }
    }

    pub fn iter_ruby(&self) -> Option<impl Iterator<Item = &RubyDefinitionInfo>> {
        match self {
            Definitions::Ruby(defs) => Some(defs.iter()),
            _ => None,
        }
    }

    pub fn iter_kotlin(&self) -> Option<impl Iterator<Item = &KotlinDefinitionInfo>> {
        match self {
            Definitions::Kotlin(defs) => Some(defs.iter()),
            _ => None,
        }
    }

    pub fn iter_java(&self) -> Option<impl Iterator<Item = &JavaDefinitionInfo>> {
        match self {
            Definitions::Java(defs) => Some(defs.iter()),
            _ => None,
        }
    }

    pub fn iter_csharp(&self) -> Option<impl Iterator<Item = &CSharpDefinitionInfo>> {
        match self {
            Definitions::CSharp(defs) => Some(defs.iter()),
            _ => None,
        }
    }

    pub fn iter_typescript(&self) -> Option<impl Iterator<Item = &TypeScriptDefinitionInfo>> {
        match self {
            Definitions::TypeScript(defs) => Some(defs.iter()),
            _ => None,
        }
    }

    pub fn iter_rust(&self) -> Option<impl Iterator<Item = &RustDefinitionInfo>> {
        match self {
            Definitions::Rust(defs) => Some(defs.iter()),
            _ => None,
        }
    }
}

/// Enum to hold imported symbols based on language
#[derive(Clone, Debug)]
pub enum ImportedSymbols {
    Java(Vec<JavaImportedSymbolInfo>),
    Kotlin(Vec<KotlinImportedSymbolInfo>),
    Python(Vec<PythonImportedSymbolInfo>),
    CSharp(Vec<CSharpImportedSymbolInfo>),
    TypeScript(Vec<TypeScriptImportedSymbolInfo>),
    Rust(Vec<RustImportedSymbolInfo>),
}

impl ImportedSymbols {
    /// Get the count of imported symbols regardless of type
    pub fn count(&self) -> usize {
        match self {
            ImportedSymbols::Java(imported_symbols) => imported_symbols.len(),
            ImportedSymbols::Kotlin(imported_symbols) => imported_symbols.len(),
            ImportedSymbols::Python(imported_symbols) => imported_symbols.len(),
            ImportedSymbols::CSharp(imported_symbols) => imported_symbols.len(),
            ImportedSymbols::TypeScript(imported_symbols) => imported_symbols.len(),
            ImportedSymbols::Rust(imported_symbols) => imported_symbols.len(),
        }
    }

    /// Check if there are any imported symbols
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn iter_kotlin(&self) -> Option<impl Iterator<Item = &KotlinImportedSymbolInfo>> {
        match self {
            ImportedSymbols::Kotlin(imported_symbols) => Some(imported_symbols.iter()),
            _ => None,
        }
    }

    pub fn iter_java(&self) -> Option<impl Iterator<Item = &JavaImportedSymbolInfo>> {
        match self {
            ImportedSymbols::Java(imported_symbols) => Some(imported_symbols.iter()),
            _ => None,
        }
    }

    pub fn iter_csharp(
        &self,
    ) -> Option<
        impl Iterator<
            Item = &parser_core::imports::ImportedSymbolInfo<
                parser_core::csharp::types::CSharpImportType,
                parser_core::csharp::types::CSharpFqn,
            >,
        >,
    > {
        match self {
            ImportedSymbols::CSharp(imported_symbols) => Some(imported_symbols.iter()),
            _ => None,
        }
    }

    pub fn iter_python(&self) -> Option<impl Iterator<Item = &PythonImportedSymbolInfo>> {
        match self {
            ImportedSymbols::Python(imported_symbols) => Some(imported_symbols.iter()),
            _ => None,
        }
    }

    pub fn iter_typescript(&self) -> Option<impl Iterator<Item = &TypeScriptImportedSymbolInfo>> {
        match self {
            ImportedSymbols::TypeScript(imported_symbols) => Some(imported_symbols.iter()),
            _ => None,
        }
    }

    pub fn iter_rust(&self) -> Option<impl Iterator<Item = &RustImportedSymbolInfo>> {
        match self {
            ImportedSymbols::Rust(imported_symbols) => Some(imported_symbols.iter()),
            _ => None,
        }
    }
}

/// Result of processing a single file using Ruby analyzer
#[derive(Clone, Debug)]
pub struct FileProcessingResult {
    /// File path
    pub file_path: String,
    /// Extension of the file
    pub extension: String,
    /// File size in bytes
    pub file_size: u64,
    /// Detected language
    pub language: SupportedLanguage,
    /// Extracted definitions
    pub definitions: Definitions,
    /// Extracted imported symbols
    pub imported_symbols: Option<ImportedSymbols>,
    /// Processing statistics
    pub stats: ProcessingStats,
    /// Whether this language is supported for analysis
    pub is_supported: bool,
}

/// Processing statistics
#[derive(Debug, Clone)]
pub struct ProcessingStats {
    /// Total processing time
    pub total_time: Duration,
    /// Time spent parsing
    pub parse_time: Duration,
    /// Time spent running rules
    pub rules_time: Duration,
    /// Time spent in Ruby analysis
    pub analysis_time: Duration,
    /// Number of rule matches found
    pub rule_matches: usize,
    /// Number of definitions extracted
    pub definitions_count: usize,
    /// Number of imported symbols extracted
    pub imported_symbols_count: usize,
}
