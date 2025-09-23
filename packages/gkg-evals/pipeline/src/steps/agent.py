import json
import time
from random import shuffle
from concurrent.futures import ThreadPoolExecutor, as_completed

from src.utils import TomlConfig, batch_list
from src.opencode.opencode import Opencode, OpencodeRunSessionData
from src.harness.swe_bench import SweBenchFixtureMetadata, SweBenchPatch
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

def process_batch(opencode: Opencode, batch: list[SweBenchFixtureMetadata], toml_config: TomlConfig, batch_id: int) -> list[OpencodeRunSessionData]:
    # Make sure the GKG server is running if it's configured for this run
    mcp_conf = opencode.toml_config.opencode.mcp
    if mcp_conf.enabled and mcp_conf.server_name == "knowledge-graph":
        gkg_port = start_gkg_server(toml_config)
        if gkg_port is None:
            for i in range(3):
                gkg_port = start_gkg_server(toml_config)
                if gkg_port is not None:
                    break
                time.sleep(3)
            print(f"GKG server failed to start for batch {batch_id+1}, skipping batch")
            return [], RuntimeError("GKG server failed to start")
        if not gkg_server_healthy(gkg_port):
            return [], RuntimeError("GKG server failed to start")
        else:
            print(f"GKG server started on port {gkg_port} for batch {batch_id+1}")

    # Process the batch
    batch_session_data: list[OpencodeRunSessionData] = []
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
                batch_session_data.append(_session_data)
            else:
                print(f"Skipping fixture {fixture.org}/{fixture.repo}#{fixture.base_commit} due to error: {error}")
    
    print(f"Completed batch {batch_id+1}")
    stop_gkg_server(toml_config.pipeline.gkg_path)

    return batch_session_data, None

def write_session_data(toml_config: TomlConfig, session_data: list[OpencodeRunSessionData]):
    # Filter out killed sessions - they should never be persisted
    successful_session_data = [session for session in session_data if not session.killed]
    killed_count = len(session_data) - len(successful_session_data)
    
    if killed_count > 0:
        print(f"Excluding {killed_count} killed fixtures from being written to files")
    
    patches_path = toml_config.pipeline.session_paths.swe_bench_patches_path
    session_data_path = toml_config.pipeline.session_paths.session_data_path
    mutate_agent_files_mode = "w"

    if patches_path.exists():
        if toml_config.pipeline.reuse_existing_patches or toml_config.pipeline.append_after_batch:
            mutate_agent_files_mode = "a"
            print(f"Appending to existing patches and session data")

    if mutate_agent_files_mode == "w":
        print(f"Writing new patches to {patches_path}")

    with open(patches_path, mutate_agent_files_mode) as f:
        if mutate_agent_files_mode == "w":
            print(f"Writing {len(successful_session_data)} patches to {patches_path}")
        else:
            print(f"Appending {len(successful_session_data)} patches to {patches_path}")
        for session in successful_session_data:
            f.write(json.dumps(session.patch.to_dict()) + "\n")

    with open(session_data_path, mutate_agent_files_mode) as f:
        if mutate_agent_files_mode == "w":
            print(f"Writing {len(successful_session_data)} session data to {session_data_path}")
        else:
            print(f"Appending {len(successful_session_data)} session data to {session_data_path}")
        for session in successful_session_data:
            f.write(json.dumps(session.to_dict()) + "\n")

def remove_killed_fixtures(toml_config: TomlConfig, killed_session_data: list[OpencodeRunSessionData]):
    """Remove killed fixtures from existing files for backwards compatibility."""
    session_data_path = toml_config.pipeline.session_paths.session_data_path

    killed_session_data_instance_ids = {s.fixture.instance_id for s in killed_session_data}
    with open(toml_config.pipeline.session_paths.swe_bench_patches_path, "r") as f:
        patches = [SweBenchPatch.from_dict(json.loads(p)) for p in f.readlines()]
        patches = [p for p in patches if p.instance_id not in killed_session_data_instance_ids]

    with open(toml_config.pipeline.session_paths.swe_bench_patches_path, "w") as f:
        f.truncate(0)
        f.seek(0)
        for patch in patches:
            f.write(json.dumps(patch.to_dict()) + "\n")

    with open(session_data_path, "r") as f:
        session_data = [OpencodeRunSessionData.from_dict(json.loads(s)) for s in f.readlines()]
        session_data = [s for s in session_data if s.fixture.instance_id not in killed_session_data_instance_ids]
    
    with open(session_data_path, "w") as f:
        f.truncate(0)
        f.seek(0)
        for session in session_data:
            f.write(json.dumps(session.to_dict()) + "\n")   


