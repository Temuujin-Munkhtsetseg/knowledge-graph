use std::collections::HashSet;
use std::path::Path;

use ignore::WalkBuilder;

// FIXME: this should be a database query
// In the essence of time, we'll use FS for now
// TODO: replace with database query
pub fn collect_paths_ignore(
    project_root: &Path,
    relative_paths: &[String],
    dir_depth: u64,
) -> Result<(Vec<String>, Vec<String>), rmcp::ErrorData> {
    let dir_depth = dir_depth.min(3);
    let project_root_canon = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    let mut files_abs: Vec<String> = Vec::new();
    let mut directories_rel_set: HashSet<String> = HashSet::new();
    let mut seen_files_abs: HashSet<String> = HashSet::new();
    let mut has_dir_input = false;

    for rel in relative_paths {
        let abs_path = project_root_canon.join(rel);
        let canon = match abs_path.canonicalize() {
            Ok(c) => c,
            Err(e) => {
                log::info!("Skipping path: {} (error: {})", abs_path.display(), e);
                continue;
            }
        };
        if !canon.starts_with(&project_root_canon) {
            continue;
        }

        if canon.is_file() {
            let s = canon.to_string_lossy().to_string();
            if seen_files_abs.insert(s.clone()) {
                files_abs.push(s);
            }
            if let Some(parent) = canon.parent()
                && let Ok(relp) = parent.strip_prefix(&project_root_canon)
            {
                directories_rel_set.insert(relp.to_string_lossy().to_string());
            }
            continue;
        }

        if canon.is_dir() {
            has_dir_input = true;
            let mut builder = WalkBuilder::new(&canon);
            builder.add(&canon);
            builder.follow_links(false);
            builder.standard_filters(true);
            builder.git_global(true).git_ignore(true).git_exclude(true);
            // For files, allow one extra level so files inside deepest listed directories are included
            builder.max_depth(Some(dir_depth as usize + 1));

            let walker = builder.build();
            for dent in walker {
                let dent = match dent {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                let depth = dent.depth();
                if let Some(ft) = dent.file_type() {
                    if ft.is_dir() {
                        if depth >= 1
                            && depth <= dir_depth as usize
                            && let Ok(relp) = dent.path().strip_prefix(&project_root_canon)
                        {
                            let rel_str = relp.to_string_lossy().to_string();
                            if !rel_str.is_empty() {
                                directories_rel_set.insert(rel_str);
                            }
                        }
                    } else if ft.is_file()
                        && let Ok(cp) = dent.path().canonicalize()
                    {
                        let s = cp.to_string_lossy().to_string();
                        if seen_files_abs.insert(s.clone()) {
                            files_abs.push(s);
                        }
                    }
                }
            }
        }
    }

    // Build directory list from files respecting depth rules.
    // - If any directory was provided as input, include ancestors up to dir_depth.
    // - If only files were provided, include immediate parent directories only.
    for file_abs in &files_abs {
        if let Ok(rel_file) = Path::new(file_abs).strip_prefix(&project_root_canon) {
            let parent = match rel_file.parent() {
                Some(p) => p,
                None => continue,
            };

            if has_dir_input {
                // include ancestors up to dir_depth using OS path operations
                let mut acc = std::path::PathBuf::new();
                let mut count = 0usize;
                for comp in parent.components() {
                    acc.push(comp.as_os_str());
                    count += 1;
                    directories_rel_set.insert(acc.to_string_lossy().to_string());
                    if count >= dir_depth as usize {
                        break;
                    }
                }
            } else {
                // only immediate parent if within depth
                let depth = parent.components().count();
                if depth <= dir_depth as usize {
                    directories_rel_set.insert(parent.to_string_lossy().to_string());
                }
            }
        }
    }

    let mut directories_rel: Vec<String> = directories_rel_set.into_iter().collect();
    directories_rel.sort();
    Ok((files_abs, directories_rel))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_tree() -> (TempDir, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        // dirs
        fs::create_dir_all(root.join("app/models")).unwrap();
        fs::create_dir_all(root.join("lib")).unwrap();
        fs::create_dir_all(root.join(".git/hooks")).unwrap();
        // files
        fs::write(root.join("main.ts"), "console.log('main');").unwrap();
        fs::write(root.join("app/app.ts"), "export {};").unwrap();
        fs::write(root.join("app/models/user.ts"), "export class User {}").unwrap();
        fs::write(root.join("lib/util.ts"), "export const util = () => {};").unwrap();
        (tmp, root)
    }

    #[test]
    fn test_collect_paths_ignore_depth_one_lists_only_first_level_dirs() {
        let (_tmp, root) = setup_tree();
        let (files, dirs) = collect_paths_ignore(&root, &[".".to_string()], 1).unwrap();
        let dir_set: std::collections::HashSet<_> = dirs.iter().cloned().collect();
        assert!(dir_set.contains("app"));
        assert!(dir_set.contains("lib"));
        assert!(!dir_set.contains("app/models"));
        // files should include depth 0 and depth 1 files, but not deeper
        let files_set: std::collections::HashSet<_> = files.iter().cloned().collect();
        assert!(
            files_set.contains(
                &root
                    .join("main.ts")
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            )
        );
        // app/app.ts is depth=2 and should be excluded at depth=1
        assert!(
            !files_set.contains(
                &root
                    .join("app/models/user.ts")
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            )
        );
    }

    #[test]
    fn test_collect_paths_ignore_depth_two_includes_nested_dirs() {
        let (_tmp, root) = setup_tree();
        let (_files, dirs) = collect_paths_ignore(&root, &[".".to_string()], 2).unwrap();
        let dir_set: std::collections::HashSet<_> = dirs.iter().cloned().collect();
        assert!(dir_set.contains("app"));
        assert!(dir_set.contains("lib"));
        assert!(dir_set.contains("app/models"));
    }

    #[test]
    fn test_collect_paths_ignore_file_input_includes_parent_dir() {
        let (_tmp, root) = setup_tree();
        let rel = vec!["app/models/user.ts".to_string()];
        let (files, dirs) = collect_paths_ignore(&root, &rel, 2).unwrap();
        assert!(dirs.contains(&"app/models".to_string()));
        assert!(files.iter().any(|f| f.ends_with("app/models/user.ts")));
    }

    #[test]
    fn test_collect_paths_ignore_skips_hidden_git() {
        let (_tmp, root) = setup_tree();
        let (_files, dirs) = collect_paths_ignore(&root, &[".".to_string()], 2).unwrap();
        assert!(!dirs.iter().any(|d| d.starts_with(".git")));
    }
}
