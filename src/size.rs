use std::path::Path;

use crate::error::Result;
use crate::node;

/// Node size information.
#[derive(Debug, Clone)]
pub struct NodeSize {
    /// Node path.
    pub path: String,
    /// Content size in bytes.
    pub bytes: usize,
    /// Number of lines.
    pub lines: usize,
    /// Estimated tokens (rough: chars / 4).
    pub tokens: usize,
}

/// Get size information for a node.
pub fn node_size(vault: &Path, node_path: &str) -> Result<NodeSize> {
    let content = node::read(vault, node_path)?;
    Ok(NodeSize {
        path: node_path.to_string(),
        bytes: content.len(),
        lines: content.lines().count(),
        tokens: content.chars().count() / 4,
    })
}

/// Get size information for all nodes.
pub fn all_sizes(vault: &Path, prefix: &str) -> Result<Vec<NodeSize>> {
    let nodes = node::list(vault, prefix)?;
    let mut sizes = Vec::new();

    for node_path in &nodes {
        if let Ok(size) = node_size(vault, node_path) {
            sizes.push(size);
        }
    }

    // Sort by size descending
    sizes.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    Ok(sizes)
}

/// Format size with appropriate unit.
pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Size threshold constants.
pub const SMALL_THRESHOLD: usize = 500;    // ~125 tokens
pub const MEDIUM_THRESHOLD: usize = 2000;  // ~500 tokens
pub const LARGE_THRESHOLD: usize = 8000;   // ~2000 tokens

/// Categorize node by size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeCategory {
    Tiny,    // < 500 bytes
    Small,   // 500-2000 bytes
    Medium,  // 2000-8000 bytes
    Large,   // > 8000 bytes
}

impl SizeCategory {
    pub fn from_bytes(bytes: usize) -> Self {
        if bytes < SMALL_THRESHOLD {
            SizeCategory::Tiny
        } else if bytes < MEDIUM_THRESHOLD {
            SizeCategory::Small
        } else if bytes < LARGE_THRESHOLD {
            SizeCategory::Medium
        } else {
            SizeCategory::Large
        }
    }

    pub fn marker(&self) -> &'static str {
        match self {
            SizeCategory::Tiny => "",
            SizeCategory::Small => " [S]",
            SizeCategory::Medium => " [M]",
            SizeCategory::Large => " [L]",
        }
    }
}

/// Vault statistics.
#[derive(Debug, Clone)]
pub struct VaultStats {
    /// Total number of nodes.
    pub node_count: usize,
    /// Total bytes.
    pub total_bytes: usize,
    /// Total estimated tokens.
    pub total_tokens: usize,
    /// Largest node path.
    pub largest_node: Option<String>,
    /// Largest node bytes.
    pub largest_bytes: usize,
    /// Number of large nodes.
    pub large_count: usize,
    /// Number of medium nodes.
    pub medium_count: usize,
    /// Number of small nodes.
    pub small_count: usize,
    /// Number of tiny nodes.
    pub tiny_count: usize,
}

/// Calculate vault statistics.
pub fn vault_stats(vault: &Path, prefix: &str) -> Result<VaultStats> {
    let sizes = all_sizes(vault, prefix)?;

    let node_count = sizes.len();
    let total_bytes: usize = sizes.iter().map(|s| s.bytes).sum();
    let total_tokens: usize = sizes.iter().map(|s| s.tokens).sum();

    let (largest_node, largest_bytes) = sizes
        .first()
        .map(|s| (Some(s.path.clone()), s.bytes))
        .unwrap_or((None, 0));

    let large_count = sizes.iter().filter(|s| s.bytes >= LARGE_THRESHOLD).count();
    let medium_count = sizes
        .iter()
        .filter(|s| s.bytes >= MEDIUM_THRESHOLD && s.bytes < LARGE_THRESHOLD)
        .count();
    let small_count = sizes
        .iter()
        .filter(|s| s.bytes >= SMALL_THRESHOLD && s.bytes < MEDIUM_THRESHOLD)
        .count();
    let tiny_count = sizes.iter().filter(|s| s.bytes < SMALL_THRESHOLD).count();

    Ok(VaultStats {
        node_count,
        total_bytes,
        total_tokens,
        largest_node,
        largest_bytes,
        large_count,
        medium_count,
        small_count,
        tiny_count,
    })
}

