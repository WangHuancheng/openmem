use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

use crate::error::Result;
use crate::node;

/// Extract all `#tag` references from markdown content.
///
/// Tags are single words preceded by #, like `#rust` or `#project-acme`.
/// Tags must start with a letter or underscore, followed by letters, digits,
/// hyphens, or underscores. Tags inside code blocks are ignored.
pub fn parse_tags(content: &str) -> Vec<String> {
    // Match #tag where tag starts with letter/underscore and contains alphanumeric, hyphen, underscore
    let re = Regex::new(r"#([a-zA-Z_][a-zA-Z0-9_-]*)").unwrap();
    let mut tags: Vec<String> = Vec::new();
    let mut in_code_block = false;

    for line in content.lines() {
        let trimmed = line.trim_start();

        // Track fenced code blocks
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            continue;
        }

        // Skip heading lines (they use # for headings, not tags)
        if trimmed.starts_with('#') && trimmed.chars().nth(1).map(|c| c == ' ').unwrap_or(false) {
            continue;
        }

        for cap in re.captures_iter(line) {
            let tag = cap[1].to_string();
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
    }

    tags.sort();
    tags
}

/// Find all nodes that have a specific tag.
///
/// Returns node paths sorted alphabetically.
pub fn find_by_tag(vault: &Path, tag: &str) -> Result<Vec<String>> {
    let all_nodes = node::list(vault, "")?;
    let target_tag = if tag.starts_with('#') {
        tag[1..].to_string()
    } else {
        tag.to_string()
    };

    let mut results = Vec::new();

    for node_path in &all_nodes {
        if let Ok(content) = node::read(vault, node_path) {
            let tags = parse_tags(&content);
            if tags.contains(&target_tag) {
                results.push(node_path.clone());
            }
        }
    }

    results.sort();
    Ok(results)
}

/// Get all tags used in the vault with their counts.
///
/// Returns tags sorted alphabetically.
pub fn list_tags(vault: &Path, scope: Option<&str>) -> Result<Vec<(String, usize)>> {
    let nodes = node::list(vault, scope.unwrap_or(""))?;
    let mut tag_counts: HashMap<String, usize> = HashMap::new();

    for node_path in &nodes {
        if let Ok(content) = node::read(vault, node_path) {
            let tags = parse_tags(&content);
            for tag in tags {
                *tag_counts.entry(tag).or_insert(0) += 1;
            }
        }
    }

    let mut result: Vec<(String, usize)> = tag_counts.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(result)
}

/// Get tags for a specific node.
pub fn get_node_tags(vault: &Path, node_path: &str) -> Result<Vec<String>> {
    let content = node::read(vault, node_path)?;
    Ok(parse_tags(&content))
}

/// Build a tag index: tag → list of nodes.
pub fn build_tag_index(vault: &Path, scope: Option<&str>) -> Result<HashMap<String, Vec<String>>> {
    let nodes = node::list(vault, scope.unwrap_or(""))?;
    let mut index: HashMap<String, Vec<String>> = HashMap::new();

    for node_path in &nodes {
        if let Ok(content) = node::read(vault, node_path) {
            let tags = parse_tags(&content);
            for tag in tags {
                index.entry(tag).or_default().push(node_path.clone());
            }
        }
    }

    // Sort node lists
    for nodes in index.values_mut() {
        nodes.sort();
    }

    Ok(index)
}

