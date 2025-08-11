#![cfg(unix)]

use assert_cmd::prelude::*;
use serde::Deserialize;
use serial_test::serial;
use std::io::{BufRead, BufReader};
use std::net::{TcpStream, ToSocketAddrs};
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

fn wait_for_port(port: u16, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    let addr = format!("127.0.0.1:{port}");
    while std::time::Instant::now() < deadline {
        if TcpStream::connect_timeout(
            &addr.to_socket_addrs().unwrap().next().unwrap(),
            Duration::from_millis(100),
        )
        .is_ok()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn wait_for_port_closed(port: u16, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    let addr = format!("127.0.0.1:{port}");
    while std::time::Instant::now() < deadline {
        if TcpStream::connect_timeout(
            &addr.to_socket_addrs().unwrap().next().unwrap(),
            Duration::from_millis(100),
        )
        .is_err()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
#[serial]
fn server_starts_and_prints_json_and_creates_lockfile() {
    let temp_home = TempDir::new().expect("temp home");
    let home_path = temp_home.path().to_path_buf();

    let mut cmd = Command::cargo_bin("gkg").expect("cargo bin gkg");
    cmd.arg("server")
        .arg("start")
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
    let obj: serde_json::Value = serde_json::from_str(contents.trim()).expect("json lock");
    let port_from_lock: u16 = obj.get("port").and_then(|v| v.as_u64()).unwrap() as u16;
    assert_eq!(port_from_lock, server_info.port);

    // Cleanup: kill server and remove lock file if still present
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(lock_path);
}

#[test]
#[serial]
fn second_server_prints_same_port_and_exits() {
    let temp_home = TempDir::new().expect("temp home");
    let home_path = temp_home.path().to_path_buf();

    // Start the first server (foreground)
    let mut cmd1 = Command::cargo_bin("gkg").expect("cargo bin gkg");
    cmd1.arg("server")
        .arg("start")
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
        .arg("start")
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

#[test]
#[serial]
fn server_start_foreground_and_stop() {
    let temp_home = TempDir::new().expect("temp home");
    let home_path = temp_home.path().to_path_buf();

    // Start foreground server
    let mut cmd = Command::cargo_bin("gkg").expect("cargo bin gkg");
    cmd.arg("server")
        .arg("start")
        .env("HOME", &home_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().expect("spawn gkg server start");
    let child_stdout = child.stdout.take().expect("capture stdout");
    let info = read_server_info_line(child_stdout);

    assert!(
        wait_for_port(info.port, Duration::from_secs(3)),
        "server did not start listening in time"
    );

    // Stop the server via CLI
    let assert = Command::cargo_bin("gkg")
        .expect("cargo bin gkg")
        .arg("server")
        .arg("stop")
        .env("HOME", &home_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .assert()
        .success();

    // Stop should print the same port
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next().expect("expected JSON from stop");
    let stop_info: ServerInfo = serde_json::from_str(line).expect("invalid JSON from stop");
    assert_eq!(stop_info.port, info.port);

    // Wait for child to exit and verify port closed
    let _ = child.wait();
    assert!(
        wait_for_port_closed(info.port, Duration::from_secs(3)),
        "server did not stop in time"
    );

    // Lock file should be removed
    assert!(!lock_file_path(&home_path).exists());
}

#[test]
#[serial]
fn server_start_detached_and_stop() {
    let temp_home = TempDir::new().expect("temp home");
    let home_path = temp_home.path().to_path_buf();

    // Start detached server
    let assert = Command::cargo_bin("gkg")
        .expect("cargo bin gkg")
        .arg("server")
        .arg("start")
        .arg("--detached")
        .env("HOME", &home_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .next()
        .expect("expected JSON from start --detached");
    let info: ServerInfo = serde_json::from_str(line).expect("invalid JSON from detached start");

    assert!(
        wait_for_port(info.port, Duration::from_secs(3)),
        "detached server did not start listening in time"
    );

    // Verify lock file exists and contains port
    let lock_path = lock_file_path(&home_path);
    let contents = std::fs::read_to_string(&lock_path).expect("read lock file");
    let obj: serde_json::Value = serde_json::from_str(contents.trim()).expect("json lock");
    let port_from_lock: u16 = obj.get("port").and_then(|v| v.as_u64()).unwrap() as u16;
    assert_eq!(port_from_lock, info.port);

    // Stop
    let assert = Command::cargo_bin("gkg")
        .expect("cargo bin gkg")
        .arg("server")
        .arg("stop")
        .env("HOME", &home_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next().expect("expected JSON from stop");
    let stop_info: ServerInfo = serde_json::from_str(line).expect("invalid JSON from stop");
    assert_eq!(stop_info.port, info.port);

    assert!(
        wait_for_port_closed(info.port, Duration::from_secs(3)),
        "detached server did not stop in time"
    );
    assert!(!lock_path.exists());
}
