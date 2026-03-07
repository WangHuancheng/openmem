use regex::Regex;
use std::path::Path;

use crate::error::Result;
use crate::node;

/// Search result for a single match.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// Node path.
    pub path: String,
    /// Line number (1-indexed).
    pub line: usize,
    /// Matched line content.
    pub content: String,
}

/// Search across all nodes for a query string or regex.
///
/// Returns matches sorted by path, then by line number.
pub fn search(vault: &Path, query: &str, options: SearchOptions) -> Result<Vec<SearchResult>> {
    let nodes = node::list(vault, options.scope.as_deref().unwrap_or(""))?;
    let mut results = Vec::new();

    let pattern = if options.regex {
        // Use query as-is for regex
        query.to_string()
    } else {
        // Escape special regex characters for literal search
        regex::escape(query)
    };

    let re = if options.case_sensitive {
        Regex::new(&pattern)
    } else {
        Regex::new(&format!("(?i){}", pattern))
    };

    let re = re.map_err(|e| crate::error::OpenMemError::VcsError(format!("Invalid pattern: {}", e)))?;

    for node_path in &nodes {
        if let Ok(content) = node::read(vault, node_path) {
            for (line_num, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    results.push(SearchResult {
                        path: node_path.clone(),
                        line: line_num + 1,
                        content: line.to_string(),
                    });

                    if results.len() >= options.max_results {
                        return Ok(results);
                    }
                }
            }
        }
    }

    // Sort by path, then by line number
    results.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.cmp(&b.line))
    });

    Ok(results)
}

/// Search options.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Limit search to nodes under this path prefix.
    pub scope: Option<String>,
    /// Maximum number of results to return.
    pub max_results: usize,
    /// Case-sensitive search.
    pub case_sensitive: bool,
    /// Treat query as regex pattern.
    pub regex: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            scope: None,
            max_results: 20,
            case_sensitive: false,
            regex: false,
        }
    }
}

/// Format search results for display.
pub fn format_results(results: &[SearchResult]) -> String {
    if results.is_empty() {
        return "No matches found.".to_string();
    }

    let mut output = String::new();
    for result in results {
        output.push_str(&format!(
            "{}:{}: {}\n",
            result.path, result.line, result.content
        ));
    }

    // Count unique nodes
    let unique_nodes: std::collections::HashSet<_> = results.iter().map(|r| &r.path).collect();
    output.push_str(&format!(
        "\n{} matches across {} nodes\n",
        results.len(),
        unique_nodes.len()
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
            "openmem_search_test_{}_{}_{}",
            std::process::id(),
            id,
            nanos
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn search_finds_matches() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "node-a", "This has the word apple.\nAnother line.").unwrap();
        node::write(&vault, "node-b", "No fruit here.\nBut apple pie is good.").unwrap();
        node::write(&vault, "node-c", "Nothing relevant.").unwrap();

        let results = search(&vault, "apple", SearchOptions::default()).unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.path == "node-a" && r.line == 1));
        assert!(results.iter().any(|r| r.path == "node-b" && r.line == 2));
    }

    #[test]
    fn search_no_matches() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "node-a", "Some content.").unwrap();

        let results = search(&vault, "nonexistent", SearchOptions::default()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_case_insensitive_by_default() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "node-a", "APPLE in caps.").unwrap();

        let results = search(&vault, "apple", SearchOptions::default()).unwrap();
        assert_eq!(results.len(), 1);

        let results = search(&vault, "APPLE", SearchOptions::default()).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_case_sensitive() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "node-a", "Apple and apple and APPLE.").unwrap();

        let options = SearchOptions {
            case_sensitive: true,
            ..Default::default()
        };
        let results = search(&vault, "apple", options).unwrap();
        assert_eq!(results.len(), 1); // Only lowercase "apple"
    }

    #[test]
    fn search_with_scope() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "global/prefs", "Contains apple.").unwrap();
        node::write(&vault, "projects/a/notes", "Contains apple.").unwrap();
        node::write(&vault, "projects/b/notes", "Contains apple.").unwrap();

        let options = SearchOptions {
            scope: Some("projects/a".to_string()),
            ..Default::default()
        };
        let results = search(&vault, "apple", options).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "projects/a/notes");
    }

    #[test]
    fn search_max_results() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "node-1", "apple").unwrap();
        node::write(&vault, "node-2", "apple").unwrap();
        node::write(&vault, "node-3", "apple").unwrap();

        let options = SearchOptions {
            max_results: 2,
            ..Default::default()
        };
        let results = search(&vault, "apple", options).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_regex_pattern() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "node-a", "test-123\nAlso test-456").unwrap();
        node::write(&vault, "node-b", "no match here").unwrap();

        let options = SearchOptions {
            regex: true,
            ..Default::default()
        };
        let results = search(&vault, r"test-\d+", options).unwrap();
        // Both test-123 and test-456 match, but they're on separate lines
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn format_results_empty() {
        let output = format_results(&[]);
        assert_eq!(output, "No matches found.");
    }

    #[test]
    fn format_results_with_matches() {
        let results = vec![
            SearchResult {
                path: "node-a".to_string(),
                line: 1,
                content: "apple here".to_string(),
            },
            SearchResult {
                path: "node-a".to_string(),
                line: 5,
                content: "another apple".to_string(),
            },
        ];

        let output = format_results(&results);
        assert!(output.contains("node-a:1: apple here"));
        assert!(output.contains("node-a:5: another apple"));
        assert!(output.contains("2 matches across 1 nodes"));
    }
}
