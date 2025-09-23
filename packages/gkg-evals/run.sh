#!/bin/bash

#####
# This will be used as an entrypoint to trigger the dockerfile and run the evals against the multi-swe-bench repo
#####

set -e  # Exit on any error

compile_gkg() {
    echo "Compiling gkg..."
    cd $PROJECT_ROOT
    echo "Current directory: $(pwd)"
    echo "Compiling gkg in debug mode..."
    cargo build -p gkg
    echo "Compiling gkg in release mode..."
    cargo build --release -p gkg
    cd $SCRIPT_DIR
}

# Function to stop GKG server
stop_gkg_server() {
    if [ -n "$GKG_PID" ]; then
        echo "Stopping gkg server (PID: $GKG_PID)..."
        kill $GKG_PID 2>/dev/null || true
        wait $GKG_PID 2>/dev/null || true
        echo "gkg server stopped"
        GKG_PID=""
    fi
}

# Cleanup function to stop gkg server on exit
cleanup() {
    local exit_code=$?
    stop_gkg_server
    exit $exit_code
}

# Set up trap to call cleanup function on script exit/termination
trap cleanup EXIT INT TERM

# Initialize GKG_PID variable
GKG_PID=""

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check and set DOCKER_HOST if needed
check_docker_host() {
    if [ -z "$DOCKER_HOST" ]; then
        # Try common Docker socket locations
        if [ -S "/var/run/docker.sock" ]; then
            export DOCKER_HOST="unix:///var/run/docker.sock"
            echo "✓ DOCKER_HOST set to unix:///var/run/docker.sock"
        elif [ -S "$HOME/.docker/run/docker.sock" ]; then
            export DOCKER_HOST="unix://$HOME/.docker/run/docker.sock"
            echo "✓ DOCKER_HOST set to unix://$HOME/.docker/run/docker.sock"
        else
            echo "Warning: Could not find Docker socket. Please set DOCKER_HOST manually."
            echo "Common locations:"
            echo "  - unix:///var/run/docker.sock (Linux/WSL)"
            echo "  - unix://$HOME/.docker/run/docker.sock (Docker Desktop)"
            echo "Example: export DOCKER_HOST=unix:///var/run/docker.sock"
        fi
    else
        echo "✓ DOCKER_HOST already set to: $DOCKER_HOST"
    fi
}

# Function to verify Docker connectivity
verify_docker() {
    if command_exists docker; then
        echo "✓ Docker command found"
        if docker info >/dev/null 2>&1; then
            echo "✓ Docker daemon is accessible"
            return 0
        else
            echo "⚠ Warning: Docker daemon is not accessible. Multi-swe-bench may fail."
            echo "Please ensure Docker is running and DOCKER_HOST is set correctly."
            return 1
        fi
    else
        echo "⚠ Warning: Docker command not found. Multi-swe-bench requires Docker."
        return 1
    fi
}

git_verify() {
    # Very if git is installed
    if ! command_exists git; then
        echo "Error: git is not found in PATH."
        echo "Please install it globally with: brew install git"
        exit 1
    fi
    echo "✓ git found in PATH"
    
    echo "Installing git lfs..."
    git lfs install
    echo "Verifying and pulling git lfs files..."
    git lfs pull
}

check_for_dependencies_and_setup() {
    # Opencode will be installed via npx
    
    if ! command_exists mise; then
        echo "Error: mise is not found in PATH."
        echo "Please install it globally with: curl https://mise.run | sh"
        exit 1
    fi

    echo "✓ mise found in PATH"

    # Create harness directory if it doesn't exist
    # mkdir -p harness
    # setup_swebench
    # setup_multiswebench
    
    # Trust and install mise tools
    echo "Setting up mise environment..."
    mise trust
    mise install
    
    # Activate mise environment for this session
    eval "$(mise activate bash)"
    cd "$SCRIPT_DIR"
    
    # Navigate to pipeline directory and install Python dependencies
    echo "Installing Python dependencies..."
    cd pipeline
    if [ ! -d ".venv" ]; then
        uv venv
    else
        echo "✓ .venv already exists - skipping creation"
    fi
    source .venv/bin/activate
    uv sync
    cd "$SCRIPT_DIR"
}

