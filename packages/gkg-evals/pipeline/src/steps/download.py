import os
import json
import subprocess
from pathlib import Path

from src.utils import run_threaded
from src.harness.swe_bench import SweBenchFixtureMetadata, get_swebench_lite_dataset
from src.utils import TomlConfig

from src.constants import BASE_DIR_SWEBENCH, FIXTURES_METADATA_PATH, FIXTURES_DIR_PATH

# MULTISWEBENCH
# from src.constants import BASE_DIR_MULTISWEBENCH
# from src.harness.multi_swe_bench import MultiSweBenchFixtureMetadata, get_fixtures_metadata

def clone_fixtures():
    FIXTURES_DIR_PATH.mkdir(parents=True, exist_ok=True)

def clone_repository(fixture: SweBenchFixtureMetadata) -> tuple[bool, Path]:
    repo_url = f"https://github.com/{fixture.org}/{fixture.repo}.git"

    ## check if repo_path has git files
    if fixture.repo_path.exists() and (fixture.repo_path / ".git").exists():
        print(f"Repository with git files already exists: {fixture.org}/{fixture.repo}")
        return True, fixture.repo_path

    try:
        print(f"Cloning repository: {fixture.org}/{fixture.repo} to {fixture.repo_path}")
        subprocess.run(
            ["git", "clone", repo_url, str(fixture.repo_path)],
            check=True,
            capture_output=True,
            text=True,
        )
        return True, fixture.repo_path
    except subprocess.CalledProcessError as e:
        print(f"Failed to clone {fixture.org}/{fixture.repo}: {e.stderr}")
        return False, None
    except Exception as e:
        print(f"Error cloning {fixture.org}/{fixture.repo}: {str(e)}")
        return False, None


def create_worktree(fixture: SweBenchFixtureMetadata) -> tuple[bool, Path]:
    repo_path = fixture.repo_path
    repo_key = f"{fixture.org}/{fixture.repo}"

    # Create a unique worktree name
    # MultiSweBench key: fixture.ref_fixture.sha[:8]
    worktree_name = f"{fixture.instance_id}_{fixture.base_commit[:8]}"
    worktree_path = repo_path / "worktrees" / worktree_name

    if worktree_path.exists() and (worktree_path / ".git").exists():
        print(f"Worktree already exists with git files: {repo_key}/{worktree_name}")
        return True, worktree_path

    try:
        print(f"Creating worktree: {repo_key}/{worktree_name}")
        worktree_path.parent.mkdir(parents=True, exist_ok=True)
        abs_repo_path = str(repo_path.resolve())
        abs_worktree_path = str(worktree_path.resolve())

        # Try to create the worktree directly with the SHA
        result = subprocess.run(
            [
                "git",
                "-C",
                abs_repo_path,
                "worktree",
                "add",
                abs_worktree_path,
                fixture.base_commit, # .sha for MultiSweBench
            ],
            capture_output=True,
            text=True,
        )

        if result.returncode != 0:
            print(f"Failed to create worktree: {repo_key}/{worktree_name}")
            return False, None

        print(f"Successfully created worktree: {repo_key}/{worktree_name}")
        fixture.add_worktree_path(worktree_path)
        return True, worktree_path
    except subprocess.CalledProcessError as e:
        import traceback
        traceback.print_exc()
        print(f"Failed to create worktree with error: {repo_key}/{worktree_name}: {e.stderr}")
        return False, None
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(f"Error processing {repo_key}/{worktree_name}: {str(e)}")
        return False, None


def rollback_worktree(fixture: SweBenchFixtureMetadata):
    subprocess.run(["git", "restore", "."], cwd=fixture.worktree_path)
    subprocess.run(["git", "clean", "-fd"], cwd=fixture.worktree_path)


def remove_worktrees(fixtures: list[SweBenchFixtureMetadata]):
    """
    Remove git worktrees for the given fixtures.
    
    Args:
        fixtures: List of FixtureMetadata objects containing worktree paths to remove
    """
    for fixture in fixtures:
        if not fixture.worktree_path or not fixture.repo_path:
            print(f"Skipping worktree removal for {fixture.org}/{fixture.repo}")
            continue
            
        if not fixture.worktree_path.exists():
            print(f"Worktree already removed: {fixture.worktree_path}")
            continue
            
        repo_key = f"{fixture.org}/{fixture.repo}"
        worktree_name = fixture.worktree_path.name
        
        print(f"Removing worktree: {repo_key}/{worktree_name}")
        
        try:
            # Use absolute paths to avoid any relative path issues
            abs_repo_path = str(fixture.repo_path.resolve())
            abs_worktree_path = str(fixture.worktree_path.resolve())
            
            # Remove the worktree using git
            subprocess.run(
                [
                    "git",
                    "-C",
                    abs_repo_path,
                    "worktree",
                    "remove",
                    "--force",  # Force removal even if worktree is dirty
                    abs_worktree_path,
                ],
                check=True,
                capture_output=True,
                text=True,
            )
            
            print(f"Successfully removed worktree: {repo_key}/{worktree_name}")
            
        except subprocess.CalledProcessError as e:
            print(f"Failed to remove worktree {repo_key}/{worktree_name}: {e.stderr}")
            # Try to manually clean up the directory if git command fails
            try:
                import shutil
                if fixture.worktree_path.exists():
                    shutil.rmtree(fixture.worktree_path)
                    print(f"Manually removed worktree directory: {fixture.worktree_path}")
            except Exception as cleanup_error:
                print(f"Failed to manually clean up {fixture.worktree_path}: {cleanup_error}")
                
        except Exception as e:
            print(f"Error removing worktree {repo_key}/{worktree_name}: {str(e)}")
    
    print("Worktree removal completed")

def download(toml_config: TomlConfig):
    ### SWEBENCH ###
    ds = get_swebench_lite_dataset()

    ## TODO: Turn fixtures into an interface via abc module

    ## Setup fixtures
    fixtures : list[SweBenchFixtureMetadata] = []
    for item in ds:
        item = dict(item)
        org, repo = item["repo"].split("/")
        item["repo"] = repo
        item["org"] = org
        metadata = SweBenchFixtureMetadata.from_dict(item)
        repo_path = BASE_DIR_SWEBENCH / org / repo
        metadata.add_repo_path(repo_path)
        if not repo_path.exists():
            os.makedirs(repo_path, exist_ok=True)
        fixtures.append(metadata)

    seen_repos = set()
    unique_fixtures = []
    for f in fixtures:
        if f"{f.org}/{f.repo}" not in seen_repos:
            seen_repos.add(f"{f.org}/{f.repo}")
            unique_fixtures.append(f)

    run_threaded(clone_repository, unique_fixtures)
    run_threaded(create_worktree, fixtures)

    with open(FIXTURES_METADATA_PATH, "w") as f:
        json.dump([fx.to_dict() for fx in fixtures], f, indent=4)

    ### MULTISWEBENCH ###
    # # Get fixtures metadata
    # fixtures_metadata = get_fixtures_metadata()

    # # Check out repositories using git worktrees
    # fixtures = checkout_repositories(fixtures_metadata)

    # # Dump to file
    # with open(FIXTURES_METADATA_PATH, "w") as f:
    #     json.dump([fx.to_dict() for fx in fixtures], f, indent=4)
