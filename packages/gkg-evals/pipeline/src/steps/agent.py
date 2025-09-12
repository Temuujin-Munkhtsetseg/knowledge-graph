import json
from random import shuffle
from concurrent.futures import ThreadPoolExecutor, as_completed

from src.utils import TomlConfig
# from src.opencode.opencode import Opencode, OpencodeRunSessionData
from src.harness.swe_bench import SweBenchFixtureMetadata
from src.steps.download import create_worktree

from src.constants import FIXTURES_METADATA_PATH, PATCHES_PATH, SESSION_DATA_PATH

# MULTISWEBENCH
# from src.harness.multiswebench import MultiSweBenchFixtureMetadata, MultiSweBenchPatch


def run_agent(toml_config: TomlConfig):
    pass
