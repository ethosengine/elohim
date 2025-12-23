//! HTTP routes for Doorway

pub mod api;
pub mod auth_routes;
pub mod blob;
pub mod health;
pub mod status;

pub use api::handle_api_request;
pub use auth_routes::handle_auth_request;
pub use blob::{handle_blob_request, error_response as blob_error_response, BlobError};
pub use health::health_check;
pub use status::status_check;
