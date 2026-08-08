//! `HeadSetSnapshot` — signed transient head-set attestation (Path C, NO DHT
//! entry type): mint, verify, delta.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md`
//! §3 L5 (types landed in T6; mint/verify/delta + the wire carry are T8).
//!
//! ## The three invariants this file exists to hold
//!
//! - **C5 — evidence, not authority.** A receiver acts on what IT re-derives.
//!   [`verify_snapshot`] re-derives the snapshot's own `corpus_digest` from
//!   the snapshot's own `entries`, re-derives the snapshot's CID from the
//!   bytes it was handed, and compares the digest against the receiver's OWN
//!   [`crate::db::content_diesel::head_corpus_digest`]. Nothing the snapshot
//!   *claims* is taken on the signer's word.
//! - **C2 — monotone authority.** [`TrustEpoch`] is a named clock derived
//!   from the signer's attestation/citation edge set. An epoch that fails to
//!   advance is [`SnapshotRefusal::EpochRegression`]; an epoch pair this
//!   receiver cannot ORDER is [`SnapshotRefusal::EpochUnorderable`]. Both
//!   refuse. Neither is ever silently accepted, and the epoch check runs
//!   BEFORE the digest comparison, because a replayed old snapshot whose
//!   digest happens to match is exactly the attack the guard exists for.
//! - **Zero-cost digest path (the L2 join).** When the receiver's own
//!   head-corpus digest equals the snapshot's, the receiver is already in
//!   sync with the signer — accepted on the receiver's own evidence, at no
//!   verification cost beyond one local digest it already computes for
//!   `ViewFederationRequest::head_corpus_digest`.
//!
//! ## Signing honesty (read before trusting a verdict)
//!
//! Verification here is REAL: the ed25519 public key is read out of the
//! `uhCAk…` `signer_agent_cid` and checked against a detached signature over
//! the snapshot's canonical dag-cbor bytes, through the canonical
//! [`elohim_epr::proof::verify`].
//!
//! **Minting a signed snapshot is not reachable from this crate.** The
//! Holochain agent private key lives in the conductor keystore; there is no
//! `sign_bytes` extern and no storage-side handle to it (the only key this
//! process holds is the libp2p transport keypair, a DIFFERENT identity
//! namespace — see the crate CLAUDE.md's identity table). So
//! [`crate::trust::snapshot_source::UnavailableSnapshotSigner`] — the
//! production signer — always refuses, and [`mint_snapshot`] emits a snapshot
//! with `proof: None`.
//!
//! That is why acceptance is a two-valued vocabulary. A digest match with no
//! verifiable proof is [`SnapshotAcceptance::UnprovenSigner`]: the CONTENT
//! claim is accepted (the receiver re-derived it), the PROVENANCE claim is
//! explicitly NOT verified and must never be recorded as "verified by
//! `<signer>`". Only [`SnapshotAcceptance::ProvenSigner`] means a signature
//! was checked and passed.

use base64::Engine;
use cid::Cid;
use seam_contracts::ReasonLabel;
use serde::{Deserialize, Serialize};

use crate::trust::snapshot_source::{SnapshotSigner, SnapshotSource, SnapshotSourceError};

// ── Entries ─────────────────────────────────────────────────────────────────

/// One entry in a [`HeadSetSnapshot`] — a content id paired with the
/// head-digest the signer attests to for it.
///
/// Field order is DAG-CBOR canonical (length-first, then bytewise) so the
/// encoded map keys are already sorted — see [`HeadSetSnapshot`]'s note.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadSetEntry {
    pub id: String,
    /// The head this entry attests for `id`. On the head plane this is the
    /// row's `dht_anchor_hash` — the same value
    /// [`crate::db::content_diesel::head_corpus_digest`] folds.
    pub head_digest: String,
}

/// The `{id}={head_digest}` entry line the head-plane corpus digest folds.
///
/// This formatting must stay byte-identical to
/// [`crate::db::content_diesel::head_corpus_digest`]'s — a snapshot whose
/// declared `corpus_digest` cannot be re-derived from its own entries is
/// refused as [`SnapshotRefusal::Malformed`], so a drift here would refuse
/// every honest snapshot rather than accept a dishonest one (fail-closed).
/// The agreement is pinned by
/// `mint_corpus_digest_equals_the_db_head_corpus_digest`, which runs both
/// derivations over one seeded database.
fn entry_line(entry: &HeadSetEntry) -> String {
    format!("{}={}", entry.id, entry.head_digest)
}

/// Fold a head-set into the head-plane corpus digest, reusing the ONE
/// sort+hash implementation ([`crate::p2p::reconcile_rails::digest_of_entry_lines`])
/// that both the sync-round corpus digest and the SQL head corpus digest use.
#[must_use]
pub fn corpus_digest_of_entries(entries: &[HeadSetEntry]) -> String {
    crate::p2p::reconcile_rails::digest_of_entry_lines(entries.iter().map(entry_line).collect())
}

// ── Epoch (C2) ──────────────────────────────────────────────────────────────

/// An opaque epoch marker — a token, never a bare numeric type. Two reasons:
/// (1) consistency with [`crate::trust::memo::VerificationMemo`]'s "no
/// numeric field, ever" discipline (§4.2 derived-not-stored) — the same
/// `TrustEpoch` type is reused on both sides of the join so there is exactly
/// one place the C2 regression check can be written; (2) the clock this
/// token names is derived from the signer's attestation/citation edge set,
/// not an incrementing counter this crate owns or mints.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TrustEpoch(pub String);

/// Textual scheme prefix of a v1 epoch token. Bump only when the derivation
/// below changes; [`compare_epochs`] refuses to order tokens whose scheme it
/// does not know rather than guessing.
pub const EPOCH_SCHEME_V1_PREFIX: &str = "epoch:v1:";

/// The signer's attestation/citation edge set — the substrate the epoch clock
/// is derived FROM.
///
/// **Inert today, and this type says so.** The standing/edge machinery has no
/// writer until T19's `standing_projector` (plan §2, evidence correction #1),
/// so the production [`crate::trust::snapshot_source::SnapshotSource`] loads
/// [`SignerEdgeSet::inert`] — an EMPTY edge set. The consequence is stated
/// plainly rather than papered over: every signer's epoch token is then the
/// same zero-count token, so [`compare_epochs`] can only ever answer `Same`,
/// and the C2 guard, while structurally live and tested, cannot fire in
/// production. What T19 upgrades: it supplies real edges, at which point
/// count advances are real advances, and a shrinking edge set (revocation)
/// becomes a real, refusable regression.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignerEdgeSet {
    /// Opaque edge identifiers (attestation / citation references). Order is
    /// irrelevant — the digest sorts.
    pub edges: Vec<String>,
}

impl SignerEdgeSet {
    /// The honest empty edge set: what a production source can derive today.
    #[must_use]
    pub fn inert() -> Self {
        Self { edges: Vec::new() }
    }

    #[must_use]
    pub fn new(edges: Vec<String>) -> Self {
        Self { edges }
    }

    /// Stable fingerprint of the edge set — the `edge_set_digest` that pairs
    /// with the epoch token. Same fold as every other digest on this plane.
    #[must_use]
    pub fn digest(&self) -> String {
        crate::p2p::reconcile_rails::digest_of_entry_lines(self.edges.clone())
    }

    /// Derive the epoch token: `epoch:v1:<16-digit edge count>:<edge set digest>`.
    ///
    /// The count is zero-padded so the token's textual order matches its
    /// numeric order, and the digest is carried so two edge sets of equal
    /// cardinality are distinguishable — an equal count with a DIFFERENT
    /// digest is a lateral move that cardinality cannot order, and
    /// [`compare_epochs`] answers `Unorderable` rather than inventing a
    /// direction.
    #[must_use]
    pub fn epoch(&self) -> TrustEpoch {
        TrustEpoch(format!(
            "{EPOCH_SCHEME_V1_PREFIX}{:016}:{}",
            self.edges.len(),
            self.digest()
        ))
    }
}

/// How a candidate epoch relates to the last one verified from the same signer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochOrdering {
    /// The signer's edge set grew — the clock advanced.
    Advanced,
    /// Byte-identical epoch — a re-issue, not a regression. Admitted: a
    /// signer re-advertising the same standing has not gone backwards.
    Same,
    /// The signer's edge set SHRANK. C2 refuses.
    Regressed,
    /// The two tokens cannot be ordered by this receiver: an unknown scheme,
    /// or equal cardinality over different edge sets. Fail-closed — refusing
    /// is the only honest answer when the direction is underivable.
    Unorderable,
}

