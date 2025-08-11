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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ServerLockInfo {
    pub port: u16,
    #[serde(default)]
    pub pid: Option<u32>,
}

pub fn read_lock_info() -> Result<Option<ServerLockInfo>> {
    let lock_file = get_lock_file_path()?;
    if !lock_file.exists() {
        return Ok(None);
    }

    let mut contents = String::new();
    fs::File::open(&lock_file)?.read_to_string(&mut contents)?;

    if let Ok(info) = serde_json::from_str::<ServerLockInfo>(contents.trim()) {
        return Ok(Some(info));
    }

    // Corrupt lock; remove and treat as not running
    let _ = fs::remove_file(lock_file);
    Ok(None)
}

pub fn write_lock_info(info: &ServerLockInfo) -> Result<()> {
    let lock_file_path = get_lock_file_path()?;
    let json = serde_json::to_string(info)?;
    fs::write(lock_file_path, json)?;
    Ok(())
}

pub fn remove_lock_file() -> Result<()> {
    let lock_file = get_lock_file_path()?;
    if lock_file.exists() {
        let _ = fs::remove_file(lock_file);
    }
    Ok(())
}

pub fn is_server_running() -> Result<Option<u16>> {
    let Some(lock) = read_lock_info()? else {
        return Ok(None);
    };
    let port = lock.port;

    // Prefer PID-based liveness when available to reduce race with startup
    if let Some(pid) = lock.pid {
        #[cfg(unix)]
        {
            if nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok() {
                return Ok(Some(port));
            } else {
                let _ = remove_lock_file();
                return Ok(None);
            }
        }
        #[cfg(not(unix))]
        {
            return Ok(Some(port));
        }
    }

    if TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse()?,
        Duration::from_millis(100),
    )
    .is_ok()
    {
        Ok(Some(port))
    } else {
        Ok(None)
    }
}

#[derive(Serialize, Deserialize)]
pub struct ServerInfo {
    pub port: u16,
}
