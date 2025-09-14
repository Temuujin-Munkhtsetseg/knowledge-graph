import json
from random import shuffle
from concurrent.futures import ThreadPoolExecutor, as_completed

from src.utils import TomlConfig, batch_list
from src.opencode.opencode import Opencode, OpencodeRunSessionData
from src.harness.swe_bench import SweBenchFixtureMetadata
from src.steps.download import create_worktree
from src.steps.gkg import start_gkg_server, stop_gkg_server, gkg_server_healthy

# MULTISWEBENCH
# from src.harness.multiswebench import MultiSweBenchFixtureMetadata, MultiSweBenchPatch

def process_fixture(opencode: Opencode, fixture: SweBenchFixtureMetadata) -> tuple[OpencodeRunSessionData, Exception]:
    """Process a single fixture and return the results."""
    try:
        print(f"Processing fixture {fixture.org}/{fixture.repo}#{fixture.base_commit}")
        session_data = opencode.run(fixture)
        print(f"Completed fixture {fixture.org}/{fixture.repo}#{fixture.base_commit}")
        return session_data, None
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(f"Error processing fixture {fixture.org}/{fixture.repo}#{fixture.base_commit}: {e}")
        return None, e


def run_agent(toml_config: TomlConfig):
    try:
        opencode = Opencode(toml_config=toml_config)
        session_data: list[OpencodeRunSessionData] = []
        
        fixtures_metadata_path = toml_config.pipeline.session_paths.fixtures_metadata_path
        with open(fixtures_metadata_path, "r") as f:
            fixtures_metadata = json.load(f)
        fixtures = [SweBenchFixtureMetadata.from_dict(f) for f in fixtures_metadata]
        shuffle(fixtures)
        print(f"Total fixtures to process: {len(fixtures)}")

        # In case you are re-running this after the evals phase:
        for fixture in fixtures:
            success, worktree_path = create_worktree(fixture)
            if success:
                print(f"Created worktree: {worktree_path}")
                fixture.add_worktree_path(worktree_path)
                repo_path = worktree_path.parent.parent
                fixture.add_repo_path(repo_path)

        # Process fixtures in batches (batch_size defined in the config)
        batches = batch_list(fixtures, toml_config.pipeline.batch_size)
        current_batch = 0
        for batch in batches:
            gkg_port = start_gkg_server(toml_config.pipeline.gkg_path)
            if not gkg_server_healthy(gkg_port):
                raise RuntimeError("GKG server failed to start")
            else:
                print(f"GKG server started on port {gkg_port} for batch {current_batch+1}")
            with ThreadPoolExecutor(max_workers=toml_config.pipeline.batch_size) as executor:
                future_to_fixture = {
                    executor.submit(process_fixture, opencode, f): f
                    for f in batch
                }
                for future in as_completed(future_to_fixture):
                    fixture : SweBenchFixtureMetadata = future_to_fixture[future]
                    _session_data, error = future.result()
                    if error is None and _session_data is not None:
                        print(f"Successfully completed patch for {fixture.org}/{fixture.repo}#{fixture.base_commit}")
                        session_data.append(_session_data)
                    else:
                        print(f"Skipping fixture {fixture.org}/{fixture.repo}#{fixture.base_commit} due to error: {error}")
            print(f"Completed batch {current_batch+1}")
            if toml_config.pipeline.break_after_first_batch:
                break
            current_batch += 1
            stop_gkg_server(toml_config.pipeline.gkg_path)
        
        print(f"Successfully processed {len(session_data)} out of {len(fixtures)} fixtures")

        patches_path = toml_config.pipeline.session_paths.swe_bench_patches_path
        with open(patches_path, "w") as f:
           for session in session_data:
               f.write(json.dumps(session.patch.to_dict()) + "\n")

        session_data_path = toml_config.pipeline.session_paths.session_data_path
        with open(session_data_path, "w") as f:
            for session in session_data:
                f.write(json.dumps(session.to_dict()) + "\n")

    except Exception as e:
        print(e)

