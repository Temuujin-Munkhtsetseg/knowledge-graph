use indexer::deployed::executor::DeployedIndexingExecutor;
use indexer::execution::config::IndexingConfigBuilder;
use std::ffi::CStr;
use std::os::raw::{c_char, c_ushort};
use std::path::PathBuf;

fn safe_c_char_to_pathbuf(c_string: *const c_char) -> Option<PathBuf> {
    if c_string.is_null() {
        return None;
    }

    unsafe {
        let c_str = CStr::from_ptr(c_string);
        c_str.to_str().ok().map(PathBuf::from)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn execute_repository_full_indexing(
    repository_path: *const c_char,
    database_path: *const c_char,
    parquet_path: *const c_char,
    threads: c_ushort,
) -> c_ushort {
    let repository_path = safe_c_char_to_pathbuf(repository_path).expect("Invalid repository path");
    let database_path = safe_c_char_to_pathbuf(database_path).expect("Invalid database path");
    let parquet_path = safe_c_char_to_pathbuf(parquet_path).expect("Invalid parquet path");

    let threads: usize = usize::from(threads);
    let config = IndexingConfigBuilder::build(threads); // Number of CPU Cores will be used instead

    let server_indexer =
        DeployedIndexingExecutor::new(repository_path, database_path, parquet_path, config);
    let result = server_indexer.execute();
    result.map_or(1, |_| 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::ptr;

    #[test]
    fn test_safe_c_char_to_pathbuf_null_pointer() {
        let result = safe_c_char_to_pathbuf(ptr::null());
        assert_eq!(result, None);
    }

    #[test]
    fn test_safe_c_char_to_pathbuf_valid_path() {
        let path_str = "/home/user/documents";
        let c_string = CString::new(path_str).unwrap();
        let c_ptr = c_string.as_ptr();

        let result = safe_c_char_to_pathbuf(c_ptr);
        assert_eq!(result, Some(PathBuf::from(path_str)));
    }

    #[test]
    fn test_safe_c_char_to_pathbuf_empty_string() {
        let c_string = CString::new("").unwrap();
        let c_ptr = c_string.as_ptr();

        let result = safe_c_char_to_pathbuf(c_ptr);
        assert_eq!(result, Some(PathBuf::from("")));
    }

    // Test for invalid UTF-8 sequences
    #[test]
    fn test_safe_c_char_to_pathbuf_invalid_utf8() {
        // Create a C string with invalid UTF-8
        let invalid_utf8_bytes = [0xFF, 0xFE, 0xFD, 0x00]; // Invalid UTF-8 sequence ending with null terminator
        let c_ptr = invalid_utf8_bytes.as_ptr() as *const c_char;

        let result = safe_c_char_to_pathbuf(c_ptr);
        assert_eq!(result, None);
    }
}
