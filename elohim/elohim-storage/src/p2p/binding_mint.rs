//! Minting this node's own cross-signed `AgentPeerBinding` (C2-S2).
//!
//! [`binding_cross_signature`](super::binding_cross_signature) says what a proof
//! IS (C2-S1); [`binding_proof_wire`](super::binding_proof_wire) says when a
//! proof COUNTS (C2-S4, plus the identity-derivation checks this slice added).
//! Neither could ever fire, because **nothing in the tree called
//! `create_agent_peer_binding`** — every binding on every fleet carried
//! `STAGE1_SIGNATURE_SENTINEL` and classified `unverified` by construction. This
//! module is the producer that closes that loop: it is the first production
//! caller of that coordinator function, and the first thing anywhere that emits
//! an `elohim:apb:1:` envelope.
//!
//! ## The two halves, and where each key lives
//!
//! - **Transport half** — signed HERE. Storage owns the libp2p `Keypair`
//!   (`identity.key`), so the endpoint attests the agent locally, with no
//!   round trip. The recorded `transport_pubkey` is taken from that same
//!   keypair, so `PeerId` derivation (the check
//!   [`binding_proof_wire::transport_id_derives_from`]) holds by construction
//!   rather than by coincidence.
//! - **Agent half** — signed by the CONDUCTOR, via the imagodei `sign_for_agent`
//!   coordinator function. Storage holds no agent key material and never will.
//!
//! ## Identity namespace — the decision this module encodes
//!
//! `peer_identity_bindings.agent_cid` is polymorphic on a live fleet: the
//! genesis seeder writes a human SLUG (`agent_cid: human.id`, see
//! `genesis/seeder/src/seed-agent-bindings.ts`), while storage's own libp2p
//! identity handshake claims the conductor cell key (`uhCAk…`, see the
//! `IdentityHandshakeRequest` built in `p2p/mod.rs`). This module mints in the
//! **handshake namespace** — `NodeIdentity::agent_pubkey()`, the same value this
//! node already broadcasts about itself — for two reasons:
//!
//! 1. it is the namespace the durable consumers key on (`humans.agent_pub_key`,
//!    `peer_selection`, `transport_resolve`), so a minted binding is usable
//!    rather than merely well-formed; and
//! 2. a HoloHash `AgentPubKey` **names its own signing key**, so the agent half
//!    is self-verifying with no resolver — the degenerate-R1 case. A slug or an
//!    Agent-EPR CIDv1 would need the head/lineage resolver (C2-S6), and the
//!    chokepoint fail-closes on both until it lands.
//!
//! `sign_for_agent`'s signer-match gate is a SEPARATE question: it resolves an
//! Agent EPR by `id`, and Agent EPR ids are human slugs on a seeded fleet. So
//! the gate argument and the bound `agent_cid` are allowed to differ — the gate
//! proves "this caller controls the key registered on Agent EPR X", while the
//! core's `agent_cid` is verified independently by derivation. See
//! [`agent_epr_gate_candidates`].
//!
//! ## Failure direction
//!
//! Degrade to the sentinel, never to a bad proof. Every failure path here logs
//! and emits NOTHING: an unverified binding is honest, a forged-looking one is
//! not. The proof is self-verified twice before it leaves this process — once
//! through the pure algebra, once through the very chokepoint every receiving
//! peer will run — so a proof that could not classify `cross_signed` on arrival
//! is never published.
//!
//! ## Which of the three writers a minted proof reaches
//!
//! Two of the three, and that is a deliberate scope line:
//! - **DHT arrival** — yes. The coordinator emits `AgentPeerBindingCreated`
//!   carrying the entry's `signature`, and `ReconcileController` classifies it.
//! - **Gossip** — yes. The controller forwards that same signature into the
//!   `IdentityBindingGossip` payload instead of substituting a sentinel, so
//!   every subscribing peer classifies the real proof.
//! - **Handshake** — NO. The `IdentityHandshakeRequest` assembled on connect
//!   (`p2p/mod.rs`) still puts a base64 of its own PeerId in the `signature`
//!   field, so handshake-source rows keep classifying `unverified` even on a
//!   node that has minted. Carrying the minted envelope there means reading the
//!   projection on the connect path; it is a separate change, not an oversight
//!   here. Until it lands, a peer learns our proof by gossip/DHT, not by
//!   handshake.
//!
//! ## What this does NOT establish
//!
//! Freshness is still anchored on the self-asserted `issued_at`/validity window,
//! not on the notarized DHT action timestamp — that is C2-S3, and the "pincer"
//! it resolves is unaddressed here. Verification remains receiver-local until
//! the integrity-zome fold (C2-S7). Minting a proof does not make this node's
//! claim notarized; it makes it *checkable*.

