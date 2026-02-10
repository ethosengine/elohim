//! Admin API for conductor pool visibility
//!
//! Available on ALL doorway instances (writer and reader replicas).
//! Every instance knows the conductor pool and can answer queries about it.
//!
//! ## Endpoints
//!
//! - `GET /admin/conductors` — list all conductors with capacity and agent counts
//! - `GET /admin/conductors/{id}/agents` — list agents hosted on a conductor
//! - `GET /admin/agents/{agent_pub_key}/conductor` — which conductor hosts an agent

use bson::doc;
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

use crate::conductor::AgentProvisioner;
use crate::db::schemas::{UserDoc, USER_COLLECTION};
use crate::server::AppState;

/// Summary of a conductor in the pool
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConductorSummary {
    pub conductor_id: String,
    pub conductor_url: String,
    pub admin_url: String,
    pub capacity_used: usize,
    pub capacity_max: usize,
    pub capacity_available: usize,
    pub agent_count: usize,
}

/// Response for GET /admin/conductors
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConductorsResponse {
    pub total: usize,
    pub total_agents: usize,
    pub total_capacity: usize,
    pub conductors: Vec<ConductorSummary>,
}

/// Agent entry for conductor listing
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSummary {
    pub agent_pub_key: String,
    pub app_id: String,
    pub assigned_at: String,
}

/// Response for GET /admin/conductors/{id}/agents
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConductorAgentsResponse {
    pub conductor_id: String,
    pub total: usize,
    pub agents: Vec<AgentSummary>,
}

/// Response for GET /admin/agents/{key}/conductor
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConductorResponse {
    pub agent_pub_key: String,
    pub conductor_id: String,
    pub conductor_url: String,
    pub app_id: String,
    pub assigned_at: String,
}

/// Handle GET /admin/conductors
pub async fn handle_list_conductors(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let Some(ref registry) = state.conductor_registry else {
        return json_response(
            StatusCode::OK,
            ConductorsResponse {
                total: 0,
                total_agents: 0,
                total_capacity: 0,
                conductors: vec![],
            },
        );
    };

    let conductors: Vec<ConductorSummary> = registry
        .list_conductors()
        .into_iter()
        .map(|c| {
            let agent_count = registry.list_agents_on_conductor(&c.conductor_id).len();
            ConductorSummary {
                conductor_id: c.conductor_id,
                conductor_url: c.conductor_url,
                admin_url: c.admin_url,
                capacity_used: c.capacity_used,
                capacity_max: c.capacity_max,
                capacity_available: c.capacity_max.saturating_sub(c.capacity_used),
                agent_count,
            }
        })
        .collect();

    let total_agents = registry.agent_count();
    let total_capacity: usize = conductors.iter().map(|c| c.capacity_max).sum();

    json_response(
        StatusCode::OK,
        ConductorsResponse {
            total: conductors.len(),
            total_agents,
            total_capacity,
            conductors,
        },
    )
}

/// Handle GET /admin/conductors/{id}/agents
pub async fn handle_conductor_agents(
    state: Arc<AppState>,
    conductor_id: &str,
) -> Response<Full<Bytes>> {
    let Some(ref registry) = state.conductor_registry else {
        return json_response(
            StatusCode::NOT_FOUND,
            serde_json::json!({"error": "Conductor registry not available"}),
        );
    };

    let agents: Vec<AgentSummary> = registry
        .list_agents_on_conductor(conductor_id)
        .into_iter()
        .map(|(key, entry)| AgentSummary {
            agent_pub_key: key,
            app_id: entry.app_id,
            assigned_at: entry.assigned_at.to_rfc3339(),
        })
        .collect();

    json_response(
        StatusCode::OK,
        ConductorAgentsResponse {
            conductor_id: conductor_id.to_string(),
            total: agents.len(),
            agents,
        },
    )
}

/// Handle GET /admin/agents/{agent_pub_key}/conductor
pub async fn handle_agent_conductor(
    state: Arc<AppState>,
    agent_pub_key: &str,
) -> Response<Full<Bytes>> {
    let Some(ref registry) = state.conductor_registry else {
        return json_response(
            StatusCode::NOT_FOUND,
            serde_json::json!({"error": "Conductor registry not available"}),
        );
    };

    match registry.get_conductor_for_agent(agent_pub_key) {
        Some(entry) => json_response(
            StatusCode::OK,
            AgentConductorResponse {
                agent_pub_key: agent_pub_key.to_string(),
                conductor_id: entry.conductor_id,
                conductor_url: entry.conductor_url,
                app_id: entry.app_id,
                assigned_at: entry.assigned_at.to_rfc3339(),
            },
        ),
        None => json_response(
            StatusCode::NOT_FOUND,
            serde_json::json!({
                "error": "Agent not found in conductor registry",
                "agent_pub_key": agent_pub_key
            }),
        ),
    }
}

