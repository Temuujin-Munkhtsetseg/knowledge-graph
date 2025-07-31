use std::fs;
use std::path::{Path, PathBuf};
extern crate anyhow;
extern crate clap;
extern crate ignore;
extern crate toml;

use anyhow::{Context, anyhow};
use clap::{Parser, ValueEnum};

/// A utility for performing repository tasks.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Sets the dependency mode (dev or prod).
    SetMode {
        #[clap(value_enum)]
        mode: Mode,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum Mode {
    /// Use local development dependencies
    Dev,
    /// Use production git dependencies
    Prod,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::SetMode { mode } => set_mode(mode),
    }
}

fn set_mode(mode: Mode) -> anyhow::Result<()> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let root_dir = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| anyhow!("Failed to find workspace root"))?;

    let cargo_config_dir = root_dir.join(".cargo");
    let cargo_config_path = cargo_config_dir.join("config.toml");

    match mode {
        Mode::Dev => {
            // Ensure .cargo directory exists
            fs::create_dir_all(&cargo_config_dir).context("Failed to create .cargo directory")?;

            // Create the config.toml with patch overrides
            let config_content = r#"# Override git dependencies with local paths for development
[patch."https://gitlab.com/gitlab-org/rust/gitlab-code-parser.git"]
parser-core = { path = "../gitlab-code-parser/crates/parser-core" }

[patch."https://gitlab.com/gitlab-org/rust/gitalisk.git"]
gitalisk-core = { path = "../gitalisk/crates/gitalisk-core" }
"#;

            fs::write(&cargo_config_path, config_content)
                .context("Failed to write .cargo/config.toml")?;

            println!("✅ Switched to development mode");
            println!("   Local dependencies will be used from relative paths");
        }
        Mode::Prod => {
            // Remove the config file if it exists
            if cargo_config_path.exists() {
                fs::remove_file(&cargo_config_path)
                    .context("Failed to remove .cargo/config.toml")?;
                println!("✅ Switched to production mode");
                println!("   Git dependencies will be used from remote repositories");
            } else {
                println!("✅ Already in production mode");
                println!("   Git dependencies will be used from remote repositories");
            }
        }
    }

    Ok(())
}
