//! Boot-time node-shape self-registration (Task C7).
//!
//! When elohim-node starts with `DEVICE_ARCHETYPE`, `HOUSEHOLD_ID`,
//! `NODE_ROLE`, and `REGION` env vars set (see main.rs), this module
//! looks up the archetype's committed resources from `genesis/data/devices/devices.json`,
//! builds a signed `NodeShapeView`, and upserts the `stewarded_nodes`
//! projection row so `/api/v1/households/{id}/devices` can return this
//! node immediately.
//!
//! The DHT commit via the `register_node_shape` coordinator zome fn is a
//! separate code path authored by the node's own agent key; post-commit
//! signal fills `dht_anchor_hash` afterward. Source of truth stays the
//! DHT NodeRegistration entry.

use crate::db::DbPool;
use crate::views::{CommittedResources, NodeShapeView};
use serde::Deserialize;
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceArchetype {
    id: String,
    #[serde(default)]
    capability_level: i32,
    #[serde(default)]
    memory_gb: i32,
    #[serde(default)]
    storage_gb: f64,
    #[serde(default)]
    cpu_cores: i32,
    #[serde(default)]
    can_steward: bool,
    #[serde(default)]
    can_infer: bool,
    #[serde(default)]
    can_doorway: bool,
    #[serde(default)]
    bandwidth_mbps: Option<i32>,
}

/// Minimal archetype fallback when devices.json can't be loaded. Yields a
/// conservative "modest device" shape so the node still registers with a
/// recognizable capability level instead of dropping the publish entirely.
fn fallback_archetype(id: &str) -> DeviceArchetype {
    DeviceArchetype {
        id: id.to_string(),
        capability_level: 2,
        memory_gb: 8,
        storage_gb: 64.0,
        cpu_cores: 4,
        can_steward: false,
        can_infer: false,
        can_doorway: false,
        bandwidth_mbps: Some(100),
    }
}

fn load_archetype_from_fixture(archetype_id: &str) -> Option<DeviceArchetype> {
    // Candidate paths — runtime may launch from the repo root, from
    // elohim-storage/, or from a k8s working dir. Try each.
    let candidates = [
        "genesis/data/devices/devices.json",
        "../genesis/data/devices/devices.json",
        "../../genesis/data/devices/devices.json",
        "/etc/elohim/devices.json",
    ];

    for p in candidates.iter() {
        if Path::new(p).exists() {
            if let Ok(raw) = std::fs::read_to_string(p) {
                // Try parsing as an array first, then as { "devices": [...] }
                if let Ok(all) = serde_json::from_str::<Vec<DeviceArchetype>>(&raw) {
                    if let Some(m) = all.into_iter().find(|a| a.id == archetype_id) {
                        return Some(m);
                    }
                }
                #[derive(Deserialize)]
                struct Wrapper {
                    devices: Vec<DeviceArchetype>,
                }
                if let Ok(w) = serde_json::from_str::<Wrapper>(&raw) {
                    if let Some(m) = w.devices.into_iter().find(|a| a.id == archetype_id) {
                        return Some(m);
                    }
                }
            }
        }
    }

    None
}

/// Build a NodeShapeView from archetype + environment context.
fn build_shape(
    node_id: String,
    hostname: String,
    archetype: &DeviceArchetype,
    household_id: String,
    role: String,
    region: Option<String>,
    agent_pubkey_hint: &str,
) -> NodeShapeView {
    let signed_at = chrono::Utc::now().to_rfc3339();
    let signature = sign_shape(&node_id, agent_pubkey_hint, &archetype.id, &signed_at);

    NodeShapeView {
        node_id,
        hostname,
        device_archetype_id: archetype.id.clone(),
        household_id,
        role,
        capability_level: archetype.capability_level,
        committed: CommittedResources {
            cpu_cores: archetype.cpu_cores,
            memory_gb: archetype.memory_gb,
            storage_tb: archetype.storage_gb / 1024.0,
            bandwidth_mbps: archetype.bandwidth_mbps,
            max_custody_gb: Some(archetype.storage_gb * 0.5),
            can_steward: archetype.can_steward,
            can_infer: archetype.can_infer,
            can_doorway: archetype.can_doorway,
        },
        steward_tier: None,
        custodian_opt_in: archetype.can_steward,
        region,
        signature,
        signed_at,
        dht_anchor_hash: None,
    }
}

fn sign_shape(node_id: &str, agent_pubkey: &str, archetype_id: &str, signed_at: &str) -> String {
    use sha2::{Digest, Sha256};
    let payload = format!(
        "{}|{}|{}|{}",
        node_id, agent_pubkey, archetype_id, signed_at
    );
    let h = Sha256::digest(payload.as_bytes());
    hex::encode(h)
}

/// Run the boot registration if all four env-var-derived config fields are
/// present. Safe to call unconditionally; when any field is missing this
/// function returns immediately without error.
pub fn register_at_boot(
    pool: &DbPool,
    config: &crate::config::Config,
    agent_pubkey_hint: &str,
) -> Result<(), crate::error::StorageError> {
    let (archetype_id, household_id, role) = match (
        config.device_archetype.as_ref(),
        config.household_id.as_ref(),
        config.node_role.as_ref(),
    ) {
        (Some(a), Some(h), Some(r)) => (a.clone(), h.clone(), r.clone()),
        _ => {
            info!("node-shape self-registration skipped (env vars not all set)");
            return Ok(());
        }
    };

    let archetype = load_archetype_from_fixture(&archetype_id).unwrap_or_else(|| {
        warn!(
            archetype = %archetype_id,
            "device archetype not found in devices.json fixture; using conservative fallback"
        );
        fallback_archetype(&archetype_id)
    });

    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| archetype_id.clone());

    let node_id = hostname.clone();
    let shape = build_shape(
        node_id.clone(),
        hostname.clone(),
        &archetype,
        household_id.clone(),
        role.clone(),
        config.region.clone(),
        agent_pubkey_hint,
    );

    let mut conn = pool.get().map_err(|e| {
        crate::error::StorageError::Internal(format!("boot_registration pool: {e}"))
    })?;
    crate::db::stewarded_nodes::upsert_from_shape(&mut conn, &shape)?;
    info!(
        node_id = %node_id,
        archetype = %archetype_id,
        household = %household_id,
        role = %role,
        "node shape self-registered at boot"
    );
    Ok(())
}