/// Parse a v1 epoch token into `(edge_count, edge_set_digest)`.
fn parse_epoch_v1(epoch: &TrustEpoch) -> Option<(u64, &str)> {
    let rest = epoch.0.strip_prefix(EPOCH_SCHEME_V1_PREFIX)?;
    let (count, digest) = rest.split_once(':')?;
    if digest.is_empty() {
        return None;
    }
    Some((count.parse().ok()?, digest))
}

/// Order a candidate epoch against the last one verified from the same signer.
///
/// **Concerns:** C2 (monotone authority). This is the decision predicate T12
/// registers as a seam row; it is pure and total by construction.
#[must_use]
pub fn compare_epochs(previous: &TrustEpoch, candidate: &TrustEpoch) -> EpochOrdering {
    let (Some((prev_count, prev_digest)), Some((next_count, next_digest))) =
        (parse_epoch_v1(previous), parse_epoch_v1(candidate))
    else {
        return EpochOrdering::Unorderable;
    };
    match next_count.cmp(&prev_count) {
        std::cmp::Ordering::Greater => EpochOrdering::Advanced,
        std::cmp::Ordering::Less => EpochOrdering::Regressed,
        std::cmp::Ordering::Equal if next_digest == prev_digest => EpochOrdering::Same,
        // Equal cardinality, different edges: a lateral move. Cardinality is
        // not a total order over edge SETS, and this receiver will not
        // fabricate one it cannot derive. T19's standing lineage is what
        // eventually orders these.
        std::cmp::Ordering::Equal => EpochOrdering::Unorderable,
    }
}

// ── Snapshot ────────────────────────────────────────────────────────────────

/// A signed, transient attestation of a corpus's head set — evidence, not
/// authority (C5): a receiver re-derives everything it acts on rather than
/// trusting this snapshot's claims outright. Never a DHT entry type; carried
/// on the wire inside a [`CarriedSnapshot`].
///
/// **Field order is DAG-CBOR canonical** (length-first on the camelCase key,
/// then bytewise: `entries` 7, `trustEpoch` 10, `corpusDigest` 12,
/// `edgeSetDigest` 13, `signerAgentCid` 14). `serde_ipld_dagcbor` emits
/// struct fields in declaration order, so declaring them in canonical order
/// is what makes [`HeadSetSnapshot::canonical_bytes`] genuinely canonical —
/// pinned by `canonical_bytes_are_strict_canonical_dag_cbor`, which runs the
/// canonical checker `elohim_epr::cbor::decode_strict` over the output.
/// Reordering these fields changes every CID on this plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadSetSnapshot {
    pub entries: Vec<HeadSetEntry>,
    /// Named clock derived from the signer's attestation/citation edge set.
    /// Regression (an epoch that does not advance relative to the last one
    /// verified from this signer) REFUSES — see
    /// [`SnapshotRefusal::EpochRegression`].
    pub trust_epoch: TrustEpoch,
    /// The L2 join key — a peer whose own `head_corpus_digest` equals this
    /// value is already in sync with the signer, at zero verification cost.
    /// Self-describing: it must be re-derivable from `entries`, and a
    /// snapshot where it is not is [`SnapshotRefusal::Malformed`].
    pub corpus_digest: String,
    /// The digest of the edge set `trust_epoch` was derived from — carried so
    /// a receiver (and T19's standing view) can tell WHICH edge set the clock
    /// names, not merely how many edges it had.
    pub edge_set_digest: String,
    pub signer_agent_cid: String,
}

impl HeadSetSnapshot {
    /// The canonical form: entries sorted by `(id, head_digest)`.
    ///
    /// A head SET has no inherent order, so the address must not depend on
    /// one. Minting and CID computation both canonicalize, which is what
    /// makes "same inputs ⇒ same CID" true regardless of the order the
    /// database handed the rows back.
    #[must_use]
    pub fn canonicalized(&self) -> Self {
        let mut out = self.clone();
        out.entries.sort();
        out
    }

    /// Canonical DAG-CBOR bytes — the preimage BOTH the CID and the detached
    /// signature commit to.
    ///
    /// # Errors
    /// Returns [`SnapshotCodecError::Encode`] if dag-cbor encoding fails.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, SnapshotCodecError> {
        serde_ipld_dagcbor::to_vec(&self.canonicalized())
            .map_err(|e| SnapshotCodecError::Encode(e.to_string()))
    }

    /// The snapshot's CID: CIDv1, dag-cbor (0x71), sha2-256 — minted through
    /// the canonical [`elohim_epr::cid::compute_cid`] rather than a
    /// re-derivation of the same three lines (the BritCid/BlobCid divergence
    /// precedent). Byte-for-byte the `epr_codec::encode_epr_head` pattern.
    ///
    /// # Errors
    /// Returns [`SnapshotCodecError::Encode`] if dag-cbor encoding fails.
    pub fn cid(&self) -> Result<Cid, SnapshotCodecError> {
        Ok(elohim_epr::cid::compute_cid(&self.canonical_bytes()?))
    }

    /// Re-derive `corpus_digest` from `entries` — the C5 self-consistency
    /// check. A snapshot that fails it describes a corpus it did not carry.
    #[must_use]
    pub fn derived_corpus_digest(&self) -> String {
        corpus_digest_of_entries(&self.entries)
    }
}

/// A detached authorship proof over a snapshot's [`HeadSetSnapshot::canonical_bytes`].
///
/// The signer's identity is NOT repeated here — it is
/// [`HeadSetSnapshot::signer_agent_cid`], and the verifying key is read out of
/// it, so a proof cannot name a different signer than the snapshot does.
///
/// Field order is DAG-CBOR canonical (`algorithm` 9, `signature` 9,
/// `schemeVersion` 13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotProof {
    /// Signing algorithm identifier. Only `ed25519` verifies today; anything
    /// else refuses as [`SnapshotRefusal::UnknownSigner`] rather than being
    /// waved through.
    pub algorithm: String,
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
    /// Must equal [`SNAPSHOT_SCHEME_VERSION`]. Bump only on a breaking change
    /// to the signing preimage or the snapshot layout.
    pub scheme_version: u16,
}

/// The snapshot scheme version carried in every proof.
pub const SNAPSHOT_SCHEME_VERSION: u16 = 1;

/// Domain separator prefixed to the canonical bytes before signing, so a
/// snapshot signature can never be lifted into another elohim signing slot
/// (the same discipline as `p2p::binding_cross_signature`'s domain tags).
pub const SNAPSHOT_SIGNING_DOMAIN: &[u8] = b"elohim:head-set-snapshot:v1";

/// The exact bytes a signer signs: domain tag followed by the snapshot's
/// canonical dag-cbor bytes.
#[must_use]
pub fn signing_preimage(canonical_bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(SNAPSHOT_SIGNING_DOMAIN.len() + canonical_bytes.len());
    out.extend_from_slice(SNAPSHOT_SIGNING_DOMAIN);
    out.extend_from_slice(canonical_bytes);
    out
}

/// What actually rides the wire: a snapshot plus its optional proof.
///
/// The proof sits OUTSIDE the addressed object on purpose — the CID names the
/// head set, not who signed it, so the same head set has one address whether
/// or not a signature is attached.
///
/// Field order is DAG-CBOR canonical (`proof` 5, `snapshot` 8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CarriedSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof: Option<SnapshotProof>,
    pub snapshot: HeadSetSnapshot,
}

impl CarriedSnapshot {
    /// Encode for the `ProjectionInventoryPayload::head_set_snapshot` wire
    /// field: base64(canonical dag-cbor).
    ///
    /// # Errors
    /// Returns [`SnapshotCodecError::Encode`] if dag-cbor encoding fails.
    pub fn encode_wire(&self) -> Result<String, SnapshotCodecError> {
        let mut canonical = self.clone();
        canonical.snapshot = canonical.snapshot.canonicalized();
        let bytes = serde_ipld_dagcbor::to_vec(&canonical)
            .map_err(|e| SnapshotCodecError::Encode(e.to_string()))?;
        Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
    }

