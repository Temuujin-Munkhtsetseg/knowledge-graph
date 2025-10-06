#![cfg(unix)]

use assert_cmd::prelude::*;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use serial_test::serial;
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdout, Command, Stdio};
use std::time::Duration;
use tempfile::{NamedTempFile, TempDir};

fn read_server_info_line(child_stdout: ChildStdout) -> String {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(child_stdout);
        let mut line = String::new();
        // Read lines until we find the "listening" line
        loop {
            line.clear();
            if reader.read_line(&mut line).is_ok() && !line.is_empty() {
                if line.contains("listening") {
                    let _ = tx.send(line);
                    break;
                }
            } else {
                break;
            }
        }
    });

    let line = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("timed out waiting for server JSON output");

    line.trim().to_string()
}

fn create_secret_file() -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().expect("create temp secret file");
    temp_file
        .write_all(b"test-secret-for-integration-tests")
        .expect("write secret to file");
    temp_file
}

#[test]
#[serial]
fn server_start_foreground_unix_socket_and_stop() {
    let temp_home = TempDir::new().expect("temp home");
    let socket_path = temp_home.path().join("socket");
    let data_dir = TempDir::new().expect("temp data dir");
    let secret_file = create_secret_file();

    // Start foreground server
    let mut cmd =
        Command::cargo_bin("http-server-deployed").expect("cargo bin http-server-deployed");
    cmd.arg("-s")
        .arg(&socket_path)
        .arg("--secret-path")
        .arg(secret_file.path())
        .arg("--data-dir")
        .arg(data_dir.path())
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().expect("spawn server start");
    let child_stdout = child.stdout.take().expect("capture stdout");
    let line = read_server_info_line(child_stdout);
    let message = format!("HTTP server listening on {}", socket_path.display());

    assert!(line.contains(&message));
    assert!(socket_path.exists());

    // Give server some time to set up async signal handlers
    std::thread::sleep(Duration::from_millis(100));

    signal::kill(Pid::from_raw(child.id() as i32), Signal::SIGTERM)
        .expect("failed to interrupt server");
    child.wait().expect("failed to wait on server shutdown");

    // Socket should be removed on ctrl+c or SIGTERM
    assert!(!socket_path.exists());
}

#[test]
#[serial]
fn server_start_foreground_tcp_socket_and_stop() {
    let bind_addr = "127.0.0.1:8833";
    let data_dir = TempDir::new().expect("temp data dir");
    let secret_file = create_secret_file();

    // Start foreground server
    let mut cmd =
        Command::cargo_bin("http-server-deployed").expect("cargo bin http-server-deployed");
    cmd.arg("-b")
        .arg(bind_addr)
        .arg("--secret-path")
        .arg(secret_file.path())
        .arg("--data-dir")
        .arg(data_dir.path())
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().expect("spawn server start");
    let child_stdout = child.stdout.take().expect("capture stdout");
    let line = read_server_info_line(child_stdout);
    let message = format!("HTTP server listening on {bind_addr}");

    assert!(line.contains(&message));

    signal::kill(Pid::from_raw(child.id() as i32), Signal::SIGTERM)
        .expect("failed to interrupt server");
    child.wait().expect("failed to wait on server shutdown");
}

#[test]
#[serial]
fn server_requires_data_dir_argument() {
    let bind_addr = "127.0.0.1:8834";
    let secret_file = create_secret_file();

    // Start server without --data_dir
    let mut cmd =
        Command::cargo_bin("http-server-deployed").expect("cargo bin http-server-deployed");
    cmd.arg("-b")
        .arg(bind_addr)
        .arg("--secret-path")
        .arg(secret_file.path());

    let result = cmd.assert();
    result.failure();
}

#[test]
#[serial]
fn server_creates_data_dir_if_missing() {
    let temp_home = TempDir::new().expect("temp home");
    let data_dir_path = temp_home.path().join("new_data_dir");
    let bind_addr = "127.0.0.1:8835";
    let secret_file = create_secret_file();

    // Verify directory doesn't exist yet
    assert!(!data_dir_path.exists());

    // Start server with non-existent data directory
    let mut cmd =
        Command::cargo_bin("http-server-deployed").expect("cargo bin http-server-deployed");
    cmd.arg("-b")
        .arg(bind_addr)
        .arg("--secret-path")
        .arg(secret_file.path())
        .arg("--data-dir")
        .arg(&data_dir_path)
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().expect("spawn server start");
    let child_stdout = child.stdout.take().expect("capture stdout");
    let line = read_server_info_line(child_stdout);
    let message = format!("HTTP server listening on {bind_addr}");

    assert!(line.contains(&message));

    // Verify directory was created
    assert!(data_dir_path.exists());
    assert!(data_dir_path.is_dir());

    signal::kill(Pid::from_raw(child.id() as i32), Signal::SIGTERM)
        .expect("failed to interrupt server");
    child.wait().expect("failed to wait on server shutdown");
}

#[test]
#[serial]
fn server_rejects_file_as_data_dir() {
    let temp_home = TempDir::new().expect("temp home");
    let file_path = temp_home.path().join("not_a_directory");
    std::fs::write(&file_path, "test").expect("create test file");

    let bind_addr = "127.0.0.1:8836";
    let secret_file = create_secret_file();

    // Start server with file instead of directory
    let mut cmd =
        Command::cargo_bin("http-server-deployed").expect("cargo bin http-server-deployed");
    cmd.arg("-b")
        .arg(bind_addr)
        .arg("--secret-path")
        .arg(secret_file.path())
        .arg("--data-dir")
        .arg(&file_path)
        .stderr(Stdio::piped());

    let output = cmd.output().expect("run command");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("DataDirectoryCreationFailed"));
}
