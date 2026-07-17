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
//!
//! Identity-namespace hazard (see `elohim-storage/CLAUDE.md` — "Identity &
//! Transport-Identity Coherence"): the DID method-specific-id and `agent_cid`
//! joins are the Holochain agent key (`uhCAk…`); `Config::self_cid` is a
//! *transport* id (`12D3Koo…` / iroh NodeId). They are NOT interchangeable, so
//! self-detection uses the own conductor cell key (`uhCAk…`), never `self_cid`,
//! and `self_cid` is only ever emitted as an `alsoKnownAs` transport id.

use async_trait::async_trait;
use did_bridge::{DidDocumentMetadata, ElohimIdentityStore, ElohimStoreError, ServiceRef};

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
            }),
            None => Ok(DidDocumentMetadata::default()),
        }
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
}
