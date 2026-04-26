//! EprStore trait — the seam between REST routes and storage backends.
//!
//! LocalEprStore wraps the diesel layer (Phase 2a).
//! FederatedEprStore wraps LocalEprStore + a libp2p swarm handle and, on local
//! miss, issues EprRequest::Resolve via /elohim/epr/1.0.0 to discover peers
//! that hold the atom. Phase 2a ships FederatedEprStore with the libp2p bridge
//! stubbed as TODO(phase-2b); Phase 2b wires the swarm handle and flips the
//! construction site from LocalEprStore → FederatedEprStore.

use diesel::SqliteConnection;

use crate::db::epr_atoms::EprAtom;
use crate::db::epr_atoms::EprListQuery;
use crate::error::StorageError;
use crate::services::epr_service::{self, EprIngestResult, EprVerifyReport, FetchedEpr};
use elohim_epr::Epr;

/// Outcome of a fetch — the atom plus its source-of-truth provenance.
#[derive(Debug, Clone)]
pub struct FetchOutcome {
    pub fetched: FetchedEpr,
    pub source: EprSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EprSource {
    /// Served from the local diesel store.
    Local,
    /// Served by fetching from a remote peer and caching locally.
    /// The String is the remote peer id.
    Peer(String),
}

impl EprSource {
    pub fn header_value(&self) -> String {
        match self {
            EprSource::Local => "local".into(),
            EprSource::Peer(id) => format!("peer:{id}"),
        }
    }
}

/// Reference to a peer that claims to have a given CID.
#[derive(Debug, Clone)]
pub struct ProviderRef {
    pub peer_id: String,
    pub advertised_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ProviderRef {
    pub fn local() -> Self {
        ProviderRef {
            peer_id: "local".into(),
            advertised_at: None,
        }
    }
}

/// Abstraction over "where EPRs live." Route handlers never know whether the
/// store is local-only or federated.
pub trait EprStore: Send + Sync {
    /// Look up an EPR by CID. Returns None if the CID is not reachable by
    /// this store (i.e., not local and not available from peers).
    fn fetch(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
    ) -> Result<Option<FetchOutcome>, StorageError>;

    /// Idempotent put. If the CID already exists locally, validates inbound
    /// matches stored and returns the existing result (200, not 409).
    fn put(&self, conn: &mut SqliteConnection, epr: Epr) -> Result<EprIngestResult, StorageError>;

    /// List atoms held locally by this store. Federation across peers is a
    /// separate concern — see `list_federated` in Phase 2c.
    fn list(
        &self,
        conn: &mut SqliteConnection,
        q: &EprListQuery,
    ) -> Result<(Vec<EprAtom>, Option<String>), StorageError>;

    /// Verify a stored EPR against a caller-supplied public key. Local
    /// operation — no network required.
    fn verify(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
        public_key: &[u8; 32],
    ) -> Result<EprVerifyReport, StorageError>;

    /// Return the set of peers advertised as holding this CID. Phase 2a
    /// returns only `[ProviderRef::local()]` when the atom is held locally,
    /// or an empty Vec when it isn't. Phase 2c extends with DHT provider
    /// records via Kad `get_providers`.
    fn providers(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
    ) -> Result<Vec<ProviderRef>, StorageError>;
}

// ---------------------------------------------------------------------------
// LocalEprStore
// ---------------------------------------------------------------------------

pub struct LocalEprStore;

impl LocalEprStore {
    pub fn new() -> Self {
        LocalEprStore
    }
}

impl Default for LocalEprStore {
    fn default() -> Self {
        LocalEprStore::new()
    }
}

impl EprStore for LocalEprStore {
    fn fetch(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
    ) -> Result<Option<FetchOutcome>, StorageError> {
        Ok(
            epr_service::fetch_by_cid(conn, cid)?.map(|fetched| FetchOutcome {
                fetched,
                source: EprSource::Local,
            }),
        )
    }

    fn put(&self, conn: &mut SqliteConnection, epr: Epr) -> Result<EprIngestResult, StorageError> {
        let cid = epr.envelope.cid.to_string();
        // Idempotent semantics: if the CID already exists, validate inbound
        // matches stored and return the existing result (catches collision attempts
        // where caller re-publishes a different signer/proof over the same cid).
        if let Some(existing) = epr_service::fetch_by_cid(conn, &cid)? {
            let inbound_canonical = epr
                .envelope
                .canonical_bytes(&epr.payload)
                .map_err(|e| StorageError::InvalidInput(format!("canonicalize: {e}")))?;
            if inbound_canonical != existing.atom.canonical_bytes {
                return Err(StorageError::InvalidInput(format!(
                    "cid {cid} already exists with different canonical bytes — not idempotent"
                )));
            }
            return Ok(EprIngestResult { cid });
        }
        epr_service::ingest(conn, epr)
    }

