//! # HTTP Server Contract System
//!
//! This module provides a type-safe contract system for defining HTTP endpoints with
//! compile-time TypeScript generation. It separates request handling into path, body, and
//! query parameters for maximum flexibility.
//!
//! ### EndpointContract Trait
//! All endpoints must implement the `EndpointContract` trait which defines:
//! - `METHOD`: HTTP method (GET, POST, etc.)
//! - `PATH`: URL path as a string literal
//! - `PathRequest`: Type for URL path parameters (e.g., `/users/{id}`)
//! - `BodyRequest`: Type for request body parameters (use `EmptyRequest` if not needed)
//! - `QueryRequest`: Type for query parameters (use `EmptyRequest` if not needed)  
//! - `Response`: Response type with status code variants
//!
//! ### Request Types
//! - **Path Request**: Dynamic URL segments (e.g., `/users/{id}` where `id` is a parameter)
//! - **Body Request**: Data sent in the request body (typically for POST/PUT/PATCH)
//! - **Query Request**: URL query parameters (e.g., `/api/users?limit=10&offset=20`)
//! - **EmptyRequest**: Use when no parameters are needed
//!
//! ## Configuration Pattern
//!
//! Create a config struct that implements `EndpointConfigTypes` to group all your request/response types:
//!
//! ### Simple GET Endpoint (no parameters)
//! ```rust,ignore
//! use http_server::define_endpoint;
//! use http_server::contract::{EmptyRequest, EndpointConfigTypes};
//!
//! pub struct GetHealthEndpointConfig;
//!
//! impl EndpointConfigTypes for GetHealthEndpointConfig {
//!     type PathRequest = EmptyRequest;
//!     type BodyRequest = EmptyRequest;
//!     type QueryRequest = EmptyRequest;
//!     type Response = HealthResponse;
//! }
//!
//! define_endpoint! {
//!     GetHealthEndpoint,
//!     GetHealthEndpointDef,
//!     Get,
//!     "/health",
//!     ts_path_type = "\"/health\"",
//!     config = GetHealthEndpointConfig
//! }
//! ```
//!
//! This generates a TypeScript type like:
//!
//! ```typescript
//! export type GetHealthEndpointDef = {
//!   method: HttpMethod,
//!   path: "/health",
//!   path_request: EmptyRequest,
//!   body_request: EmptyRequest,
//!   query_request: EmptyRequest,
//!   responses: HealthResponse
//! };
//! ```
//!
//! ### GET Endpoint with Path Parameters
//! ```rust,ignore
//! use serde::{Deserialize, Serialize};
//! use ts_rs::TS;
//!
//! #[derive(Deserialize, Serialize, TS, Default)]
//! pub struct UserPathParams {
//!     pub id: String,
//! }
//!
//! pub struct GetUserEndpointConfig;
//!
//! impl EndpointConfigTypes for GetUserEndpointConfig {
//!     type PathRequest = UserPathParams;
//!     type BodyRequest = EmptyRequest;
//!     type QueryRequest = EmptyRequest;
//!     type Response = UserResponse;
//! }
//!
//! define_endpoint! {
//!     GetUserEndpoint,
//!     GetUserEndpointDef,
//!     Get,
//!     "/users/{id}",
//!     ts_path_type = "\"/users/${string}\"",
//!     config = GetUserEndpointConfig
//! }
//! ```
//!
//! ### POST Endpoint with Body Parameters
//! ```rust,ignore
//! pub struct CreateUserEndpointConfig;
//!
//! impl EndpointConfigTypes for CreateUserEndpointConfig {
//!     type PathRequest = EmptyRequest;
//!     type BodyRequest = CreateUserRequest;
//!     type QueryRequest = EmptyRequest;
//!     type Response = UserResponse;
//! }
//!
//! define_endpoint! {
//!     CreateUserEndpoint,
//!     CreateUserEndpointDef,
//!     Post,
//!     "/users",
//!     ts_path_type = "\"/users\"",
//!     config = CreateUserEndpointConfig
//! }
//! ```
//!
//! ### GET Endpoint with Query Parameters
//! ```rust,ignore
//! #[derive(Deserialize, Serialize, TS, Default)]
//! pub struct UserQueryParams {
//!     pub limit: Option<u32>,
//!     pub offset: Option<u32>,
//!     pub search: Option<String>,
//! }
//!
//! pub struct ListUsersEndpointConfig;
//!
//! impl EndpointConfigTypes for ListUsersEndpointConfig {
//!     type PathRequest = EmptyRequest;
//!     type BodyRequest = EmptyRequest;
//!     type QueryRequest = UserQueryParams;
//!     type Response = UsersListResponse;
//! }
//!
//! define_endpoint! {
//!     ListUsersEndpoint,
//!     ListUsersEndpointDef,
//!     Get,
//!     "/users",
//!     ts_path_type = "\"/users\"",
//!     config = ListUsersEndpointConfig
//! }
//! ```
//!
//! ### Complex Endpoint with All Parameter Types
//! ```rust,ignore
//! pub struct UpdateUserEndpointConfig;
//!
//! impl EndpointConfigTypes for UpdateUserEndpointConfig {
//!     type PathRequest = UserPathParams;        // /users/{id}
//!     type BodyRequest = UpdateUserRequest;     // JSON body data
//!     type QueryRequest = UpdateQueryParams;    // ?force=true&notify=false
//!     type Response = UserResponse;
//! }
//!
//! define_endpoint! {
//!     UpdateUserEndpoint,
//!     UpdateUserEndpointDef,
//!     Put,
//!     "/users/{id}",
//!     ts_path_type = "\"/users/${string}\"",
//!     config = UpdateUserEndpointConfig
//! }
//! ```
//!
//! ## TypeScript Generation
//!
//! The contract system automatically generates TypeScript types with:
//! - **String literal types** for paths (e.g., `"/users"` instead of `string`)
//! - **Template literal types** for parameterized paths (e.g., `/users/${string}`)
//! - **Separate fields** for `path_request`, `body_request`, and `query_request`
//! - **Response type variants** matching your Rust definitions
//!
//! ## Response Type Patterns
//!
//! Response types should use serde field renaming for HTTP status codes:
//! ```rust,ignore
//! use serde::Serialize;
//! use ts_rs::TS;
//!
//! #[derive(Serialize, TS, Default)]
//! pub struct MyEndpointResponses {
//!     #[serde(rename = "200")]
//!     #[serde(skip_serializing_if = "Option::is_none")]
//!     pub ok: Option<SuccessResponse>,
//!     
//!     #[serde(rename = "400")]
//!     #[serde(skip_serializing_if = "Option::is_none")]
//!     pub bad_request: Option<ErrorResponse>,
//!     
//!     #[serde(rename = "500")]
//!     #[serde(skip_serializing_if = "Option::is_none")]
//!     pub internal_server_error: Option<ErrorResponse>,
//! }
//! ```

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[ts(export, export_to = "api.ts")]
pub enum HttpMethod {
    #[serde(rename = "GET")]
    Get,
    #[serde(rename = "POST")]
    Post,
}

