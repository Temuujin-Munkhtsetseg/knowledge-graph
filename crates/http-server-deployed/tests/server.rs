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
        // Read a single line; if this blocks, the timeout below will handle it
        if reader.read_line(&mut line).is_ok() {
            let _ = tx.send(line);
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
    let secret_file = create_secret_file();

    // Start foreground server
    let mut cmd =
        Command::cargo_bin("http-server-deployed").expect("cargo bin http-server-deployed");
    cmd.arg("-s")
        .arg(&socket_path)
        .arg("--secret-path")
        .arg(secret_file.path())
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().expect("spawn server start");
    let child_stdout = child.stdout.take().expect("capture stdout");
    let line = read_server_info_line(child_stdout);
    let message = format!("HTTP server listening on {}", socket_path.display());

    assert!(line.ends_with(&message));
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
    let secret_file = create_secret_file();

    // Start foreground server
    let mut cmd =
        Command::cargo_bin("http-server-deployed").expect("cargo bin http-server-deployed");
    cmd.arg("-b")
        .arg(bind_addr)
        .arg("--secret-path")
        .arg(secret_file.path())
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().expect("spawn server start");
    let child_stdout = child.stdout.take().expect("capture stdout");
    let line = read_server_info_line(child_stdout);
    let message = format!("HTTP server listening on {bind_addr}");

    assert!(line.ends_with(&message));

    signal::kill(Pid::from_raw(child.id() as i32), Signal::SIGTERM)
        .expect("failed to interrupt server");
    child.wait().expect("failed to wait on server shutdown");
}
