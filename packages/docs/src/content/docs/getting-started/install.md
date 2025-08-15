---
title: Install
description: Install GitLab Knowledge Graph on Linux, macOS, or Windows
sidebar:
  order: 2
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

> **Note**: A forced installation does not handle database schema upgrades. For that, you may need to [reset your data](/getting-started/troubleshooting#data-reset).

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

If pre-built binaries are not available for your platform, you can [build from the source](/contribute/build).

## Next Steps

Once installed, continue to the [Usage](/getting-started/usage) guide to learn how to use GitLab Knowledge Graph.
