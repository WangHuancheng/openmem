---
name: openmem
description: How to use OpenMem as the agent's primary persistent memory system
---

# OpenMem Memory Skill

OpenMem is the primary memory system. The vault at `~/.openmem/vault/` stores durable knowledge as plain markdown files. Use the `openmem` CLI to read/write.

## Core Principles

OpenMem follows 3 principles that guide all features:

1. **Text is king** — Everything is plain markdown. No databases, no binary formats.
2. **Progressive Disclosure** — Read overviews first, dive into details on demand.
3. **Structured Storage** — `[[links]]` and `#tags` organize knowledge without rigid schemas.

## On Conversation Start

```powershell
# 1. Check memory index (curated overview)
openmem index show 2>$null

# 2. If no index, list available nodes
openmem list 2>$null

# 3. Read user context
openmem read global/user-prefs 2>$null
openmem read global/agent-rules 2>$null
```

> [!TIP]
> Use progressive disclosure: `index show` → `list` → `read` only what's relevant.

## CLI Commands

### Core CRUD

| Command | Usage |
|---|---|
| `openmem init` | Initialize a new vault |
| `openmem list [path]` | List all nodes (or under a prefix) |
| `openmem list --sizes` | List nodes with size indicators [S/M/L] |
| `openmem read <path>` | Read a node's full content |
| `openmem read <path>#Heading` | Read a specific section |
| `openmem write <path>` | Write a node (content from stdin) |
| `openmem delete <path>` | Delete a node |

### Discovery & Navigation

| Command | Usage |
|---|---|
| `openmem search <query>` | Full-text search across all nodes |
| `openmem search <query> -s projects` | Limit search to a scope |
| `openmem search <regex> -E` | Regex search mode |
| `openmem links <path>` | Show outgoing links and backlinks |
| `openmem outline <path>` | Show heading tree of a node |

### Tags & Categorization

| Command | Usage |
|---|---|
| `openmem tags list` | List all tags with node counts |
| `openmem tags find <tag>` | Find all nodes with a tag |
| `openmem tags show <path>` | Show tags in a specific node |

### Memory Management

| Command | Usage |
|---|---|
| `openmem index update` | Generate/update memory index |
| `openmem index show` | Display current memory index |
| `openmem stats [path]` | Show vault statistics |
| `openmem survey [scope]` | Analyze vault (orphans, hubs, sparse areas) |
| `openmem log [path]` | Show change history |

### Session Memory

| Command | Usage |
|---|---|
| `openmem hippocampus extract` | Generate extraction prompt from stdin |

## Vault Layout

```
~/.openmem/vault/
├── global/           ← always-read context
│   ├── user-prefs.md     — user preferences and habits
│   ├── agent-rules.md    — agent behavior rules
│   └── index.md          — auto-generated memory index
├── missions/         ← agent mission definitions
├── projects/         ← per-project knowledge
│   └── <name>/
│       ├── goal.md
│       ├── decisions.md
│       └── ...
├── tools/            ← tool and technology knowledge
└── ...               ← any other knowledge nodes
```

## Common Workflows

### Starting a New Session

```powershell
# Get overview
openmem index show

# If working on a project
openmem read projects/<name>/goal

# Check relevant tools
openmem tags find rust
```

### Writing New Knowledge

```powershell
@"
# Project Conventions

## Code Style
- Use rustfmt defaults
- Prefer explicit error handling over unwrap

## Architecture
- Follows [[global/coding-standards]]
- Uses [[tools/rust]] patterns
"@ | openmem write projects/acme/conventions 2>$null
```

### Finding Information

```powershell
# Search for specific text
openmem search "authentication flow"

# Find by tag
openmem tags find rust

# Find related nodes
openmem links projects/acme/auth
```

### Progressive Disclosure Pattern

```powershell
# Level 0: See all nodes
openmem list

# Level 1: See sizes (avoid large nodes unless needed)
openmem list --sizes

# Level 2: See heading outline
openmem outline projects/acme/architecture

# Level 3: Read specific section
openmem read projects/acme/architecture#Authentication

# Level 4: Follow links
openmem links projects/acme/architecture
```

## Linking Nodes

Use `[[path/to/node]]` syntax in markdown to cross-reference:

```markdown
This project follows [[global/coding-standards]] and uses [[tools/rust]].

See also [[projects/acme/decisions]] for architectural choices.
```

- Links are bidirectional — `openmem links` shows both directions
- Links are exact paths — no fuzzy matching
- Links enable knowledge graphs without complex schemas

## Using Tags

Add `#tag` in markdown content for categorization:

```markdown
# Rust Error Handling

Uses #rust #error-handling #patterns.

Related: [[tools/rust]]
```