pub trait ApiRequest:
    Serialize + for<'de> Deserialize<'de> + TS + Default + Send + Sync + 'static
{
}
impl<T> ApiRequest for T where
    T: Serialize + for<'de> Deserialize<'de> + TS + Default + Send + Sync + 'static
{
}

#[derive(Serialize, Deserialize, TS, Default, Debug, Clone, PartialEq)]
#[ts(export, export_to = "api.ts")]
pub struct EmptyRequest;

pub trait ApiResponse: Serialize + TS + Default + Send + Sync + 'static {}
impl<T> ApiResponse for T where T: Serialize + TS + Default + Send + Sync + 'static {}

pub trait EndpointContract {
    const METHOD: HttpMethod;
    const PATH: &'static str;

    type PathRequest: ApiRequest;
    type BodyRequest: ApiRequest;
    type QueryRequest: ApiRequest;
    type Response: ApiResponse;
}

/// Trait for endpoint configuration - implement this for your config struct
pub trait EndpointConfigTypes {
    type PathRequest: ApiRequest;
    type BodyRequest: ApiRequest;
    type QueryRequest: ApiRequest;
    type Response: ApiResponse;
}

#[macro_export]
macro_rules! define_endpoint {
    (
        $endpoint_name:ident,
        $def_name:ident,
        $method:ident,
        $path:literal,
        ts_path_type = $ts_path_type:literal,
        config = $config_type:ty
    ) => {
        $crate::define_endpoint! {
            $endpoint_name,
            $def_name,
            $method,
            $path,
            ts_path_type = $ts_path_type,
            config = $config_type,
            export_to = "api.ts"
        }
    };
    (
        $endpoint_name:ident,
        $def_name:ident,
        $method:ident,
        $path:literal,
        ts_path_type = $ts_path_type:literal,
        config = $config_type:ty,
        export_to = $export_path:literal
    ) => {
        pub struct $endpoint_name;

        impl $crate::contract::EndpointContract for $endpoint_name {
            const METHOD: $crate::contract::HttpMethod = $crate::contract::HttpMethod::$method;
            const PATH: &'static str = $path;
            type PathRequest = <$config_type as $crate::contract::EndpointConfigTypes>::PathRequest;
            type BodyRequest = <$config_type as $crate::contract::EndpointConfigTypes>::BodyRequest;
            type QueryRequest = <$config_type as $crate::contract::EndpointConfigTypes>::QueryRequest;
            type Response = <$config_type as $crate::contract::EndpointConfigTypes>::Response;
        }

        #[derive(serde::Serialize, ts_rs::TS)]
        #[ts(export, export_to = $export_path)]
        pub struct $def_name {
            pub method: $crate::contract::HttpMethod,
            #[ts(type = $ts_path_type)]
            pub path: String,
            pub path_request: <$config_type as $crate::contract::EndpointConfigTypes>::PathRequest,
            pub body_request: <$config_type as $crate::contract::EndpointConfigTypes>::BodyRequest,
            pub query_request: <$config_type as $crate::contract::EndpointConfigTypes>::QueryRequest,
            pub responses: <$config_type as $crate::contract::EndpointConfigTypes>::Response,
        }

        impl Default for $def_name {
            fn default() -> Self {
                Self {
                    method: $crate::contract::HttpMethod::$method,
                    path: $path.to_string(),
                    path_request: <<$config_type as $crate::contract::EndpointConfigTypes>::PathRequest>::default(),
                    body_request: <<$config_type as $crate::contract::EndpointConfigTypes>::BodyRequest>::default(),
                    query_request: <<$config_type as $crate::contract::EndpointConfigTypes>::QueryRequest>::default(),
                    responses: <<$config_type as $crate::contract::EndpointConfigTypes>::Response>::default(),
                }
            }
        }
    };
}

#[cfg(test)]
mod contract_tests;
