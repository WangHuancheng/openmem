use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{OpenMemError, Result};

/// User configuration for openmem.
#[derive(Debug)]
pub struct Config {
    pub vault: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vault: default_vault_path(),
        }
    }
}

/// Load configuration from `~/.openmem/config.toml`.
/// Returns default config if the file doesn't exist.
pub fn load() -> Result<Config> {
    let config_path = config_file_path();
    if !config_path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(&config_path)?;
    parse_config(&content)
}

/// Resolve the vault root path with the following precedence:
/// 1. CLI flag (if provided)
/// 2. OPENMEM_VAULT environment variable
/// 3. Config file vault setting
/// 4. Default: ~/.openmem/vault/
pub fn vault_root(cli_flag: Option<&Path>) -> Result<PathBuf> {
    // 1. CLI flag takes highest priority
    if let Some(path) = cli_flag {
        return Ok(path.to_path_buf());
    }

    // 2. Environment variable
    if let Ok(env_vault) = env::var("OPENMEM_VAULT") {
        return Ok(PathBuf::from(env_vault));
    }

    // 3. Config file
    let config = load()?;
    Ok(config.vault)
}

/// Parse a TOML config string into a Config.
fn parse_config(content: &str) -> Result<Config> {
    // Minimal TOML parsing — just look for vault = "..."
    // We avoid pulling in the full `toml` + `serde` crates for a single key.
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("vault") {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix('=') {
                let rest = rest.trim();
                // Strip surrounding quotes
                let value = rest.trim_matches('"').trim_matches('\'');
                if value.is_empty() {
                    continue;
                }
                // Expand ~ to home directory
                let path = if value.starts_with("~/") || value == "~" {
                    if let Some(home) = dirs::home_dir() {
                        home.join(&value[2..])
                    } else {
                        PathBuf::from(value)
                    }
                } else {
                    PathBuf::from(value)
                };
                return Ok(Config { vault: path });
            }
        }
    }

    // No vault key found — use default
    Ok(Config::default())
}

/// Default vault path: ~/.openmem/vault/
fn default_vault_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openmem")
        .join("vault")
}

/// Config file path: ~/.openmem/config.toml
fn config_file_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openmem")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================
    // parse_config tests
    // ==========================================

    #[test]
    fn parse_config_with_vault_path() {
        let content = r#"vault = "/custom/vault/path""#;
        let config = parse_config(content).unwrap();
        assert_eq!(config.vault, PathBuf::from("/custom/vault/path"));
    }

    #[test]
    fn parse_config_with_tilde_expansion() {
        let content = r#"vault = "~/my-vault""#;
        let config = parse_config(content).unwrap();
        if let Some(home) = dirs::home_dir() {
            assert_eq!(config.vault, home.join("my-vault"));
        }
    }

    #[test]
    fn parse_config_with_comments_and_whitespace() {
        let content = "\
# This is a config file
  
vault = \"/custom/path\"

# Another comment
";
        let config = parse_config(content).unwrap();
        assert_eq!(config.vault, PathBuf::from("/custom/path"));
    }

    #[test]
    fn parse_config_empty_returns_default() {
        let content = "";
        let config = parse_config(content).unwrap();
        assert_eq!(config.vault, default_vault_path());
    }

    #[test]
    fn parse_config_no_vault_key_returns_default() {
        let content = "other_key = \"value\"";
        let config = parse_config(content).unwrap();
        assert_eq!(config.vault, default_vault_path());
    }

    #[test]
    fn default_config_uses_home_dir() {
        let config = Config::default();
        if let Some(home) = dirs::home_dir() {
            assert_eq!(config.vault, home.join(".openmem").join("vault"));
        }
    }

    // ==========================================
    // vault_root resolution tests
    // ==========================================

    #[test]
    fn vault_root_cli_flag_overrides_everything() {
        let flag = PathBuf::from("/cli/vault");
        let result = vault_root(Some(&flag)).unwrap();
        assert_eq!(result, PathBuf::from("/cli/vault"));
    }

    #[test]
    fn vault_root_no_flag_uses_config_or_default() {
        // Clear env var to test fallback (unsafe in Rust 2024)
        unsafe {
            env::remove_var("OPENMEM_VAULT");
        }
        let result = vault_root(None).unwrap();
        // Should be either from config or default — just ensure it doesn't error
        assert!(!result.as_os_str().is_empty());
    }
}
