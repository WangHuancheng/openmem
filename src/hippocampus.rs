use std::io::Read;
use std::path::Path;

use crate::error::Result;
use crate::link;
use crate::node;

/// Build an extraction prompt from a session transcript.
///
/// Reads the session content and relevant stored nodes to generate
/// a prompt that asks the model to extract new knowledge.
pub fn build_extraction_prompt(vault: &Path, session: &str) -> Result<String> {
    let mut prompt = String::new();

    prompt.push_str("# Memory Extraction Task\n\n");
    prompt.push_str("You are extracting knowledge from a conversation session to store in long-term memory.\n\n");

    // Add session content
    prompt.push_str("## Session Transcript\n\n");
    prompt.push_str("```\n");
    prompt.push_str(session);
    prompt.push_str("\n```\n\n");

    // Find and read relevant stored nodes
    let relevant_nodes = find_relevant_nodes(vault, session)?;

    if !relevant_nodes.is_empty() {
        prompt.push_str("## Currently Stored Memories (relevant to this session)\n\n");
        for (path, content) in &relevant_nodes {
            prompt.push_str(&format!("### [[{}]]\n\n", path));
            prompt.push_str(content);
            prompt.push_str("\n\n");
        }
    }

    // Add extraction instructions
    prompt.push_str("## Extraction Instructions\n\n");
    prompt.push_str("Analyze the session and decide what new knowledge should be stored.\n\n");
    prompt.push_str("For each item to store, output in this format:\n\n");
    prompt.push_str("```markdown\n");
    prompt.push_str("### WRITE <path>\n");
    prompt.push_str("<content>\n");
    prompt.push_str("```\n\n");
    prompt.push_str("Or to update an existing node:\n\n");
    prompt.push_str("```markdown\n");
    prompt.push_str("### UPDATE <path>\n");
    prompt.push_str("Replace section \"## SectionName\" with:\n");
    prompt.push_str("<new content>\n");
    prompt.push_str("```\n\n");
    prompt.push_str("If nothing else needs storing:\n\n");
    prompt.push_str("```markdown\n");
    prompt.push_str("### SKIP\n");
    prompt.push_str("Reasoning: <why nothing else needs to be stored>\n");
    prompt.push_str("```\n\n");

    prompt.push_str("### What to Extract\n\n");
    prompt.push_str("Look for:\n");
    prompt.push_str("- **User preferences**: \"I prefer X\", \"Always use Y\"\n");
    prompt.push_str("- **Decisions**: \"We chose X over Y because Z\"\n");
    prompt.push_str("- **Facts**: \"The API endpoint is /v2/users\"\n");
    prompt.push_str("- **Corrections**: \"Actually, the value is X not Y\"\n");
    prompt.push_str("- **Project knowledge**: Architecture decisions, deployment configs\n");
    prompt.push_str("- **Relationships**: \"Module A depends on Module B\"\n\n");

    prompt.push_str("### What NOT to Extract\n\n");
    prompt.push_str("- Transient debugging steps\n");
    prompt.push_str("- Conversation filler / pleasantries\n");
    prompt.push_str("- Information already stored verbatim\n");
    prompt.push_str("- Highly ephemeral task details\n\n");

    prompt.push_str("## Extraction Plan\n\n");
    prompt.push_str("Now output your extraction plan:\n\n");

    Ok(prompt)
}

/// Find stored nodes that are relevant to the session.
///
/// Relevance is determined by:
/// 1. Nodes referenced in the session via [[links]]
/// 2. Global memory nodes (global/*)
fn find_relevant_nodes(vault: &Path, session: &str) -> Result<Vec<(String, String)>> {
    let mut relevant = Vec::new();

    // Parse links from the session
    let session_links = link::parse_links(session);

    // Read each linked node
    for link in &session_links {
        if let Ok(content) = node::read(vault, link) {
            relevant.push((link.clone(), content));
        }
    }

    // Read global nodes
    let global_nodes = node::list(vault, "global")?;
    for path in &global_nodes {
        if !relevant.iter().any(|(p, _)| p == path) {
            if let Ok(content) = node::read(vault, path) {
                relevant.push((path.clone(), content));
            }
        }
    }

    Ok(relevant)
}

/// Parse a structured extraction plan response from the model.
///
/// Returns a list of extraction operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ExtractionOp {
    Write { path: String, content: String },
    Update { path: String, section: String, content: String },
    Skip { reason: String },
}

