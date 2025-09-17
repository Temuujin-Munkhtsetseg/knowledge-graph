use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use database::querying::QueryingService;

use super::output::{FileBlock, ImportUsageOutput};
use crate::tools::file_reader_utils::read_file_chunks;

use super::constants::FILE_READ_TIMEOUT_SECONDS;
use super::input::ImportUsageInput;
use super::repository::{ImportHit, ImportUsageRepository};

type UsageEntry = (String, i32, i32, i32, String);
type UsagesByFile = BTreeMap<String, Vec<UsageEntry>>;

pub struct ImportUsageService {
    repository: ImportUsageRepository,
}

impl ImportUsageService {
    pub fn new(querying_service: Arc<dyn QueryingService>) -> Self {
        Self {
            repository: ImportUsageRepository::new(querying_service),
        }
    }

    pub async fn analyze(
        &self,
        input: ImportUsageInput,
    ) -> Result<ImportUsageOutput, rmcp::ErrorData> {
        let import_paths: Vec<String> = input
            .packages
            .iter()
            .map(|p| p.import_path.clone())
            .collect();
        let names: Vec<String> = input.packages.iter().map(|p| p.name.clone()).collect();
        let aliases: Vec<String> = input.packages.iter().map(|p| p.alias.clone()).collect();

        let (all_imports, all_references) = self.repository.find_imports_and_references_combined(
            input.database_path.clone(),
            import_paths,
            names,
            aliases,
        )?;

        // Build output files with imports and usages
        let mut files_output: Vec<FileBlock> = Vec::new();

        // Imports by file
        let mut imports_by_file: BTreeMap<String, Vec<ImportHit>> = BTreeMap::new();
        for ih in &all_imports {
            imports_by_file
                .entry(ih.file_path.clone())
                .or_default()
                .push(ih.clone());
        }

        // Build unique import line ranges per file (dedupe by line range only)
        let mut import_ranges_by_file: BTreeMap<String, Vec<(i64, i64)>> = BTreeMap::new();
        for (file, imps) in &imports_by_file {
            let mut unique_ranges: BTreeSet<(i64, i64)> = BTreeSet::new();
            for d in imps {
                unique_ranges.insert((d.start_line, d.end_line));
            }
            let mut ranges: Vec<(i64, i64)> = unique_ranges.into_iter().collect();
            ranges.sort_by_key(|(s, e)| (*s, *e));
            import_ranges_by_file.insert(file.clone(), ranges);
        }

        // Prepare ONE combined set of file read chunks for both references and imports (deduped)
        let mut combined_keys: Vec<(String, i64, i64)> = Vec::new();
        let mut combined_chunks: Vec<(String, usize, usize)> = Vec::new();
        let mut seen: BTreeSet<(String, usize, usize)> = BTreeSet::new();

        // Add reference ranges
        for r in &all_references {
            let start = (r.start_line as usize).max(1);
            let end = (r.end_line as usize).max(start);
            let abs = std::path::Path::new(&input.project_absolute_path)
                .join(&r.file_path)
                .to_string_lossy()
                .to_string();
            let key_abs = (abs.clone(), start, end);
            if seen.insert(key_abs) {
                combined_keys.push((r.file_path.clone(), r.start_line, r.end_line));
                combined_chunks.push((abs, start, end));
            }
        }

        // Add import ranges
        for (file, ranges) in &import_ranges_by_file {
            let abs_file = std::path::Path::new(&input.project_absolute_path)
                .join(file)
                .to_string_lossy()
                .to_string();
            for (s, e) in ranges.iter() {
                let start = (*s as usize).max(1);
                let end = (*e as usize).max(start);
                let key_abs = (abs_file.clone(), start, end);
                if seen.insert(key_abs) {
                    combined_keys.push((file.clone(), *s, *e));
                    combined_chunks.push((abs_file.clone(), start, end));
                }
            }
        }

        // Perform a single file read for all chunks
        let combined_contents = if combined_chunks.is_empty() {
            Vec::new()
        } else {
            match tokio::time::timeout(
                std::time::Duration::from_secs(FILE_READ_TIMEOUT_SECONDS),
                read_file_chunks(combined_chunks),
            )
            .await
            {
                Ok(Ok(results)) => results,
                _ => Vec::new(),
            }
        };

        // Build a unified snippet map keyed by (relative file path, start, end)
        let mut snippet_map: BTreeMap<(String, i64, i64), String> = BTreeMap::new();
        for (idx, key) in combined_keys.iter().enumerate() {
            if let Some(res) = combined_contents.get(idx) {
                let snippet = res
                    .as_ref()
                    .ok()
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                snippet_map.insert(key.clone(), snippet);
            }
        }

        // Usages grouped by file
        let mut usages_by_file: UsagesByFile = BTreeMap::new();
        for r in &all_references {
            let snippet = snippet_map
                .get(&(r.file_path.clone(), r.start_line, r.end_line))
                .cloned()
                .unwrap_or_default();
            usages_by_file
                .entry(r.file_path.clone())
                .or_default()
                .push((
                    r.fqn.clone(),
                    r.def_start_line as i32,
                    r.start_line as i32,
                    r.end_line as i32,
                    snippet,
                ));
        }

        // Collect and paginate files themselves
        let all_files_sorted: Vec<String> = imports_by_file
            .keys()
            .chain(usages_by_file.keys())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .cloned()
            .collect();
        let total_files = all_files_sorted.len();
        let start_index = ((input.page - 1) * input.page_size) as usize;
        let end_index = (start_index + input.page_size as usize).min(total_files);
        let has_more = end_index < total_files;

        for file in all_files_sorted[start_index..end_index].iter() {
            let mut imports_text = String::new();
            let mut usages_text = String::new();

            if let Some(ranges) = import_ranges_by_file.get(file) {
                for (s, e) in ranges.iter() {
                    let snippet = snippet_map
                        .get(&(file.clone(), *s, *e))
                        .cloned()
                        .unwrap_or_default();
                    let one_line = snippet
                        .replace('\n', " ")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    imports_text.push_str(&format!("import {} L{}-{}\n", one_line, s, e));
                }
            }

            if let Some(entries) = usages_by_file.get(file) {
                let mut entries_sorted = entries.clone();
                entries_sorted
                    .sort_by_key(|(_, _def_start, ref_start, ref_end, _)| (*ref_start, *ref_end));
                for (fqn, def_start, ref_start, ref_end, snippet) in entries_sorted.into_iter() {
                    usages_text.push_str(&format!("usage {} L{}-{}\n", fqn, ref_start, ref_end));
                    if def_start > 0 && ref_start >= def_start {
                        usages_text.push_str("│ ...\n");
                    }
                    if !snippet.is_empty() {
                        usages_text.push_str("│ ");
                        usages_text.push_str(&snippet.replace('\n', "\n│ "));
                        usages_text.push('\n');
                    }
                    usages_text.push('\n');
                }
            }
            files_output.push(FileBlock {
                path: file.clone(),
                imports: vec![imports_text],
                usages: vec![usages_text],
            });
        }
        let next_page = if has_more { Some(input.page + 1) } else { None };
        let mut system_message = String::new();
        let summary = format!(
            "Returned {} file block(s). page={} page_size={}.{}",
            files_output.len(),
            input.page,
            input.page_size,
            if next_page.is_some() {
                " More results available via next-page."
            } else {
                ""
            }
        );
        system_message.push_str(&summary);

        Ok(ImportUsageOutput {
            files: files_output,
            next_page,
            system_message,
        })
    }
}
