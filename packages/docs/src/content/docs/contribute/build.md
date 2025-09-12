---
title: Build
description: Build the frontend, docs, and the Rust backend (gkg).
sidebar:
  order: 2
---

## Overview

GitLab Knowledge Graph has two parts: a backend (Rust workspace crates) and a frontend (Node workspace packages).

- **Backend (Rust crates)**: define the data model, API, and services (e.g., `crates/http-server-desktop`, `crates/database`, `crates/indexer`).
- **Frontend (Node workspace packages)**:
  - `@gitlab-org/gkg` (`packages/gkg/src`): TypeScript bindings generated from Rust via [`ts-rs`](https://crates.io/crates/ts-rs).
  - `@gitlab-org/gkg-frontend` (`packages/frontend`): Vue + Vite UI that depends on `@gitlab-org/gkg`.

The `http-server-desktop` crate [embeds](https://crates.io/crates/rust-embed) the frontend `dist` into the final `gkg` binary. Docs (`packages/docs`) build separately.

## Build order

1. Generate TypeScript bindings from Rust via `ts-rs`.

```bash
mise run bindings-gen
# equivalent
cargo test export_bindings --features no-frontend
```

We commit generated bindings to git; rebuild them whenever you change the Rust types that drive the API (primarily in `crates/http-server-desktop`).

2. Build the frontend packages.

```bash
npm ci
npm run build --workspace=@gitlab-org/gkg-frontend
```

The Rust HTTP server (`crates/http-server-desktop`) embeds `packages/frontend/dist` into the binary. Without `--features no-frontend`, the binary requires these assets to exist.

3. Build the main binary (`gkg`).

```bash
cargo build --release --bin gkg
```

> ⓘ If you do not need the web UI, you can skip Node installation and frontend builds. Use the `no-frontend` feature when building:

```bash
cargo build --release --bin gkg --features no-frontend
```

Docs build:

```bash
npm ci
npm run build --workspace=docs
```

> ⓘ Please check the `mise.toml` configuration file for many useful commands.

## Speed up your builds

Builds with the `--release` profile can take significantly more time than debug builds. Unless you want to run benchmarks, prefer debug builds to test the functionality of `gkg`.

### Kuzu dynamic linking

By default, Kuzu is built and statically linked to `gkg`, which makes the build at least **much** slower. For quicker
builds and tests, we recommend downloading the Kuzu binaries and using **dynamic linking** instead.
To do this:

1. [Download](https://github.com/kuzudb/kuzu/releases) and unpack the distributed `libkuzu` library files.

2. Set up your environment variables:

```bash
export KUZU_SHARED=1
export KUZU_INCLUDE_DIR=/path/to/kuzu/include
export KUZU_LIBRARY_DIR=/path/to/kuzu/lib
```

3. Now `cargo build` will skip the complex Kuzu build step!

> ⚠️ Make sure your dynamic libraries match the Kuzu version specified in `Cargo.toml`, otherwise you may encounter errors during
> build or runtime!
