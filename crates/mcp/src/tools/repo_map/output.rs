use crate::tools::xml::{XmlBuilder, remove_cdata_sections};
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Clone, Debug)]
pub struct RepoMapItem {
    pub file_rel: String,
    pub fqn: String,
    pub def_type: String,
    pub start_line_1: usize,
    pub end_line_1: usize,
    pub snippet: Option<String>,
}

fn group_items_by_file(items: Vec<RepoMapItem>) -> BTreeMap<String, Vec<RepoMapItem>> {
    let mut grouped: BTreeMap<String, Vec<RepoMapItem>> = BTreeMap::new();
    for item in items {
        grouped.entry(item.file_rel.clone()).or_default().push(item);
    }
    grouped
}

fn build_definitions_text(defs: &[RepoMapItem]) -> String {
    let mut defs_text = String::new();
    let mut printed_lines: HashSet<usize> = HashSet::new();
    for d in defs {
        defs_text.push_str(&format!(
            "{} {} L{}-{}\n",
            d.def_type.to_lowercase(),
            d.fqn,
            d.start_line_1,
            d.end_line_1
        ));
        if let Some(snippet) = &d.snippet {
            let snippet_start = d.start_line_1;
            let snippet_end = std::cmp::min(d.start_line_1 + 2, d.end_line_1);
            for (offset, line) in snippet.lines().enumerate() {
                let ln = snippet_start + offset;
                if ln < snippet_start || ln > snippet_end {
                    continue;
                }
                if printed_lines.insert(ln) {
                    defs_text.push('│');
                    defs_text.push(' ');
                    defs_text.push_str(line);
                    defs_text.push('\n');
                }
            }
        }
        defs_text.push('\n');
    }
    defs_text
}

#[derive(Default)]
struct DirNode {
    children: BTreeMap<String, DirNode>,
}

fn build_directories_ascii_tree(directories: &[String]) -> String {
    let mut root = DirNode::default();
    let mut has_root = false;
    let mut uniq: BTreeSet<String> = BTreeSet::new();
    for d in directories {
        if d == "." || d.is_empty() {
            has_root = true;
            continue;
        }
        uniq.insert(d.trim_matches('/').to_string());
    }
    for path in uniq {
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let mut node = &mut root;
        for part in parts {
            node = node.children.entry(part.to_string()).or_default();
        }
    }

    fn render(node: &DirNode, prefix: &str, out: &mut String) {
        let len = node.children.len();
        for (idx, (name, child)) in node.children.iter().enumerate() {
            let last = idx + 1 == len;
            let connector = if last { "└── " } else { "├── " };
            out.push_str(prefix);
            out.push_str(connector);
            out.push_str(name);
            out.push('\n');
            let new_prefix = if last {
                format!("{prefix}    ")
            } else {
                format!("{prefix}│   ")
            };
            render(child, &new_prefix, out);
        }
    }

    let mut out = String::new();
    if has_root {
        out.push_str(".\n");
    }
    render(&root, "", &mut out);
    out
}

