use std::fs;
use std::path::{Path, PathBuf};
extern crate anyhow;
extern crate clap;
extern crate ignore;
extern crate toml;

use anyhow::{Context, anyhow};
use clap::{Parser, ValueEnum};
use ignore::WalkBuilder;
use std::sync::Mutex;
use toml::Value;

/// A utility for performing repository tasks.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Checks that all Cargo.toml versions match the VERSION file.
    CheckVersions,
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
        Commands::CheckVersions => check_versions(),
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

fn check_versions() -> anyhow::Result<()> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let root_dir = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| anyhow!("Failed to find workspace root"))?;

    let version_file_path = root_dir.join("VERSION");
    let expected_version = fs::read_to_string(&version_file_path)
        .with_context(|| {
            format!(
                "Failed to read version from {}",
                version_file_path.display()
            )
        })?
        .trim()
        .to_string();

    println!("Expected version: {expected_version}");

    let mismatches = Mutex::new(false);
    let errors = Mutex::new(Vec::new());

    let crates_dir = root_dir.join("crates");

    WalkBuilder::new(crates_dir).build_parallel().run(|| {
        Box::new(|entry_result| {
            match entry_result {
                Ok(entry) => {
                    if entry.file_type().is_some_and(|ft| ft.is_file())
                        && entry.file_name().to_string_lossy() == "Cargo.toml"
                    {
                        let cargo_toml_path = entry.path();
                        println!("Checking {}", cargo_toml_path.display());

                        let cargo_toml_content = match fs::read_to_string(cargo_toml_path) {
                            Ok(content) => content,
                            Err(e) => {
                                errors.lock().unwrap().push(anyhow!(e).context(format!(
                                    "Failed to read {}",
                                    cargo_toml_path.display()
                                )));
                                return ignore::WalkState::Continue;
                            }
                        };

                        let toml_value = match cargo_toml_content.parse::<Value>() {
                            Ok(value) => value,
                            Err(e) => {
                                errors.lock().unwrap().push(anyhow!(e).context(format!(
                                    "Failed to parse {}",
                                    cargo_toml_path.display()
                                )));
                                return ignore::WalkState::Continue;
                            }
                        };

                        if let Some(package) = toml_value.get("package") {
                            if let Some(version_value) = package.get("version") {
                                if let Some(version_str) = version_value.as_str() {
                                    if version_str != expected_version {
                                        eprintln!(
                                            "Version mismatch in {}: expected {}, found {}",
                                            cargo_toml_path.display(),
                                            expected_version,
                                            version_str
                                        );
                                        *mismatches.lock().unwrap() = true;
                                    }
                                } else {
                                    eprintln!(
                                        "Version is not a string in {}",
                                        cargo_toml_path.display()
                                    );
                                    *mismatches.lock().unwrap() = true;
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    errors
                        .lock()
                        .unwrap()
                        .push(anyhow!(err).context("Error walking directory"));
                }
            }
            ignore::WalkState::Continue
        })
    });

    let collected_errors = errors.into_inner().unwrap();
    if !collected_errors.is_empty() {
        for err in collected_errors {
            eprintln!("{err:?}");
        }
        return Err(anyhow!("Errors occurred during version check"));
    }

    if *mismatches.lock().unwrap() {
        Err(anyhow!("Version mismatches found"))
    } else {
        println!("All Cargo.toml versions match the VERSION file.");
        Ok(())
    }
}
