//! Identity key-lineage — chain-root over the `KeyRotation` version DAG.
//!
//! Wave B (gap #1) of `genesis/docs/superpowers/plans/2026-07-17-identity-head-key-lineage-plan.md`.
//!
//! **B0 architecture decision (compose, don't build).** The identity lineage DAG
//! is the ALREADY-WIRED imagodei `KeyRotation` edges (`recovery_v2.rs`), NOT a new
//! `version_parent` field and NOT a second rotation mechanism. Each `KeyRotation`
//! carries `superseded_agent_pubkey → new_agent_pubkey`; read as a version DAG,
//! that superseded→new edge IS the `version_parent` back-pointer (a SET, for the
//! merge/recovery case). This module makes the chain QUERYABLE — deriving a stable
//! **chain-root** (the genesis key, `version_parent = []`) and the current **head** —
//! purely from existing entries + links. It is **coordinator-only → DNA-hash-neutral**:
//! it adds no entry type, no link type, and no integrity change (it reads the
//! `AgentToKeyRotation` and `HumanToCurrentAgent` links `commit_key_rotation`
//! already authors).
//!
//! Chain-root stability is the contract: the root NEVER changes across
//! rotation/recovery (every re-pointing that targets it would silently break
//! otherwise). It is proven by the pure-logic property tests below and generalizes
//! Wave A's degenerate root — for an un-rotated identity the root is the key itself.

use hdk::prelude::*;
use imagodei_integrity::{EntryTypes, KeyRotation, LinkTypes, RecoveryAuthority, StringAnchor};

// =============================================================================
// Pure logic (unit-testable without an HDK runtime) — mirrors the recovery_v2
// pure-logic + HDI-wrapper split. Operates on an in-memory edge model so the
// walk algorithm is proven deterministic and append-stable independent of DHT.
// =============================================================================

/// A single version edge read off a `KeyRotation`: `new_key`'s immediate
/// version-parent is `superseded_key` (the superseded→new relationship, read as
/// a `version_parent` SET member).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionEdge {
    pub new_key: AgentPubKey,
    pub superseded_key: AgentPubKey,
}

/// The immediate version-parents of `key` — the superseded keys of every edge
/// whose `new_key == key`. Empty ⇒ `key` is a genesis node (`version_parent = []`).
pub fn version_parents_of(key: &AgentPubKey, edges: &[VersionEdge]) -> Vec<AgentPubKey> {
    edges
        .iter()
        .filter(|e| &e.new_key == key)
        .map(|e| e.superseded_key.clone())
        .collect()
}

/// Walk the version-parent edges back to the genesis root (`version_parent = []`).
///
/// Deterministic and append-stable:
/// - On a merge (a key with multiple version-parents — the recovery-reconcile
///   case) the walk follows the byte-minimal parent, so it always terminates at
///   ONE stable root.
/// - Adding a CHILD edge (a new rotation off the current head) never changes any
///   ancestor's root — the walk from an existing node is unaffected (chain-root
///   stability is the contract).
/// - Cycle-guarded (the DAG is append-only and acyclic in practice; the guard is
///   defense-in-depth so a malformed cycle terminates rather than looping).
pub fn chain_root_of(start: &AgentPubKey, edges: &[VersionEdge]) -> AgentPubKey {
    let mut current = start.clone();
    let mut visited: Vec<AgentPubKey> = Vec::new();
    loop {
        if visited.contains(&current) {
            // Cycle — return the current node rather than loop forever.
            return current;
        }
        visited.push(current.clone());
        let mut parents = version_parents_of(&current, edges);
        if parents.is_empty() {
            return current; // genesis
        }
        // Deterministic min-by-bytes on merge keeps the root stable.
        parents.sort_by(|a, b| a.get_raw_39().cmp(b.get_raw_39()));
        current = parents.remove(0);
    }
}

