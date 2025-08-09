#![cfg(unix)]

use assert_cmd::prelude::*;
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::process::{ChildStdout, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

#[derive(Deserialize, Debug)]
struct ServerInfo {
    port: u16,
}

fn read_server_info_line(child_stdout: ChildStdout) -> ServerInfo {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(child_stdout);
        let mut line = String::new();
        // Read a single line; if this blocks, the timeout below will handle it
        if reader.read_line(&mut line).is_ok() {
            let _ = tx.send(line);
        }
    });

    let line = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("timed out waiting for server JSON output");

    serde_json::from_str::<ServerInfo>(line.trim()).expect("invalid ServerInfo JSON")
}

fn lock_file_path(home: &std::path::Path) -> std::path::PathBuf {
    home.join(".gkg").join("gkg.lock")
}

#[test]
fn server_starts_and_prints_json_and_creates_lockfile() {
    let temp_home = TempDir::new().expect("temp home");
    let home_path = temp_home.path().to_path_buf();

    let mut cmd = Command::cargo_bin("gkg").expect("cargo bin gkg");
    cmd.arg("server")
        .env("HOME", &home_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().expect("spawn gkg server");
    let child_stdout = child.stdout.take().expect("capture stdout");
    let server_info = read_server_info_line(child_stdout);
    assert!(server_info.port > 0);

    // Lock file should contain the same port
    let lock_path = lock_file_path(&home_path);
    let contents = std::fs::read_to_string(&lock_path).expect("read lock file");
    let port_from_lock: u16 = contents.trim().parse().expect("parse lock port");
    assert_eq!(port_from_lock, server_info.port);

    // Cleanup: kill server and remove lock file if still present
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(lock_path);
}

#[test]
fn second_server_prints_same_port_and_exits() {
    let temp_home = TempDir::new().expect("temp home");
    let home_path = temp_home.path().to_path_buf();

    // Start the first server (foreground)
    let mut cmd1 = Command::cargo_bin("gkg").expect("cargo bin gkg");
    cmd1.arg("server")
        .env("HOME", &home_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child1 = cmd1.spawn().expect("spawn first gkg server");
    let stdout1 = child1.stdout.take().expect("capture stdout 1");
    let info1 = read_server_info_line(stdout1);

    // Invoke second instance; it should print same port and exit quickly
    let assert = Command::cargo_bin("gkg")
        .expect("cargo bin gkg")
        .arg("server")
        .env("HOME", &home_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // It should print exactly one JSON line
    let line = stdout
        .lines()
        .next()
        .expect("expected a JSON line from second instance");
    let info2: ServerInfo = serde_json::from_str(line).expect("invalid JSON from second instance");
    assert_eq!(info2.port, info1.port);

    // Cleanup: kill first server and remove lock
    let _ = child1.kill();
    let _ = child1.wait();
    let _ = std::fs::remove_file(lock_file_path(&home_path));
}
