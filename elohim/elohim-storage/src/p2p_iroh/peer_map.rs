//! Phase 12 — peer transport manifest (graduated permanent schema).
//!
//! Replaces the Phase 10 cross_stack_peer_map bridge. Backed by the
//! `peer_transport_manifest` table (Category C operational projection).
//!
//! Identity is keyed by `agent_cid`. Either `libp2p_peer_id` or
//! `iroh_node_id` may be NULL (CHECK-enforced not-both-NULL). Per-
//! transport profiles carry listen addrs / relay URLs and supported
//! planes (kebab-case strings matching wire conventions).
//!
//! Selection at call site is via [`select_transport`] (see bottom of
//! this module).
//!
//! # Back-compat shims
//!
//! No production caller of the Phase 10 shims (`record_libp2p`,
//! `record_iroh`, `iroh_for_libp2p`, `libp2p_for_iroh`) exists at
//! Phase 12 land time — verified by crate-wide grep (see plan caller
//! inventory). They remain to keep the Phase 10 surface stable for any
//! out-of-tree consumer (steward) and for the README's documented
//! examples.
//!
//! Spec: genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md
//! lines 440-505.

use diesel::prelude::*;
use serde_json::Value as JsonValue;

use crate::db::diesel_schema::peer_transport_manifest;
use crate::error::StorageError;

