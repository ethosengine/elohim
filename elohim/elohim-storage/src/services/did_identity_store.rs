//! `did:elohim` assembly store — storage's implementation of the did-bridge
//! [`ElohimIdentityStore`] contract (design spec §3.4).
//!
//! `did:elohim:<agent_cid>` resolution **assembles, never stores** (P1): the DID
//! document is projected per request from substrate joins this crate owns. The
//! did-bridge crate defines the assembly *contract*; this module conforms to it
//! rather than inventing a bespoke shape. [`did_bridge::ElohimResolver`] drives
//! the assembly (verification method from the agent key, `authentication` /
//! `assertionMethod` references, framing services + alsoKnownAs); this store
//! only feeds it the substrate facts:
//!
//! - **`agent_exists`** — resolvable iff the `agent_cid` is this node's own
//!   conductor cell key (self) OR a `humans` projection row exists for it,
//!   scoped to [`HUMANS_HAPP_ID`] (`imagodei`). Unknown → the resolver's
//!   `notFound` path.
//! - **`transport_ids`** (→ `alsoKnownAs`) — SELF only. We hold verified
//!   transport ids (libp2p PeerId / iroh NodeId, `Config::self_cid`) for our own
//!   node. For OTHER agents we have no verified `agent_cid ↔ transport-id`
//!   binding in phase 1 — that is phase-2 witnessed-binding territory — so we
//!   omit rather than invent one.
//! - **`profile_service`** — there IS a per-agent profile surface
//!   (`/api/v1/identity/{agentId}/profile`) but no canonical *absolute* URL
//!   builder in storage today, so this store emits a profile `service` only for
//!   SELF and only when a public base URL is configured: an absolute, resolvable
//!   endpoint or nothing (never a fabricated/relative one).
//! - **`doorway_endpoints`** — empty in phase 1: storage has no per-agent
//!   doorway-registration join, and the doorway's own universal-resolver leg
//!   (distinct write-set) owns its `service` entries.
//! - **`document_metadata`** — the `humans` row's `created`/`updated` stamps when
//!   a row exists; otherwise the default empty (we never manufacture timestamps).
//! - **`identity_head`** (Wave C1 — phase-2) — the newest NOTARIZED `binds-identity`
//!   declaration whose `head_key` is this `agent_cid` (`identity_heads` projection),
//!   read **revocation-inclusive**:
//!   - live declaration → `Declared` (chain-root + controller set). The raw
//!     controller ids are framed as `did:elohim:<id>` controller DIDs here (storage
//!     owns the identity-namespace mapping; the resolver stays namespace-agnostic).
//!     Controllers come straight from the declaration — the community-recovery
//!     quorum is a controller, never an override (ontology guard).
//!   - revoked declaration → `Revoked` (chain-root + head + any named successor),
//!     which the resolver assembles as a **deactivated** document. Reading the
//!     live-only query here would hide the revocation and serve a fully-armed,
//!     implicitly self-controlled document instead — see
//!     `db::identity_heads::find_head_by_head_key`'s warning.
//!   - no row → `NeverDeclared` (the common case today) → the resolver keeps the
//!     phase-1 implicit-self document unchanged. A missing row on a gossip-fed
//!     projection is a *documented limitation*, not a settled claim; the catch-up
//!     signal it needs does not exist for this pipeline yet (ledgered at
//!     `genesis/data/timeline/backlog/identity-head-projection-catchup-signal-gap.md`).
//!
//!   An un-notarized (`dht_anchor_hash IS NULL`) declaration surfaces as neither —
//!   fail-closed, unchanged.
//!
//! Identity-namespace hazard (see `elohim-storage/CLAUDE.md` — "Identity &
//! Transport-Identity Coherence"): the DID method-specific-id and `agent_cid`
//! joins are the Holochain agent key (`uhCAk…`); `Config::self_cid` is a
//! *transport* id (`12D3Koo…` / iroh NodeId). They are NOT interchangeable, so
//! self-detection uses the own conductor cell key (`uhCAk…`), never `self_cid`,
//! and `self_cid` is only ever emitted as an `alsoKnownAs` transport id.

use async_trait::async_trait;
use did_bridge::{
    DidDocumentMetadata, ElohimIdentityStore, ElohimStoreError, IdentityHead, IdentityHeadAnswer,
    RevokedIdentity, ServiceRef,
};
use did_types::Did;

