
OPENCODE_MAX_TIME = 240
OPENCODE_TIME_ELAPSED_DEBOUNCE = 5

# https://opencode.ai/docs/agents/#available-tools
OPENCODE_MUTATING_TOOLS = {"edit", "patch", "todowrite", "write", "bash"}  # added todowrite and write, bash
OPENCODE_ALL_DEFAULT_TOOLS = {"bash", "edit", "write", "read", "grep", "glob", "list", "patch", "todowrite", "todoread", "webfetch"}

# This is just here for reference, but currently deprecated
OPENCODE_DEFAULT_MCP = {
    "knowledge-graph": {
        "type": "remote",
        "url": "http://localhost:27495/mcp",
        "enabled": True
    }
}

OPENCODE_VERSION = "0.6.4"
