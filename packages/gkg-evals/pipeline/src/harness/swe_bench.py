import os
import sys
import json
import subprocess
from pathlib import Path
from dataclasses import dataclass, field

from utils import TomlConfig

import datasets

from src.constants import SWEBENCH_LOCATION, SWEBENCH_PATCHES_PATH, SWEBENCH_FIXTURES_DIR_PATH, SWEBENCH_REPORT_DIR

@dataclass
class SweBenchFixtureMetadata:
    org: str
    repo: str
    instance_id: str
    base_commit: str
    problem_statement: str
    repo_path: Path = field(default_factory=lambda: None)
    worktree_path: Path = field(default_factory=lambda: None)

    def add_worktree_path(self, worktree_path: Path):
        self.worktree_path = worktree_path
    
    def add_repo_path(self, repo_path: Path):
        self.repo_path = repo_path

    def to_dict(self):
        return {
            "org": self.org,
            "repo": self.repo,
            "instance_id": self.instance_id,
            "base_commit": self.base_commit,
            "problem_statement": self.problem_statement,
            "repo_path": str(self.repo_path.absolute()) if self.repo_path else None,
            "worktree_path": str(self.worktree_path.absolute()) if self.worktree_path else None,
        }
    
    @classmethod
    def from_dict(cls, d: dict):
        return cls(
            org=d.get("org"),
            repo=d.get("repo"),
            instance_id=d.get("instance_id"),
            base_commit=d.get("base_commit"),
            problem_statement=d.get("problem_statement"),
            repo_path=Path(d.get("repo_path")) if d.get("repo_path") else None,
            worktree_path=Path(d.get("worktree_path")) if d.get("worktree_path") else None,
        )

@dataclass
class SweBenchPatch:
    instance_id: str
    model_name_or_path: str
    model_patch: str

    def pprint(self):
        print(json.dumps(self.to_dict(), indent=4))

    def to_dict(self):
        return {
            "instance_id": self.instance_id,
            "model_name_or_path": self.model_name_or_path,
            "model_patch": self.model_patch,
        }
    
    @classmethod
    def from_dict(cls, d: dict):
        return cls(
            instance_id=d.get("instance_id"),
            model_name_or_path=d.get("model_name_or_path"),
            model_patch=d.get("model_patch"),
        )

def get_swebench_lite_dataset(split: str = "dev"):
    os.makedirs(SWEBENCH_FIXTURES_DIR_PATH, exist_ok=True)
    ds = datasets.load_dataset("SWE-bench/SWE-bench_Lite", cache_dir=SWEBENCH_FIXTURES_DIR_PATH)
    ds = ds[split]
    return ds


@dataclass
class SweBenchConfig:
    dataset_name: str = "princeton-nlp/SWE-bench_Lite"
    predictions_path: str = SWEBENCH_PATCHES_PATH
    max_workers: int = 8
    run_id: str = "my_evaluation_run"
    split: str = "dev"
    namespace: str = "none"
    force_rebuild: bool = False
    report_dir: str = SWEBENCH_REPORT_DIR.absolute().resolve().__str__()

    def to_subprocess_args(self):
        return [
            "--dataset_name", self.dataset_name,
            "--predictions_path", self.predictions_path,
            "--split", self.split,
            "--namespace", self.namespace,
            "--force_rebuild", str(self.force_rebuild),
            "--max_workers", str(self.max_workers),
            "--run_id", self.run_id,
            "--report_dir", self.report_dir,
        ]

def run_swebench(config: SweBenchConfig, toml_config: TomlConfig):
    # https://github.com/SWE-bench/SWE-bench?tab=readme-ov-file#-usage
    cwd = SWEBENCH_LOCATION.absolute().resolve().__str__()
    command = [sys.executable, "-m", "swebench.harness.run_evaluation", *config.to_subprocess_args()]
    print(f"Running swebench evaluation command: {' '.join(command)}")
    subprocess.run(command, cwd=cwd)
