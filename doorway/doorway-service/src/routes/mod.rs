//! HTTP routes for Doorway

pub mod account;
pub mod admin;
pub mod admin_conductors;
pub mod admin_users;
pub mod api;
pub mod apps;
pub mod attestations;
pub mod auth_routes;
pub mod blob;
pub mod collectives;
pub mod compute;
pub mod contributors;
pub mod custodians;
pub mod dashboard_ws;
pub mod db;
pub mod debug_stream;
pub mod economic_events;
pub mod elohim_agent;
pub mod epr;
pub mod exchange;
pub mod federation;
pub mod flow_planning;
pub mod governance;
pub mod health;
pub mod identity;
pub mod import;
pub mod import_ws;
pub mod presence;
pub mod seed;
pub mod status;
pub mod steward;
pub mod stewarded_resources;
pub mod stewardship;
pub mod stream;
pub mod threshold;
pub mod zome_helpers;

pub use admin::{
    handle_admin_pipeline, handle_capabilities, handle_cluster_metrics, handle_custodians,
    handle_node_by_id, handle_nodes, handle_resources,
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
pub use federation::{
    handle_admin_add_federation_peer, handle_admin_federation_peers,
    handle_admin_refresh_federation_peers, handle_admin_remove_federation_peer,
    handle_doorway_keys, handle_federation_doorways, handle_federation_p2p_peers,
};
pub use health::{health_check, readiness_check, version_info};
pub use identity::{handle_did_document, handle_did_endpoint, handle_identity_api_request};
pub use import::{handle_import_request, match_import_route};
pub use import_ws::handle_import_progress_ws;
pub use seed::{handle_check_blob, handle_seed_blob, BlobUploadResponse};
pub use status::status_check;
pub use stream::handle_stream_request;
pub use threshold::handle_threshold_request;

pub use account::handle_account_request;
pub use attestations::handle_attestations_request;
pub use collectives::handle_collectives_request;
pub use compute::handle_compute_request;
pub use contributors::handle_contributors_request;
pub use custodians::handle_custodians_api_request;
pub use economic_events::handle_economic_events_request;
pub use elohim_agent::handle_elohim_agent_request;
pub use epr::handle_epr_head_request;
pub use exchange::handle_exchange_request;
pub use flow_planning::handle_flow_planning_request;
pub use governance::handle_governance_request;
pub use presence::handle_presence_request;
pub use steward::handle_steward_api_request;
pub use stewarded_resources::handle_stewarded_resources_request;
pub use stewardship::handle_stewardship_request;
