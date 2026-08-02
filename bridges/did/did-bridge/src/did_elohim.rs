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
//! `alsoKnownAs` alias.
//!
//! ## Why the head answer is a four-way enum, not an `Option`
//!
//! [`IdentityHeadAnswer`] separates four states an `Option<IdentityHead>`
//! collapsed into two — C4 (honest absence) composed with C9 (identity-lineage
//! continuity):
//!
//! | answer | what it says | document |
//! |---|---|---|
//! | [`Declared`](IdentityHeadAnswer::Declared) | a live head, with controllers | controllers + lineage alias |
//! | [`Revoked`](IdentityHeadAnswer::Revoked) | the head is revoked | **deactivated**, confers nothing |
//! | [`NeverDeclared`](IdentityHeadAnswer::NeverDeclared) | no declaration exists | the phase-1 implicit-self document, unchanged |
//! | [`Unresolvable`](IdentityHeadAnswer::Unresolvable) | we could not find out | **no document** — resolution fails closed |
//!
//! The collapse this replaces was not cosmetic. `None` meant both *"nobody ever
//! declared a head"* and *"the head is revoked"*, and because an absent
//! `controller` in a DID document means **the subject controls it**, a revoked
//! identity resolved to a perfectly ordinary self-controlled document — the
//! revocation disappeared into the same silence as never having declared. The
//! third collapse hid inside the `Err` arm's usual treatment: a store that could
//! not *read* the head had no way to say so as an answer, and the caller's
//! natural fallback was, again, the self-controlled document.
//!
//! `NeverDeclared` is the default for a phase-1 store, so a store that never
//! implements this method assembles exactly the document it always did.

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
    ///
    /// An EMPTY set is not a legal declaration: serialized, it would produce a
    /// document with no `controller`, which DID 1.1 reads as implicit
    /// self-control — the opposite of what an empty declared set says. The
    /// resolver refuses it as [`DidResolutionError::IdentityHeadMalformed`]
    /// rather than degrading to self-control.
    pub controllers: Vec<Did>,
}

/// The facts a REVOKED identity head still carries. C9: a revocation must not
/// orphan the state keyed on the revoked key — it fails loud (the document is
/// deactivated) *and* re-anchors (the lineage stays nameable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevokedIdentity {
    /// The stable chain-root identifier of the revoked identity. Preserved on
    /// purpose: a reference keyed on the revoked head can still find the lineage
    /// it belonged to instead of silently resolving to nothing.
    pub chain_root: String,
    /// The revoked head key (typically the resolved `agent_cid`).
    pub head: String,
    /// The successor head, when the revocation named one (a rotation or a
    /// community recovery). `None` for a terminal revocation — which is an
    /// honest "no successor", not "unknown successor": a store that cannot
    /// determine the successor must answer
    /// [`IdentityHeadAnswer::Unresolvable`], not `Revoked` with `None`.
    pub successor: Option<String>,
}

