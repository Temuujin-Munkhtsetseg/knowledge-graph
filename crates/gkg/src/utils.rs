use anyhow::Result;
use home::home_dir;
use serde::{Deserialize, Serialize};
use single_instance::SingleInstance;
use std::fs;
use std::io::Read;
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

const GKG_HTTP_SERVER: &str = "gkg-http-server";

pub fn get_gkg_dir() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let gkg_dir = home.join(".gkg");
    fs::create_dir_all(&gkg_dir)?;
    Ok(gkg_dir)
}

pub fn get_lock_file_path() -> Result<PathBuf> {
    Ok(get_gkg_dir()?.join("gkg.lock"))
}

// On macOS file-based lock is used so we need a different handling.
#[cfg(target_os = "macos")]
pub fn get_single_instance() -> Result<SingleInstance> {
    let home_dir = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let single_instance_path = home_dir.join(GKG_HTTP_SERVER);
    Ok(SingleInstance::new(single_instance_path.to_str().unwrap())?)
}

#[cfg(not(target_os = "macos"))]
pub fn get_single_instance() -> Result<SingleInstance> {
    Ok(SingleInstance::new(GKG_HTTP_SERVER)?)
}

pub fn is_server_running() -> Result<Option<u16>> {
    let lock_file = get_lock_file_path()?;
    if !lock_file.exists() {
        return Ok(None);
    }

    let mut contents = String::new();
    fs::File::open(&lock_file)?
        .read_to_string(&mut contents)
        .map_err(|e| anyhow::anyhow!("Could not read lock file: {}", e))?;
    let port: u16 = match contents.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            // Stale or legacy lock file content: remove and treat as not running.
            let _ = fs::remove_file(lock_file);
            return Ok(None);
        }
    };

    if TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse()?,
        Duration::from_millis(100),
    )
    .is_ok()
    {
        Ok(Some(port))
    } else {
        // Server not reachable; remove stale lock file.
        fs::remove_file(lock_file)?;
        Ok(None)
    }
}

#[derive(Serialize, Deserialize)]
pub struct ServerInfo {
    pub port: u16,
}
