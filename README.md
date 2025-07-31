# GitLab Knowledge Graph

The GitLab Knowledge Graph is a system to create a structured, queryable representation of code repositories. It captures entities like files, directories, classes, functions, and their relationships (imports, calls, inheritance, etc.), enabling advanced code understanding and AI features.

> Note: This project is not production ready and is still under active development.

## Roadmap

Follow progress in the 👉 [Knowledge Graph First Iteration epic](https://gitlab.com/groups/gitlab-org/-/epics/17514).

## Installation

> [!caution]
> You can always check the source code of our install scripts in our [repository](https://gitlab.com/gitlab-org/rust/knowledge-graph).

<details><summary>Linux and MacOS</summary>

**One-line installation (latest version):**
```shell
curl -fsSL https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.sh | bash
```

**Install specific version:**
```shell
curl -fsSL https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.sh | bash -s -- --version v0.6.0
```

**Force reinstall:**
```shell
curl -fsSL https://gitlab.com/gitlab-org/rust/knowledge-graph/-/raw/main/install.sh | bash -s -- --force
```

</details>

<details><summary>Windows</summary>

> [!note]
> The following commands should be executed in the Windows PowerShell.

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

</details>

## LFS

Because Knowledge Graph repository includes also pre-compiled static C-bindings
libraries which are large, it uses LFS to store them. Make sure you have [git
lfs](https://docs.gitlab.com/topics/git/lfs/) installed.