/// The current head of a chain: the `new_key` that is not itself superseded by
/// any edge (the tip), reachable forward from `root`. Empty edge-set ⇒ `root`
/// itself (an un-rotated identity is its own head). On a fork, byte-minimal tip
/// for determinism (full merge-reconciliation is the kinship-lineage follow-on).
pub fn chain_head_of(root: &AgentPubKey, edges: &[VersionEdge]) -> AgentPubKey {
    if edges.is_empty() {
        return root.clone();
    }
    // Tips = new_keys never appearing as a superseded_key.
    let mut tips: Vec<AgentPubKey> = edges
        .iter()
        .filter(|e| !edges.iter().any(|o| o.superseded_key == e.new_key))
        .map(|e| e.new_key.clone())
        .collect();
    tips.sort_by(|a, b| a.get_raw_39().cmp(b.get_raw_39()));
    tips.dedup();
    tips.into_iter().next().unwrap_or_else(|| root.clone())
}

// =============================================================================
// HDK wrappers (resolve DHT links → delegate to the walk). Read-only.
// =============================================================================

/// Resolve one back-step: the version-parents (superseded keys) of `key`.
/// `commit_key_rotation` anchors every rotation on `agent_rotation:{new_key}`
/// via `AgentToKeyRotation`, so a link off that anchor means `key` was the
/// `new_agent_pubkey` of a rotation — its `superseded_agent_pubkey` is the parent.
fn resolve_version_parents(key: &AgentPubKey) -> ExternResult<Vec<AgentPubKey>> {
    let anchor = StringAnchor::new("agent_rotation", &key.to_string());
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let query = LinkQuery::try_new(anchor_hash, LinkTypes::AgentToKeyRotation)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut parents = Vec::with_capacity(links.len());
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(rotation) = record.entry().to_app_option::<KeyRotation>().ok().flatten()
                {
                    parents.push(rotation.superseded_agent_pubkey);
                }
            }
        }
    }
    Ok(parents)
}

/// The immediate version-parents (superseded keys) of `agent`. Empty ⇒ genesis.
#[hdk_extern]
pub fn identity_version_parents(agent: AgentPubKey) -> ExternResult<Vec<AgentPubKey>> {
    resolve_version_parents(&agent)
}

/// The stable chain-root (genesis key) for any key in an identity's lineage.
/// Walks the `KeyRotation` superseded→new DAG backwards to `version_parent = []`.
/// For an un-rotated identity the root is the key itself (Wave A's degenerate
/// root — kept coherent so the storage projection resolves un-rotated == rooted).
#[hdk_extern]
pub fn identity_chain_root(agent: AgentPubKey) -> ExternResult<AgentPubKey> {
    let mut current = agent;
    let mut visited: Vec<AgentPubKey> = Vec::new();
    loop {
        if visited.contains(&current) {
            return Ok(current); // cycle guard
        }
        visited.push(current.clone());
        let mut parents = resolve_version_parents(&current)?;
        if parents.is_empty() {
            return Ok(current); // genesis root
        }
        parents.sort_by(|a, b| a.get_raw_39().cmp(b.get_raw_39()));
        current = parents.remove(0);
    }
}

/// The current head key of the chain anchored on `human_agent_pubkey` (the
/// stable identity anchor — the same value `commit_key_rotation` keys the
/// `current_agent:{human_agent_pubkey}` anchor on). Collects the human's
/// rotations and returns the structural tip (the `new_key` not superseded by any
/// rotation). For an un-rotated identity, the head is `human_agent_pubkey`.
#[hdk_extern]
pub fn identity_head(human_agent_pubkey: AgentPubKey) -> ExternResult<AgentPubKey> {
    let anchor = StringAnchor::new("current_agent", &human_agent_pubkey.to_string());
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let query = LinkQuery::try_new(anchor_hash, LinkTypes::HumanToCurrentAgent)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut edges: Vec<VersionEdge> = Vec::with_capacity(links.len());
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(rotation) = record.entry().to_app_option::<KeyRotation>().ok().flatten()
                {
                    edges.push(VersionEdge {
                        new_key: rotation.new_agent_pubkey,
                        superseded_key: rotation.superseded_agent_pubkey,
                    });
                }
            }
        }
    }
    Ok(chain_head_of(&human_agent_pubkey, &edges))
}