/// Request body for POST /admin/conductors/assign
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignAgentRequest {
    pub agent_pub_key: String,
    pub conductor_id: String,
    #[serde(default = "default_app_id")]
    pub app_id: String,
}

fn default_app_id() -> String {
    "elohim".to_string()
}

/// Handle POST /admin/conductors/assign — manual agent→conductor assignment
pub async fn handle_assign_agent(
    req: Request<Incoming>,
    state: Arc<AppState>,
) -> Response<Full<Bytes>> {
    let Some(ref registry) = state.conductor_registry else {
        return json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({"error": "Conductor registry not available"}),
        );
    };

    // Read and parse request body
    let body = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                serde_json::json!({"error": format!("Failed to read request body: {}", e)}),
            );
        }
    };

    let request: AssignAgentRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                serde_json::json!({"error": format!("Invalid JSON: {}", e)}),
            );
        }
    };

    // Verify conductor exists
    let conductor_exists = registry
        .list_conductors()
        .iter()
        .any(|c| c.conductor_id == request.conductor_id);
    if !conductor_exists {
        return json_response(
            StatusCode::NOT_FOUND,
            serde_json::json!({
                "error": "Conductor not found",
                "conductor_id": request.conductor_id,
            }),
        );
    }

    // Register the assignment
    match registry
        .register_agent(&request.agent_pub_key, &request.conductor_id, &request.app_id)
        .await
    {
        Ok(()) => {
            // Return the created entry
            match registry.get_conductor_for_agent(&request.agent_pub_key) {
                Some(entry) => json_response(
                    StatusCode::OK,
                    AgentConductorResponse {
                        agent_pub_key: request.agent_pub_key,
                        conductor_id: entry.conductor_id,
                        conductor_url: entry.conductor_url,
                        app_id: entry.app_id,
                        assigned_at: entry.assigned_at.to_rfc3339(),
                    },
                ),
                None => json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    serde_json::json!({"error": "Assignment succeeded but entry not found"}),
                ),
            }
        }
        Err(e) => {
            warn!(
                agent = %request.agent_pub_key,
                conductor = %request.conductor_id,
                error = %e,
                "Failed to assign agent to conductor"
            );
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("Failed to assign agent: {}", e)}),
            )
        }
    }
}

// =============================================================================
// Hosted Users — provisioning endpoints
// =============================================================================

/// Request body for POST /admin/hosted-users
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionUserRequest {
    pub identifier: String,
}

/// Summary of a hosted user with conductor assignment
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedUserSummary {
    pub identifier: String,
    pub agent_pub_key: String,
    pub conductor_id: Option<String>,
    pub human_id: String,
    pub is_active: bool,
}

/// Response for GET /admin/hosted-users
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedUsersResponse {
    pub total: usize,
    pub users: Vec<HostedUserSummary>,
}

/// Response for POST /admin/hosted-users
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionResponse {
    pub agent_pub_key: String,
    pub conductor_id: String,
    pub conductor_url: String,
    pub installed_app_id: String,
}

/// Handle POST /admin/hosted-users — manual agent provisioning
pub async fn handle_provision_user(
    req: Request<Incoming>,
    state: Arc<AppState>,
) -> Response<Full<Bytes>> {
    let Some(ref registry) = state.conductor_registry else {
        return json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({"error": "Conductor registry not available"}),
        );
    };

    // Parse request body
    let body = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                serde_json::json!({"error": format!("Failed to read body: {}", e)}),
            );
        }
    };

    let request: ProvisionUserRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                serde_json::json!({"error": format!("Invalid JSON: {}", e)}),
            );
        }
    };

    if request.identifier.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            serde_json::json!({"error": "identifier is required"}),
        );
    }

    let provisioner = AgentProvisioner::new(Arc::clone(registry))
        .with_app_id(state.args.installed_app_id.clone());

    match provisioner.provision_agent(&request.identifier).await {
        Ok(p) => {
            info!(
                conductor = %p.conductor_id,
                agent = %p.agent_pub_key,
                identifier = %request.identifier,
                "Manual agent provisioning succeeded"
            );
            json_response(
                StatusCode::CREATED,
                ProvisionResponse {
                    agent_pub_key: p.agent_pub_key,
                    conductor_id: p.conductor_id,
                    conductor_url: p.conductor_url,
                    installed_app_id: p.installed_app_id,
                },
            )
        }
        Err(e) => {
            warn!(
                identifier = %request.identifier,
                error = %e,
                "Manual agent provisioning failed"
            );
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("Provisioning failed: {}", e)}),
            )
        }
    }
}