    /// Decode a wire-carried snapshot, enforcing canonical dag-cbor form.
    ///
    /// The strictness matters: the CID and the signature both commit to the
    /// canonical encoding, so accepting a non-canonical one would let a
    /// carrier hand over bytes that verify as one thing and address as
    /// another. The check reuses the canonical checker
    /// [`elohim_epr::cbor::decode_strict`] rather than a bespoke re-encode.
    ///
    /// # Errors
    /// Returns [`SnapshotCodecError`] on bad base64, undecodable cbor, or a
    /// non-canonical encoding.
    pub fn decode_wire(encoded: &str) -> Result<Self, SnapshotCodecError> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| SnapshotCodecError::Decode(format!("base64: {e}")))?;
        elohim_epr::cbor::decode_strict(&bytes)
            .map_err(|e| SnapshotCodecError::NonCanonical(e.to_string()))?;
        serde_ipld_dagcbor::from_slice(&bytes)
            .map_err(|e| SnapshotCodecError::Decode(format!("dag-cbor: {e}")))
    }
}

/// Encode/decode failures for the snapshot codec.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SnapshotCodecError {
    #[error("snapshot encode failed: {0}")]
    Encode(String),
    #[error("snapshot decode failed: {0}")]
    Decode(String),
    #[error("snapshot bytes are not canonical dag-cbor: {0}")]
    NonCanonical(String),
}

// ── Refusal taxonomy + verdict ──────────────────────────────────────────────

/// Why a snapshot verification refused to accept. Closed vocabulary per
/// [`seam_contracts::ReasonLabel`] (C8) — every refusal is countable and
/// nameable, and there is no "other" escape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotRefusal {
    /// The receiver's own corpus digest disagrees with the snapshot's
    /// declared digest — evidence-not-authority (C5): the snapshot does not
    /// describe this receiver's corpus.
    CorpusDigestMismatch,
    /// `trust_epoch` moved BACKWARDS relative to the last epoch verified
    /// from this signer (C2 — regression REFUSES, never silently accepted).
    EpochRegression,
    /// `trust_epoch` and the last verified epoch cannot be ordered by this
    /// receiver (unknown scheme, or a lateral edge-set change at equal
    /// cardinality). Fail-closed: an underivable direction is refused, never
    /// guessed. Distinct from [`Self::EpochRegression`] so the meter can tell
    /// "the signer went backwards" from "we cannot tell" — they call for
    /// different responses.
    EpochUnorderable,
    /// A proof was attached and did NOT verify: unknown algorithm, wrong
    /// scheme version, a `signer_agent_cid` no ed25519 key can be read out
    /// of, or a signature that fails against that key. An ABSENT proof is
    /// not this — see [`SnapshotAcceptance::UnprovenSigner`].
    UnknownSigner,
    /// The snapshot is self-inconsistent or structurally unusable: a
    /// `corpus_digest` that cannot be re-derived from its own `entries`,
    /// duplicate entry ids, blank required fields, an unparseable epoch
    /// token, or bytes that are not canonical dag-cbor.
    Malformed,
}

impl ReasonLabel for SnapshotRefusal {
    const ALL: &'static [Self] = &[
        SnapshotRefusal::CorpusDigestMismatch,
        SnapshotRefusal::EpochRegression,
        SnapshotRefusal::EpochUnorderable,
        SnapshotRefusal::UnknownSigner,
        SnapshotRefusal::Malformed,
    ];

    fn label(&self) -> &'static str {
        match self {
            SnapshotRefusal::CorpusDigestMismatch => "corpus_digest_mismatch",
            SnapshotRefusal::EpochRegression => "epoch_regression",
            SnapshotRefusal::EpochUnorderable => "epoch_unorderable",
            SnapshotRefusal::UnknownSigner => "unknown_signer",
            SnapshotRefusal::Malformed => "malformed",
        }
    }
}

/// On what basis a snapshot was accepted. Two values, because "the content
/// checks out" and "we know who said it" are different claims and conflating
/// them is how a system starts reporting verification that never happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotAcceptance {
    /// Digest match AND a detached proof that verified against the ed25519
    /// key read out of `signer_agent_cid`. Provenance is established.
    ProvenSigner,
    /// Digest match, no verifiable proof attached. The CONTENT claim is
    /// accepted on the receiver's OWN re-derivation (C5 — which is the whole
    /// basis of the zero-cost path); the PROVENANCE claim is NOT verified.
    /// Never record this as "verified by `<signer>`". This is what every
    /// snapshot minted today yields, because storage-side code cannot reach
    /// the agent key (see the module doc).
    UnprovenSigner,
}

impl ReasonLabel for SnapshotAcceptance {
    const ALL: &'static [Self] = &[
        SnapshotAcceptance::ProvenSigner,
        SnapshotAcceptance::UnprovenSigner,
    ];

    fn label(&self) -> &'static str {
        match self {
            SnapshotAcceptance::ProvenSigner => "proven_signer",
            SnapshotAcceptance::UnprovenSigner => "unproven_signer",
        }
    }
}

/// The outcome of verifying a [`HeadSetSnapshot`] against a receiver's local
/// state. Acceptance carries HOW it was accepted; the receiver still acts on
/// ITS OWN re-derived state, never on the snapshot's claims (C5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotVerdict {
    Accepted(SnapshotAcceptance),
    Refused(SnapshotRefusal),
}

impl SnapshotVerdict {
    /// Stable metric label for this verdict — the accepted/refused reason,
    /// prefixed so one counter can carry both halves without collision.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            SnapshotVerdict::Accepted(a) => format!("accepted_{}", a.label()),
            SnapshotVerdict::Refused(r) => format!("refused_{}", r.label()),
        }
    }

    #[must_use]
    pub fn is_accepted(&self) -> bool {
        matches!(self, SnapshotVerdict::Accepted(_))
    }
}

// ── Signer key derivation ───────────────────────────────────────────────────

/// Multihash prefix of a Holochain `AgentPubKey` (`uhCAk…`): the 3 bytes
/// `0x84 0x20 0x24`, whose base64url form is exactly `hCAk`.
const AGENT_PUB_KEY_PREFIX: [u8; 3] = [0x84, 0x20, 0x24];
/// `prefix(3) + ed25519 key core(32) + dht location(4)`.
const AGENT_PUB_KEY_LEN: usize = 39;

/// Read the ed25519 public key out of a `uhCAk…` agent cid.
///
/// A Holochain `AgentPubKey` IS the agent's ed25519 public key wrapped in a
/// multihash prefix and a 4-byte DHT-location suffix, multibase-`u`
/// (base64url, unpadded). So a receiver can verify an agent's signature with
/// nothing but the agent cid — no key registry, no resolver.
///
/// **The 4 location bytes are NOT validated here** (they are a blake2b-derived
/// routing checksum, and blake2b is not a dependency of this crate). That is
/// the correct security property, not a shortcut: the KEY is what signs, and
/// a signature verifies against the key core regardless of the routing
/// checksum appended to it. What it does mean is that this function is a key
/// READER, not an agent-cid VALIDATOR — do not use it to decide whether a
/// string is a well-formed agent cid.
#[must_use]
pub fn ed25519_pubkey_from_agent_cid(agent_cid: &str) -> Option<[u8; 32]> {
    let body = agent_cid.strip_prefix('u')?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(body)
        .ok()?;
    if bytes.len() != AGENT_PUB_KEY_LEN || bytes[..3] != AGENT_PUB_KEY_PREFIX {
        return None;
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes[3..35]);
    Some(key)
}

// ── Mint ────────────────────────────────────────────────────────────────────

/// A freshly minted snapshot plus the two artifacts derived from it.
#[derive(Debug, Clone)]
pub struct MintedSnapshot {
    pub carried: CarriedSnapshot,
    /// CIDv1 dag-cbor over [`Self::canonical_bytes`].
    pub cid: Cid,
    /// The exact bytes the CID addresses and any proof signs.
    pub canonical_bytes: Vec<u8>,
}

impl MintedSnapshot {
    /// Whether a detached proof is attached. `false` for everything minted in
    /// production today — see the module doc's signing-honesty section.
    #[must_use]
    pub fn is_signed(&self) -> bool {
        self.carried.proof.is_some()
    }
}