// =============================================================================
// B3 — rotate_identity_key authorization (controller-policy gate).
//
// The rotation node is a `KeyRotation` (the version-DAG append); WHICH controllers
// may authorize it is the binds-identity `controller_policy` (mishpat declares it;
// the caller passes which declared policy governs — a declared dependency, the
// lens-version-DAG "which head applies is declared" principle). The two policies
// enforceable from LOCAL evidence (no cross-DNA read) are gated here; steward-set
// (which needs the notarized controller set) is Wave C. Ontology guard: the
// recovery quorum is a *controller*, named alongside self — not an override.
// =============================================================================

/// The controller policy governing a rotation, parsed from the binds-identity
/// declaration's `controller_policy.kind`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControllerPolicy {
    /// The current head key is its own controller — self-authorized rotation.
    SelfKey,
    /// A named steward-set authorizes (resolution is Wave C — reads binds-identity).
    StewardSet,
    /// An M-of-N recovery quorum authorizes (the grandma case). Reuses the wired
    /// `RecoveryAuthority`/`RecoveryRequest` semantics — validated downstream by
    /// the `KeyRotation` integrity validator on `create_entry`.
    RecoveryQuorum,
}

impl ControllerPolicy {
    /// Parse the wire discriminator (matches the mishpat binds-identity
    /// `controller_policy.kind` enum: self | steward-set | recovery-quorum).
    pub fn parse(kind: &str) -> Result<Self, String> {
        match kind {
            "self" => Ok(ControllerPolicy::SelfKey),
            "steward-set" => Ok(ControllerPolicy::StewardSet),
            "recovery-quorum" => Ok(ControllerPolicy::RecoveryQuorum),
            other => Err(format!(
                "controller_policy '{other}' not in enum (self|steward-set|recovery-quorum)"
            )),
        }
    }
}

/// Pure authorization decision for `rotate_identity_key`. `Ok(())` ⇒ authorized.
///
/// - **self**: the caller must BE the current head key (`caller_is_current_head`).
///   A non-head caller is REFUSED before any entry is written.
/// - **recovery-quorum**: the rotation must carry a recovery authority variant
///   (`has_recovery_authority`); the M-of-N quorum itself is validated by the
///   `KeyRotation` integrity validator when the entry is committed (that is the
///   notarized quorum check — not re-implemented here).
/// - **steward-set**: REFUSED this wave — resolving the notarized controller set
///   is Wave C (the did:elohim assembly). No insecure default door.
pub fn authorize_rotation(
    policy: &ControllerPolicy,
    caller_is_current_head: bool,
    has_recovery_authority: bool,
) -> Result<(), String> {
    match policy {
        ControllerPolicy::SelfKey => {
            if caller_is_current_head {
                Ok(())
            } else {
                Err(
                    "rotate_identity_key: self-policy requires the caller to be the current \
                     head key (unauthorized controller)"
                        .to_string(),
                )
            }
        }
        ControllerPolicy::RecoveryQuorum => {
            if has_recovery_authority {
                Ok(())
            } else {
                Err(
                    "rotate_identity_key: recovery-quorum policy requires a recovery authority \
                     (IntimateQuorum or CryptographicQuorum) on the rotation"
                        .to_string(),
                )
            }
        }
        ControllerPolicy::StewardSet => Err(
            "rotate_identity_key: steward-set controller resolution is Wave C (reads the \
             notarized binds-identity controller set)"
                .to_string(),
        ),
    }
}

/// Input for `rotate_identity_key` — appends a version node to an identity's
/// key-lineage DAG, authorized by the declared controller policy.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RotateIdentityKeyInput {
    /// The stable identity anchor (chain-root / genesis key) — the same value
    /// `commit_key_rotation` keys `current_agent:{human_agent_pubkey}` on.
    pub human_agent_pubkey: AgentPubKey,
    /// The new head key this rotation advances the chain to.
    pub new_agent_pubkey: AgentPubKey,
    /// The key being superseded — MUST be the chain's current head (you can only
    /// rotate the current head; a stale-key rotation is refused).
    pub superseded_agent_pubkey: AgentPubKey,
    /// The governing binds-identity `controller_policy.kind` (a DECLARED
    /// dependency): "self" | "steward-set" | "recovery-quorum".
    pub controller_policy: String,
    /// The recovery authority carried on the `KeyRotation` entry. For
    /// recovery-quorum policy this is the M-of-N quorum (validated by the
    /// `KeyRotation` integrity validator on commit). Required by the entry shape.
    pub authority: RecoveryAuthority,
    /// The recovery-request the authority references (a `KeyRotation` field;
    /// cross-referenced by the coordinator recovery flow, not by this gate).
    pub recovery_request_hash: ActionHash,
}

