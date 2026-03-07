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
        /// Show size indicators
        #[arg(short, long)]
        sizes: bool,
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
    /// Survey the vault for analysis
    Survey {
        /// Optional path prefix to limit survey scope
        scope: Option<String>,
    },
    /// Session memory extraction commands
    Hippocampus {
        #[command(subcommand)]
        command: HippocampusCommands,
    },
    /// Search across all nodes
    Search {
        /// Search query (literal or regex with --regex flag)
        query: String,
        /// Limit search to nodes under this path prefix
        #[arg(short, long)]
        scope: Option<String>,
        /// Maximum number of results
        #[arg(short = 'n', long, default_value = "20")]
        max: usize,
        /// Case-sensitive search
        #[arg(short, long)]
        case_sensitive: bool,
        /// Treat query as regex pattern
        #[arg(short = 'E', long)]
        regex: bool,
    },
    /// Tag operations
    Tags {
        #[command(subcommand)]
        command: TagsCommands,
    },
    /// Memory index operations
    Index {
        #[command(subcommand)]
        command: IndexCommands,
    },
    /// Show vault statistics
    Stats {
        /// Optional path prefix
        path: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum TagsCommands {
    /// List all tags in the vault
    List {
        /// Limit to nodes under this path prefix
        scope: Option<String>,
    },
    /// Find nodes with a specific tag
    Find {
        /// Tag to search for (with or without # prefix)
        tag: String,
    },
    /// Show tags for a specific node
    Show {
        /// Node path
        path: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum IndexCommands {
    /// Generate or update the memory index
    Update,
    /// Show the current index
    Show,
}

#[derive(Subcommand, Debug)]
pub enum HippocampusCommands {
    /// Generate an extraction prompt from a session transcript
    Extract {
        /// Optional session file path (reads from stdin if not provided)
        session: Option<String>,
    },
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
        Commands::List { path, sizes } => {
            let vault = crate::vault::ensure(&vault_path)?;
            let prefix = path.as_deref().unwrap_or("");
            let nodes = crate::node::list(&vault, prefix)?;
            if nodes.is_empty() {
                Ok("(no nodes)".to_string())
            } else if *sizes {
                let mut output = String::new();
                for node_path in &nodes {
                    if let Ok(size) = crate::size::node_size(&vault, node_path) {
                        let category = crate::size::SizeCategory::from_bytes(size.bytes);
                        output.push_str(&format!(
                            "{} {} ({} lines, ~{} tokens)\n",
                            node_path,
                            category.marker(),
                            size.lines,
                            size.tokens
                        ));
                    }
                }
                Ok(output.trim_end().to_string())
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
        Commands::Survey { scope } => {
            let vault = crate::vault::ensure(&vault_path)?;
            let survey_result = crate::survey::survey(&vault, scope.as_deref())?;
            Ok(crate::survey::format_report(&survey_result))
        }
        Commands::Hippocampus { command } => {
            let vault = crate::vault::ensure(&vault_path)?;
            match command {
                HippocampusCommands::Extract { session } => {
                    let session_path = session.as_ref().map(|s| std::path::Path::new(s));
                    let session_content = crate::hippocampus::read_session(session_path)?;
                    let prompt = crate::hippocampus::build_extraction_prompt(&vault, &session_content)?;
                    Ok(prompt)
                }
            }
        }
        Commands::Search { query, scope, max, case_sensitive, regex } => {
            let vault = crate::vault::ensure(&vault_path)?;
            let options = crate::search::SearchOptions {
                scope: scope.clone(),
                max_results: *max,
                case_sensitive: *case_sensitive,
                regex: *regex,
            };
            let results = crate::search::search(&vault, query, options)?;
            Ok(crate::search::format_results(&results))
        }
        Commands::Tags { command } => {
            let vault = crate::vault::ensure(&vault_path)?;
            match command {
                TagsCommands::List { scope } => {
                    let tags = crate::tags::list_tags(&vault, scope.as_deref())?;
                    if tags.is_empty() {
                        Ok("No tags found.".to_string())
                    } else {
                        let mut output = String::new();
                        for (tag, count) in &tags {
                            output.push_str(&format!("#{} ({} nodes)\n", tag, count));
                        }
                        Ok(output.trim_end().to_string())
                    }
                }
                TagsCommands::Find { tag } => {
                    let nodes = crate::tags::find_by_tag(&vault, tag)?;
                    if nodes.is_empty() {
                        Ok(format!("No nodes found with tag #{}", tag.trim_start_matches('#')))
                    } else {
                        Ok(nodes.join("\n"))
                    }
                }
                TagsCommands::Show { path } => {
                    let tags = crate::tags::get_node_tags(&vault, path)?;
                    if tags.is_empty() {
                        Ok(format!("No tags in {}", path))
                    } else {
                        Ok(tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" "))
                    }
                }
            }
        }
        Commands::Index { command } => {
            let vault = crate::vault::ensure(&vault_path)?;
            match command {
                IndexCommands::Update => {
                    let result = crate::index::update_index(&vault)?;
                    Ok(result)
                }
                IndexCommands::Show => {
                    if crate::index::index_exists(&vault) {
                        crate::node::read_section(&vault, crate::index::INDEX_PATH)
                    } else {
                        Ok("No index found. Run `openmem index update` to create one.".to_string())
                    }
                }
            }
        }
        Commands::Stats { path } => {
            let vault = crate::vault::ensure(&vault_path)?;
            let prefix = path.as_deref().unwrap_or("");
            let stats = crate::size::vault_stats(&vault, prefix)?;
            Ok(crate::size::format_stats(&stats))
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
            matches!(cli.command, Commands::List { path, .. } if path == Some("projects".to_string()))
        );
    }

    #[test]
    fn parse_list_command_without_path() {
        let cli = Cli::parse_from(["openmem", "list"]);
        assert!(matches!(cli.command, Commands::List { path, .. } if path.is_none()));
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

    #[test]
    fn parse_survey_command() {
        let cli = Cli::parse_from(["openmem", "survey"]);
        assert!(matches!(cli.command, Commands::Survey { scope } if scope.is_none()));
    }

    #[test]
    fn parse_survey_command_with_scope() {
        let cli = Cli::parse_from(["openmem", "survey", "projects"]);
        assert!(
            matches!(cli.command, Commands::Survey { scope } if scope == Some("projects".to_string()))
        );
    }

    #[test]
    fn parse_hippocampus_extract_no_session() {
        let cli = Cli::parse_from(["openmem", "hippocampus", "extract"]);
        assert!(matches!(
            cli.command,
            Commands::Hippocampus { command: HippocampusCommands::Extract { session } } if session.is_none()
        ));
    }

    #[test]
    fn parse_hippocampus_extract_with_session() {
        let cli = Cli::parse_from(["openmem", "hippocampus", "extract", "session.txt"]);
        assert!(matches!(
            cli.command,
            Commands::Hippocampus { command: HippocampusCommands::Extract { session } } if session == Some("session.txt".to_string())
        ));
    }

    #[test]
    fn parse_search_command() {
        let cli = Cli::parse_from(["openmem", "search", "query"]);
        assert!(matches!(cli.command, Commands::Search { query, .. } if query == "query"));
    }

    #[test]
    fn parse_search_with_options() {
        let cli = Cli::parse_from(["openmem", "search", "test", "-s", "projects", "-n", "10", "-c", "-E"]);
        match cli.command {
            Commands::Search { query, scope, max, case_sensitive, regex } => {
                assert_eq!(query, "test");
                assert_eq!(scope, Some("projects".to_string()));
                assert_eq!(max, 10);
                assert!(case_sensitive);
                assert!(regex);
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn parse_tags_list() {
        let cli = Cli::parse_from(["openmem", "tags", "list"]);
        assert!(matches!(
            cli.command,
            Commands::Tags { command: TagsCommands::List { scope } } if scope.is_none()
        ));
    }

    #[test]
    fn parse_tags_find() {
        let cli = Cli::parse_from(["openmem", "tags", "find", "rust"]);
        assert!(matches!(
            cli.command,
            Commands::Tags { command: TagsCommands::Find { tag } } if tag == "rust"
        ));
    }

    #[test]
    fn parse_tags_show() {
        let cli = Cli::parse_from(["openmem", "tags", "show", "global/prefs"]);
        assert!(matches!(
            cli.command,
            Commands::Tags { command: TagsCommands::Show { path } } if path == "global/prefs"
        ));
    }

    #[test]
    fn parse_index_update() {
        let cli = Cli::parse_from(["openmem", "index", "update"]);
        assert!(matches!(
            cli.command,
            Commands::Index { command: IndexCommands::Update }
        ));
    }

    #[test]
    fn parse_index_show() {
        let cli = Cli::parse_from(["openmem", "index", "show"]);
        assert!(matches!(
            cli.command,
            Commands::Index { command: IndexCommands::Show }
        ));
    }

    #[test]
    fn parse_stats_command() {
        let cli = Cli::parse_from(["openmem", "stats"]);
        assert!(matches!(cli.command, Commands::Stats { path } if path.is_none()));
    }

    #[test]
    fn parse_stats_with_path() {
        let cli = Cli::parse_from(["openmem", "stats", "projects"]);
        assert!(matches!(cli.command, Commands::Stats { path } if path == Some("projects".to_string())));
    }

    #[test]
    fn parse_list_with_sizes() {
        let cli = Cli::parse_from(["openmem", "list", "--sizes"]);
        match cli.command {
            Commands::List { path, sizes } => {
                assert!(path.is_none());
                assert!(sizes);
            }
            _ => panic!("Expected List command"),
        }
    }
}
