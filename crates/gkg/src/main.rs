mod cli;

use cli::{Commands, GkgCli, run_indexing};

fn main() -> anyhow::Result<()> {
    let cli = GkgCli::parse_args();

    match cli.command {
        Commands::Index {
            workspace_path,
            threads,
            output,
            database,
            verbose,
        } => {
            init_logging(verbose);
            run_indexing(workspace_path, threads, output, database)
        }
    }
}

fn init_logging(verbose: bool) {
    let log_level = if verbose { "debug" } else { "info" };

    // Initialize tracing subscriber
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    println!("Logging level set to: {log_level}");
}
