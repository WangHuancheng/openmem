use std::path::Path;

use crate::error::Result;
use crate::vcs;

/// Initialize a new openmem vault at the given path.
/// Creates the directory structure and initializes a jj repository.
pub fn init(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    vcs::init(path)?;
    Ok(())
}

/// Ensure a vault exists at the given path.
/// If it doesn't exist, creates it and initializes jj.
/// Returns the vault path.
pub fn ensure(path: &Path) -> Result<std::path::PathBuf> {
    if !path.exists() {
        init(path)?;
    } else if !path.join(".jj").exists() {
        // Directory exists but not a jj repo — initialize
        vcs::init(path)?;
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::SystemTime;

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_path() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!(
            "openmem_vault_test_{}_{}_{}",
            std::process::id(),
            id,
            nanos
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    // ==========================================
    // Positive tests
    // ==========================================

    #[test]
    fn init_creates_vault_directory() {
        let path = temp_path();
        init(&path).unwrap();
        assert!(path.exists());
        assert!(path.is_dir());
    }

    #[test]
    fn init_creates_jj_repo() {
        let path = temp_path();
        init(&path).unwrap();
        assert!(path.join(".jj").exists());
    }

    #[test]
    fn init_on_existing_vault_is_idempotent() {
        let path = temp_path();
        init(&path).unwrap();
        // Second init should not panic (may error on jj re-init, that's ok)
        let _ = init(&path);
        assert!(path.exists());
        assert!(path.join(".jj").exists());
    }

    #[test]
    fn ensure_creates_vault_if_missing() {
        let path = temp_path();
        assert!(!path.exists());
        let result = ensure(&path).unwrap();
        assert_eq!(result, path);
        assert!(path.exists());
        assert!(path.join(".jj").exists());
    }

    #[test]
    fn ensure_initializes_jj_if_dir_exists_without_jj() {
        let path = temp_path();
        fs::create_dir_all(&path).unwrap();
        assert!(!path.join(".jj").exists());
        ensure(&path).unwrap();
        assert!(path.join(".jj").exists());
    }

    #[test]
    fn ensure_is_noop_on_existing_vault() {
        let path = temp_path();
        init(&path).unwrap();
        // Should succeed without re-initializing
        let result = ensure(&path).unwrap();
        assert_eq!(result, path);
    }
}
