//! EprStore trait — the seam between REST routes and storage backends.
//!
//! LocalEprStore wraps the diesel layer (Phase 2a).
//! FederatedEprStore wraps LocalEprStore + a libp2p swarm handle and, on local
//! miss, issues EprRequest::Resolve via /elohim/epr/1.0.0 to discover peers
//! that hold the atom. Phase 2a ships FederatedEprStore with the libp2p bridge
//! stubbed as TODO(phase-2b); Phase 2c wires the swarm handle and flips the
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
// FederatedEprStore — Phase 2a stub; Phase 2c wires libp2p bridge
// ---------------------------------------------------------------------------

/// Federated store that, in Phase 2c, will bridge to the existing
/// /elohim/epr/1.0.0 libp2p protocol and Kad DHT. In Phase 2a it falls through
/// to LocalEprStore with explicit TODO markers for each federation seam.
pub struct FederatedEprStore {
    local: LocalEprStore,
    // TODO(phase-2b): swarm_handle: SwarmHandle — channel into the running
    // elohim-storage swarm so fetch/put can issue EprRequest::Resolve and
    // Kad start_providing. Requires resolving EprHead↔Envelope format
    // compatibility (see Phase 2c pivot doc).
}

impl FederatedEprStore {
    pub fn new() -> Self {
        FederatedEprStore {
            local: LocalEprStore::new(),
        }
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
        let result = self.local.put(conn, epr)?;
        // TODO(phase-2b): self.swarm_handle.kad_start_providing(result.cid.parse()?).await?;
        // This announces to the DHT that this node holds the atom, so future
        // fetch requests from other peers can find us via EprRequest::Resolve.
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
// Factory function — used by route handlers (avoids AppContext field cascade)
// ---------------------------------------------------------------------------

/// Construct the Phase 2a default store. Routes call this; Phase 2c flips the
/// return type from LocalEprStore to a fully-wired FederatedEprStore in one place.
pub fn default_epr_store() -> impl EprStore {
    FederatedEprStore::default()
}
