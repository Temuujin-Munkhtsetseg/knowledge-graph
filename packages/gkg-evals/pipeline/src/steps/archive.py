from src.utils import TomlConfig, load_toml_config
import shutil
import datetime
import time 
from pathlib import Path

from src.constants import ARCHIVE_DIR, RUNS_DIR, RUNS_CONFIG_DIR

def archive_runs():
    """Archive runs based on session names found in runs_config TOML files."""
    try:
        timestamp = datetime.datetime.now().strftime("%Y-%m-%d--%H:%M:%S")
        print(f"Archiving runs from configs to {timestamp}")
        
        # Path to the runs_config directory
        if not RUNS_CONFIG_DIR.exists():
            print(f"Runs config directory does not exist: {RUNS_CONFIG_DIR}")
            return
            
        # Find all TOML config files
        config_files = list(RUNS_CONFIG_DIR.glob("*.toml"))
        
        if not config_files:
            print("No TOML config files found in runs_config directory")
            return
            
        print(f"Found {len(config_files)} config files: {[f.name for f in config_files]}")
        
        # Use glob patterns to ignore large directories
        ignore_patterns = ['repos', 'harness', 'fixtures', '__pycache__', '.git', 'node_modules', '.venv', 'venv']
        print(f"Ignoring the following patterns during archive: {ignore_patterns}")
        
        for config_file in config_files:
            try:
                print(f"\nProcessing config file: {config_file.name}")
                
                # Load the TOML config
                toml_config: TomlConfig = load_toml_config(str(config_file))
                session_name = toml_config.pipeline.session_name
                
                print(f"Config session name: {session_name}")
                
                # Check if the run directory exists
                run_dir = RUNS_DIR / session_name
                
                if not run_dir.exists():
                    print(f"Run directory does not exist for session '{session_name}': {run_dir}")
                    continue
                    
                # Create archive directory
                archive_dir = ARCHIVE_DIR / timestamp / session_name
                archive_dir.parent.mkdir(parents=True, exist_ok=True)
                
                print(f"Archiving {session_name} from {run_dir} to {archive_dir}")
                shutil.copytree(run_dir, archive_dir, ignore=shutil.ignore_patterns(*ignore_patterns))
                
                # Also copy the TOML config file to the archive
                config_archive_path = archive_dir / "config.toml"
                shutil.copy2(config_file, config_archive_path)
                print(f"Copied config file {config_file.name} to {config_archive_path}")
                
                print(f"Successfully archived {session_name} to {archive_dir}")
                            
            except Exception as e:
                print(f"Error processing config file {config_file.name}: {e}")
                # Continue with other config files even if one fails
                continue
                
        print(f"\nFinished archiving runs from configs to {ARCHIVE_DIR / timestamp}")
        
    except Exception as e:
        print(f"Error in archive_runs_from_configs: {e}")
        raise  # Re-raise the exception so we can see the full error
