use std::collections::HashMap;
use std::path::Path;

use crate::error::Result;
use crate::link;
use crate::node;
use crate::tags;

/// Memory index node path.
pub const INDEX_PATH: &str = "global/index";

/// Generate a memory index for the vault.
///
/// The index is a curated table of contents that provides:
/// - User preferences and agent rules
/// - Project listings with status
/// - Tool references
/// - Tag categories
pub fn generate_index(vault: &Path, scope: Option<&str>) -> Result<String> {
    let mut index = String::new();

    index.push_str("# Memory Index\n\n");
    index.push_str("> Auto-generated vault overview. Last updated: ");
    index.push_str(&chrono_timestamp());
    index.push_str("\n\n");

    // Stats
    let nodes = node::list(vault, scope.unwrap_or(""))?;
    index.push_str(&format!("**{} nodes** in vault\n\n", nodes.len()));

    // Global section
    index.push_str("## Global\n\n");
    let global_nodes: Vec<_> = nodes.iter().filter(|n| n.starts_with("global/")).collect();
    if global_nodes.is_empty() {
        index.push_str("_(no global nodes)_\n\n");
    } else {
        for node_path in &global_nodes {
            let excerpt = get_excerpt(vault, node_path, 80);
            index.push_str(&format!("- [[{}]] — {}\n", node_path, excerpt));
        }
        index.push('\n');
    }

    // Projects section
    index.push_str("## Projects\n\n");
    let project_nodes: Vec<_> = nodes
        .iter()
        .filter(|n| n.starts_with("projects/"))
        .collect();

    // Group by project
    let mut projects: HashMap<String, Vec<&String>> = HashMap::new();
    for node_path in &project_nodes {
        let parts: Vec<&str> = node_path.split('/').collect();
        if parts.len() >= 2 {
            let project_name = parts[1];
            projects
                .entry(project_name.to_string())
                .or_default()
                .push(node_path);
        }
    }

    if projects.is_empty() {
        index.push_str("_(no projects)_\n\n");
    } else {
        let mut project_names: Vec<_> = projects.keys().collect();
        project_names.sort();

        for project_name in project_names {
            let project_nodes = &projects[project_name];
            index.push_str(&format!("### {}\n\n", project_name));

            for node_path in project_nodes {
                let excerpt = get_excerpt(vault, node_path, 60);
                index.push_str(&format!("- [[{}]] — {}\n", node_path, excerpt));
            }
            index.push('\n');
        }
    }

    // Tags section
    index.push_str("## Tags\n\n");
    let tag_index = tags::build_tag_index(vault, scope)?;
    if tag_index.is_empty() {
        index.push_str("_(no tags)_\n\n");
    } else {
        let mut tags_sorted: Vec<_> = tag_index.iter().collect();
        tags_sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));

        for (tag, node_list) in tags_sorted.iter().take(10) {
            index.push_str(&format!("#{} ({} nodes)\n", tag, node_list.len()));
        }
        if tags_sorted.len() > 10 {
            index.push_str(&format!("... and {} more tags\n", tags_sorted.len() - 10));
        }
        index.push('\n');
    }

    // Orphans section (nodes with no links)
    index.push_str("## Orphans\n\n");
    index.push_str("_Nodes with no incoming or outgoing links._\n\n");

    let mut orphans = Vec::new();
    for node_path in &nodes {
        if let Ok(content) = node::read(vault, node_path) {
            let outgoing = link::parse_links(&content);
            let incoming = link::backlinks(vault, node_path).unwrap_or_default();
            if outgoing.is_empty() && incoming.is_empty() {
                orphans.push(node_path.as_str());
            }
        }
    }

    if orphans.is_empty() {
        index.push_str("_(no orphans)_\n\n");
    } else {
        orphans.sort();
        for orphan in orphans.iter().take(10) {
            index.push_str(&format!("- [[{}]]\n", orphan));
        }
        if orphans.len() > 10 {
            index.push_str(&format!("... and {} more\n", orphans.len() - 10));
        }
        index.push('\n');
    }

    Ok(index)
}

