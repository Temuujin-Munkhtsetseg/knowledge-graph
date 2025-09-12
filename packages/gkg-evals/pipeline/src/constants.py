import os
from pathlib import Path

from dotenv import load_dotenv

load_dotenv()
LOCAL = os.getenv("LOCAL") == "1"

# These paths are relative to the gkg-evals subdirectory
FIXTURES_METADATA_PATH = Path("./../data/fixtures_metadata.json").absolute() if LOCAL else Path("/app/data/fixtures_metadata.json")
FIXTURES_DIR_PATH = Path("./../data/fixtures").absolute() if LOCAL else Path("/app/data/fixtures")
PATCHES_PATH = Path("./../data/patches.jsonl").absolute() if LOCAL else Path("/app/data/patches.jsonl")
SESSION_DATA_PATH = Path("./../data/session_data.jsonl").absolute() if LOCAL else Path("/app/data/session_data.jsonl")
OPENCODE_LOGS_PATH = Path("./../data/opencode_logs.txt").absolute()
OPENCODE_CONFIG_PATH = Path("./../data/opencode_config.json").absolute() # this is just for logging

# MultiSweBench
BASE_DIR_MULTISWEBENCH = Path("./../data/repos/multi-swe-bench").absolute() if LOCAL else Path("/app/data/repos/multi-swe-bench")
MULTISWEBENCH_CONFIG_PATH = Path("./../data/multiswebench_config.json").absolute().resolve()
MULTISWEBENCH_OUTPUT_DIR = Path("./../data/multiswebench_output").absolute().resolve()
MULTISWEBENCH_LOCATION = Path("./../harness/multi-swe-bench").absolute().resolve()
MULTISWEBENCH_WORKDIR = Path("./../data/evals_workdir").absolute().resolve()

# SweBench
BASE_DIR_SWEBENCH = Path("./../data/repos/swebench").absolute() if LOCAL else Path("/app/data/repos/swebench")
SWEBENCH_LOCATION = Path("./../harness/SWE-bench").absolute().resolve()
SWEBENCH_FIXTURES_DIR_PATH = Path("./../data/fixtures/swebench").absolute().resolve()
SWEBENCH_PATCHES_PATH = Path("./../data/swebench_patches.jsonl").absolute().resolve()
SWEBENCH_REPORT_DIR = Path("./../data/swebench_report").absolute().resolve()

# Executables, relative to the gkg-evals subdirectory
GKG_PATH = Path("./../../../target/release/gkg").absolute()

# OpenCode
OPENCODE_BASE_PATH = Path("~/.local/share/opencode").expanduser().absolute()
OPENCODE_AUTH_PATH = OPENCODE_BASE_PATH / "auth.json"
OPENCODE_STORAGE_PATH = OPENCODE_BASE_PATH / "storage"
OPENCODE_MESSAGES_PATH = OPENCODE_STORAGE_PATH / "message"
OPENCODE_MESSAGE_PARTS_PATH = OPENCODE_STORAGE_PATH / "part"