/// Handle GET /admin/hosted-users — list users with conductor assignments
pub async fn handle_list_hosted_users(state: Arc<AppState>) -> Response<Full<Bytes>> {
    // Try to list from MongoDB
    let Some(ref mongo) = state.mongo else {
        return json_response(
            StatusCode::OK,
            HostedUsersResponse {
                total: 0,
                users: vec![],
            },
        );
    };

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("Database error: {}", e)}),
            );
        }
    };

    // Find users with conductor_id set (hosted users)
    let filter = doc! { "conductor_id": { "$ne": null } };
    let users: Vec<HostedUserSummary> = match collection.find_many(filter).await {
        Ok(docs) => docs
            .into_iter()
            .map(|u| HostedUserSummary {
                identifier: u.identifier,
                agent_pub_key: u.agent_pub_key,
                conductor_id: u.conductor_id,
                human_id: u.human_id,
                is_active: u.is_active,
            })
            .collect(),
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("Failed to read users: {}", e)}),
            );
        }
    };

    let total = users.len();
    json_response(StatusCode::OK, HostedUsersResponse { total, users })
}

/// Handle DELETE /admin/hosted-users/{agent_key} — deprovision an agent
pub async fn handle_deprovision_user(
    state: Arc<AppState>,
    agent_key: &str,
) -> Response<Full<Bytes>> {
    let Some(ref registry) = state.conductor_registry else {
        return json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({"error": "Conductor registry not available"}),
        );
    };

    let provisioner = AgentProvisioner::new(Arc::clone(registry))
        .with_app_id(state.args.installed_app_id.clone());

    match provisioner.deprovision_agent(agent_key).await {
        Ok(()) => {
            info!(agent = %agent_key, "Agent deprovisioned via admin API");
            json_response(
                StatusCode::OK,
                serde_json::json!({
                    "status": "deprovisioned",
                    "agentPubKey": agent_key,
                }),
            )
        }
        Err(e) => {
            warn!(agent = %agent_key, error = %e, "Deprovisioning failed");
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("Deprovisioning failed: {}", e)}),
            )
        }
    }
}

// =============================================================================
// Graduation Endpoints — conductor retirement for steward users
// =============================================================================

/// Summary of a user pending graduation
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraduationPendingUser {
    pub identifier: String,
    pub human_id: String,
    pub agent_pub_key: String,
    pub conductor_id: Option<String>,
    pub key_exported_at: Option<String>,
}

/// Response for GET /admin/graduation/pending
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraduationPendingResponse {
    pub total: usize,
    pub users: Vec<GraduationPendingUser>,
}

/// Summary of a graduated (steward) user
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraduationCompletedUser {
    pub identifier: String,
    pub human_id: String,
    pub agent_pub_key: String,
    pub stewardship_at: Option<String>,
    /// True if conductor_id is still set (orphaned — deprovisioning failed earlier)
    pub needs_cleanup: bool,
    pub conductor_id: Option<String>,
}

/// Response for GET /admin/graduation/completed
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraduationCompletedResponse {
    pub total: usize,
    pub freed_capacity: usize,
    pub orphaned_count: usize,
    pub users: Vec<GraduationCompletedUser>,
}

/// Handle GET /admin/graduation/pending — users with exported keys not yet stewards
pub async fn handle_graduation_pending(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let Some(ref mongo) = state.mongo else {
        return json_response(
            StatusCode::OK,
            GraduationPendingResponse {
                total: 0,
                users: vec![],
            },
        );
    };

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("Database error: {}", e)}),
            );
        }
    };

    // Users with exported key but NOT yet steward (either field name for backward compat)
    let filter = doc! {
        "custodial_key.exported": true,
        "$and": [
            { "$or": [
                { "is_steward": { "$ne": true } },
                { "is_steward": { "$exists": false } },
            ]},
            { "$or": [
                { "is_sovereign": { "$ne": true } },
                { "is_sovereign": { "$exists": false } },
            ]},
        ]
    };

    let users: Vec<GraduationPendingUser> = match collection.find_many(filter).await {
        Ok(docs) => docs
            .into_iter()
            .map(|u| GraduationPendingUser {
                identifier: u.identifier,
                human_id: u.human_id,
                agent_pub_key: u.agent_pub_key,
                conductor_id: u.conductor_id,
                key_exported_at: u.custodial_key.as_ref().and_then(|k| {
                    k.exported_at.map(|dt| {
                        chrono::DateTime::from_timestamp(
                            dt.timestamp_millis() / 1000,
                            ((dt.timestamp_millis() % 1000) * 1_000_000) as u32,
                        )
                        .unwrap_or_default()
                        .to_rfc3339()
                    })
                }),
            })
            .collect(),
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("Failed to query users: {}", e)}),
            );
        }
    };

    let total = users.len();
    json_response(StatusCode::OK, GraduationPendingResponse { total, users })
}