// ────────────────────────────────────────────────────────────────
// In-memory shapes
// ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerTransportManifest {
    pub agent_cid: String,
    pub libp2p: Option<Libp2pTransportProfile>,
    pub iroh: Option<IrohTransportProfile>,
    pub discovery: Vec<String>,
    pub capability_level: u8,
    pub last_observed: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Libp2pTransportProfile {
    pub peer_id: String,
    pub addrs: Vec<String>,
    pub supports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrohTransportProfile {
    pub node_id: String,
    pub relays: Vec<String>,
    pub supports: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Plane {
    Blob,
    Gossip,
    Sync,
    Epr,
    EprAtom,
    Shard,
    ViewFed,
    IdentityHandshake,
    Trust,
}

impl Plane {
    pub fn as_str(self) -> &'static str {
        match self {
            Plane::Blob => "blob",
            Plane::Gossip => "gossip",
            Plane::Sync => "sync",
            Plane::Epr => "epr",
            Plane::EprAtom => "epr-atom",
            Plane::Shard => "shard",
            Plane::ViewFed => "view-fed",
            Plane::IdentityHandshake => "identity-handshake",
            Plane::Trust => "trust",
        }
    }

    pub fn parse(s: &str) -> Option<Plane> {
        match s {
            "blob" => Some(Plane::Blob),
            "gossip" => Some(Plane::Gossip),
            "sync" => Some(Plane::Sync),
            "epr" => Some(Plane::Epr),
            "epr-atom" => Some(Plane::EprAtom),
            "shard" => Some(Plane::Shard),
            "view-fed" => Some(Plane::ViewFed),
            "identity-handshake" => Some(Plane::IdentityHandshake),
            "trust" => Some(Plane::Trust),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportChoice {
    Iroh,
    Libp2p,
    Track3Bridge { hub_agent_cid: String },
    NoSharedTransport,
}

// ────────────────────────────────────────────────────────────────
// Diesel row + (de)serialization helpers
// ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = peer_transport_manifest)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct ManifestRow {
    agent_cid: String,
    libp2p_peer_id: Option<String>,
    iroh_node_id: Option<String>,
    libp2p_addrs_json: Option<String>,
    iroh_relays_json: Option<String>,
    libp2p_supports_json: Option<String>,
    iroh_supports_json: Option<String>,
    discovery_methods_json: String,
    capability_level: i32,
    last_observed: i64,
}

fn parse_string_array(json: &str, ctx: &'static str) -> Result<Vec<String>, StorageError> {
    let v: JsonValue = serde_json::from_str(json)
        .map_err(|e| StorageError::Database(format!("{ctx}: invalid json: {e}")))?;
    let arr = v
        .as_array()
        .ok_or_else(|| StorageError::Database(format!("{ctx}: expected array")))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let s = item
            .as_str()
            .ok_or_else(|| StorageError::Database(format!("{ctx}: expected string element")))?;
        out.push(s.to_string());
    }
    Ok(out)
}

fn serialize_string_array(items: &[String]) -> String {
    JsonValue::Array(items.iter().map(|s| JsonValue::String(s.clone())).collect()).to_string()
}

fn supports_to_strings(supports: &[Plane]) -> Vec<String> {
    supports.iter().map(|p| p.as_str().to_string()).collect()
}

fn row_to_manifest(row: ManifestRow) -> Result<PeerTransportManifest, StorageError> {
    let libp2p = match row.libp2p_peer_id {
        Some(peer_id) => Some(Libp2pTransportProfile {
            peer_id,
            addrs: row
                .libp2p_addrs_json
                .as_deref()
                .map(|s| parse_string_array(s, "libp2p_addrs_json"))
                .transpose()?
                .unwrap_or_default(),
            supports: row
                .libp2p_supports_json
                .as_deref()
                .map(|s| parse_string_array(s, "libp2p_supports_json"))
                .transpose()?
                .unwrap_or_default(),
        }),
        None => None,
    };
    let iroh = match row.iroh_node_id {
        Some(node_id) => Some(IrohTransportProfile {
            node_id,
            relays: row
                .iroh_relays_json
                .as_deref()
                .map(|s| parse_string_array(s, "iroh_relays_json"))
                .transpose()?
                .unwrap_or_default(),
            supports: row
                .iroh_supports_json
                .as_deref()
                .map(|s| parse_string_array(s, "iroh_supports_json"))
                .transpose()?
                .unwrap_or_default(),
        }),
        None => None,
    };
    let discovery = parse_string_array(&row.discovery_methods_json, "discovery_methods_json")?;
    Ok(PeerTransportManifest {
        agent_cid: row.agent_cid,
        libp2p,
        iroh,
        discovery,
        capability_level: row.capability_level.clamp(0, 5) as u8,
        last_observed: row.last_observed,
    })
}

// ────────────────────────────────────────────────────────────────
// Observation / capability / discovery writers
// ────────────────────────────────────────────────────────────────

pub fn record_libp2p_observation(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    peer_id: &str,
    addrs: &[String],
    supports: &[Plane],
    observed_at: i64,
) -> Result<(), StorageError> {
    use peer_transport_manifest as t;
    let addrs_json = serialize_string_array(addrs);
    let supports_json = serialize_string_array(&supports_to_strings(supports));
    conn.transaction(|conn| {
        let existing: Option<ManifestRow> = t::table
            .filter(t::agent_cid.eq(agent_cid))
            .first(conn)
            .optional()?;
        match existing {
            Some(_) => {
                diesel::update(t::table.filter(t::agent_cid.eq(agent_cid)))
                    .set((
                        t::libp2p_peer_id.eq(peer_id),
                        t::libp2p_addrs_json.eq(&addrs_json),
                        t::libp2p_supports_json.eq(&supports_json),
                        t::last_observed.eq(observed_at),
                    ))
                    .execute(conn)?;
            }
            None => {
                diesel::insert_into(t::table)
                    .values((
                        t::agent_cid.eq(agent_cid),
                        t::libp2p_peer_id.eq(Some(peer_id)),
                        t::iroh_node_id.eq::<Option<&str>>(None),
                        t::libp2p_addrs_json.eq(Some(&addrs_json)),
                        t::iroh_relays_json.eq::<Option<&str>>(None),
                        t::libp2p_supports_json.eq(Some(&supports_json)),
                        t::iroh_supports_json.eq::<Option<&str>>(None),
                        t::discovery_methods_json.eq("[\"kademlia\"]"),
                        t::capability_level.eq(5),
                        t::last_observed.eq(observed_at),
                    ))
                    .execute(conn)?;
            }
        }
        Ok::<_, diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("record_libp2p_observation: {e}")))
}

pub fn record_iroh_observation(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    node_id: &str,
    relays: &[String],
    supports: &[Plane],
    observed_at: i64,
) -> Result<(), StorageError> {
    use peer_transport_manifest as t;
    let relays_json = serialize_string_array(relays);
    let supports_json = serialize_string_array(&supports_to_strings(supports));
    conn.transaction(|conn| {
        let existing: Option<ManifestRow> = t::table
            .filter(t::agent_cid.eq(agent_cid))
            .first(conn)
            .optional()?;
        match existing {
            Some(_) => {
                diesel::update(t::table.filter(t::agent_cid.eq(agent_cid)))
                    .set((
                        t::iroh_node_id.eq(node_id),
                        t::iroh_relays_json.eq(&relays_json),
                        t::iroh_supports_json.eq(&supports_json),
                        t::last_observed.eq(observed_at),
                    ))
                    .execute(conn)?;
            }
            None => {
                diesel::insert_into(t::table)
                    .values((
                        t::agent_cid.eq(agent_cid),
                        t::libp2p_peer_id.eq::<Option<&str>>(None),
                        t::iroh_node_id.eq(Some(node_id)),
                        t::libp2p_addrs_json.eq::<Option<&str>>(None),
                        t::iroh_relays_json.eq(Some(&relays_json)),
                        t::libp2p_supports_json.eq::<Option<&str>>(None),
                        t::iroh_supports_json.eq(Some(&supports_json)),
                        t::discovery_methods_json.eq("[\"pkarr\",\"kademlia\"]"),
                        t::capability_level.eq(5),
                        t::last_observed.eq(observed_at),
                    ))
                    .execute(conn)?;
            }
        }
        Ok::<_, diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("record_iroh_observation: {e}")))
}

pub fn record_capability(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    capability_level: u8,
) -> Result<(), StorageError> {
    use peer_transport_manifest as t;
    let level = capability_level.min(5) as i32;
    let updated = diesel::update(t::table.filter(t::agent_cid.eq(agent_cid)))
        .set(t::capability_level.eq(level))
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("record_capability: {e}")))?;
    if updated == 0 {
        return Err(StorageError::Database(format!(
            "record_capability: no manifest row for agent_cid {agent_cid}"
        )));
    }
    Ok(())
}

