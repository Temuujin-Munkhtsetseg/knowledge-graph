import os
import sys
import json
import uuid
import subprocess
from pathlib import Path
from dataclasses import dataclass, field

from utils import TomlConfig

import datasets

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

def get_swebench_lite_dataset(toml_config: TomlConfig, split: str = "dev"):
    swe_bench_fixtures_dir_path = toml_config.pipeline.session_paths.swe_bench_fixtures_dir_path
    os.makedirs(swe_bench_fixtures_dir_path, exist_ok=True)
    ds = datasets.load_dataset("SWE-bench/SWE-bench_Lite", cache_dir=swe_bench_fixtures_dir_path)
    ds = ds[split]
    return ds


@dataclass
class SweBenchConfig:
    dataset_name: str = "princeton-nlp/SWE-bench_Lite"
    predictions_path: str = field(default_factory=lambda: None)
    max_workers: int = 8
    run_id: str = "my_evaluation_run"
    split: str = "dev"
    namespace: str = "none"
    force_rebuild: bool = False
    report_dir: str = field(default_factory=lambda: None)

    def to_subprocess_args(self, rand_run_id: bool = True):
        run_id = self.run_id if not rand_run_id else str(uuid.uuid4())
        return [
            "--dataset_name", self.dataset_name,
            "--predictions_path", self.predictions_path,
            "--split", self.split,
            "--namespace", self.namespace,
            "--force_rebuild", str(self.force_rebuild),
            "--max_workers", str(self.max_workers),
            "--run_id", run_id,
            "--report_dir", self.report_dir,
        ]

def prepare_swebench_images(swebench_config: SweBenchConfig, toml_config: TomlConfig):
    cwd = toml_config.pipeline.session_paths.swe_bench_harness_location_dir.absolute().resolve().__str__()
    # TODO:Latest tag is the default tag, but might eventually be problematic for reproducibility reasons
    args = [
        "--dataset_name", swebench_config.dataset_name,
        "--split", swebench_config.split,
        "--max_workers", str(swebench_config.max_workers),
        "--tag", "latest",
    ]
    command = [sys.executable, "-m", "swebench.harness.prepare_images", *args]
    print(f"Running swebench prepare images command: {' '.join(command)}")
    print(f"This will take a while... but you should only need to do this once per dataset + split")
    subprocess.run(command, cwd=cwd)

def clone_swebench_repository(toml_config: TomlConfig):
    cwd = toml_config.pipeline.session_paths.swe_bench_harness_location_dir.absolute().resolve().__str__()
    if Path(cwd).exists():
        print(f"✓ SWE-bench already exists in {cwd} - skipping clone")
        return
    
    print(f"Cloning SWE-bench repository to {cwd}")
    subprocess.run(["git", "clone", "https://github.com/princeton-nlp/SWE-bench.git", cwd], check=True)
    subprocess.run(["git", "checkout", "c7c22a916c9215e709722bc5ab18df4062dc6248"], cwd=cwd)
    subprocess.run(["rm", "-rf", ".git"], cwd=cwd)
    subprocess.run(["pip", "install", "-e", "."], cwd=cwd)
    print(f"✓ SWE-bench setup completed successfully!")
    print("Note: SWE-bench dependencies are managed through uv/pyproject.toml")

def run_swebench(config: SweBenchConfig, toml_config: TomlConfig):
    # https://github.com/SWE-bench/SWE-bench?tab=readme-ov-file#-usage
    cwd = toml_config.pipeline.session_paths.swe_bench_harness_location_dir.absolute().resolve().__str__()
    command = [sys.executable, "-m", "swebench.harness.run_evaluation", *config.to_subprocess_args()]
    print(f"Running swebench evaluation command: {' '.join(command)}")
    subprocess.run(command, cwd=cwd)
