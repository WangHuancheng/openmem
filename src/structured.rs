use regex::Regex;

/// A structured node parsed from markdown headings.
/// Represents a tree where the file is the root (level 0),
/// H1 headings are level 1 children, H2 are level 2 children, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct StructuredNode {
    /// Heading text (or filename for root node).
    pub name: String,
    /// Depth: 0 = file root, 1 = H1, 2 = H2, ..., 6 = H6.
    pub level: u8,
    /// Text between this heading and the next heading (excludes children).
    pub content: String,
    /// Sub-nodes (headings at a deeper level).
    pub children: Vec<StructuredNode>,
}

/// Parse a full markdown string into a `StructuredNode` tree.
///
/// The root node (level 0) has `name` set to the given `name` argument.
/// Any text before the first heading becomes the root's `content`.
/// Headings (`# H1`, `## H2`, etc.) create child nodes. A heading at
/// level N becomes a child of the nearest preceding heading at level < N.
///
/// Lines inside fenced code blocks (``` or ~~~) are never treated as headings.
pub fn parse(name: &str, markdown: &str) -> StructuredNode {
    let heading_re = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();

    let mut root = StructuredNode {
        name: name.to_string(),
        level: 0,
        content: String::new(),
        children: Vec::new(),
    };

    // We'll collect flat (level, name, content_lines) entries first,
    // then nest them into a tree.
    let mut entries: Vec<(u8, String, Vec<String>)> = Vec::new();
    let mut in_code_block = false;

    for line in markdown.lines() {
        let trimmed = line.trim_start();

        // Track fenced code blocks (``` or ~~~)
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            // This line is content, not a heading
            push_content_line(&mut entries, &mut root, line);
            continue;
        }

        if in_code_block {
            push_content_line(&mut entries, &mut root, line);
            continue;
        }

        if let Some(caps) = heading_re.captures(line) {
            let level = caps[1].len() as u8;
            let heading_name = caps[2].trim().to_string();
            entries.push((level, heading_name, Vec::new()));
        } else {
            push_content_line(&mut entries, &mut root, line);
        }
    }

    // Set root content (trim trailing newlines but keep internal structure)
    root.content = root.content.trim_end_matches('\n').to_string();

    // Convert flat entries into StructuredNode leaves (no children yet)
    let nodes: Vec<StructuredNode> = entries
        .into_iter()
        .map(|(level, name, lines)| {
            let content = lines.join("\n").trim_end_matches('\n').to_string();
            StructuredNode {
                name,
                level,
                content,
                children: Vec::new(),
            }
        })
        .collect();

    // Build tree by nesting nodes according to level
    root.children = build_tree(nodes);
    root
}

/// Push a content line to the last entry (if any), or to root content.
fn push_content_line(
    entries: &mut [(u8, String, Vec<String>)],
    root: &mut StructuredNode,
    line: &str,
) {
    if let Some(last) = entries.last_mut() {
        last.2.push(line.to_string());
    } else {
        if !root.content.is_empty() {
            root.content.push('\n');
        }
        root.content.push_str(line);
    }
}

/// Build a tree from a flat list of nodes ordered by appearance.
/// A node at level N becomes a child of the nearest preceding node at level < N.
/// Uses a recursive approach: consumes nodes that belong as children of `parent_level`.
fn build_tree(nodes: Vec<StructuredNode>) -> Vec<StructuredNode> {
    let mut idx = 0;
    build_children(&nodes, &mut idx, 0)
}

/// Recursively consume nodes from `nodes[*idx..]` that are children of a parent at `parent_level`.
/// A node belongs as a child if its level > parent_level.
/// Stops when it encounters a node at level <= parent_level or runs out of nodes.
fn build_children(
    nodes: &[StructuredNode],
    idx: &mut usize,
    parent_level: u8,
) -> Vec<StructuredNode> {
    let mut children = Vec::new();

    while *idx < nodes.len() {
        let node_level = nodes[*idx].level;

        if node_level <= parent_level {
            // This node belongs to an ancestor, not to us
            break;
        }

        // Take this node as a child
        let mut child = nodes[*idx].clone();
        *idx += 1;

        // Recursively collect its children (nodes at deeper levels)
        child.children = build_children(nodes, idx, child.level);
        children.push(child);
    }

    children
}

