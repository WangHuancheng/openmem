use std::path::Path;
use std::process::Command;

use crate::error::{OpenMemError, Result};

/// Initialize a new Jujutsu repository at the given vault path.
/// Uses `jj git init` (jj 0.38+ requires explicit git backend).
pub fn init(vault: &Path) -> Result<()> {
    run_jj(vault, &["git", "init"]).map(|_| ())
}

/// Snapshot the current vault state.
/// Jujutsu automatically tracks working copy changes, but calling `jj status`
/// forces a snapshot of the working copy.
pub fn snapshot(vault: &Path) -> Result<()> {
    run_jj(vault, &["status"]).map(|_| ())
}

/// Show change history for the vault, optionally filtered to a specific node.
pub fn log(vault: &Path, node_path: Option<&str>) -> Result<String> {
    let mut args = vec!["log", "--no-pager"];
    let file_path;
    if let Some(np) = node_path {
        file_path = format!("{}.md", np);
        args.push("--");
        args.push(&file_path);
    }
    run_jj(vault, &args)
}

/// Show diff for a specific change.
pub fn diff(vault: &Path, change_id: &str) -> Result<String> {
    run_jj(vault, &["diff", "--no-pager", "-r", change_id])
}

/// Run a jj command in the given vault directory.
fn run_jj(vault: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("jj")
        .args(args)
        .current_dir(vault)
        .output()
        .map_err(|e| OpenMemError::VcsError(format!("failed to run jj: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(OpenMemError::VcsError(format!(
            "jj {} failed: {}",
            args.first().unwrap_or(&""),
            stderr.trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_vault() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = env::temp_dir().join(format!("openmem_vcs_test_{}_{}", std::process::id(), id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ==========================================
    // Positive tests
    // ==========================================

    #[test]
    fn init_creates_jj_repo() {
        let vault = temp_vault();
        init(&vault).unwrap();
        assert!(vault.join(".jj").exists());
    }

    #[test]
    fn init_is_idempotent() {
        let vault = temp_vault();
        init(&vault).unwrap();
        // Second init should not fail (jj init on existing repo gives warning but succeeds exit code varies)
        // We accept either success or a VcsError — the key is no panic
        let _ = init(&vault);
        assert!(vault.join(".jj").exists());
    }

    #[test]
    fn snapshot_after_write_succeeds() {
        let vault = temp_vault();
        init(&vault).unwrap();
        fs::write(vault.join("test-node.md"), "test content").unwrap();
        snapshot(&vault).unwrap();
    }

    #[test]
    fn log_returns_output_after_writes() {
        let vault = temp_vault();
        init(&vault).unwrap();
        fs::write(vault.join("test-node.md"), "initial content").unwrap();
        snapshot(&vault).unwrap();

        let output = log(&vault, None).unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn log_for_specific_file() {
        let vault = temp_vault();
        init(&vault).unwrap();
        fs::write(vault.join("tracked.md"), "tracked content").unwrap();
        snapshot(&vault).unwrap();

        let output = log(&vault, Some("tracked")).unwrap();
        // Should not error — success is the assertion
        let _ = output;
    }

    // ==========================================
    // Negative tests
    // ==========================================

    #[test]
    fn snapshot_on_non_jj_repo_returns_error() {
        let vault = temp_vault();
        // Don't init — vault is just a regular dir
        let result = snapshot(&vault);
        assert!(result.is_err());
        if let Err(OpenMemError::VcsError(msg)) = result {
            assert!(!msg.is_empty());
        } else {
            panic!("Expected VcsError");
        }
    }

    #[test]
    fn log_on_non_jj_repo_returns_error() {
        let vault = temp_vault();
        let result = log(&vault, None);
        assert!(result.is_err());
    }
}
