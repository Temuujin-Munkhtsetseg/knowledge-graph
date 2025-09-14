import json
import tomllib
from pathlib import Path
from typing import Callable
from dataclasses import dataclass, field
from concurrent.futures import ThreadPoolExecutor, as_completed 

from src.constants import GKG_PATH_DEBUG

# General helper functions
def batch_list(l: list, batch_size: int):
    for i in range(0, len(l), batch_size):
        yield l[i:i+batch_size]


def run_threaded(func: Callable, items: list, max_workers: int = 10):
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        futures = {executor.submit(func, item): item for item in items}
        for future in as_completed(futures):
            item = futures[future]
            future.result()


### TOML Config Parsing

@dataclass
class TomlSessionPaths:
    fixtures_metadata_path: Path = field(default_factory=lambda: None)
    fixtures_dir_path: Path = field(default_factory=lambda: None)
    session_data_path: Path = field(default_factory=lambda: None)
    opencode_config_path: Path = field(default_factory=lambda: None)
    swe_bench_harness_location_dir: Path = field(default_factory=lambda: None)
    swe_bench_repos_dir: Path = field(default_factory=lambda: None)
    swe_bench_fixtures_dir_path: Path = field(default_factory=lambda: None)
    swe_bench_patches_path: Path = field(default_factory=lambda: None)
    swe_bench_report_dir: Path = field(default_factory=lambda: None)

    def pprint(self):
        print("Fixtures metadata path: ", self.fixtures_metadata_path.absolute())
        print("Fixtures dir path: ", self.fixtures_dir_path.absolute())
        print("Session data path: ", self.session_data_path.absolute())
        print("Opencode config path: ", self.opencode_config_path.absolute())
        print("Swe bench harness location dir: ", self.swe_bench_harness_location_dir.absolute())
        print("Swe bench repos dir: ", self.swe_bench_repos_dir.absolute())
        print("Swe bench fixtures dir path: ", self.swe_bench_fixtures_dir_path.absolute())
        print("Swe bench patches path: ", self.swe_bench_patches_path.absolute())
        print("Swe bench report dir: ", self.swe_bench_report_dir.absolute())

@dataclass
class TomlPipelineConfig:
    skip_download: bool = False
    skip_gkg_index: bool = False
    batch_size: int = 1
    break_after_first_batch: bool = False
    fixture_timeout: int = 240
    gkg_path: str = GKG_PATH_DEBUG
    opencode_logs_stdout: bool = True
    session_name: str = "default"
    session_dir: str = field(default_factory=lambda: None)
    session_paths: TomlSessionPaths = field(default_factory=TomlSessionPaths)

@dataclass
class TomlOpencodeMcpConfig:
    enabled: bool = False
    tools: list[str] = field(default_factory=list)
    url: str = "http://localhost:27495/mcp"
    server_name: str = "knowledge-graph"
    type: str = "remote"

    # Tools are excluded here on purpose, this is for the opencode config only
    def to_dict(self, server_name: str):
        return {
            server_name: {
                "type": self.type,
                "url": self.url,
                "enabled": self.enabled,
            }
        }

@dataclass
class TomlOpencodeLspSettings:
    language: str
    disabled: bool = True

    def to_dict(self):
        return {
            self.language: {
                "disabled": self.disabled,
            }
        }

@dataclass
class TomlOpencodeConfig:
    model: str = "anthropic/claude-sonnet-4-20250514"
    tools: list[str] = field(default_factory=list)
    mcp: TomlOpencodeMcpConfig = field(default_factory=TomlOpencodeMcpConfig)
    lsp: list[TomlOpencodeLspSettings] = field(default_factory=list)
    agent_description: str = ""
    agent_prompt: str = ""
    user_prompt: str = ""
    max_tokens: int = 8192

@dataclass
class TomlEvalsSweBenchConfig:
    dataset_name: str = "princeton-nlp/SWE-bench_Lite"
    predictions_path: str = field(default_factory=lambda: None)
    max_workers: int = 8
    run_id: str = "my_evaluation_run" # TODO: this should be set to to instance_id
    split: str = "dev"
    namespace: str = "none"
    force_rebuild: bool = False
    report_dir: str = field(default_factory=lambda: None)

@dataclass
class TomlEvalsConfig:
    framework: str = "swe-bench"

@dataclass
class TomlConfig:
    pipeline: TomlPipelineConfig
    opencode: TomlOpencodeConfig
    evals: TomlEvalsConfig
    evals_swe_bench: TomlEvalsSweBenchConfig

def load_toml_config(path: str, pprint: bool = False) -> TomlConfig:
    with open(path, "rb") as f:
        toml_config = tomllib.load(f)
        if pprint:
            print("Evals Toml config:")
            print(json.dumps(toml_config, indent=4))
    return TomlConfig(
        pipeline=TomlPipelineConfig(**toml_config["pipeline"]),
        opencode=TomlOpencodeConfig(
            model=toml_config["opencode"]["model"],
            tools=toml_config["opencode"]["tools"],
            user_prompt=toml_config["opencode"]["user_prompt"],
            max_tokens=toml_config["opencode"]["max_tokens"],
            agent_description=toml_config["opencode"]["agent_description"],
            agent_prompt=toml_config["opencode"]["agent_prompt"],
            mcp=TomlOpencodeMcpConfig(**toml_config["opencode"]["mcp"]), 
            lsp=[TomlOpencodeLspSettings(**lsp) for lsp in toml_config["opencode"]["lsp"]]),
        evals=TomlEvalsConfig(framework=toml_config["evals"]["framework"]),
        evals_swe_bench=TomlEvalsSweBenchConfig(**toml_config["evals"]["swe-bench"]),
    )