/// Why a mint could not produce a snapshot. A signer that cannot sign is NOT
/// one of these — an unsigned snapshot is a legitimate, honestly-labelled
/// artifact ([`SnapshotAcceptance::UnprovenSigner`] on the far side).
#[derive(Debug, thiserror::Error)]
pub enum MintError {
    #[error("snapshot source: {0}")]
    Source(#[from] SnapshotSourceError),
    #[error("{0}")]
    Codec(#[from] SnapshotCodecError),
}

/// Mint a [`HeadSetSnapshot`] from the local head plane.
///
/// The `corpus_digest` comes from the source (which reads
/// [`crate::db::content_diesel::head_corpus_digest_readiness`]) — the L2
/// join: the digest is the snapshot's self-description, not a second
/// derivation that could disagree with the one the peer advertises on
/// `ViewFederationRequest::head_corpus_digest`.
///
/// # Errors
/// Returns [`MintError`] if the source cannot produce inputs (including the
/// honest amber-window refusal) or the canonical encoding fails.
pub fn mint_snapshot(
    source: &dyn SnapshotSource,
    signer: &dyn SnapshotSigner,
) -> Result<MintedSnapshot, MintError> {
    let inputs = source.load()?;
    let snapshot = HeadSetSnapshot {
        entries: inputs.entries,
        trust_epoch: inputs.edge_set.epoch(),
        corpus_digest: inputs.corpus_digest,
        edge_set_digest: inputs.edge_set.digest(),
        signer_agent_cid: inputs.signer_agent_cid,
    }
    .canonicalized();

    let canonical_bytes = snapshot.canonical_bytes()?;
    let cid = elohim_epr::cid::compute_cid(&canonical_bytes);

    let proof = match signer.sign(&canonical_bytes) {
        Ok(p) => Some(p),
        Err(e) => {
            // Not an error: the snapshot is still useful evidence, it simply
            // carries no provenance claim. Logged at debug because in the
            // current fleet this is the EXPECTED path, not an incident.
            tracing::debug!(
                target: "elohim_storage::trust::snapshot",
                reason = %e,
                "head-set snapshot minted without a signer proof"
            );
            None
        }
    };

    Ok(MintedSnapshot {
        carried: CarriedSnapshot { proof, snapshot },
        cid,
        canonical_bytes,
    })
}

// ── Verify ──────────────────────────────────────────────────────────────────

/// Everything the receiver brings to a verification. All of it is the
/// RECEIVER's own state — the snapshot supplies claims, never inputs to its
/// own check (C5).
pub struct VerifyInput<'a> {
    pub carried: &'a CarriedSnapshot,
    /// The receiver's OWN [`crate::db::content_diesel::head_corpus_digest`]
    /// (the `Ready` one — a caller inside its amber window has no stable
    /// digest and should not be verifying).
    pub local_corpus_digest: &'a str,
    /// The last epoch this receiver ACCEPTED from this signer, if any. `None`
    /// means first contact: there is nothing to regress from, so the C2 check
    /// is vacuous rather than refusing.
    pub last_verified_epoch: Option<&'a TrustEpoch>,
}

/// Verify a carried snapshot against the receiver's own state.
///
/// **Concerns:** C5 (evidence-not-authority), C2 (monotone authority), C8
/// (every outcome is a labelled reason). This is the decision predicate T12
/// registers as a seam row.
///
/// Order is deliberate and is itself part of the contract:
/// 1. **Structure** — a self-inconsistent snapshot is refused before any of
///    its fields are believed enough to compare.
/// 2. **Epoch (C2)** — BEFORE the digest. A replayed old snapshot whose
///    digest happens to match is exactly what the monotone guard exists to
///    catch; checking the digest first would accept it.
/// 3. **Proof** — an attached proof that fails refuses even on a digest
///    match. A forged provenance claim is worse than none.
/// 4. **Digest** — the zero-cost path. Equality with the receiver's OWN
///    digest is the acceptance; inequality is
///    [`SnapshotRefusal::CorpusDigestMismatch`].
#[must_use]
pub fn verify_snapshot(input: &VerifyInput<'_>) -> SnapshotVerdict {
    let snapshot = &input.carried.snapshot;

    // (1) Structure / self-consistency.
    if snapshot.signer_agent_cid.trim().is_empty()
        || snapshot.corpus_digest.trim().is_empty()
        || snapshot.edge_set_digest.trim().is_empty()
    {
        return SnapshotVerdict::Refused(SnapshotRefusal::Malformed);
    }
    if parse_epoch_v1(&snapshot.trust_epoch).is_none() {
        return SnapshotVerdict::Refused(SnapshotRefusal::Malformed);
    }
    {
        let mut ids: Vec<&str> = snapshot.entries.iter().map(|e| e.id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        if ids.len() != before {
            return SnapshotVerdict::Refused(SnapshotRefusal::Malformed);
        }
    }
    if snapshot.derived_corpus_digest() != snapshot.corpus_digest {
        // The snapshot's own self-description does not describe what it
        // carried. Nothing downstream can be trusted to mean what it says.
        return SnapshotVerdict::Refused(SnapshotRefusal::Malformed);
    }

    // (2) Epoch — the C2 monotone-authority guard, before the digest.
    if let Some(previous) = input.last_verified_epoch {
        match compare_epochs(previous, &snapshot.trust_epoch) {
            EpochOrdering::Advanced | EpochOrdering::Same => {}
            EpochOrdering::Regressed => {
                return SnapshotVerdict::Refused(SnapshotRefusal::EpochRegression)
            }
            EpochOrdering::Unorderable => {
                return SnapshotVerdict::Refused(SnapshotRefusal::EpochUnorderable)
            }
        }
    }

    // (3) Proof, when one is attached.
    let acceptance = match &input.carried.proof {
        None => SnapshotAcceptance::UnprovenSigner,
        Some(proof) => {
            if !proof_verifies(snapshot, proof) {
                return SnapshotVerdict::Refused(SnapshotRefusal::UnknownSigner);
            }
            SnapshotAcceptance::ProvenSigner
        }
    };

    // (4) The zero-cost digest path: the receiver's OWN digest is the oracle.
    if snapshot.corpus_digest != input.local_corpus_digest {
        return SnapshotVerdict::Refused(SnapshotRefusal::CorpusDigestMismatch);
    }
    SnapshotVerdict::Accepted(acceptance)
}

/// Verify a detached proof against the key read out of the snapshot's own
/// `signer_agent_cid`. Uses the canonical [`elohim_epr::proof::verify`] rather
/// than a second ed25519 call site.
///
/// (Non-strict ed25519 verification is deliberate here: the canonical helper
/// is the reuse target, and nothing on this plane is keyed on signature bytes,
/// so malleability grants an attacker no distinct artifact — unlike
/// `p2p::binding_cross_signature`, where the proof itself is the entry.)
fn proof_verifies(snapshot: &HeadSetSnapshot, proof: &SnapshotProof) -> bool {
    if proof.scheme_version != SNAPSHOT_SCHEME_VERSION || proof.algorithm != "ed25519" {
        return false;
    }
    let Some(pubkey) = ed25519_pubkey_from_agent_cid(&snapshot.signer_agent_cid) else {
        return false;
    };
    let Ok(canonical) = snapshot.canonical_bytes() else {
        return false;
    };
    elohim_epr::proof::verify(&pubkey, &signing_preimage(&canonical), &proof.signature)
}

/// Decode a wire-carried snapshot and verify it in one step — the receiver-side
/// entry point for `ProjectionInventoryPayload::head_set_snapshot`.
///
/// Returns [`SnapshotRefusal::Malformed`] for anything that will not decode as
/// canonical dag-cbor, so a caller has exactly one verdict vocabulary to log.
///
/// **This returns a verdict; it never acts on one.** Wiring a verdict to head
/// adoption is later work by design (plan §3 L5) — an accepted snapshot today
/// changes nothing but a log line.
#[must_use]
pub fn verify_wire_snapshot(
    encoded: &str,
    local_corpus_digest: &str,
    last_verified_epoch: Option<&TrustEpoch>,
) -> SnapshotVerdict {
    match CarriedSnapshot::decode_wire(encoded) {
        Err(e) => {
            tracing::warn!(
                target: "elohim_storage::trust::snapshot",
                error = %e,
                "carried head-set snapshot did not decode; refusing as malformed"
            );
            SnapshotVerdict::Refused(SnapshotRefusal::Malformed)
        }
        Ok(carried) => verify_snapshot(&VerifyInput {
            carried: &carried,
            local_corpus_digest,
            last_verified_epoch,
        }),
    }
}

/// Verify a wire-carried snapshot against the receiver's head-corpus
/// READINESS rather than a bare digest — the amber-safe call site.
///
/// Returns `None` when the receiver's own corpus is
/// [`crate::db::content_diesel::HeadCorpusDigestReadiness::Amber`]: during the
/// bulk-seed witness window the relation this receiver would compare against
/// is still changing on every tick, so it has no stable oracle and must
/// ABSTAIN. Abstaining is not a refusal and not an acceptance — it means "ask
/// again when the corpus settles", and the caller falls through to the
/// ordinary verification path. That is the same fail-toward-re-verification
/// posture the responder takes when it declines to answer `in_sync`.
///
/// Prefer this over calling [`verify_wire_snapshot`] with a digest computed
/// some other way: the amber gate is structural here, so a call site cannot
/// accidentally fire the zero-cost accept over a half-witnessed corpus.
#[must_use]
pub fn verify_wire_snapshot_for_readiness(
    encoded: &str,
    readiness: &crate::db::content_diesel::HeadCorpusDigestReadiness,
    last_verified_epoch: Option<&TrustEpoch>,
) -> Option<SnapshotVerdict> {
    use crate::db::content_diesel::HeadCorpusDigestReadiness;
    match readiness {
        HeadCorpusDigestReadiness::Amber { pending } => {
            tracing::info!(
                target: "elohim_storage::trust::snapshot",
                pending,
                "carried head-set snapshot not verified: local corpus is amber; abstaining"
            );
            None
        }
        HeadCorpusDigestReadiness::Ready { digest, .. } => {
            Some(verify_wire_snapshot(encoded, digest, last_verified_epoch))
        }
    }
}

// ── Delta ───────────────────────────────────────────────────────────────────

/// One id both sides hold, at different heads.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeadDivergence {
    pub id: String,
    pub snapshot_head_digest: String,
    pub local_head_digest: String,
}

/// What differs between a snapshot's head set and the receiver's own — the
/// input a future reconcile consumer acts on.
///
/// Deliberately three disjoint buckets, not one "gaps" list: "the peer has a
/// head I have never seen", "I have a head the peer has not", and "we disagree
/// about the same id" call for different reconcile moves, and collapsing them
/// is how a divergence gets healed as an absence.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeadSetDelta {
    /// In the snapshot, absent locally.
    pub only_in_snapshot: Vec<HeadSetEntry>,
    /// Held locally, absent from the snapshot.
    pub only_local: Vec<HeadSetEntry>,
    /// Same id, different head digest.
    pub divergent: Vec<HeadDivergence>,
}