/// Append a version node to an identity's key-lineage DAG, authorized by the
/// current controllers per the active binds-identity `controller_policy`
/// (Wave B, gap #3). Coordinator-only → DNA-hash-neutral.
///
/// Authorization (the NEW controller-policy gate) runs BEFORE any entry is
/// written; on pass the shared `append_key_rotation_entry` helper commits the
/// SAME version-DAG node `commit_key_rotation` does (one rotation mechanism).
/// The chain-root (genesis) is unchanged by any rotation — only the head advances.
#[hdk_extern]
pub fn rotate_identity_key(
    input: RotateIdentityKeyInput,
) -> ExternResult<crate::KeyRotationOutput> {
    let policy = ControllerPolicy::parse(&input.controller_policy)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;

    // Resolve the chain's current head from the lineage DAG.
    let current_head = identity_head(input.human_agent_pubkey.clone())?;

    // Invariant: a rotation supersedes the CURRENT head — never a stale key
    // (that would fork the chain off an abandoned node).
    if input.superseded_agent_pubkey != current_head {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "rotate_identity_key: superseded key {} is not the chain's current head {} \
             (only the current head may be rotated)",
            input.superseded_agent_pubkey, current_head,
        ))));
    }

    // Controller-policy authorization (the new gate).
    let caller = agent_info()?.agent_initial_pubkey;
    let caller_is_current_head = caller == current_head;
    // Only the IMPLEMENTED recovery variants count as present authority at the
    // coordinator layer; the stub variants are refused early with a clear message
    // (the integrity validator would reject them downstream regardless).
    let has_recovery_authority = matches!(
        input.authority,
        RecoveryAuthority::IntimateQuorum { .. } | RecoveryAuthority::CryptographicQuorum { .. }
    );
    authorize_rotation(&policy, caller_is_current_head, has_recovery_authority)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;

    // Append the version node. The KeyRotation integrity validator enforces the
    // recovery authority (the notarized M-of-N quorum) at commit time.
    let rotation = KeyRotation {
        human_agent_pubkey: input.human_agent_pubkey,
        new_agent_pubkey: input.new_agent_pubkey,
        superseded_agent_pubkey: input.superseded_agent_pubkey,
        recovery_request_hash: input.recovery_request_hash,
        authority: input.authority,
        rotated_at: sys_time()?,
    };
    let action_hash = crate::append_key_rotation_entry(rotation.clone())?;

    Ok(crate::KeyRotationOutput {
        action_hash,
        rotation,
    })
}

