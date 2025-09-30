use crate::endpoints;
use axum::{
    extract::{Extension, Request},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::fs;
use tracing::{error, warn};

const GITLAB_ISSUER: &str = "gitlab";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Claims {
    pub(crate) iss: String,
    pub(crate) iat: i64,
    pub(crate) exp: i64,
}

impl Claims {
    pub(crate) fn new(issuer: String, ttl: Duration) -> Self {
        let now = Utc::now();
        Self {
            iss: issuer,
            iat: now.timestamp(),
            exp: (now + ttl).timestamp(),
        }
    }
}

#[derive(Clone)]
pub struct Auth {
    pub(crate) secret: Vec<u8>,
    pub(crate) issuer: String,
}

impl Auth {
    pub fn new(secret_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let secret_bytes = fs::read(secret_path)
            .map_err(|e| format!("Failed to read secret file {secret_path}: {e}"))?;

        // Convert to string, trim whitespace, then back to bytes
        let secret_str =
            std::str::from_utf8(&secret_bytes).map_err(|_| "Secret file contains invalid UTF-8")?;
        let secret = secret_str.trim().as_bytes().to_vec();

        if secret.is_empty() {
            return Err("Secret file is empty after trimming".into());
        }

        Ok(Self {
            secret,
            issuer: GITLAB_ISSUER.to_string(),
        })
    }

    fn verify_jwt(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.issuer]);

        let decoding_key = DecodingKey::from_secret(&self.secret);
        let token_data = decode::<Claims>(token, &decoding_key, &validation)?;

        Ok(token_data.claims)
    }

    fn verify_bearer_token(&self, auth_header: &str) -> Result<Claims, String> {
        const BEARER_PREFIX: &str = "Bearer ";

        if !auth_header.starts_with(BEARER_PREFIX) {
            return Err("Authorization header must start with 'Bearer '".to_string());
        }

        let token = auth_header.trim_start_matches(BEARER_PREFIX);

        self.verify_jwt(token)
            .map_err(|e| format!("JWT verification failed: {e}"))
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

async fn jwt_auth_middleware(
    Extension(auth): Extension<Auth>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            error!("Missing Authorization header");
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Missing Authorization header".to_string(),
                }),
            )
                .into_response()
        })?;

    match auth.verify_bearer_token(auth_header) {
        Ok(claims) => {
            // Token is valid, proceed with the request
            let mut request = request;
            request.extensions_mut().insert(claims);
            Ok(next.run(request).await)
        }
        Err(err) => {
            warn!("JWT verification failed: {}", err);
            Err((StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: err })).into_response())
        }
    }
}

pub async fn jwt_middleware_for_all(
    axum::extract::State(auth): axum::extract::State<Auth>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let path = request.uri().path();

    if endpoints::is_public_endpoint(path) {
        // Skip authentication for explicitly public endpoints only
        Ok(next.run(request).await)
    } else {
        // All other endpoints require JWT authentication (secure by default)
        jwt_auth_middleware(Extension(auth), request, next).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_auth_creation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temp_file, b"test-secret").unwrap();

        let auth = Auth::new(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(auth.secret, b"test-secret");
        assert_eq!(auth.issuer, GITLAB_ISSUER);
    }

    #[test]
    fn test_jwt_generation_and_verification() {
        let mut temp_file = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temp_file, b"test-secret").unwrap();

        let auth = Auth::new(temp_file.path().to_str().unwrap()).unwrap();
        let token = auth.generate_jwt(Duration::hours(1)).unwrap();

        let claims = auth.verify_jwt(&token).unwrap();
        assert_eq!(claims.iss, GITLAB_ISSUER);
    }

    #[test]
    fn test_bearer_token_verification() {
        let mut temp_file = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temp_file, b"test-secret").unwrap();

        let auth = Auth::new(temp_file.path().to_str().unwrap()).unwrap();
        let token = auth.generate_jwt(Duration::hours(1)).unwrap();
        let bearer_header = format!("Bearer {token}");

        let claims = auth.verify_bearer_token(&bearer_header).unwrap();
        assert_eq!(claims.iss, GITLAB_ISSUER);
    }

    #[test]
    fn test_bearer_token_invalid_format() {
        let mut temp_file = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temp_file, b"test-secret").unwrap();

        let auth = Auth::new(temp_file.path().to_str().unwrap()).unwrap();

        let result = auth.verify_bearer_token("Invalid token");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Bearer"));
    }
}
