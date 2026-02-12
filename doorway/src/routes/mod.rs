//! HTTP routes for Doorway

pub mod admin;
pub mod admin_conductors;
pub mod admin_users;
pub mod api;
pub mod apps;
pub mod auth_routes;
pub mod blob;
pub mod dashboard_ws;
pub mod db;
pub mod debug_stream;
pub mod federation;
pub mod health;
pub mod identity;
pub mod import;
pub mod import_ws;
pub mod seed;
pub mod status;
pub mod stream;
pub mod threshold;
pub mod zome_helpers;

pub use admin::{
    handle_cluster_metrics, handle_custodians, handle_node_by_id, handle_nodes, handle_resources,
};
pub use admin_conductors::{
    handle_agent_conductor, handle_assign_agent, handle_conductor_agents, handle_deprovision_user,
    handle_force_graduation, handle_graduation_completed, handle_graduation_pending,
    handle_list_conductors, handle_list_hosted_users, handle_provision_user,
};
pub use admin_users::{
    check_quota_if_user,
    handle_admin_users_request,
    track_bandwidth_if_user,
    track_query_if_user,
    // Usage tracking helpers for integration with other routes
    try_extract_user_id_for_tracking,
    MongoQuotaEnforcer,
    MongoUsageTracker,
    QuotaEnforcer,
    QuotaStatus,
    UsageTracker,
};
pub use api::handle_api_request;
pub use apps::handle_app_request;
pub use auth_routes::handle_auth_request;
pub use blob::{
    error_response as blob_error_response, handle_blob_request, handle_blob_request_with_fallback,
    handle_blob_request_with_storage_proxy, BlobContext, BlobError,
};
pub use dashboard_ws::handle_dashboard_ws;
pub use db::handle_db_request;
pub use debug_stream::{handle_debug_stream, DebugEvent, DebugHub};
pub use federation::{handle_doorway_keys, handle_federation_doorways};
pub use health::{health_check, readiness_check, version_info};
pub use identity::{handle_did_document, handle_did_endpoint};
pub use import::{handle_import_request, match_import_route};
pub use import_ws::handle_import_progress_ws;
pub use seed::{handle_check_blob, handle_seed_blob, BlobUploadResponse};
pub use status::status_check;
pub use stream::handle_stream_request;
pub use threshold::handle_threshold_request;
