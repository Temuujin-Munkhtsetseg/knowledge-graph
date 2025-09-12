import os
import json
import subprocess
from pathlib import Path

from src.harness.swe_bench import SweBenchFixtureMetadata
from src.utils import TomlConfig

from src.constants import FIXTURES_METADATA_PATH
LOCAL = os.getenv("LOCAL") == "1"

def index_worktree(gkg_path: str, worktree_path: Path):
    print(f"Indexing worktree from {worktree_path}")
    gkg_output = subprocess.run([gkg_path, "index", worktree_path], capture_output=True)
    print(gkg_output.stderr.decode("utf-8"))
    print(gkg_output.stdout.decode("utf-8"))

def index_worktrees(toml_config: TomlConfig):
    with open(FIXTURES_METADATA_PATH, "r") as f:
        print(f"Loading fixtures metadata from {FIXTURES_METADATA_PATH}")
        fixtures_metadata = json.load(f)
    fixtures = [SweBenchFixtureMetadata.from_dict(f) for f in fixtures_metadata]
    worktrees_paths = set([f.worktree_path for f in fixtures])
    for worktree_path in worktrees_paths:
        index_worktree(toml_config.pipeline.gkg_path, worktree_path)
    print(f"Indexed {len(worktrees_paths)} worktrees")
    print(f"Finished indexing worktrees")
