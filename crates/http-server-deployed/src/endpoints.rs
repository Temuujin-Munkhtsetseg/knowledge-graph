pub mod health;
pub mod indexer;
pub mod metrics;
pub mod webserver;

use axum::Router;

/// List of endpoints that are explicitly allowed without authentication.
/// All other endpoints require JWT authentication by default (secure by default).
pub const PUBLIC_ENDPOINTS: &[&str] = &["/health", "/metrics"];

/// Check if a path is a public endpoint (computed at compile time via constant lookup).
pub fn is_public_endpoint(path: &str) -> bool {
    PUBLIC_ENDPOINTS.contains(&path)
}

pub fn get_routes(mode: String) -> Router {
    // routes from all endpoints should be merged here
    let router = Router::new()
        // Public endpoints available in all modes
        .merge(health::get_routes())
        .merge(metrics::get_routes())
        .merge(match mode.as_str() {
            "indexer" => indexer::get_routes(),
            "webserver" => webserver::get_routes(),
            _ => {
                println!("unknown mode {mode}");
                Router::new()
            }
        });

    router
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_are_not_empty() {
        let app = get_routes("indexer".to_string());
        assert!(app.has_routes(), "no routes are defined");

        let app = get_routes("webserver".to_string());
        assert!(app.has_routes(), "no routes are defined");
    }

    #[test]
    fn test_public_endpoint_detection() {
        assert!(is_public_endpoint("/health"));
        assert!(is_public_endpoint("/metrics"));
        assert!(!is_public_endpoint("/v1/tool"));
        assert!(!is_public_endpoint("/webserver/v1/tool"));
    }
}
