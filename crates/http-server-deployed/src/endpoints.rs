pub mod health;

use axum::Router;

pub fn get_routes() -> Router {
    Router::new()
        // routes from all endpoints should be merged here
        .merge(health::get_routes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_are_not_empty() {
        let app = get_routes();

        assert!(app.has_routes(), "no routes are defined");
    }
}
