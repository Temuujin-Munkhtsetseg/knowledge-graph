use http_server_deployed::{authentication, endpoints, metrics};

use axum::{middleware, Router};
use clap::Parser;
use logging::{init, LogMode};
use std::error::Error;
use std::path::PathBuf;
use tokio::net::{TcpListener, UnixListener};
use tokio::signal;
use tracing::{error, info};
use workspace_manager::DataDirectory;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // Socket file path to use (mutually exclusive with --bind)
    #[arg(short, long, conflicts_with = "bind")]
    socket: Option<String>,
    // Bind address to use (defaults to 0.0.0.0:8080)
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    bind: String,
    // Server mode - server can run either in indexer or webserver mode
    #[arg(short, long, default_value = "indexer")]
    mode: String,
    // Path to JWT secret file for authentication (required)
    #[arg(long)]
    secret_path: String,
    // Data directory for persistent storage (required)
    #[arg(long)]
    data_dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init(LogMode::ServerDeployed, false)?;

    let args = Args::parse();

    // Initialize data directory
    let data_directory = match DataDirectory::new(args.data_dir) {
        Ok(data_dir) => data_dir,
        Err(e) => {
            error!("Failed to initialize data directory: {}", e);
            return Err(e.into());
        }
    };

    info!(
        "Using data directory: {}",
        data_directory.root_path.display()
    );

    // Initialize JWT authentication
    let auth = match authentication::Auth::new(&args.secret_path) {
        Ok(auth) => auth,
        Err(e) => {
            error!("Failed to initialize authentication: {}", e);
            return Err(e);
        }
    };

    // Create routes and apply middleware layers
    let app = endpoints::get_routes(args.mode.clone())
        // Apply metrics middleware first (before auth) to track all requests
        .layer(middleware::from_fn(metrics::request_metrics_middleware))
        // Then apply JWT authentication
        .layer(middleware::from_fn_with_state(
            auth,
            authentication::jwt_middleware_for_all,
        ));

    if let Some(socket) = args.socket {
        serve_unix_socket(socket, app).await;
    } else {
        serve_tcp_socket(args.bind, app).await;
    }

    info!("HTTP server shut down gracefully");
    Ok(())
}

async fn serve_unix_socket(socket: String, app: Router) {
    let listener = UnixListener::bind(socket.clone()).unwrap();
    info!("HTTP server listening on {}", socket);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(socket))
        .await
        .unwrap();
}

async fn serve_tcp_socket(bind: String, app: Router) {
    let listener = TcpListener::bind(bind.clone()).await.unwrap();
    info!("HTTP server listening on {}", bind);
    axum::serve(listener, app).await.unwrap();
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
