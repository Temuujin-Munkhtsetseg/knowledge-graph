---
title: Install
description: Install GitLab Knowledge Graph on Linux, macOS, or Windows
---

GitLab Knowledge Graph (`gkg`) can be installed on Linux, macOS, and Windows using our automated installation scripts.

## System Requirements

- Linux (`x86_64`, `aarch64`)
- macOS (`Intel`, `Apple Silicon`)
- Windows (`x86_64`)

## Quick Install

#### Linux and macOS

**One-line installation (latest version):**

```bash
curl -fsSL https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.sh | bash
```

**Install specific version:**

```bash
curl -fsSL https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.sh | bash -s -- --version v0.6.0
```

**Force reinstall:**

```bash
curl -fsSL https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.sh | bash -s -- --force
```

#### Windows

> **Note**: The following commands should be executed in Windows PowerShell.

**One-line installation (latest version):**

```powershell
irm https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.ps1 | iex
```

**Install specific version:**

```powershell
irm https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.ps1 -OutFile install.ps1; .\install.ps1 -Version v0.6.0
```

**Force reinstall:**

```powershell
irm https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.ps1 -OutFile install.ps1; .\install.ps1 -Force
```

## Building from Source

For development or if pre-built binaries aren't available for your platform:

1. Install [Rust](https://rustup.rs/)
2. Clone the repository:
   ```bash
   git clone https://gitlab.com/gitlab-org/rust/knowledge-graph.git
   cd knowledge-graph
   ```
3. Build and install:
   ```bash
   cargo build --release --bin gkg
   # Binary will be at target/release/gkg
   ```

## Next Steps

Once installed, continue to the [Quick Start](/getting-started/quick-start) guide to learn how to use GitLab Knowledge Graph.