    fn list(
        &self,
        conn: &mut SqliteConnection,
        q: &EprListQuery,
    ) -> Result<(Vec<EprAtom>, Option<String>), StorageError> {
        epr_service::list(conn, q)
    }

    fn verify(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
        public_key: &[u8; 32],
    ) -> Result<EprVerifyReport, StorageError> {
        epr_service::verify(conn, cid, public_key)
    }

    fn providers(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
    ) -> Result<Vec<ProviderRef>, StorageError> {
        match epr_service::fetch_by_cid(conn, cid)? {
            Some(_) => Ok(vec![ProviderRef::local()]),
            None => Ok(vec![]),
        }
    }
}

// ---------------------------------------------------------------------------
// FederatedEprStore — Phase 2b wires libp2p Kad provider advertisement
// ---------------------------------------------------------------------------

use super::epr_kind::{kind_canonical_str, pillar_for_kind_provisional};

/// Federated store that bridges to the elohim-storage libp2p swarm for Kad
/// provider advertisement (Phase 2b D.2). In Phase 2a the swarm_tx was
/// stubbed; Phase 2b wires it via `with_swarm_tx`.
pub struct FederatedEprStore {
    local: LocalEprStore,
    /// Optional sender for the P2P command channel. When Some, successful
    /// puts that match a Kad-tier fanout policy issue `KadStartProviding`.
    /// When None (default, or in test contexts without a swarm), Kad
    /// advertisement is silently skipped.
    swarm_tx: Option<tokio::sync::mpsc::Sender<crate::p2p::P2PCommand>>,
}

impl FederatedEprStore {
    pub fn new() -> Self {
        FederatedEprStore {
            local: LocalEprStore::new(),
            swarm_tx: None,
        }
    }

    /// Wire the P2P command channel so `put` can issue `KadStartProviding`
    /// when the EPR's fanout policy includes a Kad or KadLight channel.
    pub fn with_swarm_tx(mut self, tx: tokio::sync::mpsc::Sender<crate::p2p::P2PCommand>) -> Self {
        self.swarm_tx = Some(tx);
        self
    }
}

impl Default for FederatedEprStore {
    fn default() -> Self {
        FederatedEprStore::new()
    }
}

impl EprStore for FederatedEprStore {
    fn fetch(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
    ) -> Result<Option<FetchOutcome>, StorageError> {
        if let Some(outcome) = self.local.fetch(conn, cid)? {
            return Ok(Some(outcome));
        }
        // TODO(phase-2b): on local miss, issue swarm_handle.resolve_epr(cid).
        // For each returned peer, send EprRequest::Resolve { id: cid }; if
        // EprResponse::Head arrives, decode + validate + LocalEprStore::put + return
        // FetchOutcome::Peer(peer_id). Give up after N peers or T timeout.
        Ok(None)
    }

    fn put(&self, conn: &mut SqliteConnection, epr: Epr) -> Result<EprIngestResult, StorageError> {
        // Snapshot reach + kind before consuming epr in local.put.
        let reach = epr.envelope.reach;
        let epr_kind = epr.envelope.kind;

        let result = self.local.put(conn, epr)?;

        // D.2: advertise atom to Kademlia DHT when the fanout policy includes
        // a Kad or KadLight channel. Best-effort — send failure is logged and
        // dropped; the local put already succeeded and remains authoritative.
        use crate::p2p::fanout::{channels_for_reach, FanoutChannel};
        let kind_str = kind_canonical_str(epr_kind);
        let channels = channels_for_reach(reach, kind_str);
        let needs_kad = channels
            .iter()
            .any(|c| matches!(c, FanoutChannel::Kad | FanoutChannel::KadLight));
        if needs_kad {
            if let Some(tx) = &self.swarm_tx {
                match tx.try_send(crate::p2p::P2PCommand::KadStartProviding {
                    cid: result.cid.clone(),
                }) {
                    Ok(()) => {}
                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                        tracing::warn!(
                            cid = %result.cid,
                            "kad_start_providing: channel full — skipping (best-effort)"
                        );
                    }
                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                        tracing::error!(
                            cid = %result.cid,
                            "kad_start_providing: swarm command channel closed — Kad advertisement permanently lost"
                        );
                    }
                }
            }
        }

