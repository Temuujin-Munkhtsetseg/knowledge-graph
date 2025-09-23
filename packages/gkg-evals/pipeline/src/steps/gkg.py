import os
import json
import subprocess
import time
import requests
from pathlib import Path

from src.harness.swe_bench import SweBenchFixtureMetadata
from src.utils import TomlConfig
from src.steps.download import create_worktree

LOCAL = os.getenv("LOCAL") == "1"

def start_gkg_server(toml_config: TomlConfig) -> int | None:
    print(f"Starting GKG server")
    mcp_configuration_path = toml_config.pipeline.session_paths.mcp_configuration_path

    # Write the MCP configuration to the file
    with open(mcp_configuration_path, "w") as f:
        mcp_conf_gkg = {
            "disabled_tools": toml_config.opencode.mcp.disabled_tools
        }
        mcp_config_json = json.dumps(mcp_conf_gkg, indent=4)
        f.write(mcp_config_json)
        print(f"Wrote MCP configuration to {mcp_configuration_path}")
        print(f"MCP configuration: {mcp_config_json}")
    
    args = [
        toml_config.pipeline.gkg_path, 
        "server", 
        "start", 
        "--detached",
        "--mcp-configuration-path",
        mcp_configuration_path.absolute()
    ]
    try:
        gkg_output = subprocess.run(args, capture_output=True)
        stdout = gkg_output.stdout.decode("utf-8")
        stderr = gkg_output.stderr.decode("utf-8")
        print(f"GKG server start --detached stderr: {stderr}")
        print(f"GKG server start --detached stdout: {stdout}")
        parsed_output = json.loads(stdout)
        return parsed_output.get("port", None)
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(f"Error starting GKG server: {e}")
        print(f"Attempting to stop GKG server after failed start")
        stop_gkg_server(toml_config.pipeline.gkg_path)
        return None

def gkg_server_healthy(gkg_port: int, attempts: int = 10) -> bool:
    url = f"http://localhost:{gkg_port}/health"
    print(f"Checking if GKG server is healthy at {url}")
    for i in range(attempts):
        try:
            response = requests.get(url, timeout=2)
            if response.status_code == 200:
                return True
            time.sleep(3)
        except requests.RequestException:
            # import traceback
            # traceback.print_exc()
            time.sleep(3)
            continue
    return False

def stop_gkg_server(gkg_path: str):
    print(f"Stopping GKG server")
    gkg_output = subprocess.run([gkg_path, "server", "stop"], capture_output=True)
    print(f"GKG server stop stderr: {gkg_output.stderr.decode("utf-8")}")
    print(f"GKG server stop stdout: {gkg_output.stdout.decode("utf-8")}")
    time.sleep(3)

def gkg_clean(gkg_path: str):
    print(f"Cleaning GKG")
    gkg_output = subprocess.run([gkg_path, "clean"], capture_output=True)
    print(gkg_output.stderr.decode("utf-8"))
    print(gkg_output.stdout.decode("utf-8"))


def index_worktree(gkg_path: str, worktree_path: Path):
    print(f"Indexing worktree from {worktree_path}")
    gkg_output = subprocess.run([gkg_path, "index", worktree_path], capture_output=True)
    print(gkg_output.stderr.decode("utf-8"))
    print(gkg_output.stdout.decode("utf-8"))


def index_worktrees(toml_config: TomlConfig):
    fixtures_metadata_path = toml_config.pipeline.session_paths.fixtures_metadata_path
    with open(fixtures_metadata_path, "r") as f:
        print(f"Loading fixtures metadata from {fixtures_metadata_path}")
        fixtures_metadata = json.load(f)
    fixtures = [SweBenchFixtureMetadata.from_dict(f) for f in fixtures_metadata]
    worktrees_paths = set([f.worktree_path for f in fixtures])

    # In case you are re-running this after the evals phase:
    for fixture in fixtures:
        success, worktree_path = create_worktree(fixture)
        if success:
            print(f"Created worktree: {worktree_path}")
            fixture.add_worktree_path(worktree_path)
            repo_path = worktree_path.parent.parent
            fixture.add_repo_path(repo_path)

    gkg_clean(toml_config.pipeline.gkg_path)
    for worktree_path in worktrees_paths:
        index_worktree(toml_config.pipeline.gkg_path, worktree_path)
    print(f"Indexed {len(worktrees_paths)} worktrees")
    print(f"Finished indexing worktrees")