// =============================================================================
// Pure-logic unit tests — the chain-walk correctness + stability proof.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn key(seed: u8) -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![seed; 36])
    }

    fn edge(new: u8, superseded: u8) -> VersionEdge {
        VersionEdge {
            new_key: key(new),
            superseded_key: key(superseded),
        }
    }

    #[test]
    fn un_rotated_root_is_the_key_itself() {
        // Degenerate single-node chain (Wave A generalization): no edges ⇒ the
        // key IS its own root and head.
        let g = key(1);
        assert_eq!(chain_root_of(&g, &[]), g);
        assert_eq!(chain_head_of(&g, &[]), g);
        assert!(version_parents_of(&g, &[]).is_empty());
    }

    #[test]
    fn linear_chain_root_is_genesis_from_every_node() {
        // G(1) → K1(2) → K2(3): root is G from every node.
        let edges = vec![edge(2, 1), edge(3, 2)];
        assert_eq!(chain_root_of(&key(1), &edges), key(1));
        assert_eq!(chain_root_of(&key(2), &edges), key(1));
        assert_eq!(chain_root_of(&key(3), &edges), key(1));
    }

    #[test]
    fn head_is_the_tip_of_the_chain() {
        let edges = vec![edge(2, 1), edge(3, 2)];
        // From the stable anchor (genesis), head is the tip K2(3).
        assert_eq!(chain_head_of(&key(1), &edges), key(3));
    }

    #[test]
    fn root_stable_across_append() {
        // Chain-root stability contract: appending a child edge off the head
        // NEVER changes any ancestor's root.
        let before = vec![edge(2, 1)]; // G → K1
        let after = vec![edge(2, 1), edge(3, 2)]; // G → K1 → K2 (appended)
        assert_eq!(chain_root_of(&key(2), &before), key(1));
        assert_eq!(
            chain_root_of(&key(2), &after),
            key(1),
            "root must be stable across append"
        );
        assert_eq!(chain_root_of(&key(3), &after), key(1));
        // And the head advances while the root holds.
        assert_eq!(chain_head_of(&key(1), &before), key(2));
        assert_eq!(chain_head_of(&key(1), &after), key(3), "head advances");
    }

    #[test]
    fn merge_walk_is_deterministic_and_terminates() {
        // K3(4) has two version-parents (a recovery reconcile / merge): roots
        // {1} via 2 and {5} via 6. Deterministic min-bytes walk yields a single
        // stable root and terminates.
        let edges = vec![edge(2, 1), edge(4, 2), edge(6, 5), edge(4, 6)];
        let root = chain_root_of(&key(4), &edges);
        // min-bytes parent of K3(4) is 2 (2 < 6) → root walks to 1.
        assert_eq!(root, key(1));
        // stable on repeat.
        assert_eq!(chain_root_of(&key(4), &edges), key(1));
    }

    #[test]
    fn cycle_is_guarded() {
        // Malformed cycle 1↔2: the walk terminates rather than looping.
        let edges = vec![edge(2, 1), edge(1, 2)];
        let _ = chain_root_of(&key(1), &edges); // must not hang
    }

    #[test]
    fn version_parents_returns_the_superseded_set() {
        let edges = vec![edge(2, 1), edge(2, 9)]; // K1(2) merges 1 and 9
        let parents = version_parents_of(&key(2), &edges);
        assert_eq!(parents.len(), 2);
        assert!(parents.contains(&key(1)));
        assert!(parents.contains(&key(9)));
    }

    // ---- B3: authorize_rotation ----

    #[test]
    fn policy_parse_enum() {
        assert_eq!(
            ControllerPolicy::parse("self").unwrap(),
            ControllerPolicy::SelfKey
        );
        assert_eq!(
            ControllerPolicy::parse("steward-set").unwrap(),
            ControllerPolicy::StewardSet
        );
        assert_eq!(
            ControllerPolicy::parse("recovery-quorum").unwrap(),
            ControllerPolicy::RecoveryQuorum
        );
        assert!(ControllerPolicy::parse("dictator").is_err());
    }

    #[test]
    fn self_policy_authorized_only_when_caller_is_head() {
        // controller-authorized: caller IS the current head → allowed.
        assert!(authorize_rotation(&ControllerPolicy::SelfKey, true, false).is_ok());
        // unauthorized: caller is NOT the head → refused (before any entry write).
        let err = authorize_rotation(&ControllerPolicy::SelfKey, false, false).unwrap_err();
        assert!(err.contains("unauthorized controller"), "{err}");
    }

    #[test]
    fn recovery_quorum_requires_a_recovery_authority() {
        // grandma case: a valid recovery authority present → gate passes (the
        // M-of-N itself is validated by the KeyRotation integrity validator).
        assert!(authorize_rotation(&ControllerPolicy::RecoveryQuorum, false, true).is_ok());
        // no authority → refused.
        assert!(authorize_rotation(&ControllerPolicy::RecoveryQuorum, false, false).is_err());
    }

    #[test]
    fn steward_set_deferred_to_wave_c() {
        // No insecure default door — steward-set is refused this wave.
        let err = authorize_rotation(&ControllerPolicy::StewardSet, true, true).unwrap_err();
        assert!(err.contains("Wave C"), "{err}");
    }
}