/// What the store can say about an agent's identity head. The four answers are
/// deliberately distinct — see the module docs for the collapse this replaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityHeadAnswer {
    /// A head is declared and live: controllers + lineage populate the document.
    Declared(IdentityHead),
    /// The head is **revoked**. The assembled document is deactivated (DID 1.1
    /// §7.1.2) and carries no key, service, or transport material.
    Revoked(RevokedIdentity),
    /// No `binds-identity` declaration exists for this agent — an *observed*
    /// absence. The phase-1 implicit-self document is the correct answer here,
    /// and only here.
    NeverDeclared,
    /// The head state could not be determined: an undelivered DHT record, a
    /// timed-out read, an unevaluatable declaration. Nothing is established in
    /// either direction, so resolution fails closed.
    ///
    /// On a full-arc fleet a local `get` miss is exactly this — gossip has not
    /// delivered the record yet — and must NOT be reported as
    /// [`NeverDeclared`](Self::NeverDeclared).
    Unresolvable(String),
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

    /// What is known about `agent_cid`'s identity head, from the Wave-B
    /// `binds-identity` declaration: declared, revoked, never declared, or
    /// unresolvable. See [`IdentityHeadAnswer`].
    ///
    /// Defaults to [`IdentityHeadAnswer::NeverDeclared`] so a phase-1 store (no
    /// identity-head projection) is unaffected — the assembly falls back to the
    /// implicit-self-controller document, byte-unchanged — exactly as
    /// [`Self::document_metadata`] defaults empty. The common case today IS no
    /// head.
    ///
    /// ## Implementor contract (the part that is easy to get wrong)
    ///
    /// `NeverDeclared` is a **positive claim**: you looked, and no declaration
    /// exists. Anything short of that — a read that errored, a DHT record that
    /// has not been gossiped to you yet, a declaration you could not evaluate —
    /// is [`Unresolvable`](IdentityHeadAnswer::Unresolvable). On a full-arc
    /// fleet every zome `get`/`get_links` is local-only, so a miss means gossip
    /// has not delivered the record, never that no head was declared; mapping
    /// that miss to `NeverDeclared` re-opens exactly the silent degradation this
    /// enum exists to close. `Err(ElohimStoreError)` remains available for a
    /// store-level failure that is not about this agent at all.
    async fn identity_head(
        &self,
        _agent_cid: &str,
    ) -> Result<IdentityHeadAnswer, ElohimStoreError> {
        Ok(IdentityHeadAnswer::NeverDeclared)
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

        // Identity head FIRST: it decides whether an authority-bearing document
        // gets assembled at all. Reading it after the key material would mean
        // building a revoked identity's verification methods and services before
        // discovering we must not serve them.
        let identity_head = match self
            .store
            .identity_head(agent_cid)
            .await
            .map_err(|e| DidResolutionError::Internal(e.to_string()))?
        {
            // Nothing established in either direction. Assembling the
            // implicit-self document here would assert self-control we never
            // verified — the exact silent degradation the enum exists to close.
            IdentityHeadAnswer::Unresolvable(why) => {
                return Err(DidResolutionError::IdentityHeadUnresolvable(format!(
                    "{}: {why}",
                    did.as_string()
                )));
            }
            // Fail closed, out loud, and without touching another store method:
            // the deactivated document is a complete answer on its own, and every
            // further projection (services, transports) is a chance to serve a
            // revoked identity's endpoints as if they were live.
            IdentityHeadAnswer::Revoked(revoked) => {
                return Ok(assemble_deactivated_document(did, &revoked));
            }
            IdentityHeadAnswer::Declared(head) => Some(head),
            IdentityHeadAnswer::NeverDeclared => None,
        };

        // Verification method from the agent key (Multikey).
        let multikey = core32_to_multikey(&core);
        let vm_id = did.with_fragment(&multikey);
        let vm = VerificationMethod::multikey(vm_id.clone(), did.clone(), multikey);

        let mut doc = DidDocument::new(Context::did_v1_1_multikey(), did.clone());
        doc.verification_method = Some(vec![vm]);
        // Per §3.4: authentication + assertionMethod reference the agent key.
        doc.authentication = Some(vec![VerificationRelationship::Reference(vm_id.clone())]);
        doc.assertion_method = Some(vec![VerificationRelationship::Reference(vm_id)]);

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
            push_lineage_alias(&mut also_known_as, &head.chain_root, agent_cid);
        }
        if !also_known_as.is_empty() {
            doc.also_known_as = Some(also_known_as);
        }

        // controller: populated straight from the declared controller set (DID 1.1
        // Group Control — a community-recovery quorum is a controller, never an
        // override). NeverDeclared → controller stays absent (implicit
        // self-control), keeping the phase-1 document unchanged; a declared but
        // EMPTY set is a refusal, not the same thing.
        if let Some(head) = identity_head.as_ref() {
            doc.controller = Some(controller_from_head(head)?);
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

/// Append `did:elohim:<root>` to `aliases` as a lineage alias, unless it would be
/// a degenerate self-alias (`root == subject`) or is already present.
///
/// C9: the chain-root is the identifier that survives key rotation, so it is what
/// a reference should re-anchor on. Extracted as a named function because the
/// revoked path needs exactly the same rule — a revoked identity must stay
/// nameable through its lineage, and two spellings of that rule is how the two
/// paths drift apart.
fn push_lineage_alias(aliases: &mut Vec<String>, chain_root: &str, subject_cid: &str) {
    if chain_root == subject_cid {
        return;
    }
    let alias = format!("did:elohim:{chain_root}");
    if !aliases.contains(&alias) {
        aliases.push(alias);
    }
}

/// The declared controller set as a DID 1.1 `controller` value.
///
/// **Concerns:** C9 (identity lineage), C4 (an empty declaration is not an
/// absent one).
///
/// One controller emits [`Controller::One`]; several emit [`Controller::Many`]
/// (Group Control — a community-recovery quorum is a controller, never an
/// override). An EMPTY declared set is refused: a document with no `controller`
/// means *the subject controls it*, so emitting nothing for an empty declaration
/// would silently convert "these are the controllers, and there are none" into
/// "the subject is in sole control" — a strictly stronger claim than the
/// declaration made, invented by the serializer.
///
/// # Errors
///
/// [`DidResolutionError::IdentityHeadMalformed`] for an empty controller set.
pub fn controller_from_head(head: &IdentityHead) -> Result<Controller, DidResolutionError> {
    match head.controllers.as_slice() {
        [] => Err(DidResolutionError::IdentityHeadMalformed(format!(
            "identity head for chain-root {:?} declares an EMPTY controller set; \
             an absent `controller` would read as implicit self-control, which the \
             declaration does not say",
            head.chain_root
        ))),
        [one] => Ok(Controller::One(one.clone())),
        many => Ok(Controller::Many(many.to_vec())),
    }
}

/// Assemble the resolution result for a **revoked** identity: a deactivated DID
/// document (DID 1.1 §7.1.2) that confers no authority.
///
/// **Concerns:** C9 (a re-key never silently orphans state — this fails loud AND
/// re-anchors), C4 (revoked is its own answer, distinct from absent and from
/// never-declared), C5 (nothing here is asserted that the resolver did not
/// derive from the revocation itself).
///
/// Per DID Core, deactivation is a **successful resolution**, not a `notFound`:
/// the DID exists and its state is knowable, and reporting it as absent would
/// merge a refusal into an absence. So the answer is a document — but a document
/// that cannot be used:
///
/// - `didDocumentMetadata.deactivated = true`, the standards-registered signal;
/// - **no** `verificationMethod` and **no** verification relationships, so there
///   is no key to authenticate, assert, or invoke a capability with;
/// - **no** `service` and **no** transport ids, so a revoked identity's endpoints
///   are not projected as if they were live;
/// - `alsoKnownAs` carrying only the chain-root (and successor, when the
///   revocation named one), so the lineage stays followable.
///
/// A consumer that reads the metadata sees the revocation; a consumer that
/// ignores metadata entirely still finds nothing it can act with. That
/// belt-and-braces shape is the point: the defect this replaces was a revoked
/// identity resolving to an ordinary, fully-armed, implicitly self-controlled
/// document.
pub fn assemble_deactivated_document(did: &Did, revoked: &RevokedIdentity) -> DidResolutionResult {
    let mut doc = DidDocument::new(Context::did_v1_1_multikey(), did.clone());

    let mut aliases: Vec<String> = Vec::new();
    push_lineage_alias(&mut aliases, &revoked.chain_root, did.method_specific_id());
    if let Some(successor) = revoked.successor.as_ref() {
        push_lineage_alias(&mut aliases, successor, did.method_specific_id());
    }
    if !aliases.is_empty() {
        doc.also_known_as = Some(aliases);
    }

    let mut result = DidResolutionResult::success(doc);
    result.did_document_metadata.deactivated = Some(true);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUBJECT: &str = "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH";
    const ROOT: &str = "bafyreichainrootgenesisnodecid00000000000000000000000000";
    const SUCCESSOR: &str = "uhCAkcHVja32JeYONbe1Dag4rWFB8Jj4-Q5Nv6iyClY-J6-teAgTr";

    fn subject_did() -> Did {
        Did::parse(&format!("did:elohim:{SUBJECT}")).unwrap()
    }

    fn head_with(controllers: &[&str]) -> IdentityHead {
        IdentityHead {
            chain_root: ROOT.to_string(),
            head: SUBJECT.to_string(),
            controllers: controllers
                .iter()
                .map(|c| Did::parse(&format!("did:elohim:{c}")).unwrap())
                .collect(),
        }
    }

    #[test]
    fn one_controller_is_controller_one() {
        let c = controller_from_head(&head_with(&[SUBJECT])).unwrap();
        assert!(matches!(c, Controller::One(_)));
    }

    #[test]
    fn several_controllers_are_controller_many() {
        let c = controller_from_head(&head_with(&[SUBJECT, SUCCESSOR])).unwrap();
        match c {
            Controller::Many(v) => assert_eq!(v.len(), 2),
            other => panic!("expected Many, got {other:?}"),
        }
    }

    #[test]
    fn empty_controller_set_is_refused_not_silently_self_controlled() {
        let err = controller_from_head(&head_with(&[])).unwrap_err();
        assert!(matches!(err, DidResolutionError::IdentityHeadMalformed(_)));
    }

    #[test]
    fn deactivated_document_confers_no_authority() {
        let did = subject_did();
        let result = assemble_deactivated_document(
            &did,
            &RevokedIdentity {
                chain_root: ROOT.to_string(),
                head: SUBJECT.to_string(),
                successor: Some(SUCCESSOR.to_string()),
            },
        );
        assert_eq!(result.did_document_metadata.deactivated, Some(true));
        let doc = result
            .did_document
            .expect("deactivation resolves, not 404s");
        assert_eq!(doc.id, did);
        // Nothing to act with.
        assert!(doc.verification_method.is_none(), "no key material");
        assert!(doc.authentication.is_none(), "no authentication");
        assert!(doc.assertion_method.is_none(), "no assertionMethod");
        assert!(doc.key_agreement.is_none());
        assert!(doc.capability_invocation.is_none());
        assert!(doc.capability_delegation.is_none());
        assert!(doc.service.is_none(), "no endpoints projected");
        // Lineage stays followable (C9): root + successor, no transport ids.
        let aka = doc.also_known_as.expect("lineage aliases");
        assert!(aka.contains(&format!("did:elohim:{ROOT}")));
        assert!(aka.contains(&format!("did:elohim:{SUCCESSOR}")));
        assert_eq!(aka.len(), 2);
    }

    #[test]
    fn terminal_revocation_names_the_root_only() {
        let result = assemble_deactivated_document(
            &subject_did(),
            &RevokedIdentity {
                chain_root: ROOT.to_string(),
                head: SUBJECT.to_string(),
                successor: None,
            },
        );
        let aka = result.did_document.unwrap().also_known_as.unwrap();
        assert_eq!(aka, vec![format!("did:elohim:{ROOT}")]);
    }

    #[test]
    fn degenerate_self_root_emits_no_alias() {
        // chain_root == subject: the single-node chain has no separate lineage
        // name, and a self-alias is noise, not information.
        let result = assemble_deactivated_document(
            &subject_did(),
            &RevokedIdentity {
                chain_root: SUBJECT.to_string(),
                head: SUBJECT.to_string(),
                successor: None,
            },
        );
        assert!(result.did_document.unwrap().also_known_as.is_none());
    }

    #[test]
    fn the_four_head_answers_are_distinct_values() {
        // The collapse this enum replaces, asserted at the type level: no two of
        // these compare equal, so no caller can conflate them by accident.
        let answers = [
            IdentityHeadAnswer::Declared(head_with(&[SUBJECT])),
            IdentityHeadAnswer::Revoked(RevokedIdentity {
                chain_root: ROOT.to_string(),
                head: SUBJECT.to_string(),
                successor: None,
            }),
            IdentityHeadAnswer::NeverDeclared,
            IdentityHeadAnswer::Unresolvable("dht read timed out".to_string()),
        ];
        for (i, a) in answers.iter().enumerate() {
            for (j, b) in answers.iter().enumerate() {
                assert_eq!(i == j, a == b, "answers {i} and {j} compared wrongly");
            }
        }
    }
}
