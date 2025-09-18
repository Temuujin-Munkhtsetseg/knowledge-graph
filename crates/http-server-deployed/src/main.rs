mod endpoints;

use clap::Parser;
use std::error::Error;
use tokio::net::UnixListener;
use tokio::signal;
use tracing::info;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // Socket file path to use
    #[arg(short, long, default_value = "/tmp/gkg-indexer-http.sock")]
    socket: String,
    // Server mode - server can run either in indexer or webserver mode
    #[arg(short, long, default_value = "indexer")]
    mode: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let args = Args::parse();
    let listener = UnixListener::bind(&args.socket)?;

    let app = endpoints::get_routes(args.mode);

    info!("HTTP server listening on {}", args.socket);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(args.socket))
        .await
        .unwrap();

    info!("HTTP server shut down gracefully");
    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}

async fn shutdown_signal(path: String) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { shutdown(path).await },
        _ = terminate => { shutdown(path).await },
    }
}

async fn shutdown(path: String) {
    tokio::fs::remove_file(path).await.unwrap();
}
