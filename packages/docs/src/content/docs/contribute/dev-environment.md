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

### Git Hooks with Lefthook

The project uses [lefthook](https://github.com/evilmartians/lefthook) to run pre-commit and pre-push git hooks that ensure code quality and catch issues early.

#### Installation

Install lefthook using one of these methods:

```bash
# Using homebrew (macOS)
brew install lefthook

# Using npm
npm install -g lefthook

# Using go
go install github.com/evilmartians/lefthook@latest

# Or download from releases
# https://github.com/evilmartians/lefthook/releases
```

#### Setup

After installing lefthook, set up the git hooks:

```bash
lefthook install
```

#### What the hooks do

**Pre-commit hooks** (run when you `git commit`):

- **Rust formatting** - Automatically applies `rustfmt` formatting to Rust code
- **Rust linting** - Automatically applies `clippy` fixes to Rust code
- **Newline verification** - Automatically fixes files to end with proper newlines
- **Frontend linting** - Automatically fixes frontend linting issues
- **Docs linting** - Automatically fixes documentation linting issues
- **GitLab CI validation** - Validates `.gitlab-ci.yml` syntax using `glab`

The hooks will automatically fix issues where possible, making your development workflow smoother. You can review and stage the fixed files before committing.

**Pre-push hooks** (run when you `git push`):

- All pre-commit checks (for changed files since `main`)
- **Rust tests** - Runs the full test suite
- **Frontend build** - Validates frontend can build successfully

#### Shared commands via mise

The git hooks and CI use shared mise tasks, ensuring consistency between local development and the CI pipeline:

- `mise run rust-fmt` / `mise run rust-fmt:fix` - Rust formatting (check/auto-fix)
- `mise run rust-clippy` / `mise run rust-clippy:fix` - Rust linting (check/auto-fix)
- `mise run newlines-check` / `mise run newlines-check:fix` - Newline validation (check/auto-fix)
- `mise run rust-test` - Test execution
- `mise run frontend-build` - Frontend build
- `mise run frontend-lint:fix` - Frontend linting (auto-fix)
- `mise run docs-lint` / `mise run docs-lint:fix` - Documentation linting (check/auto-fix)

The hooks use `:fix` variants to automatically apply fixes, while CI uses the validation-only variants. You can run any of these commands locally to match exactly what runs in CI.

#### Skipping hooks

If you need to bypass hooks (not recommended), use:

```bash
# Skip pre-commit hooks
git commit --no-verify

# Skip pre-push hooks
git push --no-verify
```

#### Optional dependencies

- **[glab](https://gitlab.com/gitlab-org/cli)** - GitLab CI validation (install for full GitLab CI syntax checking)
- **[gitlab-xtasks](https://gitlab.com/gitlab-org/rust/gitlab-xtasks)** - File newline verification
