use crate::project::file_info::FileInfo;
use anyhow::Result;
use parser_core::{
    parser::{GenericParser, LanguageParser, SupportedLanguage, detect_language_from_extension},
    ruby::{analyzer::RubyAnalyzer, definitions::RubyDefinitionInfo},
    rules::{RuleManager, run_rules},
};
use std::time::{Duration, Instant};

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

    /// Process the file and extract definitions using Ruby analyzer
    pub fn process(&self) -> Result<FileProcessingResult> {
        let start_time = Instant::now();

        // 1. Detect language using pre-computed extension (avoids duplicate parsing)
        let language = detect_language_from_extension(&self.extension)
            .map_err(|e| anyhow::anyhow!("Failed to detect language for '{}': {}", self.path, e))?;

        // For now, only process Ruby files
        if language != SupportedLanguage::Ruby {
            return Ok(FileProcessingResult {
                file_path: self.path.clone(),
                language,
                definitions: Vec::new(),
                stats: ProcessingStats {
                    total_time: start_time.elapsed(),
                    parse_time: Duration::from_millis(0),
                    rules_time: Duration::from_millis(0),
                    analysis_time: Duration::from_millis(0),
                    rule_matches: 0,
                    definitions_count: 0,
                },
                is_supported: false,
            });
        }

        // 2. Parse the file
        let parse_start = Instant::now();
        let parser = GenericParser::default_for_language(language);
        let parse_result = parser
            .parse(self.content, Some(&self.path))
            .map_err(|e| anyhow::anyhow!("Failed to parse '{}': {}", self.path, e))?;
        let parse_time = parse_start.elapsed();

        // 3. Run rules to find matches
        let rules_start = Instant::now();
        let rule_manager = RuleManager::new(language);
        let matches = run_rules(&parse_result.ast, Some(&self.path), &rule_manager);
        let rules_time = rules_start.elapsed();

        // 4. Use Ruby analyzer to extract definitions
        let analysis_start = Instant::now();
        let analyzer = RubyAnalyzer::new();
        let analysis_result = analyzer
            .analyze(&matches, &parse_result)
            .map_err(|e| anyhow::anyhow!("Failed to analyze Ruby file '{}': {}", self.path, e))?;
        let analysis_time = analysis_start.elapsed();

        let total_time = start_time.elapsed();
        let definitions_count = analysis_result.definitions.len();

        Ok(FileProcessingResult {
            file_path: self.path.clone(),
            language,
            definitions: analysis_result.definitions,
            stats: ProcessingStats {
                total_time,
                parse_time,
                rules_time,
                analysis_time,
                rule_matches: matches.len(),
                definitions_count,
            },
            is_supported: true,
        })
    }
}

/// Result of processing a single file using Ruby analyzer
#[derive(Clone)]
pub struct FileProcessingResult {
    /// File path
    pub file_path: String,
    /// Detected language
    pub language: SupportedLanguage,
    /// Extracted definitions from Ruby analyzer
    pub definitions: Vec<RubyDefinitionInfo>,
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
}

/// Process a file from its content with pre-computed file info
pub fn process_file_info(file_info: FileInfo, content: &str) -> Result<FileProcessingResult> {
    let file = FileProcessor::from_file_info(file_info, content);
    file.process()
}

/// Process a file from its content (legacy method)
pub fn process_file_content(file_path: &str, content: &str) -> Result<FileProcessingResult> {
    let file = FileProcessor::new(file_path.to_string(), content);
    file.process()
}
