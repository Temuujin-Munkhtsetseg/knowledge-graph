import json

from src.steps.download import remove_worktrees
from src.harness.swe_bench import SweBenchFixtureMetadata, SweBenchConfig, run_swebench, prepare_swebench_images
from src.harness.multi_swe_bench import MultiSweBenchFixtureMetadata, MultiSweBenchConfig, run_multiswebench
from src.utils import TomlConfig

from src.constants import SWEBENCH_PATCHES_PATH, FIXTURES_DIR_PATH

# This is currently deprecated
def run_evals_multiswebench(toml_config: TomlConfig):
    try:
        # load fixtures metadata + remove worktrees
        fixtures_metadata_path = toml_config.pipeline.session_paths.fixtures_metadata_path
        with open(fixtures_metadata_path, "r") as f:
            fixtures_md = json.load(f)
        fixtures = [MultiSweBenchFixtureMetadata.from_dict(fixtures_md) for fixtures_md in fixtures_md]
        remove_worktrees(fixtures)


        patch_files = [SWEBENCH_PATCHES_PATH.absolute().resolve().__str__()]
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
        fixtures_metadata_path = toml_config.pipeline.session_paths.fixtures_metadata_path
        with open(fixtures_metadata_path, "r") as f:
            fixtures_md = json.load(f)
        fixtures = [SweBenchFixtureMetadata.from_dict(fixtures_md) for fixtures_md in fixtures_md]
        remove_worktrees(fixtures)

        predictions_path = toml_config.pipeline.session_paths.swe_bench_patches_path
        predictions_path = predictions_path.absolute().resolve().__str__()

        print(f"predictions_path: {predictions_path}")
        with open(predictions_path, "r") as f:
            patches = f.readlines()
            for patch in patches:
                print(patch)

        report_dir = toml_config.pipeline.session_paths.swe_bench_report_dir
        report_dir = report_dir.absolute().resolve().__str__()

        swebench_config = SweBenchConfig(
            predictions_path=predictions_path,
            report_dir=report_dir,
        )
        prepare_swebench_images(swebench_config, toml_config)
        run_swebench(swebench_config, toml_config)
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(e)
