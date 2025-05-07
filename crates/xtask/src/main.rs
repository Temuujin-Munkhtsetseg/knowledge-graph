use std::fs;
use std::path::{Path, PathBuf};
extern crate anyhow;
extern crate clap;
extern crate ignore;
extern crate toml;

use anyhow::{anyhow, Context};
use clap::Parser;
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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::CheckVersions => check_versions(),
    }
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

    println!("Expected version: {}", expected_version);

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
            eprintln!("{:?}", err);
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
