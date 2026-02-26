use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{OpenMemError, Result};

/// Read a node's content by exact path (relative to vault root).
/// The path should NOT include the `.md` extension.
pub fn read(vault: &Path, node_path: &str) -> Result<String> {
    let file_path = resolve_path(vault, node_path);
    fs::read_to_string(&file_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            OpenMemError::NodeNotFound(node_path.to_string())
        } else {
            OpenMemError::Io(e)
        }
    })
}

/// Write content to a node. Creates parent dirs if needed.
/// The path should NOT include the `.md` extension.
pub fn write(vault: &Path, node_path: &str, content: &str) -> Result<()> {
    let file_path = resolve_path(vault, node_path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&file_path, content)?;
    Ok(())
}

/// Delete a node by exact path.
pub fn delete(vault: &Path, node_path: &str) -> Result<()> {
    let file_path = resolve_path(vault, node_path);
    if !file_path.exists() {
        return Err(OpenMemError::NodeNotFound(node_path.to_string()));
    }
    fs::remove_file(&file_path)?;
    Ok(())
}

/// List all nodes under a path prefix (recursive).
/// Returns paths relative to vault root, without `.md` extension.
pub fn list(vault: &Path, prefix: &str) -> Result<Vec<String>> {
    let search_dir = if prefix.is_empty() {
        vault.to_path_buf()
    } else {
        vault.join(prefix)
    };

    if !search_dir.exists() {
        return Ok(Vec::new());
    }

    let mut nodes = Vec::new();
    collect_nodes(&search_dir, vault, &mut nodes)?;
    nodes.sort();
    Ok(nodes)
}

/// Recursively collect all `.md` files under a directory.
fn collect_nodes(dir: &Path, vault_root: &Path, nodes: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_nodes(&path, vault_root, nodes)?;
        } else if path.extension().is_some_and(|ext| ext == "md") {
            // Convert to relative path without .md extension
            let relative = path
                .strip_prefix(vault_root)
                .unwrap_or(&path)
                .with_extension("");
            let node_path = relative.to_string_lossy().replace('\\', "/");
            nodes.push(node_path);
        }
    }
    Ok(())
}

