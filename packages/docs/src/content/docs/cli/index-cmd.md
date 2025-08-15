---
title: gkg index
description: Index repositories in a workspace to create a knowledge graph
sidebar:
  order: 1
---

The `gkg index` command creates a structured, queryable knowledge graph from source code repositories in a workspace. It discovers Git repositories, analyzes code structure across multiple programming languages, and stores the results in both Parquet files and a KuzuDB graph database.

## Synopsis

```bash
gkg index [WORKSPACE_PATH] [OPTIONS]
```

How `gkg index` works is described in the [How Indexing Works](/architecture/overview#how-indexing-works) page. An important detail to note is that the `[WORKSPACE_PATH]` can either be a path to a workspace or a path to a single repository. `gkg` will automatically detect if the path is a workspace or a repository and index the appropriate data to `~/.gkg/`.

> **Note:** If you are using the `gkg server` command, you must stop it before running `gkg index`.

### Languages Indexed

The indexer will index all languages that are supported under the [Languages](/languages/overview) section even if they are in the same repository. So if you have a Ruby and Python repository, the indexer will index both languages. We currently do not connect languages together at this time, but intend to enable this in the future. To see which languages are supported, see the [Languages](/languages/overview) page.

### Basic Usage

```bash
# Index the current directory
gkg index

# Index a specific workspace and show stats
gkg index /path/to/my/project --stats
```

## Options

### `--threads` / `-t`

Specifies the number of worker threads for CPU-bound processing. Defaults to the number of available CPU cores. Higher values can increase CPU utilization but may have diminishing returns. See the [Threading Model](/architecture/overview#threading-model) page for more details.

### `--verbose` / `-v`

Enables detailed logging, showing per-file processing details and performance statistics. Useful for debugging.

### `--stats`

Outputs indexing statistics, including file counts, definition breakdowns, and processing times. An optional file path can be provided to save the report as JSON.

## Troubleshooting

- **High Memory Usage**: Reduce `--threads` to limit concurrency.
- **Slow Performance**: Increase `--threads` if CPU is underutilized. Use `--verbose` to identify bottlenecks.
- **Server Conflicts**: If the `gkg server` is running, it must be stopped with `gkg server stop` before running `gkg index`.