Rules:
- Tags start with `#` followed by letters, numbers, hyphens, or underscores
- Tags inside code blocks are ignored
- Heading `#` markers are not tags (they have a space after)

```powershell
# Find all rust-related nodes
openmem tags find rust

# List all tags in the vault
openmem tags list
```

## Memory Index

The `global/index.md` node is a curated table of contents:

```powershell
# Generate or update the index
openmem index update

# View current index
openmem index show
```

The index includes:
- Global nodes (user-prefs, agent-rules)
- Projects with their nodes
- Top tags by usage
- Orphan nodes (no links)

## Size Indicators

Use `--sizes` flag to gauge reading cost:

```powershell
openmem list --sizes

# Output:
# node-a (15 lines, ~120 tokens)
# node-b [M] (85 lines, ~680 tokens)
# node-c [L] (320 lines, ~2560 tokens)
```

Categories:
- Tiny: <500 bytes (~125 tokens)
- Small [S]: 500B-2KB (~125-500 tokens)
- Medium [M]: 2KB-8KB (~500-2000 tokens)
- Large [L]: >8KB (>2000 tokens)

## Search Capabilities

```powershell
# Literal search (case-insensitive by default)
openmem search "jwt token"

# Case-sensitive
openmem search "JWT" -c

# Regex mode
openmem search "auth-\w+" -E

# Limit scope
openmem search "config" -s projects/acme

# Limit results
openmem search "error" -n 50
```

## Vault Analysis

```powershell
# Survey the vault
openmem survey

# Output includes:
# - Node count and link statistics
# - Orphan nodes (no incoming/outgoing links)
# - Dense hubs (many backlinks)
# - Sparse areas (few nodes)
```

## Best Practices

1. **Read before researching** — Check if vault already has the knowledge
2. **Write after learning** — Persist durable context for future conversations
3. **Paths are identities** — Descriptive paths serve as natural summaries
4. **Use tags liberally** — Tags enable flexible categorization
5. **Update the index** — Run `openmem index update` after significant changes
6. **Follow the size guide** — Check sizes before diving into large nodes
7. **Use heading paths** — Read specific sections with `path#Heading`
8. **Link related knowledge** — `[[links]]` create knowledge graphs

---

## Adding Memory — Best Practices

### When to Write

Write to memory **immediately after** learning something durable:

| Trigger | Example | Location |
|---|---|---|
| User states preference | "I prefer dark mode" | `global/user-prefs` |
| Decision made | "Chose JWT over sessions" | `projects/<name>/decisions` |
| Fact discovered | "API rate limit is 100/min" | `projects/<name>/api-notes` |
| Correction learned | "Actually, the port is 8080 not 3000" | Update existing node |
| Pattern identified | "Always use `?` operator in Rust" | `tools/rust` or `global/coding-standards` |
| Error solution found | "Fix: add `--no-verify` flag" | `projects/<name>/solutions` |

### What to Write

**DO write:**
- User preferences and habits
- Design decisions and rationale
- API endpoints, configs, credentials (non-sensitive)
- Code patterns and conventions
- Error solutions and workarounds
- Project architecture and dependencies
- Tool-specific tips and gotchas

**DO NOT write:**
- Transient debugging steps
- Conversation filler
- Information already stored (update instead)
- Highly ephemeral task details
- Temporary workarounds (unless permanent)

### Memory Location Conventions

```
global/
├── user-prefs.md      ← User preferences (editor, language, style)
├── agent-rules.md     ← Agent behavior rules
└── coding-standards.md ← Language-agnostic standards

projects/<name>/
├── goal.md            ← Project goals and status
├── decisions.md       ← Key decisions with rationale
├── architecture.md    ← System design
├── api-notes.md       ← API documentation
├── conventions.md     ← Project-specific conventions
└── solutions.md       ← Solutions to recurring problems

tools/<tool>/
├── basics.md          ← Quick reference
├── patterns.md        ← Common patterns
└── gotchas.md         ← Common pitfalls

sessions/<date>.md     ← Session summaries (optional)
```

### Node Structure Template

```markdown
# <Topic>

<One-line summary for progressive disclosure>

## Details

<Main content - be concise, use bullet points>

## Related

- [[related-node-1]]
- [[related-node-2]]

## Tags

#tag1 #tag2 #tag3
```

### Writing Style Guidelines

**Be concise:**
```markdown
# Bad - verbose
After much discussion and consideration, we have decided that for this project,
we will be using the React framework for the frontend because it has good
component support.

# Good - concise
Frontend: React (component-based, team familiarity)
```

**Use headings for structure:**
```markdown
# Project Acme API

## Authentication
- JWT tokens, 1-hour expiry
- Refresh via /auth/refresh

## Rate Limits
- 100 requests/min per API key

## Endpoints
- GET /users — list users
- POST /users — create user
```