pub fn build_repo_map_xml(
    items: Vec<RepoMapItem>,
    directories: Vec<String>,
    show_directories: bool,
    show_definitions: bool,
    next_page: Option<u64>,
    depth: u64,
    system_message: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let grouped = group_items_by_file(items);

    let mut builder = XmlBuilder::new();
    builder.start_element("ToolResponse")?;

    builder.start_element("repo-map")?;
    builder.write_numeric_element("depth", depth)?;

    // Directories (ASCII tree)
    if show_directories {
        let dirs_text = build_directories_ascii_tree(&directories);
        builder.write_cdata_element("directories", &dirs_text)?;
    }

    // Files and definitions
    if show_definitions {
        builder.start_element("files")?;
        for (file_path, defs) in grouped.iter() {
            builder.start_element("file")?;
            builder.write_element("path", file_path)?;
            let defs_text = build_definitions_text(defs);
            builder.write_cdata_element("definitions", &defs_text)?;
            builder.end_element("file")?;
        }
        builder.end_element("files")?;
    }

    builder.end_element("repo-map")?;

    builder.write_optional_numeric_element("next-page", &next_page)?;
    builder.write_cdata_element("system-message", &system_message)?;

    builder.end_element("ToolResponse")?;
    let xml = builder.finish()?;
    remove_cdata_sections(&xml)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_directories_ascii_tree() {
        let dirs = vec![
            ".".to_string(),
            "app".to_string(),
            "app/models".to_string(),
            "lib".to_string(),
        ];
        let tree = build_directories_ascii_tree(&dirs);
        assert!(tree.contains(".\n"));
        assert!(tree.contains("app"));
        assert!(tree.contains("models"));
        assert!(tree.contains("lib"));
        // Should have connectors
        assert!(tree.contains("├── ") || tree.contains("└── "));
    }

    #[test]
    fn test_build_definitions_text_dedup_snippets() {
        let defs = vec![
            RepoMapItem {
                file_rel: "a.ts".to_string(),
                fqn: "A::foo".to_string(),
                def_type: "Method".to_string(),
                start_line_1: 10,
                end_line_1: 20,
                snippet: Some("line10\nline11\nline12".to_string()),
            },
            RepoMapItem {
                file_rel: "a.ts".to_string(),
                fqn: "A::bar".to_string(),
                def_type: "Method".to_string(),
                start_line_1: 11,
                end_line_1: 21,
                snippet: Some("line11\nline12\nline13".to_string()),
            },
        ];
        let text = build_definitions_text(&defs);
        // headers present
        assert!(text.contains("method A::foo L10-20"));
        assert!(text.contains("method A::bar L11-21"));
        // lines 10, 11, 12 should not duplicate
        let occurrences_line11 = text.matches("line11\n").count();
        assert!(
            occurrences_line11 <= 1,
            "expected no duplicate snippet lines"
        );
    }

    #[test]
    fn test_group_items_by_file_groups_correctly() {
        let items = vec![
            RepoMapItem {
                file_rel: "a.ts".to_string(),
                fqn: "A::foo".to_string(),
                def_type: "Function".to_string(),
                start_line_1: 1,
                end_line_1: 2,
                snippet: None,
            },
            RepoMapItem {
                file_rel: "b.ts".to_string(),
                fqn: "B::bar".to_string(),
                def_type: "Class".to_string(),
                start_line_1: 3,
                end_line_1: 4,
                snippet: None,
            },
            RepoMapItem {
                file_rel: "a.ts".to_string(),
                fqn: "A::baz".to_string(),
                def_type: "Method".to_string(),
                start_line_1: 5,
                end_line_1: 6,
                snippet: None,
            },
        ];
        let grouped = group_items_by_file(items);
        assert_eq!(grouped.len(), 2);
        assert!(grouped.contains_key("a.ts"));
        assert!(grouped.contains_key("b.ts"));
        assert_eq!(grouped.get("a.ts").unwrap().len(), 2);
        assert_eq!(grouped.get("b.ts").unwrap().len(), 1);
    }

    #[test]
    fn test_build_definitions_text_formatting_headers_only() {
        let defs = vec![RepoMapItem {
            file_rel: "x.rs".to_string(),
            fqn: "mod::Type".to_string(),
            def_type: "Class".to_string(),
            start_line_1: 100,
            end_line_1: 120,
            snippet: None,
        }];
        let text = build_definitions_text(&defs);
        assert!(text.contains("class mod::Type L100-120"));
        // No snippet lines
        assert!(!text.contains("│ "));
    }

    #[test]
    fn test_build_directories_ascii_tree_root_only() {
        let dirs = vec![".".to_string()];
        let tree = build_directories_ascii_tree(&dirs);
        assert_eq!(tree, ".\n");
    }

    #[test]
    fn test_build_directories_ascii_tree_sorted_and_nested() {
        let dirs = vec![
            "app/models".to_string(),
            "app/utils".to_string(),
            "app".to_string(),
            "lib".to_string(),
        ];
        let tree = build_directories_ascii_tree(&dirs);
        // app appears before lib due to BTree ordering
        let app_idx = tree.find("app").unwrap();
        let lib_idx = tree.find("lib").unwrap();
        assert!(app_idx < lib_idx);
        // nested entries are present
        assert!(tree.contains("models"));
        assert!(tree.contains("utils"));
    }

    #[test]
    fn test_build_repo_map_xml_flags_toggle_blocks() {
        let items = vec![RepoMapItem {
            file_rel: "a.ts".to_string(),
            fqn: "A::a".to_string(),
            def_type: "Function".to_string(),
            start_line_1: 1,
            end_line_1: 2,
            snippet: Some("x".to_string()),
        }];
        let dirs = vec![".".to_string(), "app".to_string()];

        // Both on
        let xml = build_repo_map_xml(
            items.clone(),
            dirs.clone(),
            true,
            true,
            None,
            1,
            "msg".to_string(),
        )
        .unwrap();
        assert!(xml.contains("<directories>"));
        assert!(xml.contains("<files>"));
        assert!(xml.contains("<path>a.ts</path>"));

        // Directories only
        let xml = build_repo_map_xml(
            items.clone(),
            dirs.clone(),
            true,
            false,
            None,
            1,
            "msg".to_string(),
        )
        .unwrap();
        assert!(xml.contains("<directories>"));
        assert!(!xml.contains("<files>"));

        // Definitions only
        let xml = build_repo_map_xml(
            items.clone(),
            dirs.clone(),
            false,
            true,
            None,
            1,
            "msg".to_string(),
        )
        .unwrap();
        assert!(!xml.contains("<directories>"));
        assert!(xml.contains("<files>"));
    }
}
