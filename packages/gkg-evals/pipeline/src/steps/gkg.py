import os
import json
import subprocess
import time
import requests
from pathlib import Path

from src.harness.swe_bench import SweBenchFixtureMetadata
from src.utils import TomlConfig

LOCAL = os.getenv("LOCAL") == "1"

def start_gkg_server(gkg_path: str) -> int:
    print(f"Starting GKG server")
    gkg_output = subprocess.run([gkg_path, "server", "start", "--detached"], capture_output=True)
    parsed_output = json.loads(gkg_output.stdout.decode("utf-8"))
    return parsed_output["port"]

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
    gkg_output = subprocess.run([gkg_path, "server", "stop"], capture_output=True)
    print(gkg_output.stderr.decode("utf-8"))
    print(gkg_output.stdout.decode("utf-8"))
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
    gkg_clean(toml_config.pipeline.gkg_path)
    for worktree_path in worktrees_paths:
        index_worktree(toml_config.pipeline.gkg_path, worktree_path)
    print(f"Indexed {len(worktrees_paths)} worktrees")
    print(f"Finished indexing worktrees")
