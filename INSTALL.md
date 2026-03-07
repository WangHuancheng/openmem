# OpenMem Installation Guide

## Prerequisites

- **Rust** 1.70+ (edition 2024)
- **Jujutsu (jj)** 0.38+ — for version tracking

## Install OpenMem CLI

### From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/hadrain/openmem.git
cd openmem

# Build release binary
cargo build --release

# Add to PATH (choose one method)

# Option 1: Copy to user binary directory
cp target/release/openmem.exe ~/.cargo/bin/
# or ~/.local/bin/

# Option 2: Add to custom directory and update PATH
cp target/release/openmem.exe /usr/local/bin/
# or on Windows:
copy target\release\openmem.exe C:\Tools\
# Add to PATH environment variable

# Verify installation
openmem --version
```

### From Pre-built Binaries

Download the latest release from the [Releases](https://github.com/hadrain/openmem/releases) page.

```bash
# Linux/macOS
tar -xzf openmem-*.tar.gz
mv openmem /usr/local/bin/

# Windows (PowerShell)
Expand-Archive openmem-*.zip -DestinationPath C:\Tools
# Add C:\Tools to your PATH
```

## Install jj (Jujutsu)

OpenMem uses Jujutsu for version tracking.

### Windows

```powershell
# Using winget
winget install jj

# Or download from https://github.com/martinvonz/jj/releases
```

### macOS

```bash
brew install jj
```

### Linux

```bash
# Arch Linux
pacman -S jj

# Nix
nix profile install nixpkgs#jj

# Or download from https://github.com/martinvonz/jj/releases
```

## Initialize Vault

```bash
# Initialize a new vault
openmem init

# This creates ~/.openmem/vault/ with jj version control
```

## Install Skill for Agents

Copy the skill file to make it available for AI agents:

```bash
# Create skills directory
mkdir -p ~/.config/opencode/skills/openmem

# Copy skill file
cp skills/openmem/SKILL.md ~/.config/opencode/skills/openmem/
```

### For OhMyOpenCode Users

The skill is automatically available if you clone the repository and add it to your agent's skill path.

### Manual Installation

1. Copy `skills/openmem/SKILL.md` to your agent's skills directory
2. Update your agent configuration to load the `openmem` skill
3. Restart your agent session

## Configuration

### Config File (Optional)

Create `~/.openmem/config.toml` to customize the vault location:

```toml
# Default: ~/.openmem/vault/
vault = "/custom/path/to/vault"
```

### Environment Variables

```bash
# Override vault location
export OPENMEM_VAULT="/custom/vault/path"
```

## Verify Installation

```bash
# Check version
openmem --version

# Initialize vault
openmem init

# Write test node
echo "# Test Node\n\nThis is a test." | openmem write test/hello

# Read it back
openmem read test/hello

# List nodes
openmem list

# Delete test node
openmem delete test/hello
```

## Troubleshooting

### "jj not found"

Install Jujutsu (jj) using the instructions above.

### "permission denied"

Ensure you have write permissions to the vault directory.

### "node not found"

Check the node path is correct. Use `openmem list` to see available nodes.

### Windows: "'export' is not recognized"

This is expected in PowerShell. Use `$env:` instead:
```powershell
$env:OPENMEM_VAULT = "C:\custom\vault"
```

## Next Steps

- Read the [README](README.md) for usage examples
- Check the [Architecture](docs/ARCHITECTURE.md) for system design
- Review the [Feature Proposals](docs/Feature Proposals.md) for planned features
- Install the skill for your AI agent (see above)
