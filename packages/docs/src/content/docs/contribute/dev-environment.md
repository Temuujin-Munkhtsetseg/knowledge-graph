---
title: Development Environment
description: Set up your development environment to start contributing to the project.
sidebar:
  order: 1
---

GitLab Knowledge Graph is a multi-package workspace:

- **Backend (Rust)**: CLI/server `gkg`.
- **Frontend (TypeScript + Vue 3 + Vite)**: `@gitlab-org/gkg-frontend` UI, `@gitlab-org/gkg` TypeScript bindings.
- **Docs (Astro + Starlight)**: `packages/docs`.

Dependencies (see `Cargo.toml` and package manifests):

- Rust toolchain: `stable` (managed by Mise), major crates include `axum`, `kuzu`, `ts-rs`, `tokio`,
  [gitalisk](https://gitlab.com/gitlab-org/rust/gitalisk), [gitlab-code-parser](https://gitlab.com/gitlab-org/rust/gitlab-code-parser)
- Node.js: (managed by Mise). Frontend uses Vite, Vue, TypeScript, ESLint, Tailwind CSS. Docs use Astro and Starlight.

### Prerequisites

- **VS Code** with `rust-analyzer` and `CodeLLDB` (recommended for debugging) extensions.
- **Kuzu** build prerequisites for your platform. See the Kuzu [developer guide](https://docs.kuzudb.com/developer-guide/).
  Note: you can skip this step if you enable [dynamic linking](/contribute/build#speed-up-your-builds) to the Kuzu prebuilt libraries.
- **Mise** (recommended) or install Rust and Node manually.

### Quick setup

1. Clone the repo.
   ```bash
   git clone https://gitlab.com/gitlab-org/rust/knowledge-graph.git
   cd knowledge-graph
   ```
2. Trust the project and install toolchains with Mise.
   ```bash
   mise trust && mise install
   mise ls
   ```

Manual setup (alternative):

- Install the latest [Rust](https://www.rust-lang.org/tools/install) and [Node.js](https://nodejs.org/).
  Ensure `cargo` and `npm` are available in the PATH.
