use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::error::Result;
use crate::link;
use crate::node;

/// Statistics about a vault.
#[derive(Debug, Clone)]
pub struct VaultStats {
    /// Total number of nodes.
    pub node_count: usize,
    /// Number of unique link targets referenced across all nodes.
    pub unique_link_targets: usize,
    /// Number of orphan nodes (no incoming or outgoing links).
    pub orphan_count: usize,
}

/// Information about a single node in the survey.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// Node path.
    pub path: String,
    /// Number of outgoing [[links]] from this node.
    pub outgoing_links: usize,
    /// Number of backlinks pointing to this node.
    pub backlinks: usize,
    /// Whether this node is an orphan (no incoming or outgoing links).
    pub is_orphan: bool,
}

/// Complete survey result.
#[derive(Debug, Clone)]
pub struct Survey {
    /// Vault statistics.
    pub stats: VaultStats,
    /// Information about each node, sorted by path.
    pub nodes: Vec<NodeInfo>,
    /// List of orphan node paths.
    pub orphans: Vec<String>,
    /// Nodes with many backlinks (potential hubs), sorted by backlink count descending.
    pub dense_hubs: Vec<(String, usize)>,
    /// Directory prefixes with few nodes (potential sparse areas).
    pub sparse_areas: Vec<(String, usize)>,
}

/// Survey the vault and produce a snapshot report.
///
/// This analyzes the vault structure:
/// - Counts nodes and links
/// - Identifies orphan nodes (no incoming or outgoing links)
/// - Identifies dense hubs (nodes with many backlinks)
/// - Identifies sparse areas (directories with few nodes)
pub fn survey(vault: &Path, scope: Option<&str>) -> Result<Survey> {
    let prefix = scope.unwrap_or("");
    let all_nodes = node::list(vault, prefix)?;

    // Collect all outgoing links and backlinks for each node
    let mut outgoing_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut backlink_count: HashMap<String, usize> = HashMap::new();
    let mut unique_targets: HashSet<String> = HashSet::new();

    // Initialize backlink counts for all nodes
    for path in &all_nodes {
        backlink_count.insert(path.clone(), 0);
    }

    // Process each node
    for path in &all_nodes {
        let content = node::read(vault, path).unwrap_or_default();
        let outgoing = link::parse_links(&content);
        unique_targets.extend(outgoing.clone());

        // Update backlink counts for each target
        for target in &outgoing {
            if let Some(count) = backlink_count.get_mut(target) {
                *count += 1;
            }
        }

        outgoing_map.insert(path.clone(), outgoing);
    }

    // Build node info list
    let mut nodes: Vec<NodeInfo> = Vec::new();
    let mut orphans: Vec<String> = Vec::new();

    for path in &all_nodes {
        let outgoing = outgoing_map.get(path).map(|v| v.len()).unwrap_or(0);
        let backlinks = *backlink_count.get(path).unwrap_or(&0);
        let is_orphan = outgoing == 0 && backlinks == 0;

        if is_orphan {
            orphans.push(path.clone());
        }

        nodes.push(NodeInfo {
            path: path.clone(),
            outgoing_links: outgoing,
            backlinks,
            is_orphan,
        });
    }

    // Sort nodes by path
    nodes.sort_by(|a, b| a.path.cmp(&b.path));

    // Find dense hubs (nodes with many backlinks, threshold: >= 5)
    let mut dense_hubs: Vec<(String, usize)> = nodes
        .iter()
        .filter(|n| n.backlinks >= 5)
        .map(|n| (n.path.clone(), n.backlinks))
        .collect();
    dense_hubs.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending

    // Find sparse areas (directories with <= 2 nodes)
    let mut dir_counts: HashMap<String, usize> = HashMap::new();
    for path in &all_nodes {
        // Get the directory prefix (everything before the last /)
        if let Some(pos) = path.rfind('/') {
            let dir = &path[..pos];
            *dir_counts.entry(dir.to_string()).or_insert(0) += 1;
        } else {
            // Root-level node
            *dir_counts.entry("".to_string()).or_insert(0) += 1;
        }
    }

    let mut sparse_areas: Vec<(String, usize)> = dir_counts
        .into_iter()
        .filter(|(_, count)| *count <= 2)
        .map(|(dir, count)| (dir, count))
        .collect();
    sparse_areas.sort_by(|a, b| a.0.cmp(&b.0));

    let stats = VaultStats {
        node_count: all_nodes.len(),
        unique_link_targets: unique_targets.len(),
        orphan_count: orphans.len(),
    };

    Ok(Survey {
        stats,
        nodes,
        orphans,
        dense_hubs,
        sparse_areas,
    })
}