use crate::db::context::HUMANS_HAPP_ID;
use crate::db::models::Human;
use crate::db::{humans, DbPool};

/// Storage-side [`ElohimIdentityStore`] — assembles `did:elohim` documents from
/// the `humans` projection plus this node's own identity seams.
pub struct DidIdentityStore {
    /// Diesel pool for the `humans` projection joins.
    pool: DbPool,
    /// This node's own conductor cell key (`uhCAk…`), if known. Drives the
    /// "own cell key" resolvable branch and marks which `agent_cid` is *self*.
    /// `None` when the conductor bridge is unavailable — resolution then relies
    /// solely on the `humans`-row branch (which, after the genesis self-heal,
    /// already carries the own cell key).
    self_agent_cid: Option<String>,
    /// This node's own transport identifiers (`Config::self_cid` — libp2p PeerId
    /// / iroh NodeId). Emitted as `alsoKnownAs` for SELF resolution only.
    self_transport_ids: Vec<String>,
    /// The node's configured public base URL, if any. Used to build an absolute
    /// profile `service` endpoint for SELF. `None` → no profile service emitted.
    public_base_url: Option<String>,
}

impl DidIdentityStore {
    /// Construct the store over a diesel pool and this node's own identity seams.
    pub fn new(
        pool: DbPool,
        self_agent_cid: Option<String>,
        self_transport_ids: Vec<String>,
        public_base_url: Option<String>,
    ) -> Self {
        DidIdentityStore {
            pool,
            self_agent_cid,
            self_transport_ids,
            public_base_url,
        }
    }

    /// Whether `agent_cid` is this node's own conductor cell key.
    fn is_self(&self, agent_cid: &str) -> bool {
        self.self_agent_cid.as_deref() == Some(agent_cid)
    }

    /// Fetch the `humans` projection row for `agent_cid`, scoped to the identity
    /// pillar (`HUMANS_HAPP_ID`). A row under a different `h_app_id` does not
    /// make the agent resolvable as an imagodei identity.
    fn lookup_human(&self, agent_cid: &str) -> Result<Option<Human>, ElohimStoreError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| ElohimStoreError::Backend(e.to_string()))?;
        let human = humans::get_human_by_agent_key(&mut conn, agent_cid)
            .map_err(|e| ElohimStoreError::Backend(e.to_string()))?;
        Ok(human.filter(|h| h.h_app_id == HUMANS_HAPP_ID))
    }
}

#[async_trait]
impl ElohimIdentityStore for DidIdentityStore {
    async fn agent_exists(&self, agent_cid: &str) -> Result<bool, ElohimStoreError> {
        if self.is_self(agent_cid) {
            return Ok(true);
        }
        Ok(self.lookup_human(agent_cid)?.is_some())
    }

    async fn profile_service(
        &self,
        agent_cid: &str,
    ) -> Result<Option<ServiceRef>, ElohimStoreError> {
        // Honest phase-1: SELF only, and only with a configured public base URL —
        // an absolute resolvable endpoint or nothing.
        if !self.is_self(agent_cid) {
            return Ok(None);
        }
        let Some(base) = self.public_base_url.as_deref() else {
            return Ok(None);
        };
        if self.lookup_human(agent_cid)?.is_none() {
            return Ok(None);
        }
        let base = base.trim_end_matches('/');
        Ok(Some(ServiceRef {
            id_fragment: "profile".to_string(),
            service_type: "ElohimProfile".to_string(),
            endpoint: format!("{base}/api/v1/identity/{agent_cid}/profile"),
        }))
    }

    async fn doorway_endpoints(
        &self,
        _agent_cid: &str,
    ) -> Result<Vec<ServiceRef>, ElohimStoreError> {
        // No per-agent doorway-registration join is wired into storage in phase 1;
        // the doorway's universal-resolver leg (distinct write-set) owns its own
        // `service` entries. Honest empty rather than invented.
        Ok(Vec::new())
    }

    async fn transport_ids(&self, agent_cid: &str) -> Result<Vec<String>, ElohimStoreError> {
        if self.is_self(agent_cid) {
            Ok(self.self_transport_ids.clone())
        } else {
            Ok(Vec::new())
        }
    }

