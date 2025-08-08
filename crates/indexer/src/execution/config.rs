use crate::indexer::IndexingConfig;

pub struct IndexingConfigBuilder;

impl IndexingConfigBuilder {
    pub fn build(threads: usize) -> IndexingConfig {
        let effective_threads = IndexingConfigBuilder::get_effective_threads(threads);
        IndexingConfig {
            worker_threads: effective_threads,
            max_file_size: 5_000_000,
            respect_gitignore: true,
        }
    }

    pub fn get_effective_threads(threads: usize) -> usize {
        if threads == 0 {
            num_cpus::get()
        } else {
            threads
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_with_zero_threads() {
        let config = IndexingConfigBuilder::build(0);

        assert!(config.worker_threads > 0);
        assert_eq!(config.max_file_size, 5_000_000);
        assert!(config.respect_gitignore);
    }
}
