import json
from random import shuffle
from concurrent.futures import ThreadPoolExecutor, as_completed

from src.utils import TomlConfig
from src.opencode.opencode import Opencode, OpencodeRunSessionData
from src.harness.swe_bench import SweBenchFixtureMetadata
from src.steps.download import create_worktree

from src.constants import FIXTURES_METADATA_PATH, PATCHES_PATH, SESSION_DATA_PATH

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
        
        with open(FIXTURES_METADATA_PATH, "r") as f:
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
        batch_size = toml_config.pipeline.batch_size
        for batch_start in range(0, len(fixtures), batch_size):
            batch_end = min(batch_start + batch_size, len(fixtures))
            batch_fixtures = fixtures[batch_start:batch_end]
            print(f"Processing batch {batch_start // batch_size + 1}: fixtures {batch_start + 1}-{batch_end}")
            with ThreadPoolExecutor(max_workers=batch_size) as executor:
                future_to_fixture = {
                    executor.submit(process_fixture, opencode, f): f
                    for f in batch_fixtures
                }
                for future in as_completed(future_to_fixture):
                    fixture = future_to_fixture[future]
                    _session_data, error = future.result()
                    
                    if error is None and _session_data is not None:
                        print(f"Successfully completed patch for {fixture.org}/{fixture.repo}#{fixture.base_commit}")
                        session_data.append(_session_data)
                    else:
                        print(f"Skipping fixture {fixture.org}/{fixture.repo}#{fixture.base_commit} due to error: {error}")
            
            print(f"Completed batch {batch_start // batch_size + 1}")
            break
        
        print(f"Successfully processed {len(session_data)} out of {len(fixtures)} fixtures")

        with open(PATCHES_PATH, "w") as f:
           for session in session_data:
               f.write(json.dumps(session.patch.to_dict()) + "\n")

        with open(SESSION_DATA_PATH, "w") as f:
            for session in session_data:
                f.write(json.dumps(session.to_dict()) + "\n")

    except Exception as e:
        print(e)

