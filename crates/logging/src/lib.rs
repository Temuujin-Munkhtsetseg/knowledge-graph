//! This crate provides logging initialization for the gkg application.
//!
//! It supports three modes:
//! - CLI mode: logs to STDOUT.
//! - ServerForeground mode: logs to STDERR and to a rolling file (keeps STDOUT clean for protocol output).
//! - ServerBackground mode: logs to a rolling file in the system's data directory.
//!
//! The server logs are rolled over when they reach 5 MB. Rotated logs are
//! compressed. The maximum number of rotated logs is 20.

use anyhow::Result;
use file_rotate::{ContentLimit, FileRotate, compression::Compression, suffix::AppendCount};
use tracing_subscriber::{EnvFilter, fmt::writer::MakeWriterExt};
use workspace_manager::data_directory::DataDirectory;

pub enum LogMode {
    Cli,
    ServerForeground,
    ServerBackground,
}

pub fn init(
    mode: LogMode,
    verbose: bool,
) -> Result<Option<tracing_appender::non_blocking::WorkerGuard>> {
    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    match mode {
        LogMode::Cli => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_target(false)
                .init();
            Ok(None)
        }
        LogMode::ServerForeground => {
            let data_dir = DataDirectory::get_system_data_directory()?;
            let log_dir = data_dir.join("logs");

            let writer = FileRotate::new(
                log_dir.join("logs.log"),
                AppendCount::new(20),
                ContentLimit::Bytes(5 * 1024 * 1024),
                Compression::OnRotate(1),
                None,
            );

            let (non_blocking, guard) = tracing_appender::non_blocking(writer);

            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(
                    non_blocking
                        .with_max_level(tracing::Level::INFO)
                        .and(std::io::stderr),
                )
                .with_ansi(false)
                .init();

            Ok(Some(guard))
        }
        LogMode::ServerBackground => {
            let data_dir = DataDirectory::get_system_data_directory()?;
            let log_dir = data_dir.join("logs");

            let writer = FileRotate::new(
                log_dir.join("logs.log"),
                AppendCount::new(20),
                ContentLimit::Bytes(5 * 1024 * 1024),
                Compression::OnRotate(1),
                None,
            );

            let (non_blocking, guard) = tracing_appender::non_blocking(writer);

            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(non_blocking.with_max_level(tracing::Level::INFO))
                .with_ansi(false)
                .json()
                .init();

            Ok(Some(guard))
        }
    }
}