/// Execute an extraction plan (write/update nodes).
///
/// Returns the number of nodes written/updated.
pub fn execute_extraction(vault: &Path, ops: &[ExtractionOp]) -> Result<usize> {
    let mut count = 0;

    for op in ops {
        match op {
            ExtractionOp::Write { path, content } => {
                node::write(vault, path, content)?;
                count += 1;
            }
            ExtractionOp::Update {
                path,
                section: _,
                content: new_content,
            } => {
                // For UPDATE, we currently just overwrite
                // A more sophisticated implementation would merge sections
                node::write(vault, path, new_content)?;
                count += 1;
            }
            ExtractionOp::Skip { reason: _ } => {
                // Nothing to do
            }
        }
    }

    Ok(count)
}

/// Read session content from stdin or a file.
pub fn read_session(input: Option<&Path>) -> Result<String> {
    match input {
        Some(path) => {
            let content = std::fs::read_to_string(path)?;
            Ok(content)
        }
        None => {
            let mut content = String::new();
            std::io::stdin()
                .read_to_string(&mut content)
                .map_err(|e| crate::error::OpenMemError::Io(e))?;
            Ok(content)
        }
    }
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
            "openmem_hippocampus_test_{}_{}_{}",
            std::process::id(),
            id,
            nanos
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn build_prompt_includes_session() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        let session = "User: I prefer dark mode.\nAssistant: Got it!";
        let prompt = build_extraction_prompt(&vault, session).unwrap();

        assert!(prompt.contains("# Memory Extraction Task"));
        assert!(prompt.contains("User: I prefer dark mode"));
        assert!(prompt.contains("## Extraction Instructions"));
    }

    #[test]
    fn build_prompt_includes_relevant_nodes() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "global/user-prefs", "Prefers vim over emacs.").unwrap();
        node::write(
            &vault,
            "tools/vim",
            "# Vim\nA text editor mentioned in [[global/user-prefs]].",
        )
        .unwrap();

        let session = "We should use [[tools/vim]] for editing.";
        let prompt = build_extraction_prompt(&vault, session).unwrap();

        // Should include the linked node
        assert!(prompt.contains("tools/vim"));
        assert!(prompt.contains("A text editor"));
        // Should include global nodes
        assert!(prompt.contains("global/user-prefs"));
        assert!(prompt.contains("Prefers vim"));
    }

    #[test]
    fn find_relevant_nodes_includes_linked_nodes() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "tools/react", "React framework.").unwrap();
        node::write(&vault, "tools/vue", "Vue framework.").unwrap();

        let session = "Let's use [[tools/react]] for this project.";
        let relevant = find_relevant_nodes(&vault, session).unwrap();

        assert_eq!(relevant.len(), 1);
        assert_eq!(relevant[0].0, "tools/react");
    }

    #[test]
    fn find_relevant_nodes_includes_global_nodes() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        node::write(&vault, "global/user-prefs", "User preferences.").unwrap();
        node::write(&vault, "projects/acme", "Project info.").unwrap();

        let session = "Working on something new.";
        let relevant = find_relevant_nodes(&vault, session).unwrap();

        // Should include global nodes but not project nodes
        assert_eq!(relevant.len(), 1);
        assert_eq!(relevant[0].0, "global/user-prefs");
    }

    #[test]
    fn execute_extraction_writes_nodes() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        let ops = vec![
            ExtractionOp::Write {
                path: "global/test".to_string(),
                content: "Test content.".to_string(),
            },
            ExtractionOp::Write {
                path: "projects/demo".to_string(),
                content: "Demo content.".to_string(),
            },
        ];

        let count = execute_extraction(&vault, &ops).unwrap();
        assert_eq!(count, 2);

        assert_eq!(
            node::read(&vault, "global/test").unwrap(),
            "Test content."
        );
        assert_eq!(
            node::read(&vault, "projects/demo").unwrap(),
            "Demo content."
        );
    }

    #[test]
    fn execute_extraction_skip_does_nothing() {
        let vault = temp_vault();
        crate::vault::init(&vault).unwrap();

        let ops = vec![ExtractionOp::Skip {
            reason: "Nothing to store.".to_string(),
        }];

        let count = execute_extraction(&vault, &ops).unwrap();
        assert_eq!(count, 0);
    }
}