pub fn record_discovery(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    methods: &[&str],
) -> Result<(), StorageError> {
    use peer_transport_manifest as t;
    let json = serialize_string_array(
        &methods.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(),
    );
    let updated = diesel::update(t::table.filter(t::agent_cid.eq(agent_cid)))
        .set(t::discovery_methods_json.eq(json))
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("record_discovery: {e}")))?;
    if updated == 0 {
        return Err(StorageError::Database(format!(
            "record_discovery: no manifest row for agent_cid {agent_cid}"
        )));
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────
// Lookups
// ────────────────────────────────────────────────────────────────

pub fn lookup_by_agent_cid(
    conn: &mut SqliteConnection,
    agent_cid: &str,
) -> Result<Option<PeerTransportManifest>, StorageError> {
    use peer_transport_manifest as t;
    let row: Option<ManifestRow> = t::table
        .filter(t::agent_cid.eq(agent_cid))
        .select(ManifestRow::as_select())
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("lookup_by_agent_cid: {e}")))?;
    row.map(row_to_manifest).transpose()
}

pub fn lookup_by_libp2p_peer_id(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<Option<PeerTransportManifest>, StorageError> {
    use peer_transport_manifest as t;
    let row: Option<ManifestRow> = t::table
        .filter(t::libp2p_peer_id.eq(peer_id))
        .select(ManifestRow::as_select())
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("lookup_by_libp2p_peer_id: {e}")))?;
    row.map(row_to_manifest).transpose()
}