/// Format a survey as a markdown report.
pub fn format_report(survey: &Survey) -> String {
    let mut output = String::new();

    output.push_str("# Vault Survey\n\n");

    // Stats section
    output.push_str("## Stats\n");
    output.push_str(&format!("- {} nodes\n", survey.stats.node_count));
    output.push_str(&format!(
        "- {} unique [[link]] targets\n",
        survey.stats.unique_link_targets
    ));
    output.push_str(&format!(
        "- {} orphan nodes (no incoming or outgoing links)\n",
        survey.stats.orphan_count
    ));
    output.push('\n');

    // Node index
    output.push_str("## Node Index\n");
    for node in &survey.nodes {
        let orphan_marker = if node.is_orphan { " ← ORPHAN" } else { "" };
        let hub_marker = if node.backlinks >= 5 {
            " ← heavily referenced"
        } else {
            ""
        };
        output.push_str(&format!(
            "- {} ({} outgoing, {} backlinks){}{}\n",
            node.path, node.outgoing_links, node.backlinks, orphan_marker, hub_marker
        ));
    }
    output.push('\n');

    // Potential issues
    output.push_str("## Potential Issues\n");

    if !survey.orphans.is_empty() {
        output.push_str(&format!(
            "- Orphan nodes: {}\n",
            survey.orphans.join(", ")
        ));
    }

    if !survey.dense_hubs.is_empty() {
        let hubs: Vec<String> = survey
            .dense_hubs
            .iter()
            .map(|(path, count)| format!("{} ({} backlinks)", path, count))
            .collect();
        output.push_str(&format!(
            "- Dense hubs: {} — consider if it should be split\n",
            hubs.join(", ")
        ));
    }

    if !survey.sparse_areas.is_empty() {
        let areas: Vec<String> = survey
            .sparse_areas
            .iter()
            .filter(|(dir, _)| !dir.is_empty())
            .map(|(dir, count)| format!("{} ({} nodes)", dir, count))
            .collect();
        if !areas.is_empty() {
            output.push_str(&format!(
                "- Sparse areas: {}\n",
                areas.join(", ")
            ));
        }
    }

    if survey.orphans.is_empty()
        && survey.dense_hubs.is_empty()
        && survey.sparse_areas.iter().all(|(d, _)| d.is_empty())
    {
        output.push_str("- No issues detected\n");
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
            "openmem_survey_test_{}_{}_{}",
            std::process::id(),
            id,
            nanos
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn survey_empty_vault() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        let result = survey(&vault, None).unwrap();
        assert_eq!(result.stats.node_count, 0);
        assert_eq!(result.stats.unique_link_targets, 0);
        assert_eq!(result.stats.orphan_count, 0);
        assert!(result.nodes.is_empty());
        assert!(result.orphans.is_empty());
    }

    #[test]
    fn survey_single_node_no_links() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "isolated", "No links here.").unwrap();

        let result = survey(&vault, None).unwrap();
        assert_eq!(result.stats.node_count, 1);
        assert_eq!(result.stats.orphan_count, 1);
        assert_eq!(result.orphans, vec!["isolated"]);
    }

    #[test]
    fn survey_linked_nodes() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "tools/react", "# React\nA framework.").unwrap();
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

        let result = survey(&vault, None).unwrap();
        assert_eq!(result.stats.node_count, 3);
        assert_eq!(result.stats.unique_link_targets, 1); // Only tools/react

        // Find tools/react node
        let react_node = result.nodes.iter().find(|n| n.path == "tools/react").unwrap();
        assert_eq!(react_node.backlinks, 2);
        assert_eq!(react_node.outgoing_links, 0);
        assert!(!react_node.is_orphan);

        // Find project nodes
        let acme_node = result
            .nodes
            .iter()
            .find(|n| n.path == "projects/acme/frontend")
            .unwrap();
        assert_eq!(acme_node.outgoing_links, 1);
        assert_eq!(acme_node.backlinks, 0);
        assert!(!acme_node.is_orphan);
    }

    #[test]
    fn survey_identifies_orphans() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "linked-a", "Links to [[linked-b]].").unwrap();
        node::write(&vault, "linked-b", "Referenced by a.").unwrap();
        node::write(&vault, "orphan-1", "No links.").unwrap();
        node::write(&vault, "orphan-2", "Also no links.").unwrap();

        let result = survey(&vault, None).unwrap();
        assert_eq!(result.stats.orphan_count, 2);

        let orphan_paths: Vec<&str> = result.orphans.iter().map(|s| s.as_str()).collect();
        assert!(orphan_paths.contains(&"orphan-1"));
        assert!(orphan_paths.contains(&"orphan-2"));
    }

    #[test]
    fn survey_identifies_dense_hubs() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        // Create a hub node
        node::write(&vault, "hub", "Important node.").unwrap();

        // Create many nodes that link to the hub
        for i in 0..6 {
            node::write(
                &vault,
                &format!("node-{}", i),
                &format!("References [[hub]]."),
            )
            .unwrap();
        }

        let result = survey(&vault, None).unwrap();
        assert_eq!(result.dense_hubs.len(), 1);
        assert_eq!(result.dense_hubs[0], ("hub".to_string(), 6));
    }

    #[test]
    fn survey_with_scope_prefix() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "global/prefs", "Global settings.").unwrap();
        node::write(&vault, "projects/a/goal", "Project A goal.").unwrap();
        node::write(&vault, "projects/b/goal", "Project B goal.").unwrap();

        let result = survey(&vault, Some("projects")).unwrap();
        assert_eq!(result.stats.node_count, 2);

        let paths: Vec<&str> = result.nodes.iter().map(|n| n.path.as_str()).collect();
        assert!(paths.contains(&"projects/a/goal"));
        assert!(paths.contains(&"projects/b/goal"));
        assert!(!paths.contains(&"global/prefs"));
    }

    #[test]
    fn format_report_produces_markdown() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "tools/react", "# React").unwrap();
        node::write(
            &vault,
            "projects/acme",
            "Uses [[tools/react]].",
        )
        .unwrap();

        let survey_result = survey(&vault, None).unwrap();
        let report = format_report(&survey_result);

        assert!(report.contains("# Vault Survey"));
        assert!(report.contains("## Stats"));
        assert!(report.contains("## Node Index"));
        assert!(report.contains("tools/react"));
        assert!(report.contains("projects/acme"));
    }
}
