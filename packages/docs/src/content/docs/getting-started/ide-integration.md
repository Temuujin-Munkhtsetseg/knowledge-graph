---
title: IDE Integration
description: Access your gkg instance directly from your editor.
sidebar:
  order: 4
---

GitLab Knowledge Graph integrates with GitLab Duo on JetBrains and VSCode editors. For projects
with [Agentic Chat](https://docs.gitlab.com/user/gitlab_duo_chat/agentic_chat/) enabled,
gkg exposes its tools over the [MCP](https://modelcontextprotocol.io/).

### Extension download

To get started with your preferred IDE, install the GitLab extension:

- [GitLab Workflow](https://marketplace.visualstudio.com/items?itemName=GitLab.gitlab-workflow) for VSCode
- [GitLab Duo](https://plugins.jetbrains.com/plugin/22325-gitlab-duo) for JetBrains

### GKG setup

Ensure your gkg [installation](/getting-started/install) is on your PATH:

```bash
gkg -V
```

If you built gkg from source or downloaded it manually, add its location to your PATH.

### IDE setup

❗ In the GitLab extension settings, enable **"Skip integrity checks"**. This is temporary and will be removed in future releases.

Restart your IDE.

You should see a new "Knowledge Graph" panel in your IDE. If you don't:

- In JetBrains go to **View -> Tool Windows -> Knowledge Graph**
- In VS Code press `Cmd + Shift + P` to open command palette and search for **Show Gitlab Knowledge Graph**

### Troubleshooting

If you encounter issues with gkg in the extension, enable debug logs and check them:

- [Troubleshooting the GitLab Workflow extension for VS Code](https://docs.gitlab.com/editor_extensions/visual_studio_code/troubleshooting/)
- [Troubleshooting JetBrains](https://docs.gitlab.com/editor_extensions/jetbrains_ide/jetbrains_troubleshooting/)

You can also verify indexing by running the [index command](/cli/index-cmd) and then [starting](/cli/server) a gkg server from your terminal.
