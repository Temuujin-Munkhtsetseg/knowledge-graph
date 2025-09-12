import sys
import json
import subprocess
from pathlib import Path
from typing import List
from collections import defaultdict
from dataclasses import dataclass, field

import orjson

from src.constants import MULTISWEBENCH_CONFIG_PATH, MULTISWEBENCH_OUTPUT_DIR, MULTISWEBENCH_LOCATION, BASE_DIR_MULTISWEBENCH, MULTISWEBENCH_WORKDIR, FIXTURES_DIR_PATH
FIX_PATCH_RUN_CMD = "bash -c \"apt update && apt install -y patch && sed -i 's@git apply /home/test.patch /home/fix.patch@patch --batch --fuzz=5 -p1 -i /home/test.patch;patch --batch --fuzz=5 -p1 -i /home/fix.patch@g' /home/fix-run.sh && bash /home/fix-run.sh\""

@dataclass
class MultiSweBenchFixtureIssue:
    number: int
    title: str
    body: str

    def to_dict(self):
        return {"number": self.number, "title": self.title, "body": self.body}

    @classmethod
    def from_dict(cls, d: dict):
        return cls(
            number=d.get("number"),
            title=d.get("title"),
            body=d.get("body")
        )


@dataclass
class MultiSweBenchFixtureMetadata:
    org: str
    repo: str
    number: int
    label: str
    ref: str
    sha: str
    repo_path: Path = field(default_factory=lambda: None)
    worktree_path: Path = field(default_factory=lambda: None)
    resolved_issues: list[MultiSweBenchFixtureIssue] = field(default_factory=lambda: [])

    def add_worktree_path(self, worktree_path: Path):
        self.worktree_path = worktree_path

    def add_repo_path(self, repo_path: Path):
        self.repo_path = repo_path

    def to_dict(self):
        return {
            "org": self.org,
            "repo": self.repo,
            "number": self.number,
            "label": self.label,
            "ref": self.ref,
            "sha": self.sha,
            "repo_path": str(self.repo_path.absolute()) if self.repo_path else None,
            "worktree_path": str(self.worktree_path.absolute()) if self.worktree_path else None,
            "resolved_issues": [issue.to_dict() for issue in self.resolved_issues],
        }

    @classmethod
    def from_dict(cls, d: dict):
        return cls(
            org=d.get("org"),
            repo=d.get("repo"),
            number=d.get("number"),
            label=d.get("label"),
            ref=d.get("ref"),
            sha=d.get("sha"),
            repo_path=Path(d.get("repo_path")) if d.get("repo_path") else None,
            worktree_path=Path(d.get("worktree_path")) if d.get("worktree_path") else None,
            resolved_issues=[MultiSweBenchFixtureIssue.from_dict(issue) for issue in d.get("resolved_issues", [])]
        )

def get_fixtures_metadata() -> dict[str, list[MultiSweBenchFixtureIssue]]:
    fixtures_metadata = defaultdict(list)
    for file in FIXTURES_DIR_PATH.glob("**/*.jsonl"):
        with open(file, "r") as f:
            print(file.name)
            lines = f.readlines()
            for line in lines:
                data = orjson.loads(line)
                base = data.get("base")
                resolved_issues_raw = data.get("resolved_issues", [])
                resolved_issues = []
                for resolved_issue_raw in resolved_issues_raw:
                    resolved_issues.append(
                        MultiSweBenchFixtureIssue(
                            number=resolved_issue_raw.get("number"),
                            title=resolved_issue_raw.get("title"),
                            body=resolved_issue_raw.get("body"),
                        )
                    )

                fixtures_metadata[file.name].append(
                    MultiSweBenchFixtureMetadata(
                        org=data.get("org"),
                        repo=data.get("repo"),
                        number=data.get("number"),
                        label=base.get("label"),
                        ref=base.get("ref"),
                        sha=base.get("sha"),
                        resolved_issues=resolved_issues,
                    )
                )
    return fixtures_metadata