/// Handle GET /admin/graduation/completed — steward users with freed capacity
pub async fn handle_graduation_completed(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let Some(ref mongo) = state.mongo else {
        return json_response(
            StatusCode::OK,
            GraduationCompletedResponse {
                total: 0,
                freed_capacity: 0,
                orphaned_count: 0,
                users: vec![],
            },
        );
    };

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("Database error: {}", e)}),
            );
        }
    };

    // Users who are stewards (either field name for backward compat)
    let filter = doc! {
        "$or": [
            { "is_steward": true },
            { "is_sovereign": true },
        ]
    };

    let users: Vec<GraduationCompletedUser> = match collection.find_many(filter).await {
        Ok(docs) => docs
            .into_iter()
            .map(|u| {
                let needs_cleanup = u.conductor_id.is_some();
                GraduationCompletedUser {
                    identifier: u.identifier,
                    human_id: u.human_id,
                    agent_pub_key: u.agent_pub_key,
                    stewardship_at: u.stewardship_at.map(|dt| {
                        chrono::DateTime::from_timestamp(
                            dt.timestamp_millis() / 1000,
                            ((dt.timestamp_millis() % 1000) * 1_000_000) as u32,
                        )
                        .unwrap_or_default()
                        .to_rfc3339()
                    }),
                    needs_cleanup,
                    conductor_id: u.conductor_id,
                }
            })
            .collect(),
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("Failed to query users: {}", e)}),
            );
        }
    };

    let total = users.len();
    let orphaned_count = users.iter().filter(|u| u.needs_cleanup).count();
    let freed_capacity = total - orphaned_count;

    json_response(
        StatusCode::OK,
        GraduationCompletedResponse {
            total,
            freed_capacity,
            orphaned_count,
            users,
        },
    )
}

/// Handle POST /admin/graduation/force/{agent_key} — force-graduate a user
pub async fn handle_force_graduation(
    state: Arc<AppState>,
    agent_key: &str,
) -> Response<Full<Bytes>> {
    let Some(ref mongo) = state.mongo else {
        return json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({"error": "Database not available"}),
        );
    };

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("Database error: {}", e)}),
            );
        }
    };

    // Find user by agent_pub_key
    let user = match collection
        .find_one(doc! { "agent_pub_key": agent_key })
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return json_response(
                StatusCode::NOT_FOUND,
                serde_json::json!({
                    "error": "User not found",
                    "agentPubKey": agent_key,
                }),
            );
        }
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("Database error: {}", e)}),
            );
        }
    };

    // Mark as steward + clear conductor_id
    let stewardship_time = bson::DateTime::now();
    if let Err(e) = collection
        .update_one(
            doc! { "agent_pub_key": agent_key },
            doc! {
                "$set": {
                    "is_steward": true,
                    "stewardship_at": stewardship_time,
                    "conductor_id": bson::Bson::Null,
                }
            },
        )
        .await
    {
        return json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({"error": format!("Failed to update user: {}", e)}),
        );
    }

    // Deprovision conductor cell (best effort)
    if let Some(ref registry) = state.conductor_registry {
        let provisioner = AgentProvisioner::new(Arc::clone(registry))
            .with_app_id(state.args.installed_app_id.clone());
        match provisioner.deprovision_agent(agent_key).await {
            Ok(()) => {
                info!(
                    agent = %agent_key,
                    identifier = %user.identifier,
                    "Force-graduated: conductor cell deprovisioned"
                );
            }
            Err(e) => {
                warn!(
                    agent = %agent_key,
                    error = %e,
                    "Force-graduated but deprovisioning failed"
                );
            }
        }
    }

    info!(
        agent = %agent_key,
        identifier = %user.identifier,
        "Admin force-graduated user to stewardship"
    );

    json_response(
        StatusCode::OK,
        serde_json::json!({
            "status": "graduated",
            "identifier": user.identifier,
            "humanId": user.human_id,
            "agentPubKey": agent_key,
            "stewardshipAt": chrono::Utc::now().to_rfc3339(),
        }),
    )
}

fn json_response<T: Serialize>(status: StatusCode, body: T) -> Response<Full<Bytes>> {
    match serde_json::to_string_pretty(&body) {
        Ok(json) => Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .body(Full::new(Bytes::from(json)))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from("Failed to build response")))
                    .unwrap()
            }),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from("Failed to serialize response")))
            .unwrap(),
    }
}
