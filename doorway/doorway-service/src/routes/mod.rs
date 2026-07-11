//! HTTP routes for Doorway

pub mod admin;
pub mod admin_cache;
pub mod admin_conductors;
pub mod admin_dev;
pub mod admin_users;
pub mod api;
pub mod apps;
pub mod auth_routes;
pub mod blob;
pub mod bootstrap_coherence;
pub mod chrome;
pub mod coherence;
pub mod collectives;
pub mod dashboard_ws;
// `db` module retired 2026-05-25 as part of Pattern Z anti-pattern cleanup —
// see genesis/docs/superpowers/specs/2026-05-23-doorway-access-tier-patterns.md
// and doorway/CLAUDE.md "No Per-Domain Proxy Files" rule. /db/* now flows
// through the dynamic route registry (StorageProxy → forward_to_storage).
pub mod debug_stream;
pub mod elohim_agent;
pub mod epr;
pub mod federation;
pub mod health;
pub mod identity;
pub mod import;
pub mod import_ws;
pub mod journal;
pub mod metrics;
pub mod pkarr_resolver;
pub mod seed;
pub mod self_healing;
pub mod status;
pub mod storage_proxy;
pub mod stream;
pub mod threshold;
pub mod upstream_health;
pub mod zome_helpers;

pub use admin::{
    handle_admin_capability, handle_admin_dashboard_topology, handle_admin_pipeline,
    handle_admin_render_stats, handle_capabilities, handle_cluster_metrics, handle_custodians,
    handle_node_by_id, handle_nodes, handle_resources, handle_route_registry,
    handle_steward_peers_refresh,
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
pub use apps::{handle_app_capability, handle_app_request};
pub use auth_routes::handle_auth_request;
pub use blob::{
    error_response as blob_error_response, handle_blob_request, handle_blob_request_with_fallback,
    handle_blob_request_with_storage_proxy, BlobContext, BlobError,
};
pub use chrome::handle_chrome_asset;
pub use dashboard_ws::handle_dashboard_ws;
// `handle_db_request` re-export retired with the db module — see comment above.
pub use debug_stream::{handle_debug_stream, DebugEvent, DebugHub};
pub use federation::{
    handle_admin_add_federation_peer, handle_admin_federation_peers,
    handle_admin_refresh_federation_peers, handle_admin_remove_federation_peer,
    handle_doorway_keys, handle_federation_doorways, handle_federation_p2p_peers,
};
pub use health::{health_check, readiness_check, startup_check, version_info};
pub use identity::{handle_did_document, handle_did_endpoint, handle_identity_api_request};
pub use import::{handle_import_request, match_import_route};
pub use import_ws::handle_import_progress_ws;
pub use seed::{handle_check_blob, handle_seed_blob, BlobUploadResponse};
pub use self_healing::handle_self_healing;
pub use status::{status_check, status_page};
pub use storage_proxy::{forward_blob_to_storage, forward_to_storage, ForwardCtx};
pub use stream::handle_stream_request;
pub use threshold::handle_threshold_request;
pub use upstream_health::UpstreamBreakers;

pub use collectives::handle_collectives_request;
pub use elohim_agent::handle_elohim_agent_request;
pub use epr::handle_epr_head_request;
pub use journal::{handle_journal_analyze, handle_journal_suggest};
pub use metrics::handle_metrics;
