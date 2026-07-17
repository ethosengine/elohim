//! The `did:elohim:<agent_cid>` method — self-certifying, projection-assembled.
//!
//! `did:elohim` names an agent by its `AgentPubKey` content id (`uhCAk…`).
//! Resolution **assembles, never stores** (P1): the DID document is projected
//! per request from substrate joins the identity store owns. This crate defines
//! the assembly *contract* — [`ElohimIdentityStore`] — so `elohim-storage`
//! conforms rather than invents. Assembly here (`ElohimResolver`) is the
//! executable spec of what that store must feed.
//!
//! Per spec §3.4 the assembled document carries:
//! - a Multikey verification method derived from the agent key,
//! - `authentication` + `assertionMethod` referencing it,
//! - transport ids (libp2p PeerId, iroh NodeId) as `alsoKnownAs`,
//! - profile + doorway `service` entries.
//!
//! Controller is left implicit (subject controls) in phase 1. The phase-2
//! identity head adds explicit self + community-recovery-quorum controllers
//! (DID Group Control) — see the spec's named follow-ons.

use async_trait::async_trait;
use did_types::{
    Context, Did, DidDocument, Service, ServiceEndpoint, VerificationMethod,
    VerificationRelationship,
};
use thiserror::Error;

use crate::codec::{agent_cid_to_core32, core32_to_multikey};
use crate::resolver::{DidResolutionError, DidResolutionResult, DidResolver};

/// A service the identity store reports for an agent, before the resolver frames
/// it with the subject DID as `id`. `id_fragment` becomes `#<fragment>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRef {
    /// The fragment appended to the subject DID to form the service `id`.
    pub id_fragment: String,
    /// The service `type`.
    pub service_type: String,
    /// The service endpoint URI.
    pub endpoint: String,
}

/// Errors an identity store may raise.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ElohimStoreError {
    /// An I/O or backend error occurred while reading substrate state.
    #[error("identity store backend error: {0}")]
    Backend(String),
}

/// The assembly contract `did:elohim` resolution requires. Implemented by
/// `elohim-storage` (which owns the agent-key / humans / doorway joins).
///
/// All methods are keyed by the raw `agent_cid` (`uhCAk…`) — the
/// method-specific-id of the DID.
#[async_trait]
pub trait ElohimIdentityStore: Send + Sync {
    /// Whether this agent key exists in the substrate. Drives `notFound`.
    async fn agent_exists(&self, agent_cid: &str) -> Result<bool, ElohimStoreError>;

    /// The profile service for this agent, if the humans projection has one.
    async fn profile_service(
        &self,
        agent_cid: &str,
    ) -> Result<Option<ServiceRef>, ElohimStoreError>;

    /// Doorway service endpoints projecting this agent.
    async fn doorway_endpoints(&self, agent_cid: &str)
        -> Result<Vec<ServiceRef>, ElohimStoreError>;

    /// Transport identifiers for this agent (libp2p PeerId, iroh NodeId) — placed
    /// in `alsoKnownAs` so one resolution surface names all of an agent's ids.
    async fn transport_ids(&self, agent_cid: &str) -> Result<Vec<String>, ElohimStoreError>;
}

/// Resolver for the `did:elohim` method, assembling documents from a store.
pub struct ElohimResolver<S: ElohimIdentityStore> {
    store: S,
}

impl<S: ElohimIdentityStore> ElohimResolver<S> {
    /// Construct the resolver over an identity store.
    pub fn new(store: S) -> Self {
        ElohimResolver { store }
    }
}

#[async_trait]
impl<S: ElohimIdentityStore> DidResolver for ElohimResolver<S> {
    fn method(&self) -> &'static str {
        "elohim"
    }

    async fn resolve(&self, did: &Did) -> Result<DidResolutionResult, DidResolutionError> {
        if did.method() != "elohim" {
            return Err(DidResolutionError::MethodNotSupported(
                did.method().to_string(),
            ));
        }
        let agent_cid = did.method_specific_id();

        // The method-specific-id must be a valid agent_cid.
        let core = agent_cid_to_core32(agent_cid)
            .map_err(|e| DidResolutionError::InvalidDid(e.to_string()))?;

        // Existence gate → notFound.
        let exists = self
            .store
            .agent_exists(agent_cid)
            .await
            .map_err(|e| DidResolutionError::Internal(e.to_string()))?;
        if !exists {
            return Err(DidResolutionError::NotFound(did.as_string()));
        }

        // Verification method from the agent key (Multikey).
        let multikey = core32_to_multikey(&core);
        let vm_id = did.with_fragment(&multikey);
        let vm = VerificationMethod::multikey(vm_id.clone(), did.clone(), multikey);

        let mut doc = DidDocument::new(Context::did_v1_1_multikey(), did.clone());
        doc.verification_method = Some(vec![vm]);
        // Per §3.4: authentication + assertionMethod reference the agent key.
        doc.authentication = Some(vec![VerificationRelationship::Reference(vm_id.clone())]);
        doc.assertion_method = Some(vec![VerificationRelationship::Reference(vm_id)]);

        // Transport ids → alsoKnownAs.
        let transport_ids = self
            .store
            .transport_ids(agent_cid)
            .await
            .map_err(|e| DidResolutionError::Internal(e.to_string()))?;
        if !transport_ids.is_empty() {
            doc.also_known_as = Some(transport_ids);
        }

        // Services: profile (optional) + doorway endpoints.
        let mut services: Vec<Service> = Vec::new();
        if let Some(profile) = self
            .store
            .profile_service(agent_cid)
            .await
            .map_err(|e| DidResolutionError::Internal(e.to_string()))?
        {
            services.push(frame_service(did, profile));
        }
        for endpoint in self
            .store
            .doorway_endpoints(agent_cid)
            .await
            .map_err(|e| DidResolutionError::Internal(e.to_string()))?
        {
            services.push(frame_service(did, endpoint));
        }
        if !services.is_empty() {
            doc.service = Some(services);
        }

        Ok(DidResolutionResult::success(doc))
    }
}

/// Frame a store `ServiceRef` into a DID `Service` anchored on the subject DID.
fn frame_service(did: &Did, svc: ServiceRef) -> Service {
    Service {
        id: did.with_fragment(&svc.id_fragment),
        type_: svc.service_type,
        service_endpoint: ServiceEndpoint::Uri(svc.endpoint),
    }
}
