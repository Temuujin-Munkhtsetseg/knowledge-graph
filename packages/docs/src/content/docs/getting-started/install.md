---
title: Install
description: Install GitLab Knowledge Graph on Linux, macOS, or Windows
---

GitLab Knowledge Graph (`gkg`) can be installed on Linux, macOS, and Windows using automated installation scripts.

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

**Install a specific version:**

```bash
curl -fsSL https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.sh | bash -s -- --version v0.9.0
```

**Force re-installation:**

> **Note**: A forced installation does not handle database schema upgrades. For that, you may need to [reset your data](troubleshooting.md#data-reset).

```bash
curl -fsSL https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.sh | bash -s -- --force
```

#### Windows

> **Note**: The following commands should be executed in Windows PowerShell.

**One-line installation (latest version):**

```powershell
irm https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.ps1 | iex
```

**Install a specific version:**

```powershell
irm https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.ps1 -OutFile install.ps1; .\install.ps1 -Version v0.6.0
```

**Force re-installation:**

```powershell
irm https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.ps1 -OutFile install.ps1; .\install.ps1 -Force
```

## Build from Source

> **Note**: You can skip the npm installation and front-end build steps for a `gkg` without a web UI. Just use the `--features 
no-frontend` feature flag for the final build step.

If pre-built binaries are not available for your platform, you can build from the source:

1. Install any [prerequisites](https://docs.kuzudb.com/developer-guide/) for your platform to build the Kuzu database.
1. Install [Mise](https://mise.jdx.dev/getting-started.html).
1. Clone the repository:
   ```bash
   git clone https://gitlab.com/gitlab-org/rust/knowledge-graph.git
   cd knowledge-graph
   ```
1. Trust project and install Rust + Node toolchains:
   ```bash
   mise trust && mise install
   ```
1. Verify `npm` and `rust` tools are available:
   ```bash
   mise ls
   ```
1. Build front-end assets:
   ```bash
   npm ci
   npm run build --workspace=@gitlab-org/gkg-frontend
   npm run build --workspace=@gitlab-org/gkg
   ```
1. Build the `gkg`:
   ```bash
   cargo build --release --bin gkg
   # Binary will be at target/release/gkg
   ```

## Next Steps

Once installed, continue to the [Quick Start](/getting-started/quick-start) guide to learn how to use GitLab Knowledge Graph.
