use std::path::Path;

use regex::Regex;

use crate::error::Result;
use crate::node;

/// Extract all `[[path/to/node]]` link targets from markdown content.
/// Returns a deduplicated, sorted list of link targets.
pub fn parse_links(content: &str) -> Vec<String> {
    let re = Regex::new(r"\[\[(.+?)\]\]").unwrap();
    let mut links: Vec<String> = re
        .captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect();
    links.sort();
    links.dedup();
    links
}

/// Find all nodes in the vault that contain `[[node_path]]` links to the given node.
/// Returns a sorted list of node paths that link to `node_path`.
pub fn backlinks(vault: &Path, node_path: &str) -> Result<Vec<String>> {
    let all_nodes = node::list(vault, "")?;
    let target = format!("[[{}]]", node_path);
    let mut results = Vec::new();

    for np in &all_nodes {
        // Don't count self-references as backlinks
        if np == node_path {
            continue;
        }
        if let Ok(content) = node::read(vault, np) {
            if content.contains(&target) {
                results.push(np.clone());
            }
        }
    }

    results.sort();
    Ok(results)
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
            "openmem_link_test_{}_{}_{}",
            std::process::id(),
            id,
            nanos
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ==========================================
    // parse_links tests
    // ==========================================

    #[test]
    fn parse_single_link() {
        let content = "This references [[global/user-prefs]].";
        let links = parse_links(content);
        assert_eq!(links, vec!["global/user-prefs"]);
    }

    #[test]
    fn parse_multiple_links() {
        let content = "Uses [[tools/react]] and [[tools/rust]] with [[global/coding-standards]].";
        let links = parse_links(content);
        assert_eq!(
            links,
            vec!["global/coding-standards", "tools/react", "tools/rust"]
        );
    }

    #[test]
    fn parse_no_links() {
        let content = "No links here. Just plain text.\n\nWith multiple paragraphs.";
        let links = parse_links(content);
        assert!(links.is_empty());
    }

    #[test]
    fn parse_duplicate_links_deduplicated() {
        let content = "See [[tools/react]] and also [[tools/react]] again.";
        let links = parse_links(content);
        assert_eq!(links, vec!["tools/react"]);
    }

    #[test]
    fn parse_nested_path_link() {
        let content = "Check [[projects/acme/frontend/components]].";
        let links = parse_links(content);
        assert_eq!(links, vec!["projects/acme/frontend/components"]);
    }

    #[test]
    fn parse_links_in_multiline_content() {
        let content = "\
# Project Acme

## Stack
- Frontend: [[tools/react]] with TypeScript
- Backend: [[tools/rust]] with Actix-web

## Related
- See [[projects/acme/migration-plan]] for details.
";
        let links = parse_links(content);
        assert_eq!(
            links,
            vec!["projects/acme/migration-plan", "tools/react", "tools/rust"]
        );
    }

    // ==========================================
    // backlinks tests
    // ==========================================

    #[test]
    fn backlinks_finds_linking_nodes() {
        let vault = temp_vault();
        node::write(&vault, "tools/react", "# React\nA frontend framework.").unwrap();
        node::write(
            &vault,
            "projects/acme/frontend",
            "Uses [[tools/react]] and TypeScript.",
        )
        .unwrap();
        node::write(
            &vault,
            "projects/blog/frontend",
            "Also uses [[tools/react]].",
        )
        .unwrap();
        node::write(&vault, "projects/acme/backend", "Uses [[tools/rust]].").unwrap();

        let result = backlinks(&vault, "tools/react").unwrap();
        assert_eq!(
            result,
            vec!["projects/acme/frontend", "projects/blog/frontend"]
        );
    }

    #[test]
    fn backlinks_no_incoming_links_returns_empty() {
        let vault = temp_vault();
        node::write(&vault, "isolated-node", "No one links to me.").unwrap();
        node::write(&vault, "other-node", "Just some text.").unwrap();

        let result = backlinks(&vault, "isolated-node").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn backlinks_excludes_self_reference() {
        let vault = temp_vault();
        node::write(
            &vault,
            "self-ref",
            "I reference myself: [[self-ref]]. Weird.",
        )
        .unwrap();

        let result = backlinks(&vault, "self-ref").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn backlinks_empty_vault_returns_empty() {
        let vault = temp_vault();
        let result = backlinks(&vault, "nonexistent").unwrap();
        assert!(result.is_empty());
    }
}