        // D.3: publish EPR atom announce to the reach-scoped gossipsub topic when
        // the fanout policy includes a Gossip channel. Best-effort — send failure
        // is logged and dropped; the local put already succeeded.
        let needs_gossip = channels.iter().any(|c| matches!(c, FanoutChannel::Gossip));
        if needs_gossip {
            if let Some(tx) = &self.swarm_tx {
                let pillar = pillar_for_kind_provisional(epr_kind);
                let topic = crate::p2p::topics::topic_for(&pillar, reach, None);
                match build_announce_payload(&result.cid) {
                    Ok(payload) => {
                        match tx.try_send(crate::p2p::P2PCommand::PublishEprAnnounce {
                            topic: topic.clone(),
                            payload,
                        }) {
                            Ok(()) => {}
                            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                                tracing::warn!(
                                    cid = %result.cid,
                                    topic = %topic,
                                    "publish_announce: channel full — skipping (best-effort)"
                                );
                            }
                            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                tracing::error!(
                                    cid = %result.cid,
                                    topic = %topic,
                                    "publish_announce: swarm command channel closed"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            cid = %result.cid,
                            error = %e,
                            "publish_announce: payload encoding failed — skipping"
                        );
                    }
                }
            }
        }

        Ok(result)
    }

    fn list(
        &self,
        conn: &mut SqliteConnection,
        q: &EprListQuery,
    ) -> Result<(Vec<EprAtom>, Option<String>), StorageError> {
        // Federated list is a separate concern (spans many peers + aggregation
        // + cursor across them). For now, delegate to local.
        self.local.list(conn, q)
    }

    fn verify(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
        public_key: &[u8; 32],
    ) -> Result<EprVerifyReport, StorageError> {
        self.local.verify(conn, cid, public_key)
    }