/// Format tag index for display.
pub fn format_tag_index(index: &HashMap<String, Vec<String>>) -> String {
    if index.is_empty() {
        return "No tags found.".to_string();
    }

    let mut tags: Vec<&String> = index.keys().collect();
    tags.sort();

    let mut output = String::new();
    for tag in tags {
        let nodes = &index[tag];
        let node_word = if nodes.len() == 1 { "node" } else { "nodes" };
        output.push_str(&format!("#{} ({} {})\n", tag, nodes.len(), node_word));
        for node in nodes {
            output.push_str(&format!("  - {}\n", node));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::SystemTime;

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_vault() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!(
            "openmem_tags_test_{}_{}_{}",
            std::process::id(),
            id,
            nanos
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parse_single_tag() {
        let content = "This is about #rust programming.";
        let tags = parse_tags(content);
        assert_eq!(tags, vec!["rust"]);
    }

    #[test]
    fn parse_multiple_tags() {
        let content = "Using #rust with #react and #typescript.";
        let tags = parse_tags(content);
        assert_eq!(tags, vec!["react", "rust", "typescript"]);
    }

    #[test]
    fn parse_no_tags() {
        let content = "No tags here, just plain text.\n\nWith multiple lines.";
        let tags = parse_tags(content);
        assert!(tags.is_empty());
    }

    #[test]
    fn parse_tags_deduplicates() {
        let content = "We use #rust and also #rust again.";
        let tags = parse_tags(content);
        assert_eq!(tags, vec!["rust"]);
    }

    #[test]
    fn parse_tags_with_hyphens() {
        let content = "Working on #project-acme with #my-feature.";
        let tags = parse_tags(content);
        assert_eq!(tags, vec!["my-feature", "project-acme"]);
    }

    #[test]
    fn parse_tags_ignores_headings() {
        let content = "# Heading One\n\n## Heading Two\n\n#not-heading tag";
        let tags = parse_tags(content);
        // The #not-heading should be parsed because it's not a heading (no space after #)
        assert_eq!(tags, vec!["not-heading"]);
    }

    #[test]
    fn parse_tags_ignores_code_blocks() {
        let content = "Some #visible tag.\n\n```rust\nlet x = #ignored;\n```\n\nAnother #tag.";
        let tags = parse_tags(content);
        assert_eq!(tags, vec!["tag", "visible"]);
    }

    #[test]
    fn parse_tags_ignores_numbers_start() {
        // Tags must start with letter or underscore
        let content = "Not a tag #123 but #_123 is valid.";
        let tags = parse_tags(content);
        assert_eq!(tags, vec!["_123"]);
    }

    #[test]
    fn find_by_tag_finds_nodes() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "node-a", "About #rust programming.").unwrap();
        node::write(&vault, "node-b", "Also about #rust and #cli.").unwrap();
        node::write(&vault, "node-c", "No tags here.").unwrap();

        let results = find_by_tag(&vault, "rust").unwrap();
        assert_eq!(results, vec!["node-a", "node-b"]);

        let results = find_by_tag(&vault, "cli").unwrap();
        assert_eq!(results, vec!["node-b"]);
    }

    #[test]
    fn find_by_tag_with_hash_prefix() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "node-a", "Tagged #rust.").unwrap();

        let results = find_by_tag(&vault, "#rust").unwrap();
        assert_eq!(results, vec!["node-a"]);
    }

    #[test]
    fn list_tags_returns_counts() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "node-a", "#rust #cli").unwrap();
        node::write(&vault, "node-b", "#rust #web").unwrap();

        let tags = list_tags(&vault, None).unwrap();
        assert_eq!(tags, vec![
            ("cli".to_string(), 1),
            ("rust".to_string(), 2),
            ("web".to_string(), 1),
        ]);
    }

    #[test]
    fn get_node_tags_extracts_tags() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "test-node", "Using #rust and #cli here.").unwrap();

        let tags = get_node_tags(&vault, "test-node").unwrap();
        assert_eq!(tags, vec!["cli", "rust"]);
    }

    #[test]
    fn test_build_tag_index() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "a", "#rust #cli").unwrap();
        node::write(&vault, "b", "#rust #web").unwrap();

        let index = build_tag_index(&vault, None).unwrap();

        assert_eq!(index.get("rust"), Some(&vec!["a".to_string(), "b".to_string()]));
        assert_eq!(index.get("cli"), Some(&vec!["a".to_string()]));
        assert_eq!(index.get("web"), Some(&vec!["b".to_string()]));
    }

    #[test]
    fn format_tag_index_output() {
        let mut index = HashMap::new();
        index.insert("rust".to_string(), vec!["a".to_string(), "b".to_string()]);
        index.insert("cli".to_string(), vec!["a".to_string()]);

        let output = format_tag_index(&index);

        assert!(output.contains("#cli (1 node)"));
        assert!(output.contains("#rust (2 nodes)"));
        assert!(output.contains("  - a"));
        assert!(output.contains("  - b"));
    }
}