**Link liberally:**
```markdown
# Bad - no context
Uses React.

# Good - linked context
Uses [[tools/react]] with [[global/coding-standards#TypeScript]].
```

### Update vs Create

**Update when:**
- Information already exists in a node
- Adding to an existing list/pattern
- Correcting outdated information

**Create when:**
- New topic not covered
- Distinct concern needing its own node
- Size would exceed medium (>8KB)

### Size Guidelines

| Size | Tokens | When to split |
|---|---|---|
| Tiny | <125 | Keep as-is, may need expansion |
| Small [S] | 125-500 | Ideal size for most nodes |
| Medium [M] | 500-2000 | Consider splitting into sections |
| Large [L] | >2000 | **Split into multiple linked nodes** |

To split a large node:
```powershell
# Read the large node
openmem read projects/acme/architecture

# Create sub-nodes
@"
# Authentication

(content)
"@ | openmem write projects/acme/auth

@"
# Database

(content)
"@ | openmem write projects/acme/database

# Update main node to link to sub-nodes
@"
# Architecture

- [[projects/acme/auth]] — Authentication system
- [[projects/acme/database]] — Database design
"@ | openmem write projects/acme/architecture
```

### Tagging Best Practices

**Use consistent tag names:**
```markdown
# Good - consistent
#rust #error-handling #patterns
#rust #error-handling #best-practices

# Bad - inconsistent
#Rust #ErrorHandling #pattern
#rust-lang #errors #tip
```

**Tag categories:**
- Language: `#rust`, `#python`, `#typescript`
- Domain: `#auth`, `#api`, `#frontend`
- Type: `#decision`, `#pattern`, `#gotcha`
- Status: `#active`, `#deprecated`, `#todo`

### Memory Hygiene

**After each significant session:**

```powershell
# 1. Update index
openmem index update

# 2. Check for orphans (nodes without links)
openmem survey | Select-String "Orphan"

# 3. Link orphans to related content
# (add [[links]] to connect them)

# 4. Check for duplicates
openmem search "<keyword>" 

# 5. Merge or delete duplicates
openmem delete duplicate-node
```

### Memory Lifecycle

```
                    ┌──────────────┐
                    │   LEARN      │
                    │  (session)   │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │   WRITE      │
                    │  (new node)  │
                    └──────┬───────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
 ┌──────▼───────┐   ┌──────▼───────┐   ┌──────▼───────┐
 │   LINK       │   │   TAG        │   │   INDEX      │
 │  (connect)   │   │ (categorize) │   │  (curate)    │
 └──────────────┘   └──────────────┘   └──────────────┘
```

### Anti-Patterns to Avoid

**❌ Write-only memory:**
```markdown
# Bad - no one will find this
asdf123 random notes...
```

**❌ Orphan knowledge:**
```markdown
# Bad - no links, no tags
# Some Topic
Content here...
```

**❌ Duplicate information:**
```powershell
# Bad - same info in multiple places
openmem write projects/a/config "port: 8080"
openmem write projects/a/settings "port: 8080"
```

**❌ Oversized nodes:**
```powershell
# Bad - everything in one node
openmem write projects/acme/everything  # 50KB of content
```

### Quick Reference Card

```
┌─────────────────────────────────────────────────────┐
│           WHEN TO WRITE TO MEMORY                   │
├─────────────────────────────────────────────────────┤
│ ✓ User preference     → global/user-prefs           │
│ ✓ Design decision     → projects/<name>/decisions   │
│ ✓ Learned pattern     → tools/<tool>/patterns       │
│ ✓ Fixed problem       → projects/<name>/solutions   │
│ ✓ New project info    → projects/<name>/<topic>     │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│           MEMORY WRITE CHECKLIST                    │
├─────────────────────────────────────────────────────┤
│ 1. Check if node exists → update if yes             │
│ 2. Use descriptive path → serves as summary         │
│ 3. Add [[links]]       → connect related knowledge  │
│ 4. Add #tags           → enable discovery           │
│ 5. Keep under 2KB      → split if larger            │
│ 6. Update index        → `openmem index update`     │
└─────────────────────────────────────────────────────┘
```

---

## Example Session

```powershell
# 1. Start with overview
openmem index show

# 2. Check relevant project
openmem read projects/myapp/goal

# 3. Find related knowledge
openmem tags find rust
openmem search "authentication"

# 4. Read specific implementation
openmem read projects/myapp/auth#JWT

# 5. Check relationships
openmem links projects/myapp/auth

# 6. Write new knowledge
@"
# Session Notes - 2026-03-07

## Decisions
- Chose JWT over sessions for auth
- Related: [[projects/myapp/auth]]

## Tags
#decision #auth #security
"@ | openmem write sessions/2026-03-07

# 7. Update index
openmem index update
```
