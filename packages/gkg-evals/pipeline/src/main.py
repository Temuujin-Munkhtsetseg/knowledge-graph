import os
import sys
from pathlib import Path

from src.utils import load_toml_config
from src.harness.swe_bench import clone_swebench_repository

from src.steps.noop import noop
from src.steps.archive import archive_runs

from src.steps.download import download
from src.steps.gkg import index_worktrees, stop_gkg_server
from src.steps.agent import run_agent
from src.steps.evals import run_evals_swebench
from src.steps.report import generate_report

from dotenv import load_dotenv

from src.constants import (
    ENV_PATH, 
    RUNS_DIR, 
    FIXTURES_METADATA_PATH, 
    FIXTURES_DIR_PATH, 
    SESSION_DATA_PATH, 
    OPENCODE_CONFIG_PATH,
    BASE_REPOS_DIR_SWEBENCH,
    SWEBENCH_FIXTURES_DIR_PATH,
    SWEBENCH_PATCHES_PATH,
    SWEBENCH_REPORT_DIR,
    SWEBENCH_REPORT_PATH,
    SWEBENCH_HARNESS_LOCATION_DIR,
    MCP_CONFIGURATION_PATH,
)

# MULTISWEBENCH
# from src.constants import BASE_DIR_MULTISWEBENCH
# from src.steps.evals import run_evals_multiswebench

