import os
import sys
from pathlib import Path

from src.utils import load_toml_config
# from src.steps.download import download
# from src.steps.gkg_index import index_worktrees
# from src.steps.agent import run_agent
# from src.steps.evals import run_evals_swebench
# from src.steps.report import generate_report

from src.constants import FIXTURES_METADATA_PATH, BASE_DIR_SWEBENCH

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
    
    def check_repos_cache(self) -> bool:
        """Check if repos directory already exists (cached)"""
        if BASE_DIR_SWEBENCH.exists() and FIXTURES_METADATA_PATH.exists():
            print("✓ repos/ directory already exists - skipping download phase (using cache)")
            print("Repositories are available in ./repos/ directory")
            print("Fixture metadata available in ./fixtures_metadata.json")
            return True
        return False
    
    def run_download_phase(self):
        """Run the download phase"""
        if not self.config.pipeline.skip_download or not self.check_repos_cache():
            print("Running download phase...")
            # download(self.config)
            print("✓ Download phase completed successfully!")
            print("Repositories have been cloned to ./repos/ directory")
            print("Fixture metadata saved to ./fixtures_metadata.json")
    
    def run_gkg_indexing(self):
        """Run GKG indexing phase"""
        if not self.config.pipeline.skip_gkg_index:
            print("Indexing worktrees...")
            # index_worktrees(self.config)
            print("✓ Worktrees indexed successfully!")
        else:
            print("⚠ Skipping GKG indexing (skip_gkg_index flag set)")
    
    def run_evals_phase(self):
        """Run the evaluation phase"""
        print("Running evals phase...")
        # run_evals_swebench(self.config)
        print("Evals completed successfully!")

    def run_agent_phase(self):
        """Run the agent phase"""
        print("Running agent phase...")
        # run_agent(self.config)
        print("Agent completed successfully!")

    def run_report_phase(self):
        """Run the report phase"""
        print("Running report phase...")
        # generate_report(self.config)
        print("Report completed successfully!")
    
    def run_phase(self, phase: str):
        """Run a specific phase of the pipeline"""
        if phase == "download":
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