use std::sync::Arc;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use tracing::{debug, info, warn};

use super::binding_cross_signature::{
    canonical_bytes, verify_binding_signatures, BindingCore, CrossSignatureProof, AGENT_DOMAIN,
    SCHEME_VERSION, TRANSPORT_DOMAIN, TRANSPORT_KIND_LIBP2P,
};
use super::binding_proof_wire::{
    agent_pubkey_from_agent_cid, classify_binding_signature, encode_proof,
    libp2p_peer_id_from_ed25519_pubkey,
};
use crate::db::DbPool;
use crate::error::StorageError;
use crate::hc_client::HcClient;

/// Kill switch. Any of `off` / `0` / `false` / `no` disables minting; anything
/// else (including unset) leaves it ON. Read ONCE at startup and threaded as a
/// bool — never re-read on a working path, so no test can leak a `set_var` into
/// another test (`feedback_env_var_test_flakiness`).
pub const MINT_ENABLED_ENV: &str = "ELOHIM_BINDING_MINT";

/// Operator escape hatch naming the Agent EPR id whose registered key this node
/// controls, when the local `humans` projection cannot supply it.
pub const MINT_AGENT_EPR_ENV: &str = "ELOHIM_BINDING_MINT_AGENT_EPR";

/// Validity window a freshly minted binding claims, in days.
///
/// Must stay at or under [`binding_proof_wire::MAX_VALIDITY_DAYS`] (90) or the
/// chokepoint refuses our own proof — the self-check below would catch that, but
/// it would catch it as a mystery. 30 days keeps the credential short enough
/// that a compromised key's window closes on its own.
pub const DEFAULT_VALIDITY_DAYS: i64 = 30;

/// Re-mint when a current binding has less than this much validity left, so a
/// binding is renewed BEFORE it lapses rather than after.
pub const REMINT_LEAD_DAYS: i64 = 7;

/// Zome + coordinator function names on the imagodei cell.
const IMAGODEI_ZOME: &str = "imagodei";
const SIGN_FOR_AGENT_FN: &str = "sign_for_agent";
const CREATE_BINDING_FN: &str = "create_agent_peer_binding";

// =============================================================================
// Errors
// =============================================================================

/// Why a mint attempt produced no binding. Every variant means "emitted
/// nothing"; none of them means "emitted something questionable".
#[derive(Debug, thiserror::Error)]
pub enum MintError {
    /// The libp2p keypair is not Ed25519, or refused to sign.
    #[error("transport keypair cannot sign an Ed25519 binding half: {0}")]
    TransportKey(String),
    /// The conductor's `sign_for_agent` returned a wrong-sized signature/pubkey.
    #[error("agent half has wrong length ({field}: expected {expected}, got {got})")]
    AgentHalfLength {
        /// Which field was mis-sized.
        field: &'static str,
        /// Expected byte length.
        expected: usize,
        /// Actual byte length.
        got: usize,
    },
    /// `core.transport_id` is not the id this transport key derives.
    #[error("assembled transport_id does not derive from the transport key")]
    TransportIdMismatch,
    /// `core.agent_cid` does not name the key the conductor signed with. On a
    /// coherent node these are the same key; a mismatch means the advertised
    /// `agent_cid` and the imagodei cell key have diverged, and publishing would
    /// assert something false.
    #[error(
        "agent_cid does not name the conductor's signing key — advertised identity \
         and imagodei cell key disagree"
    )]
    AgentCidMismatch,
    /// The pure algebra rejected our own proof.
    #[error("self-verification failed: {0}")]
    SelfVerify(String),
    /// The algebra accepted but the chokepoint would not classify it
    /// `cross_signed` — i.e. a receiving peer would have recorded it unverified.
    #[error("the proof would not classify cross_signed at the receiving chokepoint")]
    WouldNotClassify,
    /// No Agent EPR could be resolved for this node, so the (closed) signer-match
    /// gate refuses to sign. Correct-but-dormant, not a defect: the node keeps
    /// its honest sentinel binding until identity coherence lands.
    #[error("no Agent EPR resolved for this node — sign_for_agent refused every candidate")]
    NoAgentEpr,
    /// A conductor or database call failed.
    #[error(transparent)]
    Storage(#[from] StorageError),
}

