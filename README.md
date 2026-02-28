# OpenMem

OpenMem is a local, filesystem-based memory vault designed for AI agents and developers. It provides a simple CLI to read, write, and manage markdown-based memory nodes with bidirectional linking, all automatically version-tracked using Jujutsu (`jj`).

## Project Vision

Modern AI agents need long-term memory to maintain context across sessions, track user preferences, and navigate complex projects. OpenMem provides a **single Rust binary** that acts as a shared brain.

**Core Principles:**

- **Filesystem IS the Interface:** No databases, no background services, no cloud infrastructure. Just plain text markdown files in a local directory (`~/.openmem/vault/`).
- **Progressive Disclosure:** Agents can read high-level overviews and follow `[[links]]` to dive into specific details on demand.
- **Automatic Versioning:** Every change is automatically snapshotted using Jujutsu (`jj`), providing a full audit trail and conflict-free concurrent access.
- **Privacy First:** All data stays local.

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (Edition 2024)
- [Jujutsu (`jj`)](https://github.com/martinvonz/jj) installed and available in your PATH.

### Installation

Clone the repository and install it locally using Cargo:

```bash
git clone https://github.com/WangHuancheng/openmem.git
cd openmem
cargo install --path .
```

### Usage

1. **Initialize the Vault**
   Set up your primary memory vault (defaults to `~/.openmem/vault/`):

   ```bash
   openmem init
   ```

2. **Write a Memory Node**
   You can pipe content directly into OpenMem to create or update a node:

   ```bash
   echo "The user prefers dark mode." | openmem write global/user-prefs
   ```

3. **Read a Memory Node**
   Retrieve the contents of a node by its path:

   ```bash
   openmem read global/user-prefs
   ```

4. **List Nodes**
   View all available nodes in the vault:

   ```bash
   openmem list
   ```

5. **Delete a Memory Node**

   ```bash
   openmem delete global/user-prefs
   ```

---
*OpenMem: A shared brain, entirely in your control.*