pub fn lookup_by_iroh_node_id(
    conn: &mut SqliteConnection,
    node_id: &str,
) -> Result<Option<PeerTransportManifest>, StorageError> {
    use peer_transport_manifest as t;
    let row: Option<ManifestRow> = t::table
        .filter(t::iroh_node_id.eq(node_id))
        .select(ManifestRow::as_select())
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("lookup_by_iroh_node_id: {e}")))?;
    row.map(row_to_manifest).transpose()
}

// ────────────────────────────────────────────────────────────────
// Helpers for adapters
// ────────────────────────────────────────────────────────────────

/// Returns all libp2p PeerIds in the manifest. Used by view-fed
/// adapter to populate connected_peers.
pub fn list_libp2p_peer_ids(
    conn: &mut SqliteConnection,
) -> Result<Vec<String>, StorageError> {
    use peer_transport_manifest as t;
    t::table
        .filter(t::libp2p_peer_id.is_not_null())
        .select(t::libp2p_peer_id)
        .load::<Option<String>>(conn)
        .map(|rows| rows.into_iter().flatten().collect())
        .map_err(|e| StorageError::Database(format!("list_libp2p_peer_ids: {e}")))
}

/// Returns (libp2p PeerId string, agent_cid) pairs for all manifest
/// rows with a libp2p_peer_id. Used by trust-cache hydration.
pub fn list_libp2p_to_agent(
    conn: &mut SqliteConnection,
) -> Result<Vec<(String, String)>, StorageError> {
    use peer_transport_manifest as t;
    t::table
        .filter(t::libp2p_peer_id.is_not_null())
        .select((t::libp2p_peer_id, t::agent_cid))
        .load::<(Option<String>, String)>(conn)
        .map(|rows| rows.into_iter().filter_map(|(pid, cid)| pid.map(|p| (p, cid))).collect())
        .map_err(|e| StorageError::Database(format!("list_libp2p_to_agent: {e}")))
}

// ────────────────────────────────────────────────────────────────
// Selection algorithm (Phase 12 spec lines 480-490)
// ────────────────────────────────────────────────────────────────

fn libp2p_supports_plane(profile: &Option<Libp2pTransportProfile>, plane: Plane) -> bool {
    profile
        .as_ref()
        .map(|p| p.supports.iter().any(|s| s == plane.as_str()))
        .unwrap_or(false)
}

fn iroh_supports_plane(profile: &Option<IrohTransportProfile>, plane: Plane) -> bool {
    profile
        .as_ref()
        .map(|p| p.supports.iter().any(|s| s == plane.as_str()))
        .unwrap_or(false)
}

/// Select the transport for `plane` between `self_manifest` and
/// `peer_manifest`. See spec lines 480-490 for the algorithm.
///
/// Returns:
/// - `Iroh` if both peers list the plane in their iroh profile.
/// - `Libp2p` if both peers list the plane in their libp2p profile
///   (and the iroh path was not eligible).
/// - `Track3Bridge { hub_agent_cid }` when no transport is shared
///   AND the peer is consumer-grade (capability_level <= 2) AND
///   self is hub-capable (capability_level >= 4) — the hub carries
///   the request via the Phase 11 doorway HTTP/WS bridge.
/// - `NoSharedTransport` otherwise.
///
/// `self_manifest` is the local node's own manifest entry (the
/// caller is expected to look it up before calling, e.g. via
/// `lookup_by_agent_cid(conn, local_agent_cid)`).
pub fn select_transport(
    self_manifest: &PeerTransportManifest,
    peer_manifest: &PeerTransportManifest,
    plane: Plane,
) -> Result<TransportChoice, StorageError> {
    // Rule 2: prefer iroh when both peers support the plane on iroh.
    let self_iroh = iroh_supports_plane(&self_manifest.iroh, plane);
    let peer_iroh = iroh_supports_plane(&peer_manifest.iroh, plane);
    if self_iroh && peer_iroh {
        return Ok(TransportChoice::Iroh);
    }

    // Rule 3: fall back to libp2p when both peers support the plane
    // on libp2p (covers "either lacks iroh" and "plane verdict
    // requires libp2p" cases — both reduce to "no iroh path").
    let self_libp2p = libp2p_supports_plane(&self_manifest.libp2p, plane);
    let peer_libp2p = libp2p_supports_plane(&peer_manifest.libp2p, plane);
    if self_libp2p && peer_libp2p {
        return Ok(TransportChoice::Libp2p);
    }

    // Rule 4: Track 3 dwelling-hub bridge for consumer-grade peer +
    // hub-capable self.
    if peer_manifest.capability_level <= 2 && self_manifest.capability_level >= 4 {
        return Ok(TransportChoice::Track3Bridge {
            hub_agent_cid: self_manifest.agent_cid.clone(),
        });
    }

    // Rule 5: no path.
    Ok(TransportChoice::NoSharedTransport)
}

