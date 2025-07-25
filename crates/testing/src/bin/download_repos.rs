use anyhow::{Context, Result};
use std::path::Path;
use testing::repository::TestRepository;
use testing::zip::{download_zip, extract_zip};
use tokio::task::JoinSet;
use tracing::info;

const REPOS: &[(&str, &str)] = &[
    (
        "gitlab-shell",
        "https://gitlab.com/gitlab-org/gitlab-shell/-/archive/v13.3.0/gitlab-shell-v13.3.0.zip",
    ),
    (
        "gitlab",
        "https://gitlab.com/gitlab-org/gitlab/-/archive/v18.2.0-ee/gitlab-v18.2.0-ee.zip",
    ),
    (
        "gitlab-development-kit",
        "https://gitlab.com/gitlab-org/gitlab-development-kit/-/archive/v0.2.19/gitlab-development-kit-v0.2.19.zip",
    ),
];

fn initialize_git_repositories(gdk_dir: &Path) -> Result<()> {
    info!("Initializing Git repositories for all extracted repos");

    // Initialize git repo for gitlab-development-kit (at gdk_dir level)
    if gdk_dir.exists()
        && gdk_dir
            .read_dir()
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    {
        TestRepository::new_minimal(gdk_dir);
    }

    // Initialize git repos for gitlab and gitlab-shell subdirectories
    for repo_name in &["gitlab", "gitlab-shell"] {
        let repo_path = gdk_dir.join(repo_name);
        if repo_path.exists() {
            TestRepository::new_minimal(&repo_path);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let gdk_dir = Path::new("output/gdk");
    let temp_dir = Path::new("output/temp");

    // Create directories if they don't exist
    std::fs::create_dir_all(gdk_dir).context("Failed to create output/gdk directory")?;
    std::fs::create_dir_all(temp_dir).context("Failed to create output/temp directory")?;
    info!("Created output directories");

    // Filter repositories that need to be downloaded
    let repos_to_download: Vec<_> = REPOS
        .iter()
        .filter(|(name, _)| {
            let should_skip = if *name == "gitlab-development-kit" {
                gdk_dir.exists()
                    && gdk_dir
                        .read_dir()
                        .map(|mut d| d.next().is_some())
                        .unwrap_or(false)
            } else {
                gdk_dir.join(name).exists()
            };

            if should_skip {
                info!("Repository {} already exists, skipping", name);
            }

            !should_skip
        })
        .collect();

    if repos_to_download.is_empty() {
        info!("All repositories already exist, nothing to download");
        // Still try to initialize git repos if they don't exist
        initialize_git_repositories(gdk_dir)?;
        return Ok(());
    }

    // Download all repositories in parallel
    let mut download_tasks = JoinSet::new();

    for (name, url) in repos_to_download.iter() {
        let name = name.to_string();
        let url = url.to_string();
        let zip_path = temp_dir.join(format!("{name}.zip"));

        download_tasks.spawn(async move {
            download_zip(&url, &zip_path)
                .await
                .context(format!("Failed to download {name} from {url}"))
                .map(|_| (name, zip_path))
        });
    }

    // Wait for all downloads to complete
    let mut downloaded_repos = Vec::new();
    while let Some(result) = download_tasks.join_next().await {
        let (name, zip_path) = result??;
        info!("Successfully downloaded {}", name);
        downloaded_repos.push((name, zip_path));
    }

    // Extract and move repositories (sequentially to avoid file conflicts)
    for (name, zip_path) in downloaded_repos {
        // Extract ZIP file
        extract_zip(&zip_path, temp_dir).context(format!("Failed to extract {name}"))?;

        // Find the extracted folder (it will have a version suffix)
        let extracted_folders: Vec<_> = std::fs::read_dir(temp_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
                    && entry.file_name().to_string_lossy().starts_with(&name)
            })
            .collect();

        if let Some(extracted_folder) = extracted_folders.first() {
            let extracted_path = extracted_folder.path();

            if name == "gitlab-development-kit" {
                // For GDK, move contents to gdk_dir, not the folder itself
                for entry in std::fs::read_dir(&extracted_path)? {
                    let entry = entry?;
                    let dest = gdk_dir.join(entry.file_name());
                    std::fs::rename(entry.path(), dest).context(format!(
                        "Failed to move GDK content {} to final location",
                        entry.file_name().to_string_lossy()
                    ))?;
                }
                // Remove the now-empty extracted folder
                std::fs::remove_dir(&extracted_path)
                    .context("Failed to remove empty GDK extracted folder")?;
            } else {
                // For gitlab and gitlab-shell, move the folder to gdk_dir/name
                let repo_path = gdk_dir.join(&name);
                std::fs::rename(&extracted_path, &repo_path)
                    .context(format!("Failed to move {name} to final location"))?;
            }

            info!("Successfully extracted and moved {}", name);
        } else {
            anyhow::bail!("Could not find extracted folder for {}", name);
        }

        // Clean up ZIP file
        std::fs::remove_file(&zip_path).context(format!("Failed to remove ZIP file for {name}"))?;
    }

    // Clean up temp directory
    std::fs::remove_dir_all(temp_dir).context("Failed to remove temp directory")?;

    // Initialize Git repositories for all extracted repos
    initialize_git_repositories(gdk_dir)?;

    info!("All repositories downloaded, extracted, and initialized successfully under output/gdk/");
    Ok(())
}
