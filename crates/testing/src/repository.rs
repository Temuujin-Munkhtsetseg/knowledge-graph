use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct TestRepository {
    pub dir: PathBuf,
}

// this is a helper to create a test repository with a git repo structure
// and optionally copy fixture files from the fixtures directory, which is
// located at the root of the project under fixtures/
// example usage:
// ```rust,ignore
// let temp_dir = TempDir::new().expect("Failed to create temp directory");
// let fixtures_dir = PathBuf::from("test-repo");
// let test_repo = TestRepository::new(temp_dir, Some(&fixtures_dir));
// let repo_path = test_repo.temp_dir.path();
// assert!(repo_path.exists());
// assert!(repo_path.join(".git").exists());
// ```
impl TestRepository {
    pub fn new(dir: &Path, fixture_dir_name: Option<&str>) -> Self {
        create_git_repo_structure(dir);
        initialize_git_repo(dir);

        if let Some(fixture_dir_name) = fixture_dir_name {
            copy_fixture_files(dir, fixture_dir_name);
        }

        Self {
            dir: dir.to_path_buf(),
        }
    }
}

fn create_git_repo_structure(repo_path: &Path) {
    fs::create_dir_all(repo_path.join(".git")).expect("Failed to create .git directory");
    fs::write(
        repo_path.join(".git/config"),
        "[core]\n    repositoryformatversion = 0\n",
    )
    .expect("Failed to write git config");
}

fn copy_fixture_files(repo_path: &Path, fixture_dir_name: &str) {
    let fixtures_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join(fixture_dir_name);

    copy_dir_all(&fixtures_path, repo_path).expect("Failed to copy fixture files");
}

// TODO: migrate this logic to gitalisk https://gitlab.com/gitlab-org/rust/gitalisk/-/issues/17
fn initialize_git_repo(repo_path: &Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to initialize git repository");

    Command::new("git")
        .args(["config", "--local", "user.name", "test-gl-user"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to configure git user name");

    Command::new("git")
        .args(["config", "--local", "user.email", "test-gl-user@gitlab.com"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to configure git user email");

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("Failed to add files to git");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to create initial commit");
}

pub fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
