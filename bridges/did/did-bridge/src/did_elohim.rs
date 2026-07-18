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
//! Controller was left implicit (subject controls) in phase 1. The **phase-2
//! identity head** (implemented here) resolves the Wave-B `binds-identity`
//! declaration via [`ElohimIdentityStore::identity_head`] and, when a head is
//! declared, populates explicit `controller` entries (self / steward-set /
//! community-recovery quorum — DID 1.1 Group Control) plus a chain-root lineage
//! `alsoKnownAs` alias. With NO head declared the store returns `None` and the
//! assembled document is byte-unchanged from the phase-1 implicit-self document.

use async_trait::async_trait;
use did_types::{
    Context, Controller, Did, DidDocument, Service, ServiceEndpoint, VerificationMethod,
    VerificationRelationship,
};
use thiserror::Error;

use crate::codec::{agent_cid_to_core32, core32_to_multikey};
use crate::resolver::{DidDocumentMetadata, DidResolutionError, DidResolutionResult, DidResolver};

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

/// The identity-head facts for an agent, resolved from the Wave-B `binds-identity`
/// declaration (design §3.4/§4): the stable chain-root, the current head key, and
/// the controller set. Feeds the phase-2 `did:elohim` assembly — real `controller`
/// entries + a chain-root lineage alias — replacing the phase-1 implicit-self
/// document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityHead {
    /// The stable chain-root identifier (imagodei genesis-key / CID) — the
    /// identity's durable id, unchanged across every rotation/recovery. Reflected
    /// as a lineage `alsoKnownAs` alias in the assembled document.
    pub chain_root: String,
    /// The current head key of the chain (typically the resolved `agent_cid`).
    pub head: String,
    /// The controller DIDs from the declaration — the `controller` set (self,
    /// steward-set, or community-recovery quorum). Non-empty by the ontology guard
    /// (a head cannot exist without its controllers); the recovery quorum is a
    /// controller, never an override. Populates the document `controller` field
    /// verbatim (straight from the declaration — no editorializing).
    pub controllers: Vec<Did>,
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

    /// Document metadata (`created` / `updated`) for the DID document.
    ///
    /// Defaults to empty so a store can begin supplying timestamps later —
    /// e.g. once the phase-2 identity head records them — **without** a breaking
    /// change to this trait or its existing implementors. Phase-1 stores need
    /// not implement it.
    async fn document_metadata(
        &self,
        _agent_cid: &str,
    ) -> Result<DidDocumentMetadata, ElohimStoreError> {
        Ok(DidDocumentMetadata::default())
    }

    /// The identity-head facts for `agent_cid` — chain-root, current head, and
    /// controllers — from the Wave-B `binds-identity` declaration, or `None` when
    /// no head is declared for the agent.
    ///
    /// Defaults to `None` so a phase-1 store (no identity-head projection) is
    /// unaffected — the assembly falls back to the implicit-self-controller
    /// document, byte-unchanged — **without** a breaking change to this trait or
    /// its existing implementors, exactly as [`Self::document_metadata`] defaults
    /// empty. The common case today IS no head.
    async fn identity_head(
        &self,
        _agent_cid: &str,
    ) -> Result<Option<IdentityHead>, ElohimStoreError> {
        Ok(None)
    }
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

        // Identity head (phase 2): the Wave-B `binds-identity` declaration —
        // controllers + lineage. Absent (phase-1 store / no head declared) → the
        // document stays the phase-1 implicit-self document, byte-unchanged.
        let identity_head = self
            .store
            .identity_head(agent_cid)
            .await
            .map_err(|e| DidResolutionError::Internal(e.to_string()))?;

        // alsoKnownAs = transport ids, plus a chain-root lineage alias when a head
        // is declared. The chain-root is a stable alternate identifier for the same
        // subject, durable across key rotation; a trivial self-alias (root == the
        // subject key, the degenerate single-node chain) is skipped.
        let mut also_known_as = self
            .store
            .transport_ids(agent_cid)
            .await
            .map_err(|e| DidResolutionError::Internal(e.to_string()))?;
        if let Some(head) = identity_head.as_ref() {
            if head.chain_root != agent_cid {
                let root_alias = format!("did:elohim:{}", head.chain_root);
                if !also_known_as.contains(&root_alias) {
                    also_known_as.push(root_alias);
                }
            }
        }
        if !also_known_as.is_empty() {
            doc.also_known_as = Some(also_known_as);
        }

        // controller: populated straight from the declared controller set (DID 1.1
        // Group Control — a community-recovery quorum is a controller, never an
        // override). No head, or an empty set → controller stays absent (implicit
        // self-control), keeping the phase-1 document unchanged.
        if let Some(head) = identity_head.as_ref() {
            match head.controllers.as_slice() {
                [] => {}
                [one] => doc.controller = Some(Controller::One(one.clone())),
                many => doc.controller = Some(Controller::Many(many.to_vec())),
            }
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

        let mut result = DidResolutionResult::success(doc);
        // Populate document metadata from the store (default: empty in phase 1).
        result.did_document_metadata = self
            .store
            .document_metadata(agent_cid)
            .await
            .map_err(|e| DidResolutionError::Internal(e.to_string()))?;
        Ok(result)
    }
}

/// Frame a store `ServiceRef` into a DID `Service` anchored on the subject DID.
fn frame_service(did: &Did, svc: ServiceRef) -> Service {
    Service {
        id: did.with_fragment(&svc.id_fragment),
        type_: svc.service_type,
        service_endpoint: ServiceEndpoint::Uri(svc.endpoint),
        extra: std::collections::BTreeMap::new(),
    }
}
