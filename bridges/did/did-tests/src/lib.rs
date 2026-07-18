//! Shared test fixtures for the DID bridge integration tests.
//!
//! A `MockElohimStore` satisfies the `ElohimIdentityStore` assembly contract so
//! `did:elohim` resolution can be exercised without `elohim-storage`, and a
//! `FixtureFetch` satisfies `DidWebFetch` so `did:web` needs no network.

use std::collections::HashMap;

use async_trait::async_trait;
use did_bridge::{
    DidResolutionError, DidWebFetch, ElohimIdentityStore, ElohimStoreError, IdentityHead,
    ServiceRef,
};
use did_types::Did;

/// An in-memory identity store. Configure which agents exist and what they
/// project, then hand it to an `ElohimResolver`.
#[derive(Default, Clone)]
pub struct MockElohimStore {
    pub agents: std::collections::HashSet<String>,
    pub profiles: HashMap<String, ServiceRef>,
    pub doorways: HashMap<String, Vec<ServiceRef>>,
    pub transports: HashMap<String, Vec<String>>,
    /// Wave-B `binds-identity` heads, keyed by the head agent_cid. Absent ⇒ the
    /// phase-1 implicit-self assembly (no `controller`, no lineage alias).
    pub heads: HashMap<String, IdentityHead>,
}

impl MockElohimStore {
    /// A store where the given agent exists with a profile, one doorway, and two
    /// transport ids — the fully-populated assembly path.
    pub fn populated(agent_cid: &str) -> Self {
        let mut s = MockElohimStore::default();
        s.agents.insert(agent_cid.to_string());
        s.profiles.insert(
            agent_cid.to_string(),
            ServiceRef {
                id_fragment: "profile".to_string(),
                service_type: "ProfileService".to_string(),
                endpoint: format!("https://doorway.elohim.host/profile/{agent_cid}"),
            },
        );
        s.doorways.insert(
            agent_cid.to_string(),
            vec![ServiceRef {
                id_fragment: "doorway".to_string(),
                service_type: "DoorwayService".to_string(),
                endpoint: "https://doorway.elohim.host".to_string(),
            }],
        );
        s.transports.insert(
            agent_cid.to_string(),
            vec![
                "12D3KooWABCDEexamplePeerId".to_string(),
                "iroh:nodeidexample0000".to_string(),
            ],
        );
        s
    }

    /// Declare a Wave-B identity head for `agent_cid`: a stable `chain_root` and a
    /// controller set (each raw controller id is framed as a `did:elohim:<id>`
    /// controller DID). Builder — chains onto `populated`/`default`.
    pub fn with_head(mut self, agent_cid: &str, chain_root: &str, controller_ids: &[&str]) -> Self {
        let controllers = controller_ids
            .iter()
            .map(|c| Did::parse(&format!("did:elohim:{c}")).expect("controller id → did:elohim"))
            .collect();
        self.heads.insert(
            agent_cid.to_string(),
            IdentityHead {
                chain_root: chain_root.to_string(),
                head: agent_cid.to_string(),
                controllers,
            },
        );
        self
    }
}

#[async_trait]
impl ElohimIdentityStore for MockElohimStore {
    async fn agent_exists(&self, agent_cid: &str) -> Result<bool, ElohimStoreError> {
        Ok(self.agents.contains(agent_cid))
    }

    async fn profile_service(
        &self,
        agent_cid: &str,
    ) -> Result<Option<ServiceRef>, ElohimStoreError> {
        Ok(self.profiles.get(agent_cid).cloned())
    }

    async fn doorway_endpoints(
        &self,
        agent_cid: &str,
    ) -> Result<Vec<ServiceRef>, ElohimStoreError> {
        Ok(self.doorways.get(agent_cid).cloned().unwrap_or_default())
    }

    async fn transport_ids(&self, agent_cid: &str) -> Result<Vec<String>, ElohimStoreError> {
        Ok(self.transports.get(agent_cid).cloned().unwrap_or_default())
    }

    async fn identity_head(
        &self,
        agent_cid: &str,
    ) -> Result<Option<IdentityHead>, ElohimStoreError> {
        Ok(self.heads.get(agent_cid).cloned())
    }
}

/// An in-memory `did:web` fetch: maps derived URLs to raw did.json bytes.
#[derive(Default, Clone)]
pub struct FixtureFetch {
    pub docs: HashMap<String, Vec<u8>>,
}

impl FixtureFetch {
    /// Register a document body for a URL.
    pub fn with_doc(mut self, url: &str, body: &str) -> Self {
        self.docs.insert(url.to_string(), body.as_bytes().to_vec());
        self
    }
}

#[async_trait]
impl DidWebFetch for FixtureFetch {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, DidResolutionError> {
        self.docs
            .get(url)
            .cloned()
            .ok_or_else(|| DidResolutionError::NotFound(url.to_string()))
    }
}
