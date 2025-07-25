use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use tracing::info;

pub async fn download_zip(url: &str, dest_path: &Path) -> Result<()> {
    info!("Downloading {} to {:?}", url, dest_path);

    let response = reqwest::get(url)
        .await
        .context("Failed to make HTTP request")?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP request failed with status: {}", response.status());
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read response bytes")?;

    let mut file = File::create(dest_path).context("Failed to create destination file")?;

    file.write_all(&bytes)
        .context("Failed to write to destination file")?;

    Ok(())
}

pub fn extract_zip(zip_path: &Path, extract_to: &Path) -> Result<()> {
    info!("Extracting {:?} to {:?}", zip_path, extract_to);

    let file = File::open(zip_path).context("Failed to open ZIP file")?;

    let mut archive = zip::ZipArchive::new(file).context("Failed to read ZIP archive")?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .context("Failed to get file from archive")?;

        let outpath = match file.enclosed_name() {
            Some(path) => extract_to.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath).context("Failed to create directory")?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p).context("Failed to create parent directory")?;
                }
            }
            let mut outfile = File::create(&outpath).context("Failed to create extracted file")?;
            std::io::copy(&mut file, &mut outfile).context("Failed to copy file contents")?;
        }
    }

    Ok(())
}