// ────────────────────────────────────────────────────────────────
// Back-compat shims (Phase 10 API → Phase 12 store)
// ────────────────────────────────────────────────────────────────
//
// No production caller of these shims exists at Phase 12 land time
// (verified by crate-wide grep, see plan caller inventory). They
// remain to keep the Phase 10 surface stable for any out-of-tree
// consumer (steward) and for the README's documented examples.

fn iso_to_unix(observed_at: &str) -> Result<i64, StorageError> {
    chrono::DateTime::parse_from_rfc3339(observed_at)
        .map(|dt| dt.timestamp())
        .map_err(|e| StorageError::Database(format!("iso_to_unix({observed_at}): {e}")))
}

pub fn record_libp2p(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    peer_id: &str,
    observed_at: &str,
) -> Result<(), StorageError> {
    let ts = iso_to_unix(observed_at)?;
    record_libp2p_observation(conn, agent_cid, peer_id, &[], &[], ts)
}

pub fn record_iroh(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    node_id: &str,
    observed_at: &str,
) -> Result<(), StorageError> {
    let ts = iso_to_unix(observed_at)?;
    record_iroh_observation(conn, agent_cid, node_id, &[], &[], ts)
}

pub fn iroh_for_libp2p(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<Option<String>, StorageError> {
    Ok(lookup_by_libp2p_peer_id(conn, peer_id)?.and_then(|m| m.iroh.map(|p| p.node_id)))
}

pub fn libp2p_for_iroh(
    conn: &mut SqliteConnection,
    node_id: &str,
) -> Result<Option<String>, StorageError> {
    Ok(lookup_by_iroh_node_id(conn, node_id)?.and_then(|m| m.libp2p.map(|p| p.peer_id)))
}

// ────────────────────────────────────────────────────────────────
// Test constructors (Plan 2 cutover gate #2 helpers)
// ────────────────────────────────────────────────────────────────

impl PeerTransportManifest {
    /// Build an iroh-capable test manifest that advertises Blob on both
    /// transports (so `select_transport` returns `Iroh` for `Plane::Blob`).
    /// Only compiled for tests — never used in production code.
    #[cfg(test)]
    pub fn iroh_capable_for_test() -> Self {
        PeerTransportManifest {
            agent_cid: "test-iroh-capable".to_string(),
            libp2p: Some(Libp2pTransportProfile {
                peer_id: "12D3KooWIrohTest".to_string(),
                addrs: vec![],
                supports: vec!["blob".to_string()],
            }),
            iroh: Some(IrohTransportProfile {
                node_id: "iroh-test-node".to_string(),
                relays: vec![],
                supports: vec!["blob".to_string()],
            }),
            discovery: vec![],
            capability_level: 5,
            last_observed: 1746878400,
        }
    }

    /// Build a libp2p-only test manifest that has no iroh profile
    /// (so `select_transport` returns `Libp2p` for `Plane::Blob`).
    /// Only compiled for tests — never used in production code.
    #[cfg(test)]
    pub fn libp2p_only_for_test() -> Self {
        PeerTransportManifest {
            agent_cid: "test-libp2p-only".to_string(),
            libp2p: Some(Libp2pTransportProfile {
                peer_id: "12D3KooWLibp2pTest".to_string(),
                addrs: vec![],
                supports: vec!["blob".to_string()],
            }),
            iroh: None,
            discovery: vec![],
            capability_level: 5,
            last_observed: 1746878400,
        }
    }
}

// ────────────────────────────────────────────────────────────────
// Unit tests
// ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{init_pool_from_dir, run_migrations};
    use tempfile::tempdir;

    fn setup_conn() -> (
        tempfile::TempDir,
        diesel::r2d2::PooledConnection<
            diesel::r2d2::ConnectionManager<diesel::SqliteConnection>,
        >,
    ) {
        let dir = tempdir().unwrap();
        let pool = init_pool_from_dir(dir.path()).expect("pool");
        run_migrations(&pool).expect("migrations");
        let conn = pool.get().expect("conn");
        (dir, conn)
    }

    #[test]
    fn record_libp2p_observation_creates_row_with_supports() {
        let (_dir, mut conn) = setup_conn();
        record_libp2p_observation(
            &mut conn,
            "bafyrei...agent-1",
            "12D3KooWPeer1",
            &["/ip4/10.0.0.1/tcp/4001".to_string()],
            &[Plane::Blob, Plane::Gossip],
            1746878400,
        )
        .unwrap();
        let m = lookup_by_agent_cid(&mut conn, "bafyrei...agent-1")
            .unwrap()
            .expect("row");
        assert_eq!(m.agent_cid, "bafyrei...agent-1");
        assert!(m.libp2p.is_some());
        assert!(m.iroh.is_none());
        let lp = m.libp2p.unwrap();
        assert_eq!(lp.peer_id, "12D3KooWPeer1");
        assert_eq!(lp.supports, vec!["blob".to_string(), "gossip".to_string()]);
        assert_eq!(lp.addrs, vec!["/ip4/10.0.0.1/tcp/4001".to_string()]);
    }

    #[test]
    fn record_iroh_observation_upserts_existing_libp2p_row() {
        let (_dir, mut conn) = setup_conn();
        let agent = "bafyrei...agent-2";
        record_libp2p_observation(&mut conn, agent, "12D3KooWPeer2", &[], &[], 1746878400)
            .unwrap();
        record_iroh_observation(
            &mut conn,
            agent,
            "node-id-2",
            &["https://relay.iroh.network".to_string()],
            &[Plane::Sync],
            1746878401,
        )
        .unwrap();
        let m = lookup_by_agent_cid(&mut conn, agent).unwrap().expect("row");
        assert!(m.libp2p.is_some());
        assert!(m.iroh.is_some());
        assert_eq!(m.iroh.as_ref().unwrap().node_id, "node-id-2");
        assert_eq!(m.last_observed, 1746878401);
    }

    #[test]
    fn record_capability_overwrites_default() {
        let (_dir, mut conn) = setup_conn();
        let agent = "bafyrei...agent-3";
        record_libp2p_observation(&mut conn, agent, "12D3KooWPeer3", &[], &[], 1746878400)
            .unwrap();
        record_capability(&mut conn, agent, 3).unwrap();
        let m = lookup_by_agent_cid(&mut conn, agent).unwrap().expect("row");
        assert_eq!(m.capability_level, 3);
    }

    #[test]
    fn record_discovery_replaces_methods() {
        let (_dir, mut conn) = setup_conn();
        let agent = "bafyrei...agent-4";
        record_libp2p_observation(&mut conn, agent, "12D3KooWPeer4", &[], &[], 1746878400)
            .unwrap();
        record_discovery(&mut conn, agent, &["pkarr", "mdns"]).unwrap();
        let m = lookup_by_agent_cid(&mut conn, agent).unwrap().expect("row");
        assert_eq!(m.discovery, vec!["pkarr".to_string(), "mdns".to_string()]);
    }

    #[test]
    fn lookup_by_agent_cid_returns_full_manifest() {
        let (_dir, mut conn) = setup_conn();
        let agent = "bafyrei...agent-5";
        record_libp2p_observation(
            &mut conn,
            agent,
            "12D3KooWPeer5",
            &["/ip4/10.0.0.5/tcp/4001".to_string()],
            &[Plane::Blob],
            1746878400,
        )
        .unwrap();
        record_iroh_observation(
            &mut conn,
            agent,
            "node-id-5",
            &[],
            &[Plane::Sync, Plane::Epr],
            1746878401,
        )
        .unwrap();
        let m = lookup_by_agent_cid(&mut conn, agent).unwrap().expect("row");
        assert_eq!(m.agent_cid, agent);
        assert!(m.libp2p.is_some());
        assert!(m.iroh.is_some());
        assert_eq!(m.iroh.as_ref().unwrap().supports.len(), 2);
    }

    #[test]
    fn lookup_by_libp2p_peer_id_finds_row() {
        let (_dir, mut conn) = setup_conn();
        let agent = "bafyrei...agent-6";
        record_libp2p_observation(&mut conn, agent, "12D3KooWPeer6", &[], &[], 1746878400)
            .unwrap();
        let m = lookup_by_libp2p_peer_id(&mut conn, "12D3KooWPeer6")
            .unwrap()
            .expect("row");
        assert_eq!(m.agent_cid, agent);
    }

    #[test]
    fn lookup_by_iroh_node_id_finds_row() {
        let (_dir, mut conn) = setup_conn();
        let agent = "bafyrei...agent-7";
        record_iroh_observation(&mut conn, agent, "node-id-7", &[], &[], 1746878400).unwrap();
        let m = lookup_by_iroh_node_id(&mut conn, "node-id-7")
            .unwrap()
            .expect("row");
        assert_eq!(m.agent_cid, agent);
    }

    #[test]
    fn lookup_returns_none_for_unknown() {
        let (_dir, mut conn) = setup_conn();
        assert!(lookup_by_agent_cid(&mut conn, "bafyrei...nope").unwrap().is_none());
        assert!(lookup_by_libp2p_peer_id(&mut conn, "12D3KooWNope").unwrap().is_none());
        assert!(lookup_by_iroh_node_id(&mut conn, "node-nope").unwrap().is_none());
    }

    #[test]
    fn back_compat_record_libp2p_writes_row() {
        let (_dir, mut conn) = setup_conn();
        record_libp2p(&mut conn, "bafyrei...compat-1", "12D3KooWC1", "2026-05-10T12:00:00Z")
            .unwrap();
        assert!(lookup_by_libp2p_peer_id(&mut conn, "12D3KooWC1").unwrap().is_some());
    }

    #[test]
    fn back_compat_record_iroh_writes_row() {
        let (_dir, mut conn) = setup_conn();
        record_iroh(&mut conn, "bafyrei...compat-2", "node-compat-2", "2026-05-10T12:00:00Z")
            .unwrap();
        assert!(lookup_by_iroh_node_id(&mut conn, "node-compat-2").unwrap().is_some());
    }

    #[test]
    fn back_compat_iroh_for_libp2p_resolves() {
        let (_dir, mut conn) = setup_conn();
        let agent = "bafyrei...compat-3";
        record_libp2p(&mut conn, agent, "12D3KooWC3", "2026-05-10T12:00:00Z").unwrap();
        record_iroh(&mut conn, agent, "node-compat-3", "2026-05-10T12:01:00Z").unwrap();
        let nid = iroh_for_libp2p(&mut conn, "12D3KooWC3").unwrap();
        assert_eq!(nid.as_deref(), Some("node-compat-3"));
    }

    #[test]
    fn back_compat_libp2p_for_iroh_resolves() {
        let (_dir, mut conn) = setup_conn();
        let agent = "bafyrei...compat-4";
        record_libp2p(&mut conn, agent, "12D3KooWC4", "2026-05-10T12:00:00Z").unwrap();
        record_iroh(&mut conn, agent, "node-compat-4", "2026-05-10T12:01:00Z").unwrap();
        let pid = libp2p_for_iroh(&mut conn, "node-compat-4").unwrap();
        assert_eq!(pid.as_deref(), Some("12D3KooWC4"));
    }

    // ── select_transport tests ──────────────────────────────────

    fn make_manifest(
        agent_cid: &str,
        libp2p_supports: &[Plane],
        iroh_supports: &[Plane],
        capability_level: u8,
    ) -> PeerTransportManifest {
        let libp2p = if libp2p_supports.is_empty() {
            None
        } else {
            Some(Libp2pTransportProfile {
                peer_id: format!("12D3KooW{agent_cid}"),
                addrs: vec![],
                supports: libp2p_supports.iter().map(|p| p.as_str().to_string()).collect(),
            })
        };
        let iroh = if iroh_supports.is_empty() {
            None
        } else {
            Some(IrohTransportProfile {
                node_id: format!("node-{agent_cid}"),
                relays: vec![],
                supports: iroh_supports.iter().map(|p| p.as_str().to_string()).collect(),
            })
        };
        PeerTransportManifest {
            agent_cid: agent_cid.to_string(),
            libp2p,
            iroh,
            discovery: vec![],
            capability_level,
            last_observed: 1746878400,
        }
    }

    #[test]
    fn select_transport_both_iroh_supported_picks_iroh() {
        let s = make_manifest("self", &[], &[Plane::Blob], 5);
        let p = make_manifest("peer", &[], &[Plane::Blob], 5);
        assert_eq!(
            select_transport(&s, &p, Plane::Blob).unwrap(),
            TransportChoice::Iroh
        );
    }

    #[test]
    fn select_transport_only_libp2p_shared_falls_back() {
        // iroh on both but Plane::Trust only in libp2p.supports for one
        let s = make_manifest("self", &[Plane::Trust], &[Plane::Blob], 5);
        let p = make_manifest("peer", &[Plane::Trust], &[Plane::Blob], 5);
        assert_eq!(
            select_transport(&s, &p, Plane::Trust).unwrap(),
            TransportChoice::Libp2p
        );
    }

    #[test]
    fn select_transport_iroh_unsupported_by_peer_picks_libp2p() {
        // self has both transports, peer is libp2p-only
        let s = make_manifest("self", &[Plane::Gossip], &[Plane::Gossip], 5);
        let p = make_manifest("peer", &[Plane::Gossip], &[], 5);
        // peer has no iroh, so iroh fails; both have libp2p.gossip
        assert_eq!(
            select_transport(&s, &p, Plane::Gossip).unwrap(),
            TransportChoice::Libp2p
        );
    }

    #[test]
    fn select_transport_low_capability_no_shared_returns_track3() {
        // peer capability_level=2 and no shared transport, self capability_level=5
        let s = make_manifest("hub", &[Plane::Sync], &[], 5);
        let p = make_manifest("phone", &[], &[], 2); // no transports at all — protocol violation
                                                     // but select_transport handles it gracefully
        match select_transport(&s, &p, Plane::Sync).unwrap() {
            TransportChoice::Track3Bridge { hub_agent_cid } => {
                assert_eq!(hub_agent_cid, "hub");
            }
            other => panic!("expected Track3Bridge, got {other:?}"),
        }
    }

    #[test]
    fn select_transport_no_shared_no_hub_returns_no_shared() {
        // peer capability_level=2 but self capability_level=2 → cannot hub
        let s = make_manifest("phone-s", &[Plane::Sync], &[], 2);
        let p = make_manifest("phone-p", &[], &[], 2);
        assert_eq!(
            select_transport(&s, &p, Plane::Sync).unwrap(),
            TransportChoice::NoSharedTransport
        );
    }

    #[test]
    fn select_transport_unsupported_plane_on_both_returns_no_shared() {
        // both peers have iroh + libp2p but neither lists Plane::Shard in either
        let s = make_manifest("self", &[Plane::Blob], &[Plane::Blob], 5);
        let p = make_manifest("peer", &[Plane::Blob], &[Plane::Blob], 5);
        assert_eq!(
            select_transport(&s, &p, Plane::Shard).unwrap(),
            TransportChoice::NoSharedTransport
        );
    }
}
