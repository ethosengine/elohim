//! HTTP routes for Doorway

pub mod api;
pub mod auth_routes;
pub mod health;
pub mod status;

pub use api::handle_api_request;
pub use auth_routes::handle_auth_request;
pub use health::health_check;
pub use status::status_check;
