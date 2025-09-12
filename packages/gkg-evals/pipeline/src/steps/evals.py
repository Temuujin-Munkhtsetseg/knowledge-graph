import json

from src.steps.download import remove_worktrees
from src.harness.swe_bench import SweBenchFixtureMetadata, SweBenchConfig, run_swebench
from src.harness.multi_swe_bench import MultiSweBenchFixtureMetadata, MultiSweBenchConfig, run_multiswebench
from src.utils import TomlConfig

from src.constants import PATCHES_PATH, FIXTURES_DIR_PATH, FIXTURES_METADATA_PATH

def run_evals_multiswebench(toml_config: TomlConfig):
    try:
        # load fixtures metadata + remove worktrees
        with open(FIXTURES_METADATA_PATH, "r") as f:
            fixtures_md = json.load(f)
        fixtures = [MultiSweBenchFixtureMetadata.from_dict(fixtures_md) for fixtures_md in fixtures_md]
        remove_worktrees(fixtures)


        patch_files = [PATCHES_PATH.absolute().resolve().__str__()]
        print("patch_files:")
        for pf in patch_files:
            print(pf)

        dataset_files = [fixture_file.absolute().resolve().__str__() for fixture_file in FIXTURES_DIR_PATH.glob("**/*.jsonl")]
        print("dataset_files:")
        for df in dataset_files:
            print(df)

        run_multiswebench(MultiSweBenchConfig(
            patch_files=patch_files,
            dataset_files=dataset_files,
            force_build=toml_config.get("multiswebench", {}).get("force_build", False)
        ))
        print("Evals completed successfully!")
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(e)


def run_evals_swebench(toml_config: TomlConfig):
    try:
        with open(FIXTURES_METADATA_PATH, "r") as f:
            fixtures_md = json.load(f)
        fixtures = [SweBenchFixtureMetadata.from_dict(fixtures_md) for fixtures_md in fixtures_md]
        remove_worktrees(fixtures)

        predictions_path = PATCHES_PATH.absolute().resolve().__str__()
        print(f"predictions_path: {predictions_path}")
        with open(predictions_path, "r") as f:
            patches = f.readlines()
            for patch in patches:
                print(patch)

        swebench_config = SweBenchConfig(
            predictions_path=predictions_path,
        )
        run_swebench(swebench_config, toml_config)
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(e)