/// Find a sub-node by a `/`-separated heading path.
///
/// Example: `find(&root, "Stack/Frontend")` traverses:
///   root → child named "Stack" → child named "Frontend"
///
/// Matching is case-sensitive and exact.
pub fn find<'a>(node: &'a StructuredNode, heading_path: &str) -> Option<&'a StructuredNode> {
    if heading_path.is_empty() {
        return Some(node);
    }

    let parts: Vec<&str> = heading_path.splitn(2, '/').collect();
    let target_name = parts[0];
    let rest = if parts.len() > 1 { parts[1] } else { "" };

    for child in &node.children {
        if child.name == target_name {
            return find(child, rest);
        }
    }

    None
}

/// Render a `StructuredNode` sub-tree back to markdown.
///
/// Reconstructs headings with `#` markers at the correct level,
/// followed by content, then recursively renders children.
pub fn render(node: &StructuredNode) -> String {
    let mut out = String::new();

    // Render heading (skip for root level 0)
    if node.level > 0 {
        let hashes = "#".repeat(node.level as usize);
        out.push_str(&format!("{} {}\n", hashes, node.name));
    }

    // Render content
    if !node.content.is_empty() {
        if node.level > 0 {
            out.push('\n');
        }
        out.push_str(&node.content);
        out.push('\n');
    }

    // Render children
    for child in &node.children {
        out.push('\n');
        out.push_str(&render(child));
    }

    out
}

/// Extract an outline: a flat list of `(level, name)` pairs from the tree.
/// Includes the root if it has a meaningful name.
pub fn outline(node: &StructuredNode) -> Vec<(u8, String)> {
    let mut result = Vec::new();
    // Include the root node
    result.push((node.level, node.name.clone()));
    collect_outline(node, &mut result);
    result
}

