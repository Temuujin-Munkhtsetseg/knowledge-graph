use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let is_ci = env::var("CI").is_ok();
    if is_ci {
        println!("cargo:warning=Skipping frontend build in CI");
        return;
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let frontend_dir = workspace_root.join("packages/frontend");

    if env::var("SKIP_FRONTEND_BUILD").is_ok() {
        println!(
            "cargo:warning=Skipping frontend build due to SKIP_FRONTEND_BUILD environment variable"
        );
        return;
    }

    if !frontend_dir.exists() {
        println!("cargo:warning=Frontend directory not found, skipping frontend build");
        return;
    }

    let npm_check = Command::new("npm").arg("--version").output();

    if npm_check.is_err() {
        println!(
            "cargo:warning=npm not found, skipping frontend build. Install Node.js and npm to build frontend automatically."
        );
        return;
    }

    println!("cargo:warning=Building frontend application...");

    let install_status = Command::new("npm")
        .arg("install")
        .current_dir(&frontend_dir)
        .status()
        .expect("Failed to execute npm install");
    if !install_status.success() {
        panic!("npm install failed");
    }

    let build_output = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(&frontend_dir)
        .output()
        .expect("Failed to execute npm run build");

    if !build_output.status.success() {
        println!("cargo:warning=npm run build failed with output:");
        println!(
            "cargo:warning=stdout: {}",
            String::from_utf8_lossy(&build_output.stdout)
        );
        println!(
            "cargo:warning=stderr: {}",
            String::from_utf8_lossy(&build_output.stderr)
        );
        panic!("npm run build failed");
    }

    println!("cargo:warning=npm run build output:");
    println!(
        "cargo:warning=stdout: {}",
        String::from_utf8_lossy(&build_output.stdout)
    );
    if !build_output.stderr.is_empty() {
        println!(
            "cargo:warning=stderr: {}",
            String::from_utf8_lossy(&build_output.stderr)
        );
    }

    let dist_dir = frontend_dir.join("dist");
    if !dist_dir.exists() {
        panic!("Frontend build completed but dist directory was not created");
    }

    println!("cargo:warning=Frontend build completed successfully");
}