    async fn document_metadata(
        &self,
        agent_cid: &str,
    ) -> Result<DidDocumentMetadata, ElohimStoreError> {
        match self.lookup_human(agent_cid)? {
            Some(h) => Ok(DidDocumentMetadata {
                created: Some(h.created_at),
                updated: Some(h.updated_at),
                // OMITTED, not asserted-false: this store makes no revocation
                // claim here (the revocation-aware answer is `identity_head`'s,
                // which is already fail-closed on revoked rows). `None` is
                // `skip_serializing_if`-elided, so the emitted document is
                // byte-identical to the pre-`deactivated` shape.
                deactivated: None,
            }),
            None => Ok(DidDocumentMetadata::default()),
        }
    }

    async fn identity_head(&self, agent_cid: &str) -> Result<IdentityHeadAnswer, ElohimStoreError> {
        // The newest NOTARIZED `binds-identity` declaration whose head is this agent
        // — revoked or not (`find_notarized_head_by_head_key`). Reading the
        // revocation-INCLUSIVE query is the whole point: the live-head query hides a
        // revoked row behind `revoked_at IS NULL`, which made a revoked identity
        // indistinguishable from one that never declared a head, and therefore
        // resolve to the phase-1 implicit-self document — fully armed and implicitly
        // self-controlled. Still fail-closed on notarization: an un-notarized
        // (storage-only) declaration surfaces neither as a head nor as a revocation.
        let mut conn = self
            .pool
            .get()
            .map_err(|e| ElohimStoreError::Backend(e.to_string()))?;
        let Some(row) =
            crate::db::identity_heads::find_notarized_head_by_head_key(&mut conn, agent_cid)
                .map_err(|e| ElohimStoreError::Backend(e.to_string()))?
        else {
            // MISSING ROW ⇒ `NeverDeclared`, WITH A DOCUMENTED LIMITATION.
            //
            // The trait contract asks for a positive claim here (we looked, and no
            // declaration exists). `identity_heads` is a gossip-fed projection, so
            // strictly a missing row means "no declaration has been DELIVERED TO
            // THIS NODE" — which establishes never-declared only while the
            // projection is caught up, and is `Unresolvable("projection lagging")`
            // while it is not.
            //
            // The ruling is implementable the moment a caught-up signal exists for
            // THIS pipeline. It does not: `identity_heads` is fed by the mishpat
            // `AppSignal` subscriber (`main.rs` → `subscribe_mishpat_signals` →
            // `signals::handle_mishpat_signal`), a fire-and-forget callback with no
            // cursor, no lag stamp and no liveness state. The two catch-up signals
            // that DO exist belong to other pipelines and would answer a different
            // question: `p2p::replication::ReconcileState::caught_up` tracks CONTENT
            // replication, and `projector::status::compute_projector_status` tracks
            // the EPR-atom projector (`epr_atoms` + `projector_cursor`), which does
            // not project this table. Wiring either in would look like the ruling
            // was implemented while leaving the same hole — the invented-wiring
            // failure this enum exists to prevent.
            //
            // So this arm is honest degradation, not a resolved question: it keeps
            // the phase-1 implicit-self document for a missing row, exactly as
            // before, and the gap is ledgered rather than left as a comment —
            // genesis/data/timeline/backlog/identity-head-projection-catchup-signal-gap.md.
            // Closing it is one branch here once the mishpat projection carries a
            // cursor. Note the blast radius is bounded by what actually changed: the
            // *revoked* case (the armed-and-revoked document) is closed by the branch
            // that follows; this is the never-delivered case, whose worst outcome is
            // the unchanged phase-1 document.
            return Ok(IdentityHeadAnswer::NeverDeclared);
        };

        // Revoked ⇒ the head is revoked, and the resolver assembles a DEACTIVATED
        // document (no key material, no relationships, no services, lineage aliases
        // only). C9's two halves ride the row: `chain_root` re-anchors the lineage,
        // and `successor_head_key` re-anchors the continuation when the declaration
        // named one. An ABSENT successor stays honestly `None` — that is a TERMINAL
        // revocation, not an unknown one: we determined it by reading the column.
        // (A store that could not determine it must answer `Unresolvable`; a read
        // failure lands in the `Backend` error above, never here.)
        if let Some(revoked_at) = row.revoked_at.as_deref() {
            tracing::debug!(
                head_key = %agent_cid,
                cid = %row.cid,
                revoked_at = %revoked_at,
                successor = ?row.successor_head_key,
                "did:elohim identity_head: revoked head → deactivated document (confers nothing)"
            );
            return Ok(IdentityHeadAnswer::Revoked(RevokedIdentity {
                chain_root: row.chain_root,
                head: row.head_key,
                successor: row.successor_head_key,
            }));
        }

        // Controllers come straight from the declaration. Frame each raw controller
        // id as a `did:elohim:<id>` controller DID (storage owns the namespace
        // mapping). A controller id that cannot form a valid DID is skipped rather
        // than aborting the whole resolution — the validator already guarantees
        // non-empty ids, so this is a defensive filter, not a lossy transform. On the
        // recovery-quorum path a dropped controller IS data loss, so it is warned
        // (never silent).
        let controller_ids: Vec<String> = serde_json::from_str(&row.controllers_json)
            .map_err(|e| ElohimStoreError::Backend(format!("controllers_json parse: {e}")))?;
        let mut controllers = Vec::with_capacity(controller_ids.len());
        for c in &controller_ids {
            match Did::parse(&format!("did:elohim:{c}")) {
                Ok(did) => controllers.push(did),
                Err(e) => tracing::warn!(
                    controller_id = %c,
                    head_key = %agent_cid,
                    error = %e,
                    "did:elohim identity_head: dropping a controller id that cannot form a valid did:elohim DID"
                ),
            }
        }

        Ok(IdentityHeadAnswer::Declared(IdentityHead {
            chain_root: row.chain_root,
            head: row.head_key,
            controllers,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::humans::CreateHumanInput;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::sqlite::SqliteConnection;

    // A real fleet agent key that passes the did-bridge codec (shared with the
    // crate's own conformance fixtures).
    const AGENT_KEY: &str = "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH";

    /// Shared-cache in-memory pool with the real migrations applied — mirrors
    /// `db::humans::tests::test_pool`.
    fn test_pool() -> DbPool {
        let url = format!(
            "file:did_identity_store_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    /// Insert a `humans` projection row keyed by `agent_pub_key`, scoped to the
    /// identity pillar.
    fn insert_human(pool: &DbPool, id: &str, agent_cid: &str) {
        let mut conn = pool.get().unwrap();
        humans::create_human(
            &mut conn,
            CreateHumanInput {
                id: id.to_string(),
                agent_pub_key: Some(agent_cid.to_string()),
                display_name: id.to_string(),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "commons".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: HUMANS_HAPP_ID.to_string(),
                household_id: None,
            },
        )
        .expect("insert human");
    }

    #[tokio::test]
    async fn agent_exists_true_for_humans_row() {
        let pool = test_pool();
        insert_human(&pool, "human-x", AGENT_KEY);
        let store = DidIdentityStore::new(pool, None, vec![], None);
        assert!(store.agent_exists(AGENT_KEY).await.unwrap());
    }

    #[tokio::test]
    async fn agent_exists_true_for_own_cell_key_without_row() {
        // No humans row — resolvable purely because it is this node's own cell key.
        let pool = test_pool();
        let store = DidIdentityStore::new(pool, Some(AGENT_KEY.to_string()), vec![], None);
        assert!(store.agent_exists(AGENT_KEY).await.unwrap());
    }

    #[tokio::test]
    async fn agent_exists_false_for_unknown() {
        let pool = test_pool();
        let store = DidIdentityStore::new(pool, None, vec![], None);
        assert!(!store.agent_exists(AGENT_KEY).await.unwrap());
    }

    #[tokio::test]
    async fn agent_exists_ignores_other_happ_scope() {
        // A row under a different h_app_id must NOT make the agent resolvable.
        let pool = test_pool();
        {
            let mut conn = pool.get().unwrap();
            humans::create_human(
                &mut conn,
                CreateHumanInput {
                    id: "human-other".to_string(),
                    agent_pub_key: Some(AGENT_KEY.to_string()),
                    display_name: "other".to_string(),
                    bio: None,
                    affinities: "[]".to_string(),
                    profile_reach: "commons".to_string(),
                    location: None,
                    profile_photo_url: None,
                    h_app_id: "some-other-app".to_string(),
                    household_id: None,
                },
            )
            .expect("insert other-scope human");
        }
        let store = DidIdentityStore::new(pool, None, vec![], None);
        assert!(!store.agent_exists(AGENT_KEY).await.unwrap());
    }

    #[tokio::test]
    async fn transport_ids_self_only() {
        let pool = test_pool();
        let store = DidIdentityStore::new(
            pool,
            Some(AGENT_KEY.to_string()),
            vec!["12D3KooWSelfPeerId".to_string()],
            None,
        );
        assert_eq!(
            store.transport_ids(AGENT_KEY).await.unwrap(),
            vec!["12D3KooWSelfPeerId".to_string()]
        );
        // A different agent gets no transport ids (no verified binding in phase 1).
        assert!(store
            .transport_ids("uhCAkOTHERAGENTKEY")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn profile_service_self_with_base_url() {
        let pool = test_pool();
        insert_human(&pool, "human-self", AGENT_KEY);
        let store = DidIdentityStore::new(
            pool,
            Some(AGENT_KEY.to_string()),
            vec![],
            Some("https://node.example.host/".to_string()),
        );
        let svc = store.profile_service(AGENT_KEY).await.unwrap().unwrap();
        assert_eq!(svc.id_fragment, "profile");
        assert_eq!(svc.service_type, "ElohimProfile");
        assert_eq!(
            svc.endpoint,
            format!("https://node.example.host/api/v1/identity/{AGENT_KEY}/profile")
        );
    }

    #[tokio::test]
    async fn profile_service_none_without_base_url() {
        let pool = test_pool();
        insert_human(&pool, "human-self", AGENT_KEY);
        let store = DidIdentityStore::new(pool, Some(AGENT_KEY.to_string()), vec![], None);
        assert!(store.profile_service(AGENT_KEY).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn profile_service_none_for_other_agent() {
        let pool = test_pool();
        insert_human(&pool, "human-other", AGENT_KEY);
        let store = DidIdentityStore::new(
            pool,
            Some("uhCAkSELFDIFFERENT".to_string()),
            vec![],
            Some("https://node.example.host".to_string()),
        );
        assert!(store.profile_service(AGENT_KEY).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn document_metadata_from_humans_row() {
        let pool = test_pool();
        insert_human(&pool, "human-x", AGENT_KEY);
        let store = DidIdentityStore::new(pool, None, vec![], None);
        let meta = store.document_metadata(AGENT_KEY).await.unwrap();
        assert!(meta.created.is_some(), "created stamp from humans row");
        assert!(meta.updated.is_some(), "updated stamp from humans row");
    }

    #[tokio::test]
    async fn document_metadata_default_when_no_row() {
        let pool = test_pool();
        let store = DidIdentityStore::new(pool, Some(AGENT_KEY.to_string()), vec![], None);
        let meta = store.document_metadata(AGENT_KEY).await.unwrap();
        assert_eq!(meta, DidDocumentMetadata::default());
    }

    /// Seed a notarized `binds-identity` head projection keyed on `head_key`.
    fn insert_identity_head(pool: &DbPool, cid: &str, head_key: &str, controllers: &[&str]) {
        insert_identity_head_with_successor(pool, cid, head_key, controllers, None)
    }

    /// As [`insert_identity_head`], plus the successor a rotation/recovery named.
    /// No declaration carries one today (the mishpat validator does not require the
    /// field), so this seeds the column directly to exercise the C9 re-anchor path
    /// ahead of its producer.
    fn insert_identity_head_with_successor(
        pool: &DbPool,
        cid: &str,
        head_key: &str,
        controllers: &[&str],
        successor: Option<&str>,
    ) {
        use crate::db::models::NewIdentityHead;
        let mut conn = pool.get().unwrap();
        let controllers_json = serde_json::to_string(controllers).unwrap();
        crate::db::identity_heads::upsert_with_anchor(
            &mut conn,
            NewIdentityHead {
                cid: cid.to_string(),
                chain_root: "bafyreichainrootgenesis0000".to_string(),
                head_key: head_key.to_string(),
                controllers_json,
                controller_policy_json: r#"{"kind":"recovery-quorum","m":2,"n":3}"#.to_string(),
                signed_at: "2026-07-17T00:00:00Z".to_string(),
                revoked_at: None,
                successor_head_key: successor.map(str::to_string),
                dht_anchor_hash: Some(format!("{cid}-anchor")),
            },
        )
        .expect("insert identity head");
    }

    #[tokio::test]
    async fn identity_head_resolves_controllers_from_binds_identity() {
        // Wave C1: a minted binds-identity head is read back with the right
        // chain_root + controllers (as did:elohim controller DIDs).
        const RECOVERY: &str = "uhCAkRecoveryQuorumKey0000";
        let pool = test_pool();
        insert_identity_head(&pool, "ih:1", AGENT_KEY, &[AGENT_KEY, RECOVERY]);
        let store = DidIdentityStore::new(pool, Some(AGENT_KEY.to_string()), vec![], None);

        let IdentityHeadAnswer::Declared(head) = store.identity_head(AGENT_KEY).await.unwrap()
        else {
            panic!("a live notarized head must resolve as Declared");
        };
        assert_eq!(head.chain_root, "bafyreichainrootgenesis0000");
        assert_eq!(head.head, AGENT_KEY);
        // Controllers framed as did:elohim DIDs, straight from the declaration.
        let expected_self = Did::parse(&format!("did:elohim:{AGENT_KEY}")).unwrap();
        let expected_recovery = Did::parse(&format!("did:elohim:{RECOVERY}")).unwrap();
        assert_eq!(head.controllers.len(), 2);
        assert!(head.controllers.contains(&expected_self));
        assert!(head.controllers.contains(&expected_recovery));
    }

    #[tokio::test]
    async fn identity_head_never_declared_when_no_row() {
        // No binds-identity row for the agent → NeverDeclared (the phase-1 self-only
        // document; the common case today).
        //
        // DOCUMENTED LIMITATION, not a settled claim: `identity_heads` is gossip-fed,
        // so a missing row strictly means "no declaration delivered HERE" — which
        // establishes never-declared only while the projection is caught up. No
        // caught-up signal exists for the mishpat→identity_heads pipeline (see the
        // `identity_head` else-arm), so this stays honest degradation, ledgered at
        // genesis/data/timeline/backlog/identity-head-projection-catchup-signal-gap.md.
        let pool = test_pool();
        let store = DidIdentityStore::new(pool, Some(AGENT_KEY.to_string()), vec![], None);
        assert_eq!(
            store.identity_head(AGENT_KEY).await.unwrap(),
            IdentityHeadAnswer::NeverDeclared
        );
    }

    #[tokio::test]
    async fn identity_head_fail_closed_on_revoked() {
        // THE row-8 assertion. A revoked head must surface as `Revoked` — NOT as
        // `NeverDeclared`.
        //
        // Asserting NeverDeclared here would launder the vulnerability into a green
        // test: an absent `controller` in a DID document means THE SUBJECT CONTROLS
        // IT, so a revoked identity answering NeverDeclared assembles the phase-1
        // implicit-self document — fully armed, implicitly self-controlled — and the
        // revocation disappears into the same silence as never having declared.
        let pool = test_pool();
        insert_identity_head(&pool, "ih:rev", AGENT_KEY, &[AGENT_KEY]);
        {
            let mut conn = pool.get().unwrap();
            crate::db::identity_heads::set_revoked_at(&mut conn, "ih:rev", "2026-07-17T03:00:00Z")
                .expect("revoke");
        }
        let store = DidIdentityStore::new(pool, Some(AGENT_KEY.to_string()), vec![], None);

        assert_eq!(
            store.identity_head(AGENT_KEY).await.unwrap(),
            IdentityHeadAnswer::Revoked(RevokedIdentity {
                chain_root: "bafyreichainrootgenesis0000".to_string(),
                head: AGENT_KEY.to_string(),
                // Honest absence: this declaration named no successor, so the
                // revocation is TERMINAL. `None` here is a determination (we read
                // the column), never "unknown".
                successor: None,
            }),
            "a revoked head must answer Revoked; NeverDeclared would serve a \
             fully-armed implicitly self-controlled document"
        );
    }

    #[tokio::test]
    async fn identity_head_revoked_carries_the_named_successor() {
        // C9's re-anchor half: when the declaration names a continuation, a reference
        // keyed on the revoked head can follow the identity forward instead of
        // dead-ending. (No producer declares one today — the column is seeded
        // directly; see `insert_identity_head_with_successor`.)
        const SUCCESSOR: &str = "uhCAkSuccessorHeadKey0000";
        let pool = test_pool();
        insert_identity_head_with_successor(
            &pool,
            "ih:rotated",
            AGENT_KEY,
            &[AGENT_KEY],
            Some(SUCCESSOR),
        );
        {
            let mut conn = pool.get().unwrap();
            crate::db::identity_heads::set_revoked_at(
                &mut conn,
                "ih:rotated",
                "2026-07-17T03:00:00Z",
            )
            .expect("revoke");
        }
        let store = DidIdentityStore::new(pool, Some(AGENT_KEY.to_string()), vec![], None);

        let IdentityHeadAnswer::Revoked(revoked) = store.identity_head(AGENT_KEY).await.unwrap()
        else {
            panic!("a revoked head must answer Revoked");
        };
        assert_eq!(revoked.successor.as_deref(), Some(SUCCESSOR));
    }

    #[tokio::test]
    async fn identity_head_un_notarized_revoked_row_does_not_surface() {
        // Only the revoked filter was relaxed — notarization stays fail-closed. An
        // un-notarized (storage-only) declaration must surface as neither a head nor
        // a revocation, so an unauthenticated row cannot deactivate a live identity.
        use crate::db::models::NewIdentityHead;
        let pool = test_pool();
        {
            let mut conn = pool.get().unwrap();
            crate::db::identity_heads::upsert_with_anchor(
                &mut conn,
                NewIdentityHead {
                    cid: "ih:unanchored".to_string(),
                    chain_root: "bafyreichainrootgenesis0000".to_string(),
                    head_key: AGENT_KEY.to_string(),
                    controllers_json: format!(r#"["{AGENT_KEY}"]"#),
                    controller_policy_json: r#"{"kind":"self"}"#.to_string(),
                    signed_at: "2026-07-17T00:00:00Z".to_string(),
                    revoked_at: Some("2026-07-17T03:00:00Z".to_string()),
                    successor_head_key: None,
                    dht_anchor_hash: None,
                },
            )
            .expect("insert un-notarized revoked head");
        }
        let store = DidIdentityStore::new(pool, Some(AGENT_KEY.to_string()), vec![], None);
        assert_eq!(
            store.identity_head(AGENT_KEY).await.unwrap(),
            IdentityHeadAnswer::NeverDeclared,
            "an un-notarized declaration establishes nothing in either direction"
        );
    }

    /// End-to-end over the REAL resolver: the row-8 vulnerability, closed at the
    /// surface a caller actually sees. Before the fix this assembled an ordinary
    /// `did:elohim` document with verification methods, services and transport ids —
    /// a fully-armed, implicitly self-controlled identity whose head was revoked.
    #[tokio::test]
    async fn revoked_head_resolves_to_a_deactivated_document_that_confers_nothing() {
        use did_bridge::{DidResolver, ElohimResolver};
        use did_types::Did;

        let pool = test_pool();
        insert_human(&pool, "human-self", AGENT_KEY);
        insert_identity_head(&pool, "ih:rev", AGENT_KEY, &[AGENT_KEY]);
        {
            let mut conn = pool.get().unwrap();
            crate::db::identity_heads::set_revoked_at(&mut conn, "ih:rev", "2026-07-17T03:00:00Z")
                .expect("revoke");
        }
        let store = DidIdentityStore::new(
            pool,
            Some(AGENT_KEY.to_string()),
            vec!["12D3KooWSelfPeerId".to_string()],
            Some("https://node.example.host".to_string()),
        );
        let resolver = ElohimResolver::new(store);
        let did = Did::parse(&format!("did:elohim:{AGENT_KEY}")).unwrap();

        // Deactivation is a SUCCESSFUL resolution (DID Core), not a notFound: the DID
        // exists and its state is knowable.
        let result = resolver.resolve(&did).await.expect("deactivation resolves");
        assert_eq!(
            result.did_document_metadata.deactivated,
            Some(true),
            "the standards-registered deactivation signal must be set"
        );
        let doc = result.did_document.expect("a document, not a 404");

        // Nothing to act with — belt and braces, for a consumer that ignores metadata.
        assert!(doc.verification_method.is_none(), "no key material");
        assert!(doc.authentication.is_none(), "no authentication");
        assert!(doc.assertion_method.is_none(), "no assertionMethod");
        assert!(doc.capability_invocation.is_none());
        assert!(doc.capability_delegation.is_none());
        assert!(doc.service.is_none(), "no endpoints projected");
        assert!(
            doc.controller.is_none(),
            "no controller set is asserted for a revoked head"
        );

        // Lineage stays followable (C9): the chain-root alias, and NOT the live
        // transport ids — a revoked identity's transports must not be advertised.
        let aka = doc.also_known_as.expect("chain-root lineage alias");
        assert!(aka.contains(&"did:elohim:bafyreichainrootgenesis0000".to_string()));
        assert!(
            !aka.iter().any(|a| a.contains("12D3KooW")),
            "a revoked identity's transport ids must not be projected: {aka:?}"
        );
    }

    /// The load-bearing contract test: a fully-assembled `did:elohim` document
    /// (over the real store) validates against the did-bridge W3C DID 1.1 schema.
    #[tokio::test]
    async fn assembled_document_validates_against_did11_schema() {
        use did_bridge::{DidResolver, ElohimResolver};
        use did_types::Did;
        use serde_json::Value;

        const SCHEMA: &str =
            include_str!("../../../../bridges/did/schemas/did-document-1.1.schema.json");

        let pool = test_pool();
        insert_human(&pool, "human-self", AGENT_KEY);
        let store = DidIdentityStore::new(
            pool,
            Some(AGENT_KEY.to_string()),
            vec!["12D3KooWSelfPeerId".to_string()],
            Some("https://node.example.host".to_string()),
        );
        let resolver = ElohimResolver::new(store);
        let did = Did::parse(&format!("did:elohim:{AGENT_KEY}")).unwrap();

        let result = resolver.resolve(&did).await.expect("resolution succeeds");
        let doc = result.did_document.expect("document present");

        // Sanity: the populated seams we assemble are actually present.
        assert!(doc.verification_method.is_some(), "verification method");
        assert!(doc.also_known_as.is_some(), "alsoKnownAs transport ids");
        assert!(doc.service.is_some(), "profile service");

        // Conformance: validate against the hand-derived W3C DID 1.1 schema.
        let schema: Value = serde_json::from_str(SCHEMA).expect("schema is valid JSON");
        let validator = jsonschema::validator_for(&schema).expect("schema compiles");
        let instance = serde_json::to_value(&doc).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| format!("{e} (at {})", e.instance_path))
            .collect();
        assert!(
            errors.is_empty(),
            "assembled did:elohim document violates DID 1.1 schema:\n  - {}",
            errors.join("\n  - ")
        );

        // Metadata rides through from the humans row.
        assert!(result.did_document_metadata.created.is_some());
    }

    /// Wave C1 contract test: a `did:elohim` document assembled over the real store
    /// WITH a notarized `binds-identity` head has its `controller` populated from the
    /// declaration and still validates against the W3C DID 1.1 schema.
    #[tokio::test]
    async fn assembled_document_with_head_populates_controllers_and_conforms() {
        use did_bridge::{DidResolver, ElohimResolver};
        use did_types::{Controller, Did};
        use serde_json::Value;

        const SCHEMA: &str =
            include_str!("../../../../bridges/did/schemas/did-document-1.1.schema.json");
        const RECOVERY: &str = "uhCAkRecoveryQuorumKey0000";

        let pool = test_pool();
        insert_human(&pool, "human-self", AGENT_KEY);
        insert_identity_head(&pool, "ih:conf", AGENT_KEY, &[AGENT_KEY, RECOVERY]);
        let store = DidIdentityStore::new(
            pool,
            Some(AGENT_KEY.to_string()),
            vec!["12D3KooWSelfPeerId".to_string()],
            Some("https://node.example.host".to_string()),
        );
        let resolver = ElohimResolver::new(store);
        let did = Did::parse(&format!("did:elohim:{AGENT_KEY}")).unwrap();

        let doc = resolver
            .resolve(&did)
            .await
            .expect("resolution succeeds")
            .did_document
            .expect("document present");

        // controller populated from the declared set (self + recovery quorum).
        match doc
            .controller
            .as_ref()
            .expect("controller populated from head")
        {
            Controller::Many(cs) => assert_eq!(cs.len(), 2),
            other => panic!("expected Controller::Many, got {other:?}"),
        }
        // Lineage: the chain-root surfaces as an alsoKnownAs alias next to transport ids.
        let aka = doc.also_known_as.as_ref().unwrap();
        assert!(
            aka.iter()
                .any(|a| a == "did:elohim:bafyreichainrootgenesis0000"),
            "chain-root lineage alias present: {aka:?}"
        );

        // Conformance: the with-controllers document still validates against DID 1.1.
        let schema: Value = serde_json::from_str(SCHEMA).expect("schema is valid JSON");
        let validator = jsonschema::validator_for(&schema).expect("schema compiles");
        let instance = serde_json::to_value(&doc).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| format!("{e} (at {})", e.instance_path))
            .collect();
        assert!(
            errors.is_empty(),
            "with-controllers did:elohim document violates DID 1.1 schema:\n  - {}",
            errors.join("\n  - ")
        );
    }
}
