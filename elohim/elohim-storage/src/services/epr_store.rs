//! EprStore trait — the seam between REST routes and storage backends.
//!
//! LocalEprStore wraps the diesel layer (Phase 2a).
//! FederatedEprStore wraps LocalEprStore + a libp2p swarm handle. Phase 2B
//! wired the swarm handle and flipped the construction site from LocalEprStore
//! → FederatedEprStore. Kad `start_providing` + gossipsub tiered fanout are
//! live as of Phase 2B Batch D. On local miss, cold-fetch via
//! `swarm_handle.resolve_epr(cid)` is the remaining Phase 3 work item
//! (see `TODO(phase-3)` at the miss site below).
//!
//! ## D.4: Receiver-side pre-authorization (seam placeholder)
//!
//! `crate::p2p::reach_authorization::classify_pre_authorization` is the pure
//! policy function that decides whether this node has standing in a given
//! gossipsub topic scope. It will be called when storage decides which topics
//! to subscribe to and which collectives to act as Kad provider for. That
//! wiring is downstream of D.4 (Phase 3+ work) — D.4 ships the policy module
//! and tests so Stage 2/3 graph walks (qahal household memberships, imagodei
//! relationship attestations) land at a clear seam without rewriting call
//! sites. NO subscription logic is added here in D.4.

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
    /// D.4: DB pool for Stage 1 reach earning resolution via
    /// `peer_identity_bindings`. When None (no pool configured, or in
    /// test/in-memory contexts), the signer_is_known check fails-open at
    /// Stage 1 — same as a pool-get error.
    db_pool: Option<crate::db::DbPool>,
    /// D.4: CID of the local conductor agent. When the signer CID matches
    /// this, the signer is trivially known (they are this node's own user).
    /// Set via `with_local_agent_cid`. When None, only the
    /// `peer_identity_bindings` lookup path is used.
    ///
    /// TODO(D.4): wire local_agent_cid from conductor signing client when available.
    local_agent_cid: Option<String>,
    /// D.7 dedup fix: the local node's libp2p PeerId as a string.
    ///
    /// Kademlia's `get_providers` may return the local node's own PeerId when
    /// this node is a provider. Without this field we cannot recognise it as
    /// `"local"` (since `peer.to_string()` yields `"12D3KooW..."`, never `"local"`),
    /// which would produce duplicate entries — one `"local"` and one raw PeerId.
    ///
    /// Set via `with_local_libp2p_peer_id`. When `None`, self-advertisement
    /// dedup is skipped (safe: produces a harmless duplicate in that edge-case).
    local_libp2p_peer_id: Option<String>,
}

impl FederatedEprStore {
    pub fn new() -> Self {
        FederatedEprStore {
            local: LocalEprStore::new(),
            swarm_tx: None,
            db_pool: None,
            local_agent_cid: None,
            local_libp2p_peer_id: None,
        }
    }

    /// Wire the P2P command channel so `put` can issue `KadStartProviding`
    /// when the EPR's fanout policy includes a Kad or KadLight channel.
    pub fn with_swarm_tx(mut self, tx: tokio::sync::mpsc::Sender<crate::p2p::P2PCommand>) -> Self {
        self.swarm_tx = Some(tx);
        self
    }

    /// D.4: Wire the DB pool so `put` can resolve `signer_is_known_agent`
    /// via `peer_identity_bindings`. Without this the resolver fails-open
    /// (Stage 1 policy: allow during projection-rebuild windows).
    pub fn with_db_pool(mut self, pool: crate::db::DbPool) -> Self {
        self.db_pool = Some(pool);
        self
    }

    /// D.4: Set the local conductor agent CID so that an EPR signed by
    /// this node's own user is recognized as a known agent without a DB
    /// lookup.
    pub fn with_local_agent_cid(mut self, cid: String) -> Self {
        self.local_agent_cid = Some(cid);
        self
    }

