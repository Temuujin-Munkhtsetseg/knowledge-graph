---
title: Troubleshooting
description: Troubleshoot common issues with GitLab Knowledge Graph.
---

Knowledge Graph stores its state, index, and log files in the `~/.gkg` folder on Unix-like systems and the `%USERPROFILE%/.gkg` folder on Windows.

## Viewing Logs

Logs are available in the `.gkg/logs` folder. The latest log file is `.gkg/logs/logs.log`.

Additionally, a server started with the `gkg server start` command will write all logs to the **stderr** output.

## Data Reset

A data reset may be required after installing a different version of `gkg`. To do this, you can remove the `.gkg` folder entirely and let Knowledge Graph rebuild its index.
