# GitLab Knowledge Graph

The GitLab Knowledge Graph is a system to create a structured, queryable representation of code repositories. It captures entities like files, directories, classes, functions, and their relationships (imports, calls, inheritance, etc.), enabling advanced code understanding and AI features.

> Note: This project is not production ready and is still under active development.

## Features

*   **Repository Analysis:** Generates a comprehensive knowledge graph from source code.
*   **Rust Core:** Built with Rust for performance and safety, leveraging [Kuzu](https://docs.kuzudb.com/get-started/) for graph storage.
*   **Component-Based:** Utilizes [`gitlab-code-parser`](https://gitlab.com/gitlab-org/rust/gitlab-code-parser) for AST extraction and [`gitalisk`](https://gitlab.com/gitlab-org/rust/gitalisk) for repository access on the client.
*   **Extensible:** Designed for integration with various GitLab features, including:
    *   Standalone CLI for local indexing and exploration.
    *   Language Server integration for IDEs.
    *   GitLab Server integration for platform features.

## Quick Start

### Prerequisites

| Purpose                   | Minimum Version |
| ------------------------- | --------------- |
| Rust tool-chain           | stable (via mise) |

### Setup with mise

This project uses [mise](https://mise.jdx.dev/) for toolchain management. All required tool versions are specified in `mise.toml`.

1.  **Install mise** (if not already):
    ```bash
    curl https://mise.run | bash
    # or see https://mise.jdx.dev/ for more options
    ```
2.  **Install the required tools:**
    ```bash
    mise install
    ```
    This will install the correct Rust toolchain and any other tools specified in `mise.toml`.

### Rust toolchain

The Rust toolchain is pinned via [`rust-toolchain.toml`](./rust-toolchain.toml):
```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

### Build the Rust crate

```bash
$ git clone https://gitlab.com/gitlab-org/rust/knowledge-graph
$ cd knowledge-graph
$ cargo build --release
```

### Index a repository (Example) - TODO

This project will provide a CLI for indexing repositories.
Example usage:
```bash
TODO
```

## Roadmap

Follow progress in the 👉 [Knowledge Graph First Iteration epic](https://gitlab.com/groups/gitlab-org/-/epics/17514).

## Development

### Testing

Run all tests:
```bash
cargo test --all
```

### Linting (clippy)

Run [clippy](https://github.com/rust-lang/rust-clippy) for lint checks:
```bash
cargo clippy --all -- -D warnings
```

### Formatting

Format the codebase using [rustfmt](https://github.com/rust-lang/rustfmt):
```bash
cargo fmt --all
```