/// Format vault stats for display.
pub fn format_stats(stats: &VaultStats) -> String {
    let mut output = String::new();

    output.push_str("# Vault Statistics\n\n");

    output.push_str("## Overview\n");
    output.push_str(&format!("- Nodes: {}\n", stats.node_count));
    output.push_str(&format!("- Total size: {}\n", format_size(stats.total_bytes)));
    output.push_str(&format!("- Estimated tokens: ~{}\n\n", stats.total_tokens));

    if let Some(ref largest) = stats.largest_node {
        output.push_str("## Largest Node\n");
        output.push_str(&format!("- [[{}]] ({})\n\n", largest, format_size(stats.largest_bytes)));
    }

    output.push_str("## Size Distribution\n");
    output.push_str(&format!("- Tiny (<{}): {}\n", SMALL_THRESHOLD, stats.tiny_count));
    output.push_str(&format!(
        "- Small ({}-{}): {}\n",
        SMALL_THRESHOLD, MEDIUM_THRESHOLD, stats.small_count
    ));
    output.push_str(&format!(
        "- Medium ({}-{}): {}\n",
        MEDIUM_THRESHOLD, LARGE_THRESHOLD, stats.medium_count
    ));
    output.push_str(&format!(
        "- Large (>{}): {}\n",
        LARGE_THRESHOLD, stats.large_count
    ));

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
            "openmem_size_test_{}_{}_{}",
            std::process::id(),
            id,
            nanos
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn node_size_calculates() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "test", "Hello\nWorld\n").unwrap();

        let size = node_size(&vault, "test").unwrap();
        assert_eq!(size.bytes, 12); // "Hello\nWorld\n"
        assert_eq!(size.lines, 2);
    }

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(500), "500B");
        }

    #[test]
    fn format_size_kb() {
        assert_eq!(format_size(2048), "2.0KB");
    }

    #[test]
    fn size_category_tiny() {
        assert_eq!(SizeCategory::from_bytes(100), SizeCategory::Tiny);
    }

    #[test]
    fn size_category_small() {
        assert_eq!(SizeCategory::from_bytes(1000), SizeCategory::Small);
    }

    #[test]
    fn size_category_medium() {
        assert_eq!(SizeCategory::from_bytes(4000), SizeCategory::Medium);
    }

    #[test]
    fn size_category_large() {
        assert_eq!(SizeCategory::from_bytes(10000), SizeCategory::Large);
    }

    #[test]
    fn vault_stats_calculates() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "tiny", &"x".repeat(100)).unwrap();
        node::write(&vault, "small", &"x".repeat(1000)).unwrap();
        node::write(&vault, "medium", &"x".repeat(4000)).unwrap();
        node::write(&vault, "large", &"x".repeat(10000)).unwrap();

        let stats = vault_stats(&vault, "").unwrap();

        assert_eq!(stats.node_count, 4);
        assert_eq!(stats.tiny_count, 1);
        assert_eq!(stats.small_count, 1);
        assert_eq!(stats.medium_count, 1);
        assert_eq!(stats.large_count, 1);
        assert_eq!(stats.largest_node, Some("large".to_string()));
    }

    #[test]
    fn format_stats_output() {
        let stats = VaultStats {
            node_count: 10,
            total_bytes: 50000,
            total_tokens: 12500,
            largest_node: Some("big-node".to_string()),
            largest_bytes: 10000,
            large_count: 1,
            medium_count: 2,
            small_count: 3,
            tiny_count: 4,
        };

        let output = format_stats(&stats);

        assert!(output.contains("# Vault Statistics"));
        assert!(output.contains("Nodes: 10"));
        assert!(output.contains("big-node"));
    }
}