    fn providers(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
    ) -> Result<Vec<ProviderRef>, StorageError> {
        let providers = self.local.providers(conn, cid)?;
        // TODO(phase-2b): extend providers with DHT provider records:
        //   let dht_providers = self.swarm_handle.kad_get_providers(cid).await?;
        //   providers.extend(dht_providers.into_iter().map(ProviderRef::from));
        Ok(providers)
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Encode an EPR CID as a MessagePack announce payload.
///
/// The payload is intentionally minimal: just the CID string. Receivers can
/// fetch the full atom via the EPR atom protocol if they want the payload.
/// This keeps gossip bandwidth low for high-reach tiers.
fn build_announce_payload(cid: &str) -> Result<Vec<u8>, crate::error::StorageError> {
    rmp_serde::to_vec(&cid.to_string())
        .map_err(|e| crate::error::StorageError::Internal(format!("encode announce: {e}")))
}

// ---------------------------------------------------------------------------
// Factory function — used by route handlers (avoids AppContext field cascade)
// ---------------------------------------------------------------------------

/// Construct the Phase 2b default store. Routes call this; the optional
/// `swarm_tx` wires Kad provider advertisement when the P2P swarm is running.
pub fn default_epr_store(
    swarm_tx: Option<tokio::sync::mpsc::Sender<crate::p2p::P2PCommand>>,
) -> impl EprStore {
    let store = FederatedEprStore::new();
    match swarm_tx {
        Some(tx) => store.with_swarm_tx(tx),
        None => store,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;
    use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
    use tokio::sync::mpsc;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn setup_conn() -> diesel::SqliteConnection {
        use diesel::Connection;
        let mut conn =
            diesel::SqliteConnection::establish(":memory:").expect("open in-memory SQLite");
        let sql = std::fs::read_to_string("migrations/2026-04-22-050000_add_epr_tables/up.sql")
            .expect("load epr tables migration");
        conn.batch_execute(&sql)
            .expect("apply epr tables migration");
        let a7_sql =
            std::fs::read_to_string("migrations/2026-04-25-000000_verified_at_on_epr_atoms/up.sql")
                .expect("load a7 verified_at migration");
        conn.batch_execute(&a7_sql).expect("apply a7 migration");
        conn
    }

    fn sample_epr_with_reach(reach: Reach) -> Epr {
        let kp = AgentKeypair::from_secret(&[99u8; 32]).unwrap();
        let signer_cid = compute_cid(b"test-signer");
        let schema_ref = compute_cid(b"test-schema-ref");
        let gov = compute_cid(b"test-governance");
        let coupling = Coupling {
            knowledge: None,
            value: None,
            governance: Some(gov),
        };
        Epr::builder()
            .kind(EprKind::Manifest)
            .schema_ref(schema_ref)
            .schema_key("test/schema")
            .reach(reach)
            .coupling(coupling)
            .issued_at(chrono::Utc::now())
            .payload(b"test-payload".to_vec())
            .sign(&kp, signer_cid)
            .expect("sign test EPR")
    }

    // -----------------------------------------------------------------------
    // D.2 tests
    // -----------------------------------------------------------------------

    /// Drain all commands from the channel into a Vec for inspection.
    fn drain_commands(
        rx: &mut mpsc::Receiver<crate::p2p::P2PCommand>,
    ) -> Vec<crate::p2p::P2PCommand> {
        let mut cmds = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(cmd) => cmds.push(cmd),
                Err(_) => break,
            }
        }
        cmds
    }

    // -----------------------------------------------------------------------
    // D.2 tests — Kad advertisement
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn put_with_commons_reach_sends_kad_start_providing() {
        let (tx, mut rx) = mpsc::channel::<crate::p2p::P2PCommand>(8);
        let store = FederatedEprStore::new().with_swarm_tx(tx);
        let mut conn = setup_conn();

        let epr = sample_epr_with_reach(Reach::Commons);
        let expected_cid = epr.envelope.cid.to_string();

        let result = store.put(&mut conn, epr).expect("put must succeed");
        assert_eq!(result.cid, expected_cid);

        // D.2+D.3: Commons fanout = [Kad, Gossip] → two commands.
        // Drain all commands and assert KadStartProviding is present.
        let cmds = drain_commands(&mut rx);
        let kad_cmd = cmds
            .iter()
            .find(|c| matches!(c, crate::p2p::P2PCommand::KadStartProviding { .. }));
        match kad_cmd {
            Some(crate::p2p::P2PCommand::KadStartProviding { cid }) => {
                assert_eq!(cid, &expected_cid, "KadStartProviding CID must match");
            }
            _ => panic!("KadStartProviding not found in {} commands", cmds.len()),
        }
    }

    #[tokio::test]
    async fn put_with_private_reach_does_not_send_kad() {
        let (tx, mut rx) = mpsc::channel::<crate::p2p::P2PCommand>(8);
        let store = FederatedEprStore::new().with_swarm_tx(tx);
        let mut conn = setup_conn();

        let epr = sample_epr_with_reach(Reach::Private);
        let result = store.put(&mut conn, epr).expect("put must succeed");
        assert!(!result.cid.is_empty());

        // Private reach → DirectOnly fanout; no commands at all.
        let cmds = drain_commands(&mut rx);
        assert!(
            cmds.is_empty(),
            "Private reach must NOT send any P2P commands"
        );
    }

    #[tokio::test]
    async fn put_with_public_reach_sends_kad_start_providing() {
        // Public reach → [Gossip, Kad] fanout; KadStartProviding must be sent.
        let (tx, mut rx) = mpsc::channel::<crate::p2p::P2PCommand>(8);
        let store = FederatedEprStore::new().with_swarm_tx(tx);
        let mut conn = setup_conn();

        let epr = sample_epr_with_reach(Reach::Public);
        let expected_cid = epr.envelope.cid.to_string();

        let result = store.put(&mut conn, epr).expect("put must succeed");
        assert_eq!(result.cid, expected_cid);

        // D.2+D.3: Public fanout = [Gossip, Kad] → two commands.
        let cmds = drain_commands(&mut rx);
        let kad_cmd = cmds
            .iter()
            .find(|c| matches!(c, crate::p2p::P2PCommand::KadStartProviding { .. }));
        match kad_cmd {
            Some(crate::p2p::P2PCommand::KadStartProviding { cid }) => {
                assert_eq!(cid, &expected_cid, "KadStartProviding CID must match");
            }
            _ => panic!("KadStartProviding not found in {} commands", cmds.len()),
        }
    }

    #[tokio::test]
    async fn put_without_swarm_tx_is_ok() {
        // FederatedEprStore::new() has no swarm_tx — should not panic even for
        // Commons reach (which would need Kad + Gossip if a swarm were present).
        let store = FederatedEprStore::new();
        let mut conn = setup_conn();

        let epr = sample_epr_with_reach(Reach::Commons);
        let result = store
            .put(&mut conn, epr)
            .expect("put must succeed without swarm_tx");
        assert!(!result.cid.is_empty());
    }

    // -----------------------------------------------------------------------
    // D.3 tests — gossip announce publish
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn put_with_familiar_reach_sends_publish_announce() {
        // Familiar → [Gossip] only — no Kad, but a PublishEprAnnounce must be sent.
        let (tx, mut rx) = mpsc::channel::<crate::p2p::P2PCommand>(8);
        let store = FederatedEprStore::new().with_swarm_tx(tx);
        let mut conn = setup_conn();

        // Use EprKind::Manifest → kind_str = "manifest" → pillar placeholder = "manifest"
        let epr = sample_epr_with_reach(Reach::Familiar);
        let expected_cid = epr.envelope.cid.to_string();

        let result = store.put(&mut conn, epr).expect("put must succeed");
        assert_eq!(result.cid, expected_cid);

        let cmds = drain_commands(&mut rx);

        // No KadStartProviding for Familiar reach.
        assert!(
            !cmds
                .iter()
                .any(|c| matches!(c, crate::p2p::P2PCommand::KadStartProviding { .. })),
            "Familiar reach must NOT send KadStartProviding"
        );

        // Exactly one PublishEprAnnounce.
        let announce_cmd = cmds
            .iter()
            .find(|c| matches!(c, crate::p2p::P2PCommand::PublishEprAnnounce { .. }));
        match announce_cmd {
            Some(crate::p2p::P2PCommand::PublishEprAnnounce { topic, payload }) => {
                // sample_epr_with_reach uses EprKind::Manifest → pillar = "manifest",
                // reach = Familiar → exact topic = "elohim/manifest/familiar".
                assert_eq!(
                    topic, "elohim/manifest/familiar",
                    "topic must be exact: elohim/manifest/familiar, got: {topic}"
                );

                // Payload must decode back to the CID.
                let decoded_cid: String =
                    rmp_serde::from_slice(payload).expect("payload must be valid msgpack string");
                assert_eq!(
                    decoded_cid, expected_cid,
                    "payload CID must match put result CID"
                );
            }
            _ => panic!("PublishEprAnnounce not found in {} commands", cmds.len()),
        }
    }

    #[tokio::test]
    async fn put_with_commons_reach_sends_both_kad_and_announce() {
        // Commons → [Kad, Gossip]; both KadStartProviding AND PublishEprAnnounce must be sent.
        let (tx, mut rx) = mpsc::channel::<crate::p2p::P2PCommand>(8);
        let store = FederatedEprStore::new().with_swarm_tx(tx);
        let mut conn = setup_conn();

        let epr = sample_epr_with_reach(Reach::Commons);
        let expected_cid = epr.envelope.cid.to_string();

        let result = store.put(&mut conn, epr).expect("put must succeed");
        assert_eq!(result.cid, expected_cid);

        let cmds = drain_commands(&mut rx);

        // KadStartProviding must be present.
        let has_kad = cmds
            .iter()
            .any(|c| matches!(c, crate::p2p::P2PCommand::KadStartProviding { cid } if cid == &expected_cid));
        assert!(has_kad, "Commons reach must send KadStartProviding");

        // PublishEprAnnounce must be present.
        let announce_cmd = cmds
            .iter()
            .find(|c| matches!(c, crate::p2p::P2PCommand::PublishEprAnnounce { .. }));
        match announce_cmd {
            Some(crate::p2p::P2PCommand::PublishEprAnnounce { topic, payload }) => {
                assert!(
                    topic.ends_with("/commons"),
                    "topic must end with /commons, got: {topic}"
                );
                let decoded_cid: String =
                    rmp_serde::from_slice(payload).expect("payload must be valid msgpack string");
                assert_eq!(decoded_cid, expected_cid, "payload CID must match");
            }
            _ => panic!("PublishEprAnnounce not found in {} commands", cmds.len()),
        }

        // Exactly two commands: one Kad, one Gossip.
        assert_eq!(
            cmds.len(),
            2,
            "Commons reach must send exactly two commands"
        );
    }

    #[tokio::test]
    async fn put_with_private_reach_sends_neither_kad_nor_announce() {
        // Private → [DirectOnly]; no P2P commands at all.
        let (tx, mut rx) = mpsc::channel::<crate::p2p::P2PCommand>(8);
        let store = FederatedEprStore::new().with_swarm_tx(tx);
        let mut conn = setup_conn();

        let epr = sample_epr_with_reach(Reach::Private);
        let result = store.put(&mut conn, epr).expect("put must succeed");
        assert!(!result.cid.is_empty());

        let cmds = drain_commands(&mut rx);
        assert!(
            cmds.is_empty(),
            "Private reach must send neither Kad nor announce commands"
        );
    }
}
