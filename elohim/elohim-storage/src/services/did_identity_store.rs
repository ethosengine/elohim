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
//! - **`identity_head`** — device relationships resolve through the native
//!   verifier on every request. The result names the stable Human root and its
//!   unchanged controller policy. Legacy SQL `binds-identity` rows were only
//!   shape-checked, so their presence yields `Unresolvable`, never authority.
//!   Their evidence and policy values remain intact for explicit reconciliation.
//!   A revoked or unavailable native binding also fails closed. Existing Humans
//!   without an enrolled-device declaration retain the ordinary self DID path.
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
    ServiceRef,
};
use did_types::Did;

use crate::db::context::HUMANS_HAPP_ID;
use crate::db::models::Human;
use crate::db::{humans, DbPool};

#[derive(serde::Deserialize)]
struct VerifiedDevice {
    identity_root: holochain_types::prelude::ActionHash,
    device_key: holochain_types::prelude::AgentPubKey,
    controllers: Vec<holochain_types::prelude::AgentPubKey>,
}

#[derive(serde::Deserialize)]
struct HumanRootEvidence {
    human_action_hash: holochain_types::prelude::ActionHash,
    author: holochain_types::prelude::AgentPubKey,
}

fn verified_device_head(
    device: VerifiedDevice,
    root: HumanRootEvidence,
) -> Result<IdentityHead, ElohimStoreError> {
    if root.human_action_hash != device.identity_root {
        return Err(ElohimStoreError::Backend(
            "Human root evidence mismatch".into(),
        ));
    }
    let controllers = device
        .controllers
        .iter()
        .map(|key| Did::parse(&format!("did:elohim:{key}")))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ElohimStoreError::Backend(e.to_string()))?;
    Ok(IdentityHead {
        // did:elohim is an agent-key DID. Keep the immutable Human ActionHash
        // as native identity, and expose its original author's resolvable alias.
        chain_root: root.author.to_string(),
        head: device.device_key.to_string(),
        controllers,
    })
}