impl HeadSetDelta {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.only_in_snapshot.is_empty() && self.only_local.is_empty() && self.divergent.is_empty()
    }

    /// Total differing ids across all three buckets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.only_in_snapshot.len() + self.only_local.len() + self.divergent.len()
    }
}

/// Compute the delta between a snapshot's head set and the local one. PURE:
/// no clock, no database, no I/O — the caller supplies both sides.
///
/// Output is sorted by id in every bucket, so a delta is comparable and
/// loggable without the caller re-sorting. Duplicate ids on either side are
/// last-wins; a verified snapshot cannot contain them
/// ([`SnapshotRefusal::Malformed`]).
///
/// **Concerns:** C5 — this is the shape a receiver acts on, computed from its
/// own state on one side. This is the decision predicate T12 registers as a
/// seam row.
#[must_use]
pub fn head_set_delta(snapshot_entries: &[HeadSetEntry], local: &[HeadSetEntry]) -> HeadSetDelta {
    use std::collections::BTreeMap;

    let local_by_id: BTreeMap<&str, &str> = local
        .iter()
        .map(|e| (e.id.as_str(), e.head_digest.as_str()))
        .collect();
    let snapshot_by_id: BTreeMap<&str, &str> = snapshot_entries
        .iter()
        .map(|e| (e.id.as_str(), e.head_digest.as_str()))
        .collect();

    let mut delta = HeadSetDelta::default();
    for (id, snapshot_head) in &snapshot_by_id {
        match local_by_id.get(id) {
            None => delta.only_in_snapshot.push(HeadSetEntry {
                id: (*id).to_string(),
                head_digest: (*snapshot_head).to_string(),
            }),
            Some(local_head) if local_head != snapshot_head => {
                delta.divergent.push(HeadDivergence {
                    id: (*id).to_string(),
                    snapshot_head_digest: (*snapshot_head).to_string(),
                    local_head_digest: (*local_head).to_string(),
                });
            }
            Some(_) => {}
        }
    }
    for (id, local_head) in &local_by_id {
        if !snapshot_by_id.contains_key(id) {
            delta.only_local.push(HeadSetEntry {
                id: (*id).to_string(),
                head_digest: (*local_head).to_string(),
            });
        }
    }
    delta
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::snapshot_source::{
        FixedSnapshotSource, LocalKeySnapshotSigner, SnapshotInputs, UnavailableSnapshotSigner,
    };

    // ── helpers ─────────────────────────────────────────────────────────

    fn entries(pairs: &[(&str, &str)]) -> Vec<HeadSetEntry> {
        pairs
            .iter()
            .map(|(id, head)| HeadSetEntry {
                id: (*id).to_string(),
                head_digest: (*head).to_string(),
            })
            .collect()
    }

    /// Encode a 32-byte ed25519 public key in the holo_hash `uhCAk…` textual
    /// form with the 4 DHT-location bytes ZEROED.
    ///
    /// TEST ONLY, and it must stay that way: the real location bytes are a
    /// blake2b-derived routing checksum this crate cannot compute, so this
    /// produces a string a conductor would never mint. It is admissible here
    /// because [`ed25519_pubkey_from_agent_cid`] reads only the key core (and
    /// documents that it does not validate the location) — which is exactly
    /// the property under test: a signature verifies against the KEY.
    fn test_agent_cid(pubkey: &[u8; 32]) -> String {
        let mut raw = Vec::with_capacity(AGENT_PUB_KEY_LEN);
        raw.extend_from_slice(&AGENT_PUB_KEY_PREFIX);
        raw.extend_from_slice(pubkey);
        raw.extend_from_slice(&[0u8; 4]);
        format!(
            "u{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw)
        )
    }

    fn test_keypair() -> elohim_epr::AgentKeypair {
        elohim_epr::AgentKeypair::from_secret(&[7u8; 32]).expect("32-byte seed")
    }

    fn fixed_source(
        entries: Vec<HeadSetEntry>,
        signer_agent_cid: &str,
        edge_set: SignerEdgeSet,
    ) -> FixedSnapshotSource {
        let corpus_digest = corpus_digest_of_entries(&entries);
        FixedSnapshotSource::new(SnapshotInputs {
            entries,
            corpus_digest,
            signer_agent_cid: signer_agent_cid.to_string(),
            edge_set,
        })
    }

    fn mint_unsigned(entries: Vec<HeadSetEntry>, signer: &str) -> MintedSnapshot {
        mint_snapshot(
            &fixed_source(entries, signer, SignerEdgeSet::inert()),
            &UnavailableSnapshotSigner,
        )
        .expect("mint")
    }

    // ── CID stability ───────────────────────────────────────────────────

    #[test]
    fn cid_is_dag_cbor_0x71_with_the_bafyrei_prefix() {
        let minted = mint_unsigned(entries(&[("c:a", "anc-a")]), "uSigner");
        assert_eq!(
            minted.cid.codec(),
            crate::epr_codec::DAG_CBOR_CODEC,
            "the snapshot CID must use the dag-cbor codec (0x71), never raw 0x55"
        );
        let s = minted.cid.to_string();
        assert!(
            s.starts_with("bafyrei"),
            "dag-cbor + sha2-256 CIDv1 must render as bafyrei…, got {s}"
        );
    }

    #[test]
    fn same_inputs_mint_the_same_cid_regardless_of_entry_order() {
        let forward = mint_unsigned(entries(&[("c:a", "anc-a"), ("c:b", "anc-b")]), "uSigner");
        let reverse = mint_unsigned(entries(&[("c:b", "anc-b"), ("c:a", "anc-a")]), "uSigner");
        assert_eq!(
            forward.cid, reverse.cid,
            "a head SET has no order; the address must not depend on one"
        );
        assert_eq!(forward.canonical_bytes, reverse.canonical_bytes);
    }

    #[test]
    fn any_entry_change_changes_the_cid() {
        let base = mint_unsigned(entries(&[("c:a", "anc-a")]), "uSigner");
        let head_changed = mint_unsigned(entries(&[("c:a", "anc-a2")]), "uSigner");
        let id_changed = mint_unsigned(entries(&[("c:a2", "anc-a")]), "uSigner");
        let added = mint_unsigned(entries(&[("c:a", "anc-a"), ("c:b", "anc-b")]), "uSigner");
        assert_ne!(base.cid, head_changed.cid, "a changed head must re-address");
        assert_ne!(base.cid, id_changed.cid, "a changed id must re-address");
        assert_ne!(base.cid, added.cid, "an added entry must re-address");
    }

    #[test]
    fn a_changed_digest_or_epoch_or_signer_changes_the_cid() {
        let base = mint_unsigned(entries(&[("c:a", "anc-a")]), "uSigner");
        let other_signer = mint_unsigned(entries(&[("c:a", "anc-a")]), "uOtherSigner");
        assert_ne!(
            base.cid, other_signer.cid,
            "the signer is part of the claim"
        );

        let mut epoch_changed = base.carried.snapshot.clone();
        epoch_changed.trust_epoch = SignerEdgeSet::new(vec!["edge:1".into()]).epoch();
        assert_ne!(
            base.cid,
            epoch_changed.cid().expect("cid"),
            "the epoch is part of the claim"
        );

        let mut digest_changed = base.carried.snapshot.clone();
        digest_changed.corpus_digest = "sha256:tampered".into();
        assert_ne!(
            base.cid,
            digest_changed.cid().expect("cid"),
            "the corpus digest is part of the claim"
        );
    }

    #[test]
    fn canonical_bytes_are_strict_canonical_dag_cbor() {
        // Field DECLARATION order is what makes the encoded map canonical
        // (serde_ipld_dagcbor does not sort struct fields). The canonical
        // checker is the oracle — reordering the struct fails here.
        let minted = mint_unsigned(entries(&[("c:a", "anc-a"), ("c:b", "anc-b")]), "uSigner");
        elohim_epr::cbor::decode_strict(&minted.canonical_bytes)
            .expect("snapshot canonical bytes must be canonical dag-cbor");

        let wire = minted.carried.encode_wire().expect("encode");
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&wire)
            .expect("b64");
        elohim_epr::cbor::decode_strict(&bytes)
            .expect("carried snapshot bytes must be canonical dag-cbor");
    }

    #[test]
    fn cid_matches_the_canonical_epr_codec_pattern() {
        // The snapshot CID is minted through elohim_epr::cid::compute_cid;
        // this pins that it is byte-for-byte the epr_codec::encode_epr_head
        // pattern (dag-cbor 0x71 + sha2-256 + CIDv1) and not a second
        // derivation that could drift from it.
        use multihash_codetable::{Code, MultihashDigest};
        let minted = mint_unsigned(entries(&[("c:a", "anc-a")]), "uSigner");
        let expected = Cid::new_v1(
            crate::epr_codec::DAG_CBOR_CODEC,
            Code::Sha2_256.digest(&minted.canonical_bytes),
        );
        assert_eq!(minted.cid, expected);
    }

    #[test]
    fn wire_encoding_round_trips() {
        let minted = mint_unsigned(entries(&[("c:a", "anc-a")]), "uSigner");
        let wire = minted.carried.encode_wire().expect("encode");
        let back = CarriedSnapshot::decode_wire(&wire).expect("decode");
        assert_eq!(back, minted.carried);
        assert_eq!(
            back.snapshot.cid().expect("cid"),
            minted.cid,
            "a decoded snapshot must re-derive the same address"
        );
    }

    // ── L2 join: the minted digest IS the db digest ─────────────────────

    #[test]
    fn mint_corpus_digest_equals_the_db_head_corpus_digest() {
        // The join the whole zero-cost path rests on: the digest a snapshot
        // self-describes with must be the SAME string
        // db::content_diesel::head_corpus_digest produces over the same
        // relation, and must be re-derivable from the snapshot's own entries.
        use crate::db::content_diesel::{create_content, CreateContentInput};
        let pool = crate::test_util::test_pool();
        let ctx = crate::db::AppContext::default_lamad();
        {
            let mut conn = pool.get().expect("pool conn");
            let mk = |id: &str, reach: &str, anchor: Option<&str>| CreateContentInput {
                id: id.to_string(),
                title: id.to_string(),
                description: None,
                content_type: "concept".to_string(),
                content_format: "markdown".to_string(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: reach.to_string(),
                created_by: None,
                tags: vec![],
                content_body: None,
                dht_anchor_hash: anchor.map(str::to_string),
            };
            create_content(&mut conn, &ctx, mk("snap:a", "public", Some("anc-a"))).unwrap();
            create_content(&mut conn, &ctx, mk("snap:b", "commons", Some("anc-b"))).unwrap();
            // Scoped reach must NOT enter the cross-peer relation.
            create_content(
                &mut conn,
                &ctx,
                mk("snap:private", "private", Some("anc-p")),
            )
            .unwrap();
        }

        let source = crate::trust::snapshot_source::LocalHeadPlaneSnapshotSource::new(
            pool.clone(),
            ctx.clone(),
            "uSigner".to_string(),
        );
        let minted = mint_snapshot(&source, &UnavailableSnapshotSigner).expect("mint");

        let db_digest = {
            let mut conn = pool.get().expect("pool conn");
            crate::db::content_diesel::head_corpus_digest(&mut conn, &ctx).expect("digest")
        };
        assert_eq!(
            minted.carried.snapshot.corpus_digest, db_digest,
            "the snapshot's self-description must be the db head corpus digest"
        );
        assert_eq!(
            minted.carried.snapshot.derived_corpus_digest(),
            db_digest,
            "and it must be re-derivable from the entries the snapshot carried \
             (the C5 self-consistency check)"
        );
        let ids: Vec<&str> = minted
            .carried
            .snapshot
            .entries
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        assert_eq!(
            ids,
            vec!["snap:a", "snap:b"],
            "a scoped-reach row must never enter a cross-peer snapshot"
        );
    }

    #[test]
    fn local_source_refuses_to_mint_during_the_amber_window() {
        use crate::db::content_diesel::{create_content, CreateContentInput};
        let pool = crate::test_util::test_pool();
        let ctx = crate::db::AppContext::default_lamad();
        {
            let mut conn = pool.get().expect("pool conn");
            let mk = |id: &str, anchor: Option<&str>| CreateContentInput {
                id: id.to_string(),
                title: id.to_string(),
                description: None,
                content_type: "concept".to_string(),
                content_format: "markdown".to_string(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "public".to_string(),
                created_by: None,
                tags: vec![],
                content_body: None,
                dht_anchor_hash: anchor.map(str::to_string),
            };
            create_content(&mut conn, &ctx, mk("snap:green", Some("anc-g"))).unwrap();
            create_content(&mut conn, &ctx, mk("snap:amber", None)).unwrap();
        }
        let source = crate::trust::snapshot_source::LocalHeadPlaneSnapshotSource::new(
            pool,
            ctx,
            "uSigner".into(),
        );
        let err = mint_snapshot(&source, &UnavailableSnapshotSigner)
            .expect_err("an amber corpus must refuse to mint");
        assert!(
            matches!(
                err,
                MintError::Source(SnapshotSourceError::CorpusAmber { pending: 1 })
            ),
            "expected CorpusAmber{{pending:1}}, got {err:?}"
        );
    }

    // ── Verify verdicts ─────────────────────────────────────────────────

    #[test]
    fn accepts_with_unproven_signer_on_digest_match_at_zero_cost() {
        // The zero-cost path: the receiver's OWN digest equals the
        // snapshot's, so it is already in sync — accepted on the receiver's
        // own evidence, with provenance HONESTLY marked unverified because
        // nothing storage-side can sign.
        let minted = mint_unsigned(entries(&[("c:a", "anc-a")]), "uSigner");
        let verdict = verify_snapshot(&VerifyInput {
            carried: &minted.carried,
            local_corpus_digest: &minted.carried.snapshot.corpus_digest,
            last_verified_epoch: None,
        });
        assert_eq!(
            verdict,
            SnapshotVerdict::Accepted(SnapshotAcceptance::UnprovenSigner)
        );
        assert_eq!(verdict.label(), "accepted_unproven_signer");
        assert!(!minted.is_signed(), "production mints carry no proof today");
    }

    #[test]
    fn accepts_with_proven_signer_when_a_real_signature_verifies() {
        let kp = test_keypair();
        let agent_cid = test_agent_cid(&kp.public_key_bytes());
        let minted = mint_snapshot(
            &fixed_source(
                entries(&[("c:a", "anc-a")]),
                &agent_cid,
                SignerEdgeSet::inert(),
            ),
            &LocalKeySnapshotSigner::new(kp),
        )
        .expect("mint");
        assert!(minted.is_signed());

        let verdict = verify_snapshot(&VerifyInput {
            carried: &minted.carried,
            local_corpus_digest: &minted.carried.snapshot.corpus_digest,
            last_verified_epoch: None,
        });
        assert_eq!(
            verdict,
            SnapshotVerdict::Accepted(SnapshotAcceptance::ProvenSigner),
            "a signature that verifies against the key read out of signer_agent_cid \
             is the only thing that may claim provenance"
        );
    }

    #[test]
    fn refuses_corpus_digest_mismatch() {
        let minted = mint_unsigned(entries(&[("c:a", "anc-a")]), "uSigner");
        let verdict = verify_snapshot(&VerifyInput {
            carried: &minted.carried,
            local_corpus_digest: "sha256:something-else",
            last_verified_epoch: None,
        });
        assert_eq!(
            verdict,
            SnapshotVerdict::Refused(SnapshotRefusal::CorpusDigestMismatch)
        );
    }

    #[test]
    fn refuses_epoch_regression_even_when_the_digest_matches() {
        // C2: a replayed snapshot whose digest matches is precisely the
        // attack — the epoch guard must run before the digest shortcut.
        let minted = mint_snapshot(
            &fixed_source(
                entries(&[("c:a", "anc-a")]),
                "uSigner",
                SignerEdgeSet::new(vec!["edge:1".into()]),
            ),
            &UnavailableSnapshotSigner,
        )
        .expect("mint");
        let newer = SignerEdgeSet::new(vec!["edge:1".into(), "edge:2".into()]).epoch();

        let verdict = verify_snapshot(&VerifyInput {
            carried: &minted.carried,
            local_corpus_digest: &minted.carried.snapshot.corpus_digest,
            last_verified_epoch: Some(&newer),
        });
        assert_eq!(
            verdict,
            SnapshotVerdict::Refused(SnapshotRefusal::EpochRegression),
            "an epoch that went backwards refuses, never silently accepts"
        );
    }

    #[test]
    fn refuses_epoch_unorderable_on_a_lateral_edge_set_change() {
        let minted = mint_snapshot(
            &fixed_source(
                entries(&[("c:a", "anc-a")]),
                "uSigner",
                SignerEdgeSet::new(vec!["edge:1".into()]),
            ),
            &UnavailableSnapshotSigner,
        )
        .expect("mint");
        // Same cardinality, different edges — cardinality cannot order this.
        let lateral = SignerEdgeSet::new(vec!["edge:other".into()]).epoch();
        let verdict = verify_snapshot(&VerifyInput {
            carried: &minted.carried,
            local_corpus_digest: &minted.carried.snapshot.corpus_digest,
            last_verified_epoch: Some(&lateral),
        });
        assert_eq!(
            verdict,
            SnapshotVerdict::Refused(SnapshotRefusal::EpochUnorderable),
            "an underivable direction is refused, never guessed"
        );
    }

    #[test]
    fn accepts_a_re_issued_identical_epoch() {
        let minted = mint_unsigned(entries(&[("c:a", "anc-a")]), "uSigner");
        let same = minted.carried.snapshot.trust_epoch.clone();
        let verdict = verify_snapshot(&VerifyInput {
            carried: &minted.carried,
            local_corpus_digest: &minted.carried.snapshot.corpus_digest,
            last_verified_epoch: Some(&same),
        });
        assert!(
            verdict.is_accepted(),
            "an unchanged edge set is a re-issue, not a regression"
        );
    }

    #[test]
    fn refuses_unknown_signer_when_an_attached_proof_does_not_verify() {
        let kp = test_keypair();
        let agent_cid = test_agent_cid(&kp.public_key_bytes());
        let mut minted = mint_snapshot(
            &fixed_source(
                entries(&[("c:a", "anc-a")]),
                &agent_cid,
                SignerEdgeSet::inert(),
            ),
            &LocalKeySnapshotSigner::new(kp),
        )
        .expect("mint");

        // Tamper with the signature bytes.
        if let Some(proof) = minted.carried.proof.as_mut() {
            proof.signature[0] ^= 0xFF;
        }
        assert_eq!(
            verify_snapshot(&VerifyInput {
                carried: &minted.carried,
                local_corpus_digest: &minted.carried.snapshot.corpus_digest,
                last_verified_epoch: None,
            }),
            SnapshotVerdict::Refused(SnapshotRefusal::UnknownSigner),
            "a forged provenance claim refuses even on a digest match"
        );
    }

    #[test]
    fn refuses_unknown_signer_when_the_agent_cid_carries_no_readable_key() {
        let kp = test_keypair();
        let agent_cid = test_agent_cid(&kp.public_key_bytes());
        let mut minted = mint_snapshot(
            &fixed_source(
                entries(&[("c:a", "anc-a")]),
                &agent_cid,
                SignerEdgeSet::inert(),
            ),
            &LocalKeySnapshotSigner::new(kp),
        )
        .expect("mint");
        // A signer cid no ed25519 key can be read out of. The digest is
        // re-derived so the snapshot stays self-consistent — this isolates
        // the key-derivation failure from Malformed.
        minted.carried.snapshot.signer_agent_cid = "not-a-holo-hash".into();
        assert_eq!(
            verify_snapshot(&VerifyInput {
                carried: &minted.carried,
                local_corpus_digest: &minted.carried.snapshot.corpus_digest,
                last_verified_epoch: None,
            }),
            SnapshotVerdict::Refused(SnapshotRefusal::UnknownSigner)
        );
    }

    #[test]
    fn refuses_malformed_when_the_digest_is_not_re_derivable_from_the_entries() {
        let mut minted = mint_unsigned(entries(&[("c:a", "anc-a")]), "uSigner");
        let claimed = minted.carried.snapshot.corpus_digest.clone();
        // Swap the entries out from under the (unchanged) declared digest.
        minted.carried.snapshot.entries = entries(&[("c:evil", "anc-evil")]);
        assert_eq!(
            verify_snapshot(&VerifyInput {
                carried: &minted.carried,
                local_corpus_digest: &claimed,
                last_verified_epoch: None,
            }),
            SnapshotVerdict::Refused(SnapshotRefusal::Malformed),
            "a snapshot must describe what it carried"
        );
    }

    #[test]
    fn refuses_malformed_on_duplicate_ids_blank_fields_and_bad_epochs() {
        let base = mint_unsigned(entries(&[("c:a", "anc-a")]), "uSigner").carried;

        let mut dup = base.clone();
        dup.snapshot.entries = entries(&[("c:a", "anc-a"), ("c:a", "anc-b")]);
        dup.snapshot.corpus_digest = corpus_digest_of_entries(&dup.snapshot.entries);
        assert_eq!(
            verify_snapshot(&VerifyInput {
                carried: &dup,
                local_corpus_digest: &dup.snapshot.corpus_digest,
                last_verified_epoch: None,
            }),
            SnapshotVerdict::Refused(SnapshotRefusal::Malformed),
            "two heads for one id is not a head set"
        );

        let mut blank = base.clone();
        blank.snapshot.signer_agent_cid = "   ".into();
        assert_eq!(
            verify_snapshot(&VerifyInput {
                carried: &blank,
                local_corpus_digest: &blank.snapshot.corpus_digest,
                last_verified_epoch: None,
            }),
            SnapshotVerdict::Refused(SnapshotRefusal::Malformed)
        );

        let mut bad_epoch = base;
        bad_epoch.snapshot.trust_epoch = TrustEpoch("epoch:v99:whatever".into());
        assert_eq!(
            verify_snapshot(&VerifyInput {
                carried: &bad_epoch,
                local_corpus_digest: &bad_epoch.snapshot.corpus_digest,
                last_verified_epoch: None,
            }),
            SnapshotVerdict::Refused(SnapshotRefusal::Malformed),
            "an epoch token this receiver cannot parse is structurally unusable"
        );
    }

    #[test]
    fn refuses_malformed_on_undecodable_wire_bytes() {
        assert_eq!(
            verify_wire_snapshot("not base64!!", "sha256:whatever", None),
            SnapshotVerdict::Refused(SnapshotRefusal::Malformed)
        );
        assert_eq!(
            verify_wire_snapshot(
                &base64::engine::general_purpose::STANDARD.encode([0xA0, 0x01, 0x02]),
                "sha256:whatever",
                None
            ),
            SnapshotVerdict::Refused(SnapshotRefusal::Malformed)
        );
    }

    #[test]
    fn wire_verify_abstains_while_the_receivers_own_corpus_is_amber() {
        use crate::db::content_diesel::HeadCorpusDigestReadiness;
        let minted = mint_unsigned(entries(&[("c:a", "anc-a")]), "uSigner");
        let wire = minted.carried.encode_wire().expect("encode");

        assert_eq!(
            verify_wire_snapshot_for_readiness(
                &wire,
                &HeadCorpusDigestReadiness::Amber { pending: 3 },
                None
            ),
            None,
            "a receiver with unwitnessed rows has no stable oracle — it must abstain, \
             never fire the zero-cost accept over a half-witnessed corpus"
        );
        assert_eq!(
            verify_wire_snapshot_for_readiness(
                &wire,
                &HeadCorpusDigestReadiness::Ready {
                    digest: minted.carried.snapshot.corpus_digest.clone(),
                    total: 1,
                },
                None
            ),
            Some(SnapshotVerdict::Accepted(
                SnapshotAcceptance::UnprovenSigner
            )),
            "and once the corpus settles, the same snapshot verifies"
        );
    }

    #[test]
    fn wire_verify_accepts_a_round_tripped_snapshot() {
        let minted = mint_unsigned(entries(&[("c:a", "anc-a")]), "uSigner");
        let wire = minted.carried.encode_wire().expect("encode");
        assert_eq!(
            verify_wire_snapshot(&wire, &minted.carried.snapshot.corpus_digest, None),
            SnapshotVerdict::Accepted(SnapshotAcceptance::UnprovenSigner)
        );
    }

    // ── Epoch ordering ──────────────────────────────────────────────────

    #[test]
    fn epoch_orders_by_edge_cardinality_and_refuses_what_it_cannot_order() {
        let one = SignerEdgeSet::new(vec!["e1".into()]).epoch();
        let two = SignerEdgeSet::new(vec!["e1".into(), "e2".into()]).epoch();
        let lateral = SignerEdgeSet::new(vec!["e9".into()]).epoch();

        assert_eq!(compare_epochs(&one, &two), EpochOrdering::Advanced);
        assert_eq!(compare_epochs(&two, &one), EpochOrdering::Regressed);
        assert_eq!(compare_epochs(&one, &one), EpochOrdering::Same);
        assert_eq!(compare_epochs(&one, &lateral), EpochOrdering::Unorderable);
        assert_eq!(
            compare_epochs(&one, &TrustEpoch("epoch:v2:0:x".into())),
            EpochOrdering::Unorderable,
            "an unknown scheme is never ordered against a known one"
        );
    }

    #[test]
    fn the_edge_set_digest_pairs_with_the_epoch_and_is_order_independent() {
        let a = SignerEdgeSet::new(vec!["e1".into(), "e2".into()]);
        let b = SignerEdgeSet::new(vec!["e2".into(), "e1".into()]);
        assert_eq!(a.digest(), b.digest());
        assert_eq!(a.epoch(), b.epoch());
        assert!(a.epoch().0.contains(&a.digest()));
    }

    #[test]
    fn the_inert_edge_set_yields_a_zero_count_epoch() {
        // The honest state until T19: every signer's epoch is this token, so
        // the C2 guard can only answer Same. Documented, tested, not hidden.
        let epoch = SignerEdgeSet::inert().epoch();
        assert!(epoch.0.starts_with("epoch:v1:0000000000000000:"));
        assert_eq!(
            compare_epochs(&epoch, &SignerEdgeSet::inert().epoch()),
            EpochOrdering::Same
        );
    }

    // ── Delta ───────────────────────────────────────────────────────────

    #[test]
    fn delta_splits_both_directions_and_divergence() {
        let snapshot = entries(&[("c:a", "anc-a"), ("c:b", "anc-b"), ("c:c", "anc-c1")]);
        let local = entries(&[("c:b", "anc-b"), ("c:c", "anc-c2"), ("c:d", "anc-d")]);
        let delta = head_set_delta(&snapshot, &local);

        assert_eq!(delta.only_in_snapshot, entries(&[("c:a", "anc-a")]));
        assert_eq!(delta.only_local, entries(&[("c:d", "anc-d")]));
        assert_eq!(
            delta.divergent,
            vec![HeadDivergence {
                id: "c:c".into(),
                snapshot_head_digest: "anc-c1".into(),
                local_head_digest: "anc-c2".into(),
            }]
        );
        assert_eq!(delta.len(), 3);
        assert!(!delta.is_empty());
    }

    #[test]
    fn delta_of_identical_sets_is_empty_regardless_of_order() {
        let a = entries(&[("c:a", "anc-a"), ("c:b", "anc-b")]);
        let b = entries(&[("c:b", "anc-b"), ("c:a", "anc-a")]);
        assert!(head_set_delta(&a, &b).is_empty());
        assert!(head_set_delta(&[], &[]).is_empty());
    }

    #[test]
    fn delta_against_an_empty_side_is_wholly_one_directional() {
        let some = entries(&[("c:a", "anc-a"), ("c:b", "anc-b")]);
        let from_snapshot = head_set_delta(&some, &[]);
        assert_eq!(from_snapshot.only_in_snapshot, some);
        assert!(from_snapshot.only_local.is_empty());
        assert!(from_snapshot.divergent.is_empty());

        let from_local = head_set_delta(&[], &some);
        assert_eq!(from_local.only_local, some);
        assert!(from_local.only_in_snapshot.is_empty());
    }

    #[test]
    fn delta_buckets_are_sorted_by_id() {
        let snapshot = entries(&[("c:z", "anc-z"), ("c:a", "anc-a")]);
        let delta = head_set_delta(&snapshot, &[]);
        let ids: Vec<&str> = delta
            .only_in_snapshot
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        assert_eq!(ids, vec!["c:a", "c:z"]);
    }

    // ── Key derivation + ReasonLabel conformance ────────────────────────

    #[test]
    fn agent_cid_key_reader_accepts_the_uhcak_form_and_rejects_the_rest() {
        let kp = test_keypair();
        let pk = kp.public_key_bytes();
        let cid = test_agent_cid(&pk);
        assert!(cid.starts_with("uhCAk"), "got {cid}");
        assert_eq!(ed25519_pubkey_from_agent_cid(&cid), Some(pk));

        assert_eq!(ed25519_pubkey_from_agent_cid(""), None);
        assert_eq!(ed25519_pubkey_from_agent_cid("uhCAk"), None);
        assert_eq!(ed25519_pubkey_from_agent_cid("not-a-hash"), None);
        // Right length, wrong multihash prefix (an ActionHash-shaped string).
        let mut wrong = Vec::from([0x84u8, 0x21, 0x24]);
        wrong.extend_from_slice(&[0u8; 36]);
        assert_eq!(
            ed25519_pubkey_from_agent_cid(&format!(
                "u{}",
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(wrong)
            )),
            None,
            "only an AgentPubKey prefix yields a signing key"
        );
    }

    #[test]
    fn snapshot_refusal_is_reason_label_conformant() {
        seam_contracts::assert_reason_labels_conformant::<SnapshotRefusal>();
        seam_contracts::assert_reason_labels_discriminating::<SnapshotRefusal>();
        seam_contracts::assert_reason_labels_stable::<SnapshotRefusal>(&[
            "corpus_digest_mismatch",
            "epoch_regression",
            "epoch_unorderable",
            "unknown_signer",
            "malformed",
        ]);
    }

    #[test]
    fn snapshot_acceptance_is_reason_label_conformant() {
        seam_contracts::assert_reason_labels_conformant::<SnapshotAcceptance>();
        seam_contracts::assert_reason_labels_discriminating::<SnapshotAcceptance>();
        seam_contracts::assert_reason_labels_stable::<SnapshotAcceptance>(&[
            "proven_signer",
            "unproven_signer",
        ]);
    }

    #[test]
    fn verdict_labels_do_not_collide_across_the_accept_refuse_split() {
        let mut labels: Vec<String> = SnapshotAcceptance::ALL
            .iter()
            .map(|a| SnapshotVerdict::Accepted(*a).label())
            .chain(
                SnapshotRefusal::ALL
                    .iter()
                    .map(|r| SnapshotVerdict::Refused(*r).label()),
            )
            .collect();
        let total = labels.len();
        labels.sort();
        labels.dedup();
        assert_eq!(labels.len(), total, "verdict labels must be unique");
    }

    #[test]
    fn trust_epoch_is_an_opaque_token_not_a_number() {
        // Type-level assertion: this compiles only because TrustEpoch wraps
        // a String, not an integer/float. If a future edit changes the
        // inner type to a numeric primitive, this construction — and the
        // "no numeric field, ever" discipline it stands in for — breaks
        // loudly at compile time instead of silently at review time.
        let epoch = TrustEpoch("edge-set-digest-derived-token".to_string());
        assert_eq!(epoch.0, "edge-set-digest-derived-token");
    }

    #[test]
    fn verdict_refused_carries_the_reason() {
        let verdict = SnapshotVerdict::Refused(SnapshotRefusal::EpochRegression);
        assert_eq!(
            verdict,
            SnapshotVerdict::Refused(SnapshotRefusal::EpochRegression)
        );
        assert_ne!(
            verdict,
            SnapshotVerdict::Accepted(SnapshotAcceptance::ProvenSigner)
        );
    }
}