@dataclass
class MultiSweBenchConfig:
    mode: str = "evaluation"
    workdir: str = MULTISWEBENCH_WORKDIR.absolute().__str__()
    patch_files: List[str] = field(default_factory=list)
    dataset_files: List[str] = field(default_factory=list)
    force_build: bool = False
    output_dir: str = MULTISWEBENCH_OUTPUT_DIR.absolute().__str__()
    specifics: List[str] = field(default_factory=list)
    skips: List[str] = field(default_factory=list)
    repo_dir: str = BASE_DIR_MULTISWEBENCH.absolute().__str__()
    need_clone: bool = False
    global_env: List[str] = field(default_factory=list)
    clear_env: bool = True
    stop_on_error: bool = True
    max_workers: int = 8
    max_workers_build_image: int = 8
    max_workers_run_instance: int = 8
    log_dir: str = "./data/logs"
    log_level: str = "DEBUG"
    fix_patch_run_cmd: str = FIX_PATCH_RUN_CMD

    def add_patch_file(self, patch_file: str):
        self.patch_files.append(patch_file)
    
    def add_dataset_file(self, dataset_file: str):
        self.dataset_files.append(dataset_file)

    def pprint(self):
        print(json.dumps(self.to_dict(), indent=4))

    def to_dict(self):
        return {
            "mode": self.mode,
            "workdir": self.workdir,
            "patch_files": self.patch_files,
            "dataset_files": self.dataset_files,
            "force_build": self.force_build,
            "output_dir": self.output_dir,
            "specifics": self.specifics,
            "skips": self.skips,
            "repo_dir": self.repo_dir,
            "need_clone": self.need_clone,
            "global_env": self.global_env,
            "clear_env": self.clear_env,
            "stop_on_error": self.stop_on_error,
            "max_workers": self.max_workers,
            "max_workers_build_image": self.max_workers_build_image,
            "max_workers_run_instance": self.max_workers_run_instance,
            "log_dir": self.log_dir,
            "log_level": self.log_level,
            "fix_patch_run_cmd": self.fix_patch_run_cmd
        }

    @classmethod
    def from_dict(cls, d: dict):
        return cls(
            mode=d.get("mode"),
            workdir=d.get("workdir"),
            patch_files=d.get("patch_files", []),
            dataset_files=d.get("dataset_files", []),
            force_build=d.get("force_build", False),
            output_dir=d.get("output_dir"),
            specifics=d.get("specifics", []),
            skips=d.get("skips", []),
            repo_dir=d.get("repo_dir", "./data/repos"),
            need_clone=d.get("need_clone", False),
            global_env=d.get("global_env", []),
            clear_env=d.get("clear_env", True),
            stop_on_error=d.get("stop_on_error", True),
            max_workers=d.get("max_workers", 8),
            max_workers_build_image=d.get("max_workers_build_image", 8),
            max_workers_run_instance=d.get("max_workers_run_instance", 8),
            log_dir=d.get("log_dir", "./data/logs"),
            log_level=d.get("log_level", "DEBUG"),
            fix_patch_run_cmd=d.get("fix_patch_run_cmd", FIX_PATCH_RUN_CMD)
        )

@dataclass
class MultiSweBenchPatch:
    org: str
    repo: str
    number: int
    fix_patch: str

    def to_dict(self):
        return {"org": self.org, "repo": self.repo, "number": self.number, "fix_patch": self.fix_patch}

    @classmethod
    def from_dict(cls, d: dict):
        return cls(
            org=d.get("org"),
            repo=d.get("repo"),
            number=d.get("number"),
            fix_patch=d.get("fix_patch")
        )

def run_multiswebench(config: MultiSweBenchConfig):
    cwd = MULTISWEBENCH_LOCATION.absolute().resolve().__str__()
    MULTISWEBENCH_WORKDIR.mkdir(parents=True, exist_ok=True)
    with open(MULTISWEBENCH_CONFIG_PATH, "w") as f:
        json.dump(config.to_dict(), f)
    print(f"Running multiswebench with config: {config.pprint()}")
    subprocess.run([sys.executable, "-m", "multi_swe_bench.harness.run_evaluation", "--config", MULTISWEBENCH_CONFIG_PATH], cwd=cwd)