/// Get a short excerpt from a node (first non-empty line, truncated).
fn get_excerpt(vault: &Path, node_path: &str, max_len: usize) -> String {
    if let Ok(content) = node::read(vault, node_path) {
        for line in content.lines() {
            let trimmed = line.trim();
            // Skip empty lines and headings
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            // Skip list markers
            let text = trimmed.trim_start_matches("- ").trim_start_matches("* ");
            if !text.is_empty() {
                if text.len() > max_len {
                    return format!("{}...", &text[..max_len]);
                }
                return text.to_string();
            }
        }
    }
    "_(empty)_".to_string()
}

/// Get current timestamp for index.
fn chrono_timestamp() -> String {
    // Simple timestamp without chrono dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Convert to human-readable (approximation)
    let days = now / 86400;
    let years = 1970 + days / 365;
    let remaining_days = days % 365;
    let months = remaining_days / 30 + 1;
    let day = remaining_days % 30 + 1;

    format!("{}-{:02}-{:02}", years, months, day)
}

/// Update the global/index.md node with the generated index.
pub fn update_index(vault: &Path) -> Result<String> {
    let index_content = generate_index(vault, None)?;
    node::write(vault, INDEX_PATH, &index_content)?;
    Ok(format!("Updated: {}", INDEX_PATH))
}

/// Check if index exists.
pub fn index_exists(vault: &Path) -> bool {
    node::read(vault, INDEX_PATH).is_ok()
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
            "openmem_index_test_{}_{}_{}",
            std::process::id(),
            id,
            nanos
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn generate_index_includes_global_nodes() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "global/user-prefs", "# User Prefs\nPrefers dark mode.").unwrap();
        node::write(&vault, "global/agent-rules", "# Agent Rules\nAlways test code.").unwrap();

        let index = generate_index(&vault, None).unwrap();

        assert!(index.contains("# Memory Index"));
        assert!(index.contains("## Global"));
        assert!(index.contains("[[global/user-prefs]]"));
        assert!(index.contains("[[global/agent-rules]]"));
    }

    #[test]
    fn generate_index_includes_projects() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "projects/acme/goal", "# Goal\nBuild the thing.").unwrap();
        node::write(&vault, "projects/acme/status", "# Status\nIn progress.").unwrap();
        node::write(&vault, "projects/blog/goal", "# Goal\nWrite posts.").unwrap();

        let index = generate_index(&vault, None).unwrap();

        assert!(index.contains("## Projects"));
        assert!(index.contains("### acme"));
        assert!(index.contains("### blog"));
        assert!(index.contains("[[projects/acme/goal]]"));
    }

    #[test]
    fn generate_index_includes_tags() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "node-a", "About #rust and #cli.").unwrap();
        node::write(&vault, "node-b", "More #rust content.").unwrap();

        let index = generate_index(&vault, None).unwrap();

        assert!(index.contains("## Tags"));
        assert!(index.contains("#rust"));
    }

    #[test]
    fn generate_index_includes_orphans() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "orphan-node", "No links here.").unwrap();
        node::write(&vault, "linked-node", "Links to [[other]].").unwrap();

        let index = generate_index(&vault, None).unwrap();

        assert!(index.contains("## Orphans"));
        assert!(index.contains("[[orphan-node]]"));
    }

    #[test]
    fn update_index_writes_node() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        let result = update_index(&vault).unwrap();
        assert_eq!(result, "Updated: global/index");

        let content = node::read(&vault, "global/index").unwrap();
        assert!(content.contains("# Memory Index"));
    }

    #[test]
    fn index_exists_checks() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        assert!(!index_exists(&vault));

        node::write(&vault, "global/index", "Index content.").unwrap();

        assert!(index_exists(&vault));
    }

    #[test]
    fn generate_index_with_scope() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "global/prefs", "Global.").unwrap();
        node::write(&vault, "projects/a/node", "Project A.").unwrap();
        node::write(&vault, "projects/b/node", "Project B.").unwrap();

        let index = generate_index(&vault, Some("projects/a")).unwrap();

        assert!(index.contains("projects/a/node"));
        assert!(!index.contains("projects/b/node"));
    }
}
