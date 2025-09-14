---
title: Configuration
description: Documentation for the gkg MCP configuration.
sidebar:
  order: 3
---

The `gkg` server alows for users to manage some MCP server behaviours.

The server looks for the `mcp.settings.json` file in the `gkg` data directory, which is typically located at `~/.gkg/mcp.settings.json`. If the file doesn't exist, `gkg` will create a default version when it starts up.

You can also specify a custom path to the configuration file by using the `--mcp-configuration-path` command-line argument when launching the `gkg` server.

### Structure

The `mcp.settings.json` file has a simple structure:

```json
{
  "disabled_tools": ["tool_name_1", "tool_name_2"]
}
```

- `disabled_tools`: An array of strings, where each string is the name of a tool to disable.