class GkgEvalsPipeline:
    def __init__(self, config_path: str):
        self.config = load_toml_config(config_path, pprint=True)
        
        # Set environment variables based on config
        if os.environ.get("LOCAL"):
            os.environ["LOCAL"] = "1"
        
        # Set PYTHONPATH
        current_dir = Path.cwd()
        os.environ["PYTHONPATH"] = str(current_dir)

        # Validate .env file
        if not ENV_PATH.exists():
            print("Error: .env file not found")
            print(f"Expected path: {ENV_PATH}")
            sys.exit(1)
        
        # Load .env file
        load_dotenv()

        # Check if env contains ANTHROPIC_API_KEY
        if os.environ.get("ANTHROPIC_API_KEY") is None:
            print(f"Expected path of env file: {ENV_PATH}")
            print("Error: ANTHROPIC_API_KEY not found in .env file")
            sys.exit(1)

        self.session_dir = self.create_pipeline_session_dir()
        self.config.pipeline.session_dir = self.session_dir
        self.create_session_paths()
        clone_swebench_repository(self.config)

        # Stop old versions of the gkg server
        stop_gkg_server(self.config.pipeline.gkg_path)

    def create_pipeline_session_dir(self):
        """Create a session directory for the pipeline"""
        session_dir = RUNS_DIR / self.config.pipeline.session_name
        session_dir.mkdir(parents=True, exist_ok=True)
        return session_dir

    def create_session_paths(self):
        """Create session paths for the pipeline"""
        self.config.pipeline.session_paths.fixtures_metadata_path = self.session_dir / FIXTURES_METADATA_PATH
        self.config.pipeline.session_paths.fixtures_dir_path = self.session_dir / FIXTURES_DIR_PATH
        self.config.pipeline.session_paths.session_data_path = self.session_dir / SESSION_DATA_PATH
        self.config.pipeline.session_paths.opencode_config_path = self.session_dir / OPENCODE_CONFIG_PATH
        self.config.pipeline.session_paths.swe_bench_repos_dir = self.session_dir / BASE_REPOS_DIR_SWEBENCH
        self.config.pipeline.session_paths.swe_bench_fixtures_dir_path = self.session_dir / SWEBENCH_FIXTURES_DIR_PATH
        self.config.pipeline.session_paths.swe_bench_patches_path = self.session_dir / SWEBENCH_PATCHES_PATH
        self.config.pipeline.session_paths.swe_bench_report_dir = self.session_dir / SWEBENCH_REPORT_DIR
        self.config.pipeline.session_paths.swe_bench_report_path = self.session_dir / SWEBENCH_REPORT_PATH
        self.config.pipeline.session_paths.swe_bench_harness_location_dir = self.session_dir / SWEBENCH_HARNESS_LOCATION_DIR
        self.config.pipeline.session_paths.mcp_configuration_path = self.session_dir / MCP_CONFIGURATION_PATH
        self.config.pipeline.session_paths.pprint()
    
    def check_repos_cache(self) -> bool:
        """Check if repos directory already exists (cached)"""
        swe_bench_dir = self.config.pipeline.session_paths.swe_bench_repos_dir
        fixtures_metadata_path = self.config.pipeline.session_paths.fixtures_metadata_path
        if swe_bench_dir.exists() and fixtures_metadata_path.exists():
            print("✓ repos/ directory already exists - skipping download phase (using cache)")
            print("Repositories are available in ./repos/ directory")
            print("Fixture metadata available in ./fixtures_metadata.json")
            print(f"Session directory: {self.session_dir}")
            return True
        return False

    def run_noop_phase(self):
        """Run the noop phase"""
        print("Running noop phase...")
        noop(self.config)
        print("Noop completed successfully!")

    def run_archive_phase(self):
        """Run the archive phase"""
        print("Running archive phase...")
        archive_runs()
        print("Archive completed successfully!")
    
    def run_download_phase(self):
        """Run the download phase"""
        if not self.config.pipeline.skip_download or not self.check_repos_cache():
            print("Running download phase...")
            download(self.config)
            print("✓ Download phase completed successfully!")
            print("Repositories have been cloned to ./repos/ directory")
            print("Fixture metadata saved to ./fixtures_metadata.json")
    
    def run_gkg_indexing(self):
        """Run GKG indexing phase"""
        if not self.config.pipeline.skip_gkg_index:
            print("Indexing worktrees...")
            index_worktrees(self.config)
            print("✓ Worktrees indexed successfully!")
        else:
            print("⚠ Skipping GKG indexing (skip_gkg_index flag set)")
    
    def run_evals_phase(self):
        """Run the evaluation phase"""
        print("Running evals phase...")
        run_evals_swebench(self.config)
        print("Evals completed successfully!")

    def run_agent_phase(self):
        """Run the agent phase"""
        print("Running agent phase...")
        run_agent(self.config)
        print("Agent completed successfully!")

    def run_report_phase(self):
        """Run the report phase"""
        print("Running report phase...")
        generate_report(self.config)
        print("Report completed successfully!")
    
    def run_phase(self, phase: str):
        """Run a specific phase of the pipeline"""
        if phase == "noop":
            self.run_noop_phase()
        elif phase == "archive":
            self.run_archive_phase()
        elif phase == "download":
            self.run_download_phase()
        elif phase == "index":
            self.run_gkg_indexing()
        elif phase == "agent":
            self.run_agent_phase()
        elif phase == "evals":
            self.run_evals_phase()
        elif phase == "report":
            self.run_report_phase()
        elif phase == "all":
            self.run_download_phase()
            self.run_gkg_indexing()
            self.run_agent_phase()
            self.run_evals_phase()
            self.run_report_phase()
        else:
            raise ValueError(f"Unknown phase: {phase}")


if __name__ == "__main__":
    if len(sys.argv) not in [2, 3]:
        print("Usage: python main.py <config_path> [phase]")
        print("Phases: download, index, agent, evals, report, all (default: all)")
        print("Dev-only phases: noop (use this to test in-progress work or new features)")
        sys.exit(1)
    
    config_path = sys.argv[1]
    phase = sys.argv[2] if len(sys.argv) == 3 else "all"
    
    if not Path(config_path).exists():
        print(f"Config file not found: {config_path}")
        sys.exit(1)
    
    pipeline = GkgEvalsPipeline(config_path)
    try:
        pipeline.run_phase(phase)
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(f"Phase '{phase}' failed: {e}")
        sys.exit(1)
