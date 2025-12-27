//! HTTP routes for Doorway

pub mod admin;
pub mod api;
pub mod apps;
pub mod auth_routes;
pub mod blob;
pub mod dashboard_ws;
pub mod health;
pub mod status;
pub mod stream;

pub use admin::{
    handle_nodes, handle_node_by_id, handle_cluster_metrics,
    handle_resources, handle_custodians,
};
pub use api::handle_api_request;
pub use auth_routes::handle_auth_request;
pub use blob::{handle_blob_request, error_response as blob_error_response, BlobError};
pub use dashboard_ws::handle_dashboard_ws;
pub use health::health_check;
pub use status::status_check;
pub use stream::handle_stream_request;