/// Convert a node path to an actual file path.
fn resolve_path(vault: &Path, node_path: &str) -> PathBuf {
    vault.join(format!("{}.md", node_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Create a unique temporary vault directory for testing.
    fn temp_vault() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = env::temp_dir().join(format!("openmem_test_{}_{}", std::process::id(), id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ==========================================
    // Memory samples of varying size & complexity
    // ==========================================

    /// Minimal: a one-liner preference
    const SAMPLE_TINY: &str = "Prefers dark mode.";

    /// Small: a few lines with a heading
    const SAMPLE_SMALL: &str = "\
# Coding Style

- Uses 4-space indentation
- Prefers explicit error handling over exceptions
- Variable names in snake_case
";

    /// Medium: structured project knowledge with headings and links
    const SAMPLE_MEDIUM: &str = "\
# Project Acme

## Stack
- Frontend: [[tools/react]] with TypeScript
- Backend: [[tools/rust]] with Actix-web
- Database: PostgreSQL 15

## Conventions
- All API endpoints return JSON
- Authentication via JWT tokens stored in httpOnly cookies
- Error responses follow RFC 7807 Problem Details format

## Known Issues
- The WebSocket handler leaks connections under high load
- See [[projects/acme/migration-plan]] for the database migration strategy
";

    /// Large: a detailed technical document with many sections
    const SAMPLE_LARGE: &str = "\
# Rust Error Handling Patterns

## Overview
This document captures preferred error handling patterns across all Rust projects.
See [[global/coding-standards]] for general coding guidelines.

## Custom Error Types
Always use `thiserror` for library code and `anyhow` for application code.

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error(\"IO error: {0}\")]
    Io(#[from] std::io::Error),
    #[error(\"Parse error at line {line}: {message}\")]
    Parse { line: usize, message: String },
}
```

## Error Propagation
- Use `?` operator consistently
- Never unwrap in library code
- In tests, use `.unwrap()` with a comment explaining why it's safe

## Logging Errors
- Log at `error!` level only for unrecoverable errors
- Log at `warn!` level for recoverable errors that indicate a problem
- Include the full error chain using `{:#}` format

## Related
- [[tools/rust]]
- [[projects/acme/backend]]
- [[global/coding-standards]]
";

    // ==========================================
    // Positive tests: CRUD operations
    // ==========================================

    #[test]
    fn write_and_read_tiny_node() {
        let vault = temp_vault();
        write(&vault, "global/user-prefs", SAMPLE_TINY).unwrap();
        let content = read(&vault, "global/user-prefs").unwrap();
        assert_eq!(content, SAMPLE_TINY);
    }

    #[test]
    fn write_and_read_small_node() {
        let vault = temp_vault();
        write(&vault, "global/coding-style", SAMPLE_SMALL).unwrap();
        let content = read(&vault, "global/coding-style").unwrap();
        assert_eq!(content, SAMPLE_SMALL);
    }

    #[test]
    fn write_and_read_medium_node() {
        let vault = temp_vault();
        write(&vault, "projects/acme/overview", SAMPLE_MEDIUM).unwrap();
        let content = read(&vault, "projects/acme/overview").unwrap();
        assert_eq!(content, SAMPLE_MEDIUM);
    }

    #[test]
    fn write_and_read_large_node() {
        let vault = temp_vault();
        write(&vault, "knowledge/rust-errors", SAMPLE_LARGE).unwrap();
        let content = read(&vault, "knowledge/rust-errors").unwrap();
        assert_eq!(content, SAMPLE_LARGE);
    }

    #[test]
    fn write_creates_parent_dirs() {
        let vault = temp_vault();
        write(&vault, "deep/nested/path/node", "content").unwrap();
        assert!(vault.join("deep/nested/path/node.md").exists());
    }

    #[test]
    fn write_overwrites_existing_node() {
        let vault = temp_vault();
        write(&vault, "global/prefs", "version 1").unwrap();
        write(&vault, "global/prefs", "version 2").unwrap();
        let content = read(&vault, "global/prefs").unwrap();
        assert_eq!(content, "version 2");
    }

    #[test]
    fn delete_removes_node() {
        let vault = temp_vault();
        write(&vault, "temp/to-delete", "delete me").unwrap();
        assert!(read(&vault, "temp/to-delete").is_ok());
        delete(&vault, "temp/to-delete").unwrap();
        assert!(read(&vault, "temp/to-delete").is_err());
    }

    #[test]
    fn list_returns_all_nodes_sorted() {
        let vault = temp_vault();
        write(&vault, "b-node", "b").unwrap();
        write(&vault, "a-node", "a").unwrap();
        write(&vault, "sub/c-node", "c").unwrap();

        let nodes = list(&vault, "").unwrap();
        assert_eq!(nodes, vec!["a-node", "b-node", "sub/c-node"]);
    }

    #[test]
    fn list_with_prefix_filters() {
        let vault = temp_vault();
        write(&vault, "global/prefs", "p").unwrap();
        write(&vault, "global/persona", "q").unwrap();
        write(&vault, "projects/acme/goal", "g").unwrap();

        let global = list(&vault, "global").unwrap();
        assert_eq!(global, vec!["global/persona", "global/prefs"]);

        let projects = list(&vault, "projects").unwrap();
        assert_eq!(projects, vec!["projects/acme/goal"]);
    }

    #[test]
    fn list_nonexistent_prefix_returns_empty() {
        let vault = temp_vault();
        let result = list(&vault, "nonexistent").unwrap();
        assert!(result.is_empty());
    }

    // ==========================================
    // Identity tests: path IS the identity
    // ==========================================

    #[test]
    fn path_identity_is_exact() {
        let vault = temp_vault();
        write(&vault, "global/user-prefs", "content A").unwrap();
        write(&vault, "projects/user-prefs", "content B").unwrap();

        // Same file name, different paths = different nodes
        let a = read(&vault, "global/user-prefs").unwrap();
        let b = read(&vault, "projects/user-prefs").unwrap();
        assert_eq!(a, "content A");
        assert_eq!(b, "content B");
        assert_ne!(a, b);
    }

    #[test]
    fn path_maps_to_filesystem() {
        let vault = temp_vault();
        write(&vault, "projects/acme/frontend", "react stuff").unwrap();

        // The file must exist at the exact filesystem path
        let expected_path = vault.join("projects").join("acme").join("frontend.md");
        assert!(expected_path.exists());
        assert_eq!(fs::read_to_string(&expected_path).unwrap(), "react stuff");
    }

    // ==========================================
    // Negative tests: error handling
    // ==========================================

    #[test]
    fn read_nonexistent_node_returns_not_found() {
        let vault = temp_vault();
        let err = read(&vault, "does/not/exist").unwrap_err();
        assert!(matches!(err, OpenMemError::NodeNotFound(_)));
        assert!(err.to_string().contains("does/not/exist"));
    }

    #[test]
    fn delete_nonexistent_node_returns_not_found() {
        let vault = temp_vault();
        let err = delete(&vault, "ghost").unwrap_err();
        assert!(matches!(err, OpenMemError::NodeNotFound(_)));
        assert!(err.to_string().contains("ghost"));
    }

    #[test]
    fn read_directory_as_node_fails() {
        let vault = temp_vault();
        write(&vault, "projects/acme/goal", "goal").unwrap();
        // "projects/acme" is a directory, not a node
        let result = read(&vault, "projects/acme");
        assert!(result.is_err());
    }

    #[test]
    fn list_empty_vault_returns_empty() {
        let vault = temp_vault();
        let result = list(&vault, "").unwrap();
        assert!(result.is_empty());
    }
}
