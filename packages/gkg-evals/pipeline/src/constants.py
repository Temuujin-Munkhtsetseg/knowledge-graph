import os
from pathlib import Path

from dotenv import load_dotenv

ENV_PATH = Path("../.env").absolute()
load_dotenv(ENV_PATH)
LOCAL = os.getenv("LOCAL") == "1"

# Global dirs (relative to the gkg-evals/pipeline subdirectory)
GKG_EVALS_DIR = Path("./../").absolute()
DATA_DIR = Path("./../data").absolute()
RUNS_DIR = DATA_DIR / "runs"

# These paths are relative to a gkg-evals/pipeline/runs/<session_name> subdirectory
FIXTURES_METADATA_PATH = "fixtures_metadata.json"
FIXTURES_DIR_PATH = "fixtures"
SESSION_DATA_PATH = "session_data.jsonl"
OPENCODE_LOGS_PATH = "opencode_logs.txt"
OPENCODE_CONFIG_PATH = "opencode_config.json" # this is just for logging

# MultiSweBench (These are outdated for now, but do not change them)
BASE_DIR_MULTISWEBENCH = DATA_DIR / "repos/multi-swe-bench"
MULTISWEBENCH_CONFIG_PATH = DATA_DIR / "multiswebench_config.json"
MULTISWEBENCH_OUTPUT_DIR = DATA_DIR / "multiswebench_output"
MULTISWEBENCH_LOCATION = DATA_DIR / "harness/multi-swe-bench"
MULTISWEBENCH_WORKDIR = DATA_DIR / "evals_workdir"

# SweBench
SWEBENCH_HARNESS_LOCATION_DIR = "harness/SWE-bench"
BASE_REPOS_DIR_SWEBENCH = "repos/swebench"
SWEBENCH_FIXTURES_DIR_PATH = "fixtures/swebench"
SWEBENCH_PATCHES_PATH = "swebench_patches.jsonl"
SWEBENCH_REPORT_DIR = "swebench_report"

# Executables, relative to the gkg-evals subdirectory
GKG_PATH_RELEASE = Path("./../../../target/release/gkg").absolute()
GKG_PATH_DEBUG = Path("./../../../target/debug/gkg").absolute()

# OpenCode
OPENCODE_BASE_PATH = Path("~/.local/share/opencode").expanduser().absolute()
OPENCODE_AUTH_PATH = OPENCODE_BASE_PATH / "auth.json"
OPENCODE_STORAGE_PATH = OPENCODE_BASE_PATH / "storage"
OPENCODE_MESSAGES_PATH = OPENCODE_STORAGE_PATH / "message"
OPENCODE_MESSAGE_PARTS_PATH = OPENCODE_STORAGE_PATH / "part"