run_pipeline_step() {
    local local_mode="$1"
    local config_abs_path="$2"
    local phase="$3"
    local pythonpath="$4"
    echo "Running $phase phase with config: $config_abs_path"
    cd pipeline
    
    # Use provided PYTHONPATH or default to src
    if [ -z "$pythonpath" ]; then
        pythonpath="."
    fi
    
    LOCAL=$local_mode PYTHONPATH=$pythonpath uv run python src/main.py "$config_abs_path" "$phase"
    cd ..
}

run_full_pipeline() {
    local local_mode="$1"
    local config_abs_path="$2"

    echo "Running full pipeline with config: $config_abs_path"
    run_pipeline_step "$local_mode" "$config_abs_path" "download" "."
    run_pipeline_step "$local_mode" "$config_abs_path" "index" "."
    run_pipeline_step "$local_mode" "$config_abs_path" "agent" "."
    run_pipeline_step "$local_mode" "$config_abs_path" "evals" "../harness/SWE-bench"
    run_pipeline_step "$local_mode" "$config_abs_path" "report" "."
}


# Function to run locally
run_local() {
    local config_path="$1"
    local phase="$2"
    local local_mode="$3"
    local config_abs_path=$(realpath "$config_path")
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    PROJECT_ROOT="$SCRIPT_DIR/../../"

    echo "Running locally..."
    if [ "$local_mode" = "1" ]; then
        echo "Local mode enabled"
    fi

    cd "$SCRIPT_DIR"

    # Compile both debug and release builds of gkg
    compile_gkg

    # In case we have remote fixtures, pull them in
    git_verify

    # Check for mise & opencode - setup mise env, setup uv
    check_for_dependencies_and_setup
    
    # Verify Docker environment is set up correctly
    check_docker_host
    verify_docker

    cd "$SCRIPT_DIR"
    mkdir -p data
    
    # Handle different phases
    case "$phase" in
        download|index|agent|report)
            run_pipeline_step "$local_mode" "$config_abs_path" $phase "."
            ;;
        evals)
            run_pipeline_step "$local_mode" "$config_abs_path" $phase "."
            ;;
        archive|noop)
            run_pipeline_step "$local_mode" "$config_abs_path" $phase "."
            ;;
        all)
            run_full_pipeline "$local_mode" "$config_abs_path"
            ;;
    esac
    
    echo "Phase '$phase' completed successfully!"
    exit 0
}

# Parse command line arguments
local_mode=0
config_path=""
phase="all"

# Process all arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --local)
            local_mode=1
            shift
            ;;
        --help|-h)
            echo "Usage: $0 <config.toml> [phase] [--local]"
            echo "  config.toml       Path to TOML configuration file"
            echo "  phase             Optional phase: noop, download, index, agent, evals, report, all (default: all)"
            echo "  --local           Run in local mode"
            echo "  --help, -h        Show this help message"
            exit 0
            ;;
        -*)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
        *)
            if [ -z "$config_path" ]; then
                config_path="$1"
            elif [ "$phase" = "all" ]; then
                phase="$1"
            else
                echo "Too many arguments"
                echo "Use --help for usage information"
                exit 1
            fi
            shift
            ;;
    esac
done

# Validate config file exists
if [ -z "$config_path" ]; then
    echo "Error: Config file is required"
    echo "Use --help for usage information"
    exit 1
fi

if [ ! -f "$config_path" ]; then
    echo "Error: Config file not found: $config_path"
    exit 1
fi

# Validate phase
case "$phase" in
    noop|archive|download|index|agent|evals|report|all)
        ;;
    *)
        echo "Error: Invalid phase '$phase'. Valid phases: noop, archive, download, index, agent, evals, report, all"
        exit 1
        ;;
esac

echo "Using config: $config_path"
echo "Running phase: $phase"
echo "Local mode: $local_mode"

# Run the pipeline
run_local "$config_path" "$phase" "$local_mode"
