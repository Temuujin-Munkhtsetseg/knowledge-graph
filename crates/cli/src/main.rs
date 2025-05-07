use gitalisk_core::{get_repository_status, GitRepositoryStatus, GitStatusError};
use indexer::add_two_numbers;

pub fn get_status(path: &str) -> Result<GitRepositoryStatus, GitStatusError> {
    get_repository_status(path)
}

// Note: this is temporary code we will remove later
fn main() {
    let current_dir = std::env::current_dir().unwrap();
    println!("Current directory: {}", current_dir.to_str().unwrap());
    let status = get_status(current_dir.to_str().unwrap());
    println!(
        "Repository path: {}",
        status.as_ref().unwrap().repository_path
    );
    println!(
        "Branch name: {}",
        status.as_ref().unwrap().branch_name.as_ref().unwrap()
    );
    println!("File count: {}", status.as_ref().unwrap().files.len());
    println!("Add two numbers: {}", add_two_numbers(2, 2));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_status() {
        let current_dir = std::env::current_dir().unwrap();
        // two directories up from the current directory
        let repository_root = current_dir.parent().unwrap().parent().unwrap();
        let status = get_status(repository_root.to_str().unwrap()).unwrap();
        println!("Repository path: {}", status.repository_path);
        println!("Branch name: {}", status.branch_name.as_ref().unwrap());
        println!("File count: {}", status.files.len());
        assert_eq!(status.repository_path, repository_root.to_str().unwrap());
        assert!(status.branch_name.is_some());
        assert!(!status.files.is_empty());
    }
}
