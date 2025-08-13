---
title: Troubleshooting
description: Troubleshoot common issues with GitLab Knowledge Graph.
sidebar:
  order: 5
---

## Storage

Knowledge Graph stores its state, index, and log files in `~/.gkg` on Unix-like systems and `%USERPROFILE%/.gkg` on Windows.

### View Logs

Logs are available in `.gkg/logs`. The latest file is `.gkg/logs/logs.log`.

Additionally, a server started with `gkg server start` writes all logs to stderr.

### Data reset

A data reset may be required after installing a different version of `gkg`. Remove the `.gkg` folder and let Knowledge Graph rebuild its index. For platform-specific location see [Storage](#storage).

## Common problems

### Indexing does not start

Ensure Git is installed and your workspace has Git initialized:

```bash
git -v
git status
```

Verify your workspace contains files in a [supported language](/languages/overview).

For IDE-specific integrations, see [IDE integration](/getting-started/ide-integration).

### GKG is in an inconsistent state

While gkg is in active development, it may get stuck, crash, or end up in an inconsistent state. Try the following:

Stop the server with the `stop` command:

```bash
gkg server stop
```

If that fails, check if gkg is running in your system task manager or run:

```bash
pgrep gkg # prints the PID if gkg is running
```

Stop it forcefully:

```bash
kill <PID>
kill -9 <PID> # force kill
```

Before starting the server, ensure there are no stale `gkg.lock` files:

```bash
rm ~/.gkg/gkg.lock
```

## Report issues

Report bugs on the [Knowledge Graph issue tracker](https://gitlab.com/gitlab-org/rust/knowledge-graph/-/issues),
please use our [Bug Report](https://gitlab.com/gitlab-org/rust/knowledge-graph/-/issues/new?description_template=Bug_report) template.
