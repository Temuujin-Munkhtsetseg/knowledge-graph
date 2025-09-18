pub mod health;
pub mod indexer;
pub mod webserver;

use axum::Router;

pub fn get_routes(mode: String) -> Router {
    // routes from all endpoints should be merged here
    let router = Router::new()
        // health endpoint is used in all modes
        .merge(health::get_routes())
        .merge(match mode.as_str() {
            "indexer" => indexer::get_routes(),
            "webserver" => webserver::get_routes(),
            _ => {
                println!("unknown mode {}", mode);
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
}
