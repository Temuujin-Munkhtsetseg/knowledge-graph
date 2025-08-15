---
title: Debug
description: Debugging the gkg server and integrating with your IDE.
sidebar:
  order: 3
---

### CLI debugging

VS Code `.vscode/launch.json` configuration already contains the settings to debug the `gkg index` command.

1. Adjust the path to the workspace you want to index while debugging in `.vscode/launch.json`.
2. Set breakpoints if needed.
3. Open **Run and Debug** window (`Shift+CMD+D`) and select **Debug gkg index**.

Indexing the specified project will start with the debugger attached.

### Server debugging

You can also debug MCP or HTTP calls made from external clients, e.g., [IDE Integration](/getting-started/ide-integration).
To do this:

1. Make sure no `gkg` servers are already running.
2. Set breakpoints if needed.
3. Open **Run and Debug** window (`Shift+CMD+D`) and select **Debug server**.
4. Trigger breakpoints by sending HTTP requests or by starting your IDE with `gkg` integration.

Note: `gkg` should be on the PATH for the IDE's Language Server to connect to it.

The above works because `gkg` ensures only a single instance exists on a system. By starting the `gkg` server in debug mode, you
ensure that all clients will connect to your instance with the debugger attached.