    /// D.7 dedup fix: Set the local node's libp2p PeerId string so that
    /// `providers()` can recognise and skip self-advertisement from Kademlia
    /// results.  Call this at construction time from wherever the swarm's
    /// PeerId is available (e.g. `P2PNode::peer_id().to_string()`).
    pub fn with_local_libp2p_peer_id(mut self, peer_id: String) -> Self {
        self.local_libp2p_peer_id = Some(peer_id);
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
        // TODO(phase-3): on local miss, issue swarm_handle.resolve_epr(cid).
        // For each returned peer, send EprRequest::Resolve { id: cid }; if
        // EprResponse::Head arrives, decode + validate + LocalEprStore::put + return
        // FetchOutcome::Peer(peer_id). Give up after N peers or T timeout.
        // See genesis/docs/plans/2026-04-26-epr-phase-3-manifest-resolver-kickoff-prompt.md
        // §Task: Cold-fetch via swarm.
        Ok(None)
    }

    fn put(&self, conn: &mut SqliteConnection, epr: Epr) -> Result<EprIngestResult, StorageError> {
        // D.4 Stage 1: author-side reach earning. Refuse the put if the signer
        // has not earned the declared reach. Receive side does NOT filter —
        // see memory `project_reach_earned_at_authoring` (the email-collapse
        // anti-pattern). This gate runs BEFORE any local persistence or
        // network publication (Kad/gossip) so that unauthorized EPRs never
        // enter the network.
        let reach = epr.envelope.reach;
        let signer_cid = epr.envelope.proof.signer.to_string();
        let signer_known = match self.db_pool.as_ref() {
            Some(pool) => crate::p2p::reach_authorization::signer_is_known_agent(
                pool,
                self.local_agent_cid.as_deref(),
                &signer_cid,
            ),
            None => true, // No pool = test/in-memory; fail-open at Stage 1
        };
        match crate::p2p::reach_authorization::classify_reach_authorization(
            reach,
            &signer_cid,
            signer_known,
        ) {
            crate::p2p::reach_authorization::ReachAuthDecision::Authorized => {}
            crate::p2p::reach_authorization::ReachAuthDecision::Refused { reason } => {
                return Err(crate::error::StorageError::Unauthorized(format!(
                    "reach authorization refused: {reason}"
                )));
            }
        }

        // Snapshot kind before consuming epr in local.put.
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
        let mut providers = self.local.providers(conn, cid)?;

        // D.7: extend with DHT providers from Kademlia `get_providers`.
        // Guard: skip if no swarm channel or no tokio runtime (e.g. sync unit tests).
        if let Some(tx) = &self.swarm_tx {
            match tokio::runtime::Handle::try_current() {
                Err(_) => {
                    tracing::debug!(
                        cid = %cid,
                        "providers: no tokio runtime — skipping Kad query"
                    );
                }
                Ok(handle) => {
                    let (reply_tx, reply_rx) =
                        tokio::sync::oneshot::channel::<Vec<libp2p::PeerId>>();
                    let cmd = crate::p2p::P2PCommand::KadGetProviders {
                        cid: cid.to_string(),
                        reply: reply_tx,
                    };
                    match tx.try_send(cmd) {
                        Err(e) => {
                            tracing::warn!(
                                cid = %cid,
                                error = ?e,
                                "providers: KadGetProviders command send failed"
                            );
                        }
                        Ok(()) => {
                            // Block the current thread safely: `block_in_place` moves other
                            // tokio tasks off this thread, then `block_on` drives the timeout
                            // future to completion. This avoids "cannot start a runtime from
                            // within a runtime" panics when called from an async HTTP handler.
                            let result = tokio::task::block_in_place(|| {
                                handle.block_on(async {
                                    tokio::time::timeout(
                                        crate::p2p::KAD_GET_PROVIDERS_TIMEOUT,
                                        reply_rx,
                                    )
                                    .await
                                })
                            });
                            match result {
                                Ok(Ok(peer_ids)) => {
                                    for peer_id in peer_ids {
                                        let peer_str = peer_id.to_string();
                                        // Skip self-advertisement: Kad may return the local
                                        // node's own PeerId; it is already present as
                                        // ProviderRef::local() (peer_id == "local").
                                        // Compare against the stored libp2p PeerId string,
                                        // NOT the literal "local" — peer.to_string() yields
                                        // "12D3KooW...", never "local".
                                        let is_self = self
                                            .local_libp2p_peer_id
                                            .as_deref()
                                            .map(|id| id == peer_str)
                                            .unwrap_or(false);
                                        if !is_self
                                            && !providers.iter().any(|p| p.peer_id == peer_str)
                                        {
                                            providers.push(ProviderRef {
                                                peer_id: peer_str,
                                                advertised_at: None,
                                            });
                                        }
                                    }
                                }
                                Ok(Err(_channel_closed)) => {
                                    tracing::debug!(
                                        cid = %cid,
                                        "providers: Kad reply channel closed before result"
                                    );
                                }
                                Err(_timeout) => {
                                    tracing::debug!(
                                        cid = %cid,
                                        "providers: Kad get_providers timed out after 5s"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

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

/// Construct the Phase 2b default store.
///
/// Routes call this; the optional `swarm_tx` wires Kad provider advertisement
/// when the P2P swarm is running. `pool` wires the Stage 1 reach-earning
/// resolver (D.4); when `None` the resolver fails-open. `local_agent_cid`
/// identifies the local conductor agent so self-signed EPRs are recognized
/// as known-agent without a DB lookup (D.4). `local_libp2p_peer_id` enables
/// correct self-advertisement dedup in `providers()` (D.7 fix): pass
/// `Some(node.peer_id().to_string())` where available; `None` is safe (dedup
/// is skipped, producing a harmless duplicate in that edge-case).
pub fn default_epr_store(
    swarm_tx: Option<tokio::sync::mpsc::Sender<crate::p2p::P2PCommand>>,
    pool: Option<crate::db::DbPool>,
    local_agent_cid: Option<String>,
    local_libp2p_peer_id: Option<String>,
) -> impl EprStore {
    let mut store = FederatedEprStore::new();
    if let Some(tx) = swarm_tx {
        store = store.with_swarm_tx(tx);
    }
    if let Some(p) = pool {
        store = store.with_db_pool(p);
    }
    if let Some(cid) = local_agent_cid {
        store = store.with_local_agent_cid(cid);
    }
    if let Some(pid) = local_libp2p_peer_id {
        store = store.with_local_libp2p_peer_id(pid);
    }
    store
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
        while let Ok(cmd) = rx.try_recv() {
            cmds.push(cmd);
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

    // -----------------------------------------------------------------------
    // D.4 tests — Stage 1 author-side reach earning
    // -----------------------------------------------------------------------

    /// Build a test DB pool using in-memory SQLite with EPR migrations applied.
    fn setup_pool() -> crate::db::DbPool {
        use crate::db::{init_pool, run_migrations};
        let url = format!(
            "file:epr_store_d4_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = init_pool(&url).expect("test pool init");
        run_migrations(&pool).expect("test migrations");
        pool
    }

    /// Insert a `peer_identity_bindings` row mapping `agent_cid` to a
    /// placeholder `peer_id` so the Stage 1 resolver recognises the signer.
    fn insert_binding(pool: &crate::db::DbPool, agent_cid: &str) {
        use crate::db::diesel_schema::peer_identity_bindings::dsl as p;
        use diesel::prelude::*;
        let mut conn = pool.get().expect("pool conn");
        diesel::insert_or_ignore_into(p::peer_identity_bindings)
            .values((
                p::peer_id.eq(format!("peer-for-{agent_cid}")),
                p::agent_cid.eq(agent_cid),
                p::dht_anchor_hash.eq("anchor-test"),
                p::valid_from.eq("2026-01-01T00:00:00Z"),
                p::valid_until.eq::<Option<String>>(None),
                p::observed_at.eq("2026-04-25T00:00:00Z"),
                p::source.eq("handshake"),
            ))
            .execute(&mut conn)
            .expect("insert binding");
    }

    /// Build an EPR whose `signer` CID is deterministic from `signer_seed`.
    fn sample_epr_with_signer_seed(reach: Reach, signer_seed: u8) -> (Epr, String) {
        use elohim_epr::cid::compute_cid;
        use elohim_epr::proof::AgentKeypair;
        use elohim_epr::Coupling;

        let kp = AgentKeypair::from_secret(&[signer_seed; 32]).unwrap();
        let signer_cid = compute_cid(&[signer_seed]);
        let schema_ref = compute_cid(b"test-schema-ref");
        let gov = compute_cid(b"test-governance");
        let coupling = Coupling {
            knowledge: None,
            value: None,
            governance: Some(gov),
        };
        let signer_cid_str = signer_cid.to_string();
        let epr = Epr::builder()
            .kind(elohim_epr::EprKind::Manifest)
            .schema_ref(schema_ref)
            .schema_key("test/schema")
            .reach(reach)
            .coupling(coupling)
            .issued_at(chrono::Utc::now())
            .payload(format!("payload-for-signer-{signer_seed}").into_bytes())
            .sign(&kp, signer_cid)
            .expect("sign test EPR");
        (epr, signer_cid_str)
    }

    /// D.4 test 1: Familiar reach + unknown signer (no binding, no local_agent_cid)
    /// → Err(StorageError::Unauthorized) AND no swarm command sent.
    #[tokio::test]
    async fn put_refuses_familiar_reach_for_unknown_signer() {
        let pool = setup_pool();
        let (tx, mut rx) = mpsc::channel::<crate::p2p::P2PCommand>(8);
        let store = FederatedEprStore::new()
            .with_db_pool(pool)
            .with_swarm_tx(tx);

        let (epr, _signer) = sample_epr_with_signer_seed(Reach::Familiar, 42);
        let mut conn = setup_conn();

        let err = store.put(&mut conn, epr).expect_err("should be refused");
        assert!(
            matches!(err, crate::error::StorageError::Unauthorized(_)),
            "expected Unauthorized, got: {err:?}"
        );

        // No swarm command must have been sent.
        let cmds = drain_commands(&mut rx);
        assert!(
            cmds.is_empty(),
            "No P2P command must be sent when reach authorization is refused; {} command(s) received",
            cmds.len()
        );
    }

    /// D.4 test 2: Familiar reach + known signer (peer_identity_bindings row)
    /// → Ok + announce sent.
    #[tokio::test]
    async fn put_authorizes_familiar_reach_for_known_signer() {
        let pool = setup_pool();
        let (epr, signer_cid) = sample_epr_with_signer_seed(Reach::Familiar, 43);

        // Pre-insert the binding so the signer is "known".
        insert_binding(&pool, &signer_cid);

        let (tx, mut rx) = mpsc::channel::<crate::p2p::P2PCommand>(8);
        let store = FederatedEprStore::new()
            .with_db_pool(pool)
            .with_swarm_tx(tx);

        let mut conn = setup_conn();
        let result = store.put(&mut conn, epr).expect("should succeed");
        assert!(!result.cid.is_empty());

        // Familiar reach → [Gossip] fanout → PublishEprAnnounce must be sent.
        let cmds = drain_commands(&mut rx);
        let has_announce = cmds
            .iter()
            .any(|c| matches!(c, crate::p2p::P2PCommand::PublishEprAnnounce { .. }));
        assert!(
            has_announce,
            "Expected PublishEprAnnounce for known signer at Familiar reach"
        );
    }

    /// D.4 test 3: Commons reach + unknown signer (no pool, no binding)
    /// → Ok (open reach, no known-agent requirement).
    #[tokio::test]
    async fn put_authorizes_commons_reach_for_unknown_signer() {
        // No pool needed — Commons is open broadcast; anyone can author.
        let pool = setup_pool();
        let (epr, _signer) = sample_epr_with_signer_seed(Reach::Commons, 44);

        let (tx, mut rx) = mpsc::channel::<crate::p2p::P2PCommand>(8);
        let store = FederatedEprStore::new()
            .with_db_pool(pool)
            .with_swarm_tx(tx);

        let mut conn = setup_conn();
        let result = store
            .put(&mut conn, epr)
            .expect("Commons reach must succeed for anyone");
        assert!(!result.cid.is_empty());

        // Commons fanout = [Kad, Gossip].
        let cmds = drain_commands(&mut rx);
        assert!(
            !cmds.is_empty(),
            "Commons reach must send P2P commands (Kad + Gossip)"
        );
    }

    // -----------------------------------------------------------------------
    // D.7 tests — providers() returns local + DHT providers
    // -----------------------------------------------------------------------

    /// D.7 test 1: store without swarm_tx — providers() returns the single
    /// local entry for a known CID, no panic.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn providers_local_only_when_no_swarm_tx() {
        let store = FederatedEprStore::new(); // no swarm_tx
        let mut conn = setup_conn();

        // Put an atom so it has a local provider.
        let epr = sample_epr_with_reach(Reach::Commons);
        let result = store.put(&mut conn, epr).expect("put must succeed");

        let providers = store
            .providers(&mut conn, &result.cid)
            .expect("providers must not error");
        assert_eq!(
            providers.len(),
            1,
            "local-only store must return exactly one provider"
        );
        assert_eq!(providers[0].peer_id, "local");
    }

    /// D.7 test 2: store with swarm_tx where the receiver replies with a known
    /// PeerId — providers() returns both local AND that PeerId.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn providers_includes_kad_results_when_swarm_provides() {
        let (tx, mut rx) = mpsc::channel::<crate::p2p::P2PCommand>(8);
        let store = FederatedEprStore::new().with_swarm_tx(tx);
        let mut conn = setup_conn();

        // Put a local atom.
        let epr = sample_epr_with_reach(Reach::Commons);
        let result = store.put(&mut conn, epr).expect("put must succeed");
        // Drain the put's Kad+Gossip commands.
        let _ = drain_commands(&mut rx);

        // Spawn a task that services the KadGetProviders command and replies with a peer ID.
        let fake_peer_str = "12D3KooWGfzSBGbqkBdgPRMjVU6VvBnEKYqF3ZnHBmfpFyG1sD7T";
        let fake_peer: libp2p::PeerId = fake_peer_str.parse().expect("valid PeerId for test");
        tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                if let crate::p2p::P2PCommand::KadGetProviders { reply, .. } = cmd {
                    let _ = reply.send(vec![fake_peer]);
                    break;
                }
            }
        });

        let providers = store
            .providers(&mut conn, &result.cid)
            .expect("providers must not error");

        assert_eq!(
            providers.len(),
            2,
            "expected local + 1 DHT provider, got: {:?}",
            providers.iter().map(|p| &p.peer_id).collect::<Vec<_>>()
        );
        assert!(
            providers.iter().any(|p| p.peer_id == "local"),
            "local must be present"
        );
        assert!(
            providers.iter().any(|p| p.peer_id == fake_peer_str),
            "DHT peer must be present"
        );
    }

    /// D.7 test 3: store with swarm_tx but the receiver is dropped (simulating
    /// a timeout / channel close) — providers() returns just the local entry
    /// without panicking.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn providers_handles_kad_timeout_gracefully() {
        let (tx, rx) = mpsc::channel::<crate::p2p::P2PCommand>(8);
        let store = FederatedEprStore::new().with_swarm_tx(tx);
        let mut conn = setup_conn();

        // Put a local atom.
        let epr = sample_epr_with_reach(Reach::Commons);
        let result = store.put(&mut conn, epr).expect("put must succeed");

        // Drop the receiver immediately to simulate timeout (channel closed).
        drop(rx);

        // providers() must not panic, must return just the local entry.
        let providers = store
            .providers(&mut conn, &result.cid)
            .expect("providers must not error even when swarm is gone");
        assert_eq!(
            providers.len(),
            1,
            "should get only local provider on channel close"
        );
        assert_eq!(providers[0].peer_id, "local");
    }

    /// D.7 test 4: self-advertisement dedup — when the swarm returns the
    /// local node's own libp2p PeerId, providers() must NOT produce a duplicate
    /// entry.  The local ProviderRef ("local") is already present; the Kad
    /// result for the same node must be suppressed.
    ///
    /// Regression guard for the broken `peer_str != "local"` guard: a real
    /// PeerId string is never equal to the literal "local", so the old code
    /// would push a second entry.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn providers_dedups_self_when_kad_returns_local_peer_id() {
        // Generate a deterministic libp2p keypair so the PeerId is known.
        let local_kp = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = local_kp.public().to_peer_id();
        let local_peer_id_str = local_peer_id.to_string();

        let (tx, mut rx) = mpsc::channel::<crate::p2p::P2PCommand>(8);
        let store = FederatedEprStore::new()
            .with_swarm_tx(tx)
            .with_local_libp2p_peer_id(local_peer_id_str.clone());
        let mut conn = setup_conn();

        // Put an atom so the local store has a provider entry ("local").
        let epr = sample_epr_with_reach(Reach::Commons);
        let result = store.put(&mut conn, epr).expect("put must succeed");
        // Drain the put's Kad+Gossip commands so the channel is clear.
        let _ = drain_commands(&mut rx);

        // Fake responder: returns the local node's own PeerId — this is the
        // case Kademlia produces when the node is itself a provider.
        let local_peer_id_clone = local_peer_id;
        tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                if let crate::p2p::P2PCommand::KadGetProviders { reply, .. } = cmd {
                    // Return the local node as if Kad found it.
                    let _ = reply.send(vec![local_peer_id_clone]);
                    break;
                }
            }
        });

        let providers = store
            .providers(&mut conn, &result.cid)
            .expect("providers must not error");

        assert_eq!(
            providers.len(),
            1,
            "expected exactly 1 provider (local), got {}: {:?}",
            providers.len(),
            providers.iter().map(|p| &p.peer_id).collect::<Vec<_>>()
        );
        assert_eq!(
            providers[0].peer_id, "local",
            "the single provider must be 'local', not the raw PeerId"
        );
    }

    /// D.4 test 4: Familiar reach + local_agent_cid matches signer (no DB row)
    /// → Ok (self-authored EPR recognized without DB lookup).
    #[tokio::test]
    async fn put_authorizes_familiar_reach_for_local_agent() {
        let pool = setup_pool();
        let (epr, signer_cid) = sample_epr_with_signer_seed(Reach::Familiar, 45);

        // No binding row — but signer IS the local agent.
        let (tx, mut rx) = mpsc::channel::<crate::p2p::P2PCommand>(8);
        let store = FederatedEprStore::new()
            .with_db_pool(pool)
            .with_local_agent_cid(signer_cid.clone())
            .with_swarm_tx(tx);

        let mut conn = setup_conn();
        let result = store
            .put(&mut conn, epr)
            .expect("local agent must be authorized for Familiar reach");
        assert!(!result.cid.is_empty());

        // Announce should be sent (Familiar → Gossip).
        let cmds = drain_commands(&mut rx);
        let has_announce = cmds
            .iter()
            .any(|c| matches!(c, crate::p2p::P2PCommand::PublishEprAnnounce { .. }));
        assert!(
            has_announce,
            "Expected PublishEprAnnounce for local agent at Familiar reach"
        );
    }
}