// =============================================================================
// The sealed result
// =============================================================================

/// A complete, self-verified binding ready to be committed to the DHT.
#[derive(Debug, Clone)]
pub struct SealedBinding {
    /// The tuple both keys signed.
    pub core: BindingCore,
    /// The proof carrying both halves.
    pub proof: CrossSignatureProof,
    /// `elohim:apb:1:…` — the exact string that rides
    /// `AgentPeerBinding.signature` (as UTF-8 bytes).
    pub envelope: String,
}

/// What one mint attempt did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MintOutcome {
    /// A current cross-signed binding already exists; nothing was emitted.
    AlreadyCurrent,
    /// A new binding was committed. Carries the DHT action hash.
    Minted(String),
}

// =============================================================================
// Assembly (pure)
// =============================================================================

/// A fresh 16-byte nonce, base64url-no-pad. Derived from a v4 UUID, the same
/// CSPRNG-backed source the identity handshake already uses for its nonce —
/// `rand` is not a dependency under the `p2p` feature.
fn fresh_nonce() -> String {
    URL_SAFE_NO_PAD.encode(uuid::Uuid::new_v4().as_bytes())
}

fn rfc3339(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Assemble the tuple this node will ask both keys to sign.
///
/// Pure and total: no clock read (the caller passes `now`), no randomness beyond
/// the nonce, no I/O. `valid_until` is always `Some` — an open-ended credential
/// is refused by the chokepoint on purpose, so minting one would be minting a
/// dud.
pub fn assemble_core(
    agent_cid: &str,
    transport_id: &str,
    now: DateTime<Utc>,
    validity_days: i64,
) -> BindingCore {
    BindingCore {
        agent_cid: agent_cid.to_string(),
        transport_id: transport_id.to_string(),
        transport_kind: TRANSPORT_KIND_LIBP2P,
        valid_from: rfc3339(now),
        valid_until: Some(rfc3339(now + Duration::days(validity_days))),
        nonce: fresh_nonce(),
        issued_at: rfc3339(now),
    }
}

/// The bytes the CONDUCTOR must sign for the agent half. Handed to
/// `sign_for_agent` verbatim.
pub fn agent_half_preimage(core: &BindingCore) -> Vec<u8> {
    canonical_bytes(AGENT_DOMAIN, core)
}

/// Sign the transport half locally and seal a complete, twice-verified proof.
///
/// **This is the unit-testable heart of the mint.** The agent half arrives as
/// bytes, so a test can inject a stub signer (a real Ed25519 key, or garbage)
/// without a conductor, and the assertions it makes are the same ones production
/// makes.
///
/// The two verifications are deliberately different questions:
/// 1. [`verify_binding_signatures`] — does the ALGEBRA accept both halves?
/// 2. [`classify_binding_signature`] over [`encode_proof`] — would a RECEIVING
///    PEER record this as `cross_signed`? This one round-trips the wire encoding
///    and re-applies every row-level policy (bounded window, kind assignment,
///    both identity derivations), so a proof that cannot survive the network
///    never leaves this process.
///
/// A failure here is a hard `Err` and the caller emits nothing. Degrading to the
/// sentinel is the correct direction; emitting an unverifiable envelope is not.
pub fn seal_proof(
    core: &BindingCore,
    transport_keypair: &libp2p::identity::Keypair,
    agent_signature: &[u8],
    agent_pubkey: &[u8],
) -> Result<SealedBinding, MintError> {
    let transport_pubkey = transport_keypair
        .public()
        .try_into_ed25519()
        .map_err(|e| MintError::TransportKey(format!("public key is not Ed25519: {e}")))?
        .to_bytes();

    // The derivation the chokepoint will check — asserted HERE so a mismatch
    // reads as "your assembled core is wrong", not as an opaque classification
    // failure three steps later.
    if libp2p_peer_id_from_ed25519_pubkey(&transport_pubkey).as_deref()
        != Some(core.transport_id.as_str())
    {
        return Err(MintError::TransportIdMismatch);
    }

    let agent_pubkey: [u8; 32] =
        agent_pubkey
            .try_into()
            .map_err(|_| MintError::AgentHalfLength {
                field: "agent_pubkey",
                expected: 32,
                got: agent_pubkey.len(),
            })?;
    let agent_signature: [u8; 64] =
        agent_signature
            .try_into()
            .map_err(|_| MintError::AgentHalfLength {
                field: "agent_signature",
                expected: 64,
                got: agent_signature.len(),
            })?;

    if agent_pubkey_from_agent_cid(&core.agent_cid) != Some(agent_pubkey) {
        return Err(MintError::AgentCidMismatch);
    }

    let transport_signature: [u8; 64] = transport_keypair
        .sign(&canonical_bytes(TRANSPORT_DOMAIN, core))
        .map_err(|e| MintError::TransportKey(format!("sign transport half: {e}")))?
        .try_into()
        .map_err(|v: Vec<u8>| MintError::AgentHalfLength {
            field: "transport_signature",
            expected: 64,
            got: v.len(),
        })?;

    let proof = CrossSignatureProof {
        scheme_version: SCHEME_VERSION,
        transport_kind: core.transport_kind,
        transport_pubkey,
        transport_signature,
        agent_pubkey,
        agent_signature,
        nonce: core.nonce.clone(),
        issued_at: core.issued_at.clone(),
    };

    verify_binding_signatures(core, &proof).map_err(|e| MintError::SelfVerify(e.to_string()))?;

    let envelope = encode_proof(&proof);
    if !classify_binding_signature(
        &core.agent_cid,
        &core.transport_id,
        core.transport_kind,
        &core.valid_from,
        core.valid_until.as_deref(),
        &envelope,
    )
    .is_cross_signed()
    {
        return Err(MintError::WouldNotClassify);
    }

    Ok(SealedBinding {
        core: core.clone(),
        proof,
        envelope,
    })
}

// =============================================================================
// Conductor plumbing
// =============================================================================

/// Wire mirror of the imagodei coordinator's `CreateAgentPeerBindingInput`.
#[derive(Debug, Clone, serde::Serialize)]
struct CreateAgentPeerBindingInput {
    peer_id: String,
    agent_cid: String,
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    device_archetype: String,
    signature: Vec<u8>,
}

/// Wire mirror of `CreateAgentPeerBindingOutput`.
///
/// `action_hash` is an `ActionHash` on the DNA side, which msgpack carries as
/// raw BYTES — decoding it into a `String` silently loses the signal (the
/// `project_conductor_signal_msgpack_decode_class` trap). [`HoloHashB64`] is the
/// typed mirror that accepts bytes and normalizes.
#[derive(Debug, Clone, serde::Deserialize)]
struct CreateAgentPeerBindingOutput {
    action_hash: crate::signals::HoloHashB64,
}

/// The Agent-EPR ids to try as `sign_for_agent`'s gate argument, most specific
/// first.
///
/// The gate resolves an Agent EPR by its `id`, and that id is NOT the same thing
/// as the `agent_cid` this node binds (see the module doc). The candidates:
///
/// 1. `ELOHIM_BINDING_MINT_AGENT_EPR` — the operator naming it outright.
/// 2. The local `humans` projection: the human whose `agent_pub_key` is this
///    node's `agent_cid`. On a seeded fleet the Agent EPR id and the human id
///    are the same string (`create_human` → `create_agent` with `human.id`), so
///    this is the ordinary answer wherever identity coherence has landed.
/// 3. The `agent_cid` itself — correct on any node whose Agent EPR was keyed by
///    its agent key rather than a slug.
///
/// Returns candidates in order; the caller stops at the first that signs. An
/// empty result, or one where every candidate is refused, is a dormant mint —
/// honest, not a failure.
pub fn agent_epr_gate_candidates(conn: &mut SqliteConnection, agent_cid: &str) -> Vec<String> {
    use crate::db::diesel_schema::humans::dsl as h;

    let mut out: Vec<String> = Vec::new();
    if let Ok(explicit) = std::env::var(MINT_AGENT_EPR_ENV) {
        let explicit = explicit.trim().to_string();
        if !explicit.is_empty() {
            out.push(explicit);
        }
    }
    match h::humans
        .filter(h::agent_pub_key.eq(agent_cid))
        .select(h::id)
        .load::<String>(conn)
    {
        Ok(ids) => out.extend(ids),
        Err(e) => debug!(
            error = %e,
            "binding mint: humans lookup for the Agent-EPR gate candidate failed"
        ),
    }
    out.push(agent_cid.to_string());
    out.dedup();
    out
}

/// Ask the conductor to sign the agent half, trying each gate candidate until
/// one is accepted.
///
/// With the `sign_for_agent` carve-out CLOSED (this slice), a candidate that
/// resolves no Agent EPR is REFUSED by the zome rather than silently signed — so
/// the loop is how a node discovers which of its identities the conductor will
/// actually vouch for, and [`MintError::NoAgentEpr`] is the honest answer when
/// none will.
async fn sign_agent_half(
    hc: &HcClient,
    candidates: &[String],
    preimage: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), MintError> {
    for candidate in candidates {
        let input = crate::signing::SignForAgentInput {
            agent_cid: candidate.clone(),
            canonical_bytes: preimage.to_vec(),
        };
        let payload = match rmp_serde::to_vec_named(&input) {
            Ok(p) => p,
            Err(e) => {
                warn!(error = %e, "binding mint: encode SignForAgentInput failed");
                continue;
            }
        };
        match hc
            .call_zome_imagodei(IMAGODEI_ZOME, SIGN_FOR_AGENT_FN, payload)
            .await
        {
            Ok(bytes) => {
                match rmp_serde::from_slice::<crate::signing::SignForAgentOutput>(&bytes) {
                    Ok(out) => {
                        debug!(
                            agent_epr = %candidate,
                            "binding mint: conductor signed the agent half"
                        );
                        return Ok((out.signature, out.signer_pubkey));
                    }
                    Err(e) => warn!(
                        agent_epr = %candidate,
                        error = %e,
                        "binding mint: decode SignForAgentOutput failed"
                    ),
                }
            }
            Err(e) => debug!(
                agent_epr = %candidate,
                error = %e,
                "binding mint: sign_for_agent refused this Agent-EPR candidate"
            ),
        }
    }
    Err(MintError::NoAgentEpr)
}

/// Commit the sealed binding to the DHT.
///
/// The signature field carries the envelope's UTF-8 bytes. From there the
/// existing pipeline needs no change at all: the coordinator emits
/// `AgentPeerBindingCreated`, `ReconcileController::on_agent_peer_binding`
/// projects and classifies it, and the gossip re-publish forwards the real
/// signature to every peer — each of which runs the same chokepoint.
async fn commit_binding(
    hc: &HcClient,
    sealed: &SealedBinding,
    device_archetype: &str,
    now: DateTime<Utc>,
    validity_days: i64,
) -> Result<String, MintError> {
    let input = CreateAgentPeerBindingInput {
        peer_id: sealed.core.transport_id.clone(),
        agent_cid: sealed.core.agent_cid.clone(),
        valid_from_micros: now.timestamp_micros(),
        valid_until_micros: Some((now + Duration::days(validity_days)).timestamp_micros()),
        device_archetype: device_archetype.to_string(),
        signature: sealed.envelope.clone().into_bytes(),
    };
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        MintError::Storage(StorageError::Serialization(format!(
            "binding mint: encode CreateAgentPeerBindingInput: {e}"
        )))
    })?;
    let bytes = hc
        .call_zome_imagodei(IMAGODEI_ZOME, CREATE_BINDING_FN, payload)
        .await?;
    let out: CreateAgentPeerBindingOutput = rmp_serde::from_slice(&bytes).map_err(|e| {
        MintError::Storage(StorageError::Serialization(format!(
            "binding mint: decode CreateAgentPeerBindingOutput: {e}"
        )))
    })?;
    Ok(out.action_hash.as_str().to_string())
}