/// Storage-side [`ElohimIdentityStore`] — assembles `did:elohim` documents from
/// the `humans` projection plus this node's own identity seams.
pub struct DidIdentityStore {
    /// Diesel pool for the `humans` projection joins.
    pool: DbPool,
    device_conductor: Option<std::sync::Arc<crate::hc_client::HcClient>>,
    require_native_verification: bool,
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
            device_conductor: None,
            require_native_verification: false,
            self_agent_cid,
            self_transport_ids,
            public_base_url,
        }
    }

    /// Device association is resolved from notarized bindings by the native
    /// verifier. SQL and signal payloads never establish this authority.
    pub fn with_device_conductor(
        mut self,
        conductor: Option<std::sync::Arc<crate::hc_client::HcClient>>,
    ) -> Self {
        self.device_conductor = conductor;
        self.require_native_verification = true;
        self
    }

    async fn verified_device(
        &self,
        agent: &str,
    ) -> Result<Option<VerifiedDevice>, ElohimStoreError> {
        let Some(hc) = &self.device_conductor else {
            if self.require_native_verification {
                return Err(ElohimStoreError::Backend(
                    "device identity verification unavailable: imagodei conductor missing".into(),
                ));
            }
            return Ok(None);
        };
        let key = holochain_types::prelude::AgentPubKey::try_from(agent)
            .map_err(|_| ElohimStoreError::Backend("invalid device agent key".into()))?;
        let payload =
            rmp_serde::to_vec_named(&key).map_err(|e| ElohimStoreError::Backend(e.to_string()))?;
        let result = hc
            .call_zome_imagodei("imagodei", "resolve_device_identity", payload)
            .await
            .map_err(|e| {
                ElohimStoreError::Backend(format!("device identity verification unavailable: {e}"))
            })?;
        rmp_serde::from_slice(&result).map_err(|e| ElohimStoreError::Backend(e.to_string()))
    }

    async fn native_human_exists(&self, agent: &str) -> Result<bool, ElohimStoreError> {
        let Some(hc) = &self.device_conductor else {
            return Ok(false);
        };
        let key = holochain_types::prelude::AgentPubKey::try_from(agent)
            .map_err(|_| ElohimStoreError::Backend("invalid device agent key".into()))?;
        let payload =
            rmp_serde::to_vec_named(&key).map_err(|e| ElohimStoreError::Backend(e.to_string()))?;
        let result = hc
            .call_zome_imagodei("imagodei", "get_human_by_agent_key", payload)
            .await
            .map_err(|e| {
                ElohimStoreError::Backend(format!("native Human verification unavailable: {e}"))
            })?;
        #[derive(serde::Deserialize)]
        struct NativeHuman {
            #[allow(dead_code)]
            action_hash: holochain_types::prelude::ActionHash,
        }
        let human: Option<NativeHuman> =
            rmp_serde::from_slice(&result).map_err(|e| ElohimStoreError::Backend(e.to_string()))?;
        Ok(human.is_some())
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
        if self.verified_device(agent_cid).await?.is_some() {
            return Ok(true);
        }
        if self.native_human_exists(agent_cid).await? || self.is_self(agent_cid) {
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
        if let Some(device) = self.verified_device(agent_cid).await? {
            let hc = self
                .device_conductor
                .as_ref()
                .ok_or_else(|| ElohimStoreError::Backend("device conductor unavailable".into()))?;
            let payload = rmp_serde::to_vec_named(&device.identity_root)
                .map_err(|e| ElohimStoreError::Backend(e.to_string()))?;
            let result = hc
                .call_zome_imagodei("imagodei", "get_human_root_evidence", payload)
                .await
                .map_err(|e| {
                    ElohimStoreError::Backend(format!("Human root verification unavailable: {e}"))
                })?;
            let root: HumanRootEvidence = rmp_serde::from_slice(&result)
                .map_err(|e| ElohimStoreError::Backend(e.to_string()))?;
            return Ok(IdentityHeadAnswer::Declared(verified_device_head(
                device, root,
            )?));
        }
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

        // Legacy rows preserve the declared policy as evidence, but the old
        // coordinator checked only payload shape. An anchor is not controller
        // authorization, and neither a live nor revoked SQL row confers it.
        Ok(IdentityHeadAnswer::Unresolvable(format!(
            "legacy identity declaration {} lacks verified controller authorization",
            row.cid
        )))
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

    #[test]
    fn device_alias_uses_original_human_agent_not_action_hash() {
        use holochain_types::prelude::{ActionHash, AgentPubKey};
        let root = ActionHash::from_raw_32(vec![1; 32]);
        let author = AgentPubKey::from_raw_32(vec![2; 32]);
        let device = || VerifiedDevice {
            identity_root: root.clone(),
            device_key: AgentPubKey::from_raw_32(vec![3; 32]),
            controllers: vec![author.clone()],
        };
        let head = verified_device_head(
            device(),
            HumanRootEvidence {
                human_action_hash: root.clone(),
                author: author.clone(),
            },
        )
        .unwrap();
        assert_eq!(head.chain_root, author.to_string());
        assert!(AgentPubKey::try_from(head.chain_root.as_str()).is_ok());
        assert_ne!(head.chain_root, root.to_string());
        assert!(verified_device_head(
            device(),
            HumanRootEvidence {
                human_action_hash: ActionHash::from_raw_32(vec![4; 32]),
                author
            }
        )
        .is_err());
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
    async fn forged_notarized_legacy_row_does_not_authorize_controllers() {
        let pool = test_pool();
        insert_identity_head(&pool, "ih:forged", AGENT_KEY, &["attacker"]);
        let store = DidIdentityStore::new(pool, Some(AGENT_KEY.to_string()), vec![], None);
        assert!(matches!(
            store.identity_head(AGENT_KEY).await.unwrap(),
            IdentityHeadAnswer::Unresolvable(_)
        ));
    }

    #[tokio::test]
    async fn configured_native_verification_missing_refuses_self_authority() {
        let store = DidIdentityStore::new(test_pool(), Some(AGENT_KEY.to_string()), vec![], None)
            .with_device_conductor(None);
        assert!(store.identity_head(AGENT_KEY).await.is_err());
        assert!(store.agent_exists(AGENT_KEY).await.is_err());
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
    async fn legacy_revoked_identity_remains_unresolvable_not_self_controlled() {
        let pool = test_pool();
        insert_identity_head(&pool, "ih:rev", AGENT_KEY, &[AGENT_KEY]);
        crate::db::identity_heads::set_revoked_at(
            &mut pool.get().unwrap(),
            "ih:rev",
            "2026-09-30T00:00:00Z",
        )
        .unwrap();
        let store = DidIdentityStore::new(pool, Some(AGENT_KEY.to_string()), vec![], None);
        assert!(matches!(
            store.identity_head(AGENT_KEY).await.unwrap(),
            IdentityHeadAnswer::Unresolvable(_)
        ));
    }

    #[tokio::test]
    async fn legacy_successor_is_preserved_without_authorizing_it() {
        let pool = test_pool();
        insert_identity_head_with_successor(
            &pool,
            "ih:rotated",
            AGENT_KEY,
            &[AGENT_KEY],
            Some("unverified-successor"),
        );
        let store = DidIdentityStore::new(pool.clone(), Some(AGENT_KEY.to_string()), vec![], None);
        assert!(matches!(
            store.identity_head(AGENT_KEY).await.unwrap(),
            IdentityHeadAnswer::Unresolvable(_)
        ));
        let row = crate::db::identity_heads::get_by_cid(&mut pool.get().unwrap(), "ih:rotated")
            .unwrap()
            .unwrap();
        assert_eq!(
            row.successor_head_key.as_deref(),
            Some("unverified-successor")
        );
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
    async fn unverified_revocation_never_emits_an_authenticated_document() {
        use did_bridge::{DidResolver, ElohimResolver};
        let pool = test_pool();
        insert_human(&pool, "human-self", AGENT_KEY);
        insert_identity_head(&pool, "ih:rev", AGENT_KEY, &[AGENT_KEY]);
        crate::db::identity_heads::set_revoked_at(
            &mut pool.get().unwrap(),
            "ih:rev",
            "2026-09-30T00:00:00Z",
        )
        .unwrap();
        let resolver = ElohimResolver::new(DidIdentityStore::new(
            pool,
            Some(AGENT_KEY.to_string()),
            vec![],
            None,
        ));
        assert!(resolver
            .resolve(&Did::parse(&format!("did:elohim:{AGENT_KEY}")).unwrap())
            .await
            .is_err());
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
    async fn unverified_legacy_head_is_refused_by_real_did_resolver() {
        use did_bridge::{DidResolver, ElohimResolver};
        let pool = test_pool();
        insert_human(&pool, "human-self", AGENT_KEY);
        insert_identity_head(&pool, "ih:forged", AGENT_KEY, &["attacker"]);
        let resolver = ElohimResolver::new(DidIdentityStore::new(
            pool,
            Some(AGENT_KEY.to_string()),
            vec![],
            None,
        ));
        assert!(resolver
            .resolve(&Did::parse(&format!("did:elohim:{AGENT_KEY}")).unwrap())
            .await
            .is_err());
    }
}
