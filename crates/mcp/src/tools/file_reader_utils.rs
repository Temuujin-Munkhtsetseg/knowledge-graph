use futures::future::join_all;
use std::io::{self};
use tokio::fs::File as AsyncFile;
use tokio::io::{AsyncBufReadExt, BufReader as AsyncBufReader};

/// Reads multiple line ranges from files concurrently and returns per-chunk results in input order.
///
/// - Errors are isolated per chunk; one failure does not fail the whole call.
/// - Each successful chunk contains lines joined with trailing newlines preserved.
/// - Returns the same number of results as input chunks.
///
/// Example (simplified):
/// ```rust,ignore
/// // Prepare input: (path, start_line, end_line)
/// let chunks = vec![
///     ("/tmp/file1.txt".to_string(), 1, 1),        // OK => "A\n"
///     ("/tmp/missing.txt".to_string(), 1, 1),      // Err(NotFound)
///     ("/tmp/file3.txt".to_string(), 2, 3),        // OK => "Y\nZ\n"
/// ];
///
/// // Call the function (async)
/// let results = read_file_chunks(chunks).await?;
///
/// // Inspect outcomes stay aligned with input order
/// assert!(results[0].is_ok());
/// assert!(results[1].is_err());
/// assert!(results[2].is_ok());
/// ```
///
/// Sample return shape:
/// ```text
/// [
///   Ok("A\n"),
///   Err(NotFound),
///   Ok("Y\nZ\n"),
/// ]
/// ```
pub async fn read_file_chunks(
    chunks: Vec<(String, usize, usize)>,
) -> io::Result<Vec<io::Result<String>>> {
    let chunks_len = chunks.len();
    let mut tasks = Vec::with_capacity(chunks_len);

    for (path, start_line, end_line) in chunks {
        let task =
            tokio::spawn(async move { read_file_chunk_async(&path, start_line, end_line).await });
        tasks.push(task);
    }

    let task_results = join_all(tasks).await;

    let mut results = Vec::with_capacity(chunks_len);
    for task_result in task_results {
        match task_result {
            Ok(chunk_result) => results.push(chunk_result),
            Err(join_error) => results.push(Err(io::Error::other(format!(
                "Task join error: {}",
                join_error
            )))),
        }
    }

    Ok(results)
}

async fn read_file_chunk_async(
    path: &str,
    start_line: usize,
    end_line: usize,
) -> io::Result<String> {
    if start_line == 0 || end_line < start_line {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid line range: start_line must be >= 1 and end_line must be >= start_line",
        ));
    }

    let file = AsyncFile::open(path).await?;
    let reader = AsyncBufReader::new(file);
    let mut lines = reader.lines();

    // Skip lines before start_line
    for _ in 1..start_line {
        if lines.next_line().await?.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Start line {} exceeds file length", start_line),
            ));
        }
    }

    let mut result = String::new();
    let mut current_line = start_line;

    while let Some(line) = lines.next_line().await? {
        if current_line > end_line {
            break;
        }
        result.push_str(&line);
        result.push('\n');
        current_line += 1;
    }

    // Check if we reached EOF before end_line
    if current_line <= end_line {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "End line {} exceeds file length (file has {} lines)",
                end_line,
                current_line - 1
            ),
        ));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp_file(contents: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("create temp file");
        file.write_all(contents.as_bytes()).expect("write contents");
        file.flush().expect("flush");
        file
    }

    #[tokio::test]
    async fn reads_single_chunk_ok() {
        let file = write_temp_file("line1\nline2\nline3\nline4\n");
        let path = file.path().to_string_lossy().to_string();

        let res = read_file_chunks(vec![(path, 2, 3)]).await.unwrap();
        let out = res.into_iter().next().unwrap().as_ref().unwrap().clone();
        assert_eq!(out, "line2\nline3\n");
    }

    #[tokio::test]
    async fn reads_multiple_chunks_ok() {
        let file = write_temp_file("a\nb\nc\nd\ne\n");
        let path = file.path().to_string_lossy().to_string();

        let chunks = vec![
            (path.clone(), 1, 1),
            (path.clone(), 2, 4),
            (path.clone(), 5, 5),
        ];

        let results = read_file_chunks(chunks).await.unwrap();
        let outputs: Vec<_> = results.into_iter().map(|r| r.unwrap()).collect();

        assert_eq!(outputs[0], "a\n");
        assert_eq!(outputs[1], "b\nc\nd\n");
        assert_eq!(outputs[2], "e\n");
    }

    #[tokio::test]
    async fn invalid_range_start_zero() {
        let file = write_temp_file("x\ny\n");
        let path = file.path().to_string_lossy().to_string();
        let results = read_file_chunks(vec![(path, 0, 1)]).await.unwrap();
        assert!(results[0].is_err());
    }

    #[tokio::test]
    async fn invalid_range_end_lt_start() {
        let file = write_temp_file("x\ny\n");
        let path = file.path().to_string_lossy().to_string();
        let results = read_file_chunks(vec![(path, 2, 1)]).await.unwrap();
        assert!(results[0].is_err());
    }

    #[tokio::test]
    async fn start_beyond_eof_errors() {
        let file = write_temp_file("1\n2\n3\n");
        let path = file.path().to_string_lossy().to_string();
        let results = read_file_chunks(vec![(path, 5, 6)]).await.unwrap();
        let err = results[0].as_ref().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("Start line"));
    }

    #[tokio::test]
    async fn end_beyond_eof_errors() {
        let file = write_temp_file("1\n2\n3\n");
        let path = file.path().to_string_lossy().to_string();
        let results = read_file_chunks(vec![(path, 2, 10)]).await.unwrap();
        let err = results[0].as_ref().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("End line"));
    }

    #[tokio::test]
    async fn non_existent_file_errors() {
        let path = "/nonexistent/path/xyz.txt".to_string();
        let results = read_file_chunks(vec![(path, 1, 1)]).await.unwrap();
        assert!(results[0].is_err());
    }

    #[tokio::test]
    async fn mixed_ok_and_error_keeps_order() {
        let f1 = write_temp_file("A\nB\n");
        let f3 = write_temp_file("X\nY\nZ\n");
        let p1 = f1.path().to_string_lossy().to_string();
        let p3 = f3.path().to_string_lossy().to_string();
        let p2 = "/nonexistent/path/xyz.txt".to_string();

        let chunks = vec![(p1, 1, 1), (p2, 1, 1), (p3, 2, 3)];

        let results = read_file_chunks(chunks).await.unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].as_ref().unwrap(), "A\n");
        assert!(results[1].is_err());
        assert_eq!(results[2].as_ref().unwrap(), "Y\nZ\n");
    }
}