// =============================================================================
// The orchestrator
// =============================================================================

/// Mint this node's own cross-signed binding, once.
///
/// Idempotent: returns [`MintOutcome::AlreadyCurrent`] without touching the
/// conductor when the projection already holds a current cross-signed row for
/// `(agent_cid, own peer id)`.
///
/// `agent_cid` MUST be the value this node already claims for itself in the
/// identity handshake (`NodeIdentity::agent_pubkey()`) — passing a parallel
/// notion of self is exactly how the namespaces fragment.
pub async fn mint_own_binding(
    hc: &HcClient,
    pool: &DbPool,
    agent_cid: &str,
    transport_keypair: &libp2p::identity::Keypair,
    device_archetype: &str,
    now: DateTime<Utc>,
) -> Result<MintOutcome, MintError> {
    let transport_id = libp2p::PeerId::from(transport_keypair.public()).to_base58();

    // Cheap local refusal before any conductor work: an agent_cid we cannot bind
    // to a key can never produce a cross_signed row, so attempting the mint
    // would burn two zome calls to fail the self-check.
    if agent_pubkey_from_agent_cid(agent_cid).is_none() {
        warn!(
            agent_cid = %agent_cid,
            "binding mint: advertised agent_cid is not a HoloHash AgentPubKey — a proof \
             over it cannot be verified until the C2-S6 resolver lands; keeping the \
             self-asserted binding"
        );
        return Err(MintError::AgentCidMismatch);
    }

    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Database(format!("binding mint: pool checkout: {e}")))?;

    let remint_after = rfc3339(now + Duration::days(REMINT_LEAD_DAYS));
    if crate::db::peer_identity_bindings::has_current_cross_signed(
        &mut conn,
        agent_cid,
        &transport_id,
        &remint_after,
    )? {
        debug!(
            agent_cid = %agent_cid,
            peer_id = %transport_id,
            "binding mint: a current cross-signed binding already exists — skipping"
        );
        return Ok(MintOutcome::AlreadyCurrent);
    }

    let candidates = agent_epr_gate_candidates(&mut conn, agent_cid);
    drop(conn);

    let core = assemble_core(agent_cid, &transport_id, now, DEFAULT_VALIDITY_DAYS);
    let (agent_signature, agent_pubkey) =
        sign_agent_half(hc, &candidates, &agent_half_preimage(&core)).await?;
    let sealed = seal_proof(&core, transport_keypair, &agent_signature, &agent_pubkey)?;
    let action_hash =
        commit_binding(hc, &sealed, device_archetype, now, DEFAULT_VALIDITY_DAYS).await?;

    info!(
        agent_cid = %agent_cid,
        peer_id = %transport_id,
        action_hash = %action_hash,
        valid_until = ?sealed.core.valid_until,
        "binding mint: committed a CROSS-SIGNED AgentPeerBinding — this node's \
         transport identity is now cryptographically bound, not self-asserted"
    );
    Ok(MintOutcome::Minted(action_hash))
}