# NOTE: This mutates pipeline state
def get_fixtures_to_process(toml_config: TomlConfig) -> list[SweBenchFixtureMetadata]:
    fixtures_metadata_path = toml_config.pipeline.session_paths.fixtures_metadata_path
    with open(fixtures_metadata_path, "r") as f:
        fixtures_metadata = json.load(f)
    fixtures = [SweBenchFixtureMetadata.from_dict(f) for f in fixtures_metadata]
    shuffle(fixtures)
    print(f"Total fixtures to process: {len(fixtures)}")

    # Check existing patches if we are have the system setting to reuse existing patches
    patches_path = toml_config.pipeline.session_paths.swe_bench_patches_path
    if patches_path.exists() and toml_config.pipeline.reuse_existing_patches:
        print(f"Reusing existing patches from {patches_path}")
        with open(patches_path, "r") as f:
            patches = f.readlines()
        patches = [SweBenchPatch.from_dict(json.loads(p)) for p in patches]
        patches_instance_ids = {p.instance_id for p in patches} 
        fixtures = [f for f in fixtures if f.instance_id not in patches_instance_ids]
        print(f"Found {len(patches)} existing patches, running {len(fixtures)} fixtures")

    # Clean up any existing killed fixtures from previous runs (backwards compatibility)
    session_data_path = toml_config.pipeline.session_paths.session_data_path
    if session_data_path.exists():
        with open(session_data_path, "r") as f:
            existing_session_data = [OpencodeRunSessionData.from_dict(json.loads(s)) for s in f.readlines()]
        killed_session_data = [s for s in existing_session_data if s.killed and s.killed_reason == "error"]
        if killed_session_data:
            print(f"Found {len(killed_session_data)} killed fixtures from previous runs, removing them")
            remove_killed_fixtures(toml_config, killed_session_data)

    fixtures = list({ f.instance_id: f for f in fixtures }.values())
    return fixtures

def process_fixtures_with_agent(toml_config: TomlConfig):
    try:
        opencode = Opencode(toml_config=toml_config)
        session_data: list[OpencodeRunSessionData] = []
        fixtures = get_fixtures_to_process(toml_config)

        if len(fixtures) == 0:
            print(f"No fixtures to run")
            stop_gkg_server(toml_config.pipeline.gkg_path)
            return

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
            batch_session_data, error = process_batch(opencode, batch, toml_config, current_batch)
            current_batch += 1
            if error is None:
                session_data.extend(batch_session_data)
                if toml_config.pipeline.append_after_batch:
                    write_session_data(toml_config, batch_session_data)
            else:
                print(f"Error processing batch {current_batch}: {error}")
                break
            if toml_config.pipeline.break_after_first_batch:
                break
            if toml_config.pipeline.break_after_batch_n > 0 and current_batch >= toml_config.pipeline.break_after_batch_n:
                break

        print(f"Successfully processed {len(session_data)} out of {len(fixtures)} fixtures")

        if not toml_config.pipeline.append_after_batch:
            write_session_data(toml_config, session_data)

    except Exception as e:
        print(e)

def run_agent(toml_config: TomlConfig):
    """Retry the agent phase if there are any fixtures to run
    This will retry any killed fixtures and/or empty patches
    """
    retry_limit = toml_config.pipeline.retry_limit_agent_phase
    for i in range(retry_limit):
        process_fixtures_with_agent(toml_config)
        fixtures = get_fixtures_to_process(toml_config)
        if len(fixtures) == 0 and i > 0:
            print(f"No fixtures to run after retry {i}")
            break
        else:
            print(f"Total fixtures to process after retry {i}: {len(fixtures)}")