fn collect_outline(node: &StructuredNode, result: &mut Vec<(u8, String)>) {
    for child in &node.children {
        result.push((child.level, child.name.clone()));
        collect_outline(child, result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================
    // parse() tests
    // ==========================================

    #[test]
    fn parse_empty_content() {
        let node = parse("empty", "");
        assert_eq!(node.name, "empty");
        assert_eq!(node.level, 0);
        assert_eq!(node.content, "");
        assert!(node.children.is_empty());
    }

    #[test]
    fn parse_body_only_no_headings() {
        let md = "Just some text.\nWith multiple lines.\n\nAnd a blank line.";
        let node = parse("notes", md);
        assert_eq!(node.name, "notes");
        assert!(node.children.is_empty());
        assert!(node.content.contains("Just some text."));
        assert!(node.content.contains("And a blank line."));
    }

    #[test]
    fn parse_single_h1() {
        let md = "# Overview\n\nThis is the overview.";
        let node = parse("doc", md);
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].name, "Overview");
        assert_eq!(node.children[0].level, 1);
        assert!(node.children[0].content.contains("This is the overview."));
    }

    #[test]
    fn parse_nested_headings() {
        let md = "\
# Project

Overview text.

## Stack

- React
- Rust

### Frontend

React details.

### Backend

Rust details.

## Conventions

Code style notes.";

        let node = parse("acme", md);
        assert_eq!(node.children.len(), 1); // One H1: "Project"

        let project = &node.children[0];
        assert_eq!(project.name, "Project");
        assert!(project.content.contains("Overview text."));
        assert_eq!(project.children.len(), 2); // "Stack" and "Conventions"

        let stack = &project.children[0];
        assert_eq!(stack.name, "Stack");
        assert!(stack.content.contains("- React"));
        assert_eq!(stack.children.len(), 2); // "Frontend" and "Backend"

        assert_eq!(stack.children[0].name, "Frontend");
        assert_eq!(stack.children[1].name, "Backend");

        let conventions = &project.children[1];
        assert_eq!(conventions.name, "Conventions");
        assert!(conventions.content.contains("Code style notes."));
    }

    #[test]
    fn parse_sibling_headings() {
        let md = "\
# First

Content A.

# Second

Content B.

# Third

Content C.";

        let node = parse("doc", md);
        assert_eq!(node.children.len(), 3);
        assert_eq!(node.children[0].name, "First");
        assert_eq!(node.children[1].name, "Second");
        assert_eq!(node.children[2].name, "Third");
    }

    #[test]
    fn parse_skipped_levels() {
        // H1 directly to H3 — H3 should still nest under H1
        let md = "\
# Top

## Middle

Content.

### Deep

Deeply nested.";

        let node = parse("doc", md);
        let top = &node.children[0];
        assert_eq!(top.name, "Top");
        let middle = &top.children[0];
        assert_eq!(middle.name, "Middle");
        let deep = &middle.children[0];
        assert_eq!(deep.name, "Deep");
        assert!(deep.content.contains("Deeply nested."));
    }

    #[test]
    fn parse_content_between_headings() {
        let md = "\
Preamble text.

# Section One

Content of section one.

# Section Two

Content of section two.";

        let node = parse("doc", md);
        assert!(node.content.contains("Preamble text."));
        assert_eq!(node.children.len(), 2);
        assert!(node.children[0].content.contains("Content of section one."));
        assert!(node.children[1].content.contains("Content of section two."));
    }

    #[test]
    fn parse_preserves_code_blocks() {
        let md = "\
# Config

```yaml
# This is NOT a heading
server:
  port: 8080
```

After code block.";

        let node = parse("doc", md);
        let config = &node.children[0];
        assert_eq!(config.name, "Config");
        assert!(config.content.contains("# This is NOT a heading"));
        assert!(config.content.contains("server:"));
        assert!(config.content.contains("After code block."));
        assert!(config.children.is_empty()); // No false children from code block
    }

    // ==========================================
    // find() tests
    // ==========================================

    #[test]
    fn find_top_level_heading() {
        let md = "\
# Overview

Overview text.

# Stack

Stack text.";

        let node = parse("doc", md);
        let found = find(&node, "Stack").unwrap();
        assert_eq!(found.name, "Stack");
        assert!(found.content.contains("Stack text."));
    }

    #[test]
    fn find_nested_heading() {
        let md = "\
# Project

## Stack

### Frontend

React details.";

        let node = parse("doc", md);
        let found = find(&node, "Project/Stack/Frontend").unwrap();
        assert_eq!(found.name, "Frontend");
        assert!(found.content.contains("React details."));
    }

    #[test]
    fn find_nonexistent_returns_none() {
        let md = "# Exists\n\nContent.";
        let node = parse("doc", md);
        assert!(find(&node, "DoesNotExist").is_none());
        assert!(find(&node, "Exists/Nope").is_none());
    }

    #[test]
    fn find_empty_path_returns_root() {
        let md = "# Heading\n\nContent.";
        let node = parse("doc", md);
        let found = find(&node, "").unwrap();
        assert_eq!(found.name, "doc");
    }

    // ==========================================
    // render() tests
    // ==========================================

    #[test]
    fn render_subtree() {
        let md = "\
# Project

Overview.

## Stack

- React
- Rust

## Notes

Some notes.";

        let node = parse("doc", md);
        let stack = find(&node, "Project/Stack").unwrap();
        let rendered = render(stack);
        assert!(rendered.contains("## Stack"));
        assert!(rendered.contains("- React"));
        // Should NOT contain sibling "Notes"
        assert!(!rendered.contains("Notes"));
    }

    #[test]
    fn render_leaf_node() {
        let md = "# Leaf\n\nJust content.";
        let node = parse("doc", md);
        let leaf = find(&node, "Leaf").unwrap();
        let rendered = render(leaf);
        assert!(rendered.contains("# Leaf"));
        assert!(rendered.contains("Just content."));
    }

    // ==========================================
    // outline() tests
    // ==========================================

    #[test]
    fn outline_returns_level_name_pairs() {
        let md = "\
# First

## Nested

# Second";

        let node = parse("doc", md);
        let items = outline(&node);
        assert_eq!(items[0], (0, "doc".to_string()));
        assert_eq!(items[1], (1, "First".to_string()));
        assert_eq!(items[2], (2, "Nested".to_string()));
        assert_eq!(items[3], (1, "Second".to_string()));
    }

    #[test]
    fn outline_empty_doc() {
        let node = parse("empty", "");
        let items = outline(&node);
        assert_eq!(items.len(), 1); // just the root
        assert_eq!(items[0], (0, "empty".to_string()));
    }
}