/// Is minting enabled for this process? Read ONCE at startup by the caller.
pub fn mint_enabled_from_env() -> bool {
    match std::env::var(MINT_ENABLED_ENV) {
        Ok(v) => !matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "off" | "0" | "false" | "no"
        ),
        Err(_) => true,
    }
}

/// Retry schedule for the mint task, in seconds. Bounded and short: the
/// conductor's cells settle within the first minute of pod boot, and a node that
/// still cannot mint after this has a real identity-coherence gap that retrying
/// will not close. Stopping loudly beats a forever-loop against a conductor.
const MINT_RETRY_BACKOFF_SECS: [u64; 6] = [5, 15, 45, 120, 300, 600];

/// Spawn the boot-time mint task.
///
/// Runs AFTER the swarm identity and the conductor connection are up (the caller
/// owns that ordering). Retries on transient failure until the projection shows
/// the cross-signed row; gives up loudly on
/// [`MintError::AgentCidMismatch`]/[`MintError::NoAgentEpr`], which retrying
/// cannot fix.
pub fn spawn_binding_mint_task(
    hc: Arc<HcClient>,
    pool: DbPool,
    agent_cid: String,
    transport_keypair: libp2p::identity::Keypair,
    device_archetype: String,
) {
    tokio::spawn(async move {
        for (attempt, backoff) in MINT_RETRY_BACKOFF_SECS.iter().enumerate() {
            match mint_own_binding(
                &hc,
                &pool,
                &agent_cid,
                &transport_keypair,
                &device_archetype,
                Utc::now(),
            )
            .await
            {
                Ok(MintOutcome::AlreadyCurrent) => {
                    debug!("binding mint: already current — task done");
                    return;
                }
                Ok(MintOutcome::Minted(_)) => return,
                // Terminal: no amount of retrying resolves an identity the
                // conductor will not vouch for. The node keeps its honest
                // self-asserted binding.
                Err(e @ (MintError::AgentCidMismatch | MintError::NoAgentEpr)) => {
                    warn!(
                        error = %e,
                        "binding mint: cannot mint on this node — the transport binding stays \
                         self-asserted (unverified), which is honest. Set \
                         ELOHIM_BINDING_MINT_AGENT_EPR if an Agent EPR exists under another id."
                    );
                    return;
                }
                Err(e) => {
                    warn!(
                        attempt = attempt + 1,
                        retry_in_secs = backoff,
                        error = %e,
                        "binding mint: attempt failed — retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(*backoff)).await;
                }
            }
        }
        warn!(
            "binding mint: gave up after {} attempts — this node's binding stays \
             self-asserted; economic attribution for it will count as unverified",
            MINT_RETRY_BACKOFF_SECS.len()
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    use crate::p2p::binding_proof_wire::agent_cid_from_agent_pubkey;

    fn transport_keypair() -> libp2p::identity::Keypair {
        libp2p::identity::Keypair::ed25519_from_bytes([11u8; 32]).expect("keypair")
    }

    fn agent_key() -> SigningKey {
        SigningKey::from_bytes(&[12u8; 32])
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-18T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn own_core() -> BindingCore {
        let agent_cid = agent_cid_from_agent_pubkey(&agent_key().verifying_key().to_bytes());
        let transport_id = libp2p::PeerId::from(transport_keypair().public()).to_base58();
        assemble_core(&agent_cid, &transport_id, now(), DEFAULT_VALIDITY_DAYS)
    }

    /// The honest conductor: signs the preimage it was given with the agent key.
    fn honest_agent_half(core: &BindingCore) -> (Vec<u8>, Vec<u8>) {
        let sk = agent_key();
        (
            sk.sign(&agent_half_preimage(core)).to_bytes().to_vec(),
            sk.verifying_key().to_bytes().to_vec(),
        )
    }

    #[test]
    fn a_minted_proof_classifies_cross_signed_through_the_real_chokepoint() {
        let core = own_core();
        let (sig, pk) = honest_agent_half(&core);
        let sealed = seal_proof(&core, &transport_keypair(), &sig, &pk).expect("seal");

        // The assertion that matters: not "we built a struct", but "a receiving
        // peer running the shipped chokepoint would record this as proven".
        assert!(
            classify_binding_signature(
                &core.agent_cid,
                &core.transport_id,
                core.transport_kind,
                &core.valid_from,
                core.valid_until.as_deref(),
                &sealed.envelope,
            )
            .is_cross_signed(),
            "the whole point of C2-S2 is that the thing we emit verifies on arrival"
        );
        assert!(sealed.envelope.starts_with("elohim:apb:1:"));
    }

    #[test]
    fn an_agent_signer_returning_garbage_produces_no_binding() {
        let core = own_core();
        let (_, pk) = honest_agent_half(&core);
        let garbage = vec![0xAAu8; 64];
        let err = seal_proof(&core, &transport_keypair(), &garbage, &pk)
            .expect_err("a bad agent half must not seal");
        assert!(
            matches!(err, MintError::SelfVerify(_)),
            "expected self-verification to reject, got {err:?}"
        );
    }

    #[test]
    fn a_wrong_length_agent_half_is_rejected_by_length_not_by_panic() {
        let core = own_core();
        let (sig, _) = honest_agent_half(&core);
        assert!(matches!(
            seal_proof(&core, &transport_keypair(), &sig, &[0u8; 31]),
            Err(MintError::AgentHalfLength {
                field: "agent_pubkey",
                ..
            })
        ));
        assert!(matches!(
            seal_proof(&core, &transport_keypair(), &sig[..63], &[0u8; 32]),
            Err(MintError::AgentHalfLength {
                field: "agent_signature",
                ..
            })
        ));
    }

    /// A conductor that signs correctly but with a DIFFERENT key than the one
    /// `agent_cid` names — the identity-incoherence case. Signing succeeded, so
    /// only the derivation check catches it.
    #[test]
    fn a_signer_that_is_not_the_advertised_agent_is_refused() {
        let core = own_core();
        let other = SigningKey::from_bytes(&[99u8; 32]);
        let sig = other.sign(&agent_half_preimage(&core)).to_bytes().to_vec();
        let pk = other.verifying_key().to_bytes().to_vec();
        assert!(matches!(
            seal_proof(&core, &transport_keypair(), &sig, &pk),
            Err(MintError::AgentCidMismatch)
        ));
    }

    /// A core assembled against someone else's endpoint cannot be sealed with
    /// this node's transport key.
    #[test]
    fn a_core_naming_another_endpoint_is_refused() {
        let mut core = own_core();
        core.transport_id =
            libp2p::PeerId::from(libp2p::identity::Keypair::generate_ed25519().public())
                .to_base58();
        let (sig, pk) = honest_agent_half(&core);
        assert!(matches!(
            seal_proof(&core, &transport_keypair(), &sig, &pk),
            Err(MintError::TransportIdMismatch)
        ));
    }

    #[test]
    fn the_assembled_window_is_bounded_and_within_the_protocol_maximum() {
        let core = own_core();
        assert!(core.valid_until.is_some(), "never open-ended");
        // Compile-time: a minted window longer than the chokepoint's maximum
        // would be a dud by construction, and a re-mint lead longer than the
        // window would re-mint on every boot.
        const MAX: i64 = crate::p2p::binding_proof_wire::MAX_VALIDITY_DAYS;
        const { assert!(DEFAULT_VALIDITY_DAYS <= MAX) };
        const { assert!(REMINT_LEAD_DAYS < DEFAULT_VALIDITY_DAYS) };
    }

    #[test]
    fn nonces_are_fresh_and_at_least_sixteen_bytes() {
        let a = fresh_nonce();
        let b = fresh_nonce();
        assert_ne!(a, b, "a reused nonce would let two mints share a preimage");
        assert!(URL_SAFE_NO_PAD.decode(&a).expect("base64").len() >= 16);
    }

    #[test]
    fn the_kill_switch_reads_off_values_only() {
        // Pure parse check on the same match the env reader uses — asserted
        // without touching the process environment, so no parallel test can be
        // poisoned by a leaked `set_var`.
        let off = |v: &str| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "off" | "0" | "false" | "no"
            )
        };
        for v in ["off", "0", "false", "no", "OFF", " False "] {
            assert!(off(v), "{v} must disable minting");
        }
        for v in ["on", "1", "true", "", "yes"] {
            assert!(!off(v), "{v} must leave minting enabled");
        }
    }
}
