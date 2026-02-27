use std::io::Read;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Openmem — universal agent memory system
#[derive(Parser, Debug)]
#[command(name = "openmem", version, about = "Universal agent memory system")]
pub struct Cli {
    /// Vault root directory (overrides config and env var)
    #[arg(long, global = true)]
    pub vault: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Read a node's content
    Read {
        /// Node path (e.g. global/user-prefs)
        path: String,
    },
    /// Write a node (reads content from stdin)
    Write {
        /// Node path (e.g. global/user-prefs)
        path: String,
    },
    /// List nodes under a path
    List {
        /// Path prefix to list under (default: all)
        path: Option<String>,
    },
    /// Show outgoing links and backlinks for a node
    Links {
        /// Node path
        path: String,
    },
    /// Show heading outline of a node
    Outline {
        /// Node path
        path: String,
    },
    /// Delete a node
    Delete {
        /// Node path
        path: String,
    },
    /// Show change history
    Log {
        /// Optional node path to filter history
        path: Option<String>,
    },
    /// Initialize a new vault
    Init,
}

/// Execute the CLI command and return the output string.
/// Separating execution from printing makes this testable.
pub fn execute(cli: &Cli) -> crate::error::Result<String> {
    let vault_path = crate::config::vault_root(cli.vault.as_deref())?;

    match &cli.command {
        Commands::Init => {
            crate::vault::init(&vault_path)?;
            Ok(format!("Initialized vault at {}", vault_path.display()))
        }
        Commands::Read { path } => {
            let vault = crate::vault::ensure(&vault_path)?;
            // If path contains #, read just that heading section
            let content = crate::node::read_section(&vault, path)?;
            Ok(content)
        }
        Commands::Write { path } => {
            let vault = crate::vault::ensure(&vault_path)?;
            let mut content = String::new();
            std::io::stdin()
                .read_to_string(&mut content)
                .map_err(|e| crate::error::OpenMemError::Io(e))?;
            crate::node::write(&vault, path, &content)?;
            crate::vcs::snapshot(&vault)?;
            Ok(format!("Written: {}", path))
        }
        Commands::List { path } => {
            let vault = crate::vault::ensure(&vault_path)?;
            let prefix = path.as_deref().unwrap_or("");
            let nodes = crate::node::list(&vault, prefix)?;
            if nodes.is_empty() {
                Ok("(no nodes)".to_string())
            } else {
                Ok(nodes.join("\n"))
            }
        }
        Commands::Links { path } => {
            let vault = crate::vault::ensure(&vault_path)?;
            let content = crate::node::read(&vault, path)?;
            let outgoing = crate::link::parse_links(&content);
            let incoming = crate::link::backlinks(&vault, path)?;

            let mut output = String::new();
            output.push_str("Outgoing links:\n");
            if outgoing.is_empty() {
                output.push_str("  (none)\n");
            } else {
                for link in &outgoing {
                    output.push_str(&format!("  → {}\n", link));
                }
            }
            output.push_str("Backlinks:\n");
            if incoming.is_empty() {
                output.push_str("  (none)\n");
            } else {
                for link in &incoming {
                    output.push_str(&format!("  ← {}\n", link));
                }
            }
            Ok(output)
        }
        Commands::Outline { path } => {
            let vault = crate::vault::ensure(&vault_path)?;
            let items = crate::node::outline(&vault, path)?;
            let mut output = String::new();
            for (level, name) in &items {
                let indent = "  ".repeat(*level as usize);
                output.push_str(&format!("{}{}\n", indent, name));
            }
            Ok(output)
        }
        Commands::Delete { path } => {
            let vault = crate::vault::ensure(&vault_path)?;
            crate::node::delete(&vault, path)?;
            crate::vcs::snapshot(&vault)?;
            Ok(format!("Deleted: {}", path))
        }
        Commands::Log { path } => {
            let vault = crate::vault::ensure(&vault_path)?;
            let output = crate::vcs::log(&vault, path.as_deref())?;
            Ok(output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // ==========================================
    // CLI parsing tests
    // ==========================================

    #[test]
    fn parse_read_command() {
        let cli = Cli::parse_from(["openmem", "read", "global/user-prefs"]);
        assert!(matches!(cli.command, Commands::Read { path } if path == "global/user-prefs"));
        assert!(cli.vault.is_none());
    }

    #[test]
    fn parse_write_command() {
        let cli = Cli::parse_from(["openmem", "write", "global/user-prefs"]);
        assert!(matches!(cli.command, Commands::Write { path } if path == "global/user-prefs"));
    }

    #[test]
    fn parse_list_command_with_path() {
        let cli = Cli::parse_from(["openmem", "list", "projects"]);
        assert!(
            matches!(cli.command, Commands::List { path } if path == Some("projects".to_string()))
        );
    }

    #[test]
    fn parse_list_command_without_path() {
        let cli = Cli::parse_from(["openmem", "list"]);
        assert!(matches!(cli.command, Commands::List { path } if path.is_none()));
    }

    #[test]
    fn parse_links_command() {
        let cli = Cli::parse_from(["openmem", "links", "projects/acme"]);
        assert!(matches!(cli.command, Commands::Links { path } if path == "projects/acme"));
    }

    #[test]
    fn parse_delete_command() {
        let cli = Cli::parse_from(["openmem", "delete", "temp/old"]);
        assert!(matches!(cli.command, Commands::Delete { path } if path == "temp/old"));
    }

    #[test]
    fn parse_log_command() {
        let cli = Cli::parse_from(["openmem", "log"]);
        assert!(matches!(cli.command, Commands::Log { path } if path.is_none()));
    }

    #[test]
    fn parse_init_command() {
        let cli = Cli::parse_from(["openmem", "init"]);
        assert!(matches!(cli.command, Commands::Init));
    }

    #[test]
    fn parse_vault_flag() {
        let cli = Cli::parse_from(["openmem", "--vault", "/custom/path", "read", "test"]);
        assert_eq!(cli.vault, Some(PathBuf::from("/custom/path")));
    }

    #[test]
    fn parse_missing_required_arg_fails() {
        // "read" requires a path argument
        let result = Cli::try_parse_from(["openmem", "read"]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_outline_command() {
        let cli = Cli::parse_from(["openmem", "outline", "global/user-prefs"]);
        assert!(matches!(cli.command, Commands::Outline { path } if path == "global/user-prefs"));
    }

    #[test]
    fn parse_read_with_hash_path() {
        let cli = Cli::parse_from(["openmem", "read", "global/user-prefs#Stack"]);
        assert!(
            matches!(cli.command, Commands::Read { path } if path == "global/user-prefs#Stack")
        );
    }
}
