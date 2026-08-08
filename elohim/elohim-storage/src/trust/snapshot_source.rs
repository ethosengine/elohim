//! The two seams a [`crate::trust::snapshot::mint_snapshot`] depends on:
//! **where** the head-plane inputs come from ([`SnapshotSource`]) and **who**
//! — if anyone — can prove authorship of them ([`SnapshotSigner`]).
//!
//! Shape follows `services::commitment_fetcher.rs`: trait + production impl +
//! test double, all in one file, with the double NOT behind `cfg(test)` so
//! other modules (and integration tests) can compose against it.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md`
//! §3 L5.

use crate::db::{AppContext, DbPool};
use crate::trust::snapshot::{HeadSetEntry, SignerEdgeSet, SnapshotProof, SNAPSHOT_SCHEME_VERSION};

// ── Source ──────────────────────────────────────────────────────────────────

/// Everything a snapshot is minted FROM. Deliberately not the snapshot itself:
/// the epoch and the edge-set digest are DERIVED from [`Self::edge_set`] by
/// the mint, so a source cannot hand over an epoch that disagrees with the
/// edge set it names.
#[derive(Debug, Clone)]
pub struct SnapshotInputs {
    /// `(id, head_digest)` over the distribution-safe head relation.
    pub entries: Vec<HeadSetEntry>,
    /// The corpus digest the snapshot self-describes with — the L2 join.
    pub corpus_digest: String,
    /// The agent whose head plane this describes.
    pub signer_agent_cid: String,
    /// The signer's attestation/citation edge set — the epoch clock's
    /// substrate. [`SignerEdgeSet::inert`] until T19 lands a writer.
    pub edge_set: SignerEdgeSet,
}

/// Why snapshot inputs could not be produced.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotSourceError {
    /// The distribution-safe head relation still has rows awaiting DHT
    /// witness. Refusing is the honest answer: during a witness sweep the
    /// relation changes on every tick, so a snapshot minted now would
    /// advertise a digest that is already stale — and the receiver-side
    /// comparison would report a predictable, meaningless mismatch. Same
    /// abstention the view-federation responder makes.
    #[error(
        "head corpus is amber ({pending} distribution-safe rows awaiting DHT witness); \
         refusing to mint over a relation that is still changing"
    )]
    CorpusAmber { pending: usize },
    #[error("storage: {0}")]
    Storage(String),
}

/// Where a [`crate::trust::snapshot::HeadSetSnapshot`]'s inputs come from.
///
/// Synchronous on purpose: the production implementation is a diesel read off
/// the local projection, and there is no conductor round-trip anywhere on this
/// path (a snapshot describes what this node already holds — asking the
/// network what our own head plane is would invert the whole point).
pub trait SnapshotSource: Send + Sync {
    /// # Errors
    /// Returns [`SnapshotSourceError`] when the relation is unreadable or not
    /// stable enough to describe.
    fn load(&self) -> Result<SnapshotInputs, SnapshotSourceError>;
}

/// Production source: the local head plane, read through the SAME
/// distribution-safe relation the corpus digest folds.
///
/// Both reads come from `db::content_diesel` rather than a query written here
/// — `DISTRIBUTION_SAFE_REACH` has exactly one home, and a second copy of a
/// reach whitelist is how a scoped tier eventually leaks into a cross-peer
/// surface.
pub struct LocalHeadPlaneSnapshotSource {
    pool: DbPool,
    app_ctx: AppContext,
    signer_agent_cid: String,
}

impl LocalHeadPlaneSnapshotSource {
    #[must_use]
    pub fn new(pool: DbPool, app_ctx: AppContext, signer_agent_cid: String) -> Self {
        Self {
            pool,
            app_ctx,
            signer_agent_cid,
        }
    }
}

impl SnapshotSource for LocalHeadPlaneSnapshotSource {
    fn load(&self) -> Result<SnapshotInputs, SnapshotSourceError> {
        use crate::db::content_diesel::{
            head_corpus_digest_readiness, list_content_anchor_inventory, HeadCorpusDigestReadiness,
        };

        let mut conn = self
            .pool
            .get()
            .map_err(|e| SnapshotSourceError::Storage(format!("db conn: {e}")))?;

        // Readiness FIRST: an amber relation is not describable, and asking
        // for it also gives us the digest from the same SQL snapshot, so the
        // digest and the entries cannot straddle a concurrent write that
        // turns a partial relation into an advertised "whole corpus".
        let corpus_digest = match head_corpus_digest_readiness(&mut conn, &self.app_ctx)
            .map_err(|e| SnapshotSourceError::Storage(e.to_string()))?
        {
            HeadCorpusDigestReadiness::Amber { pending } => {
                return Err(SnapshotSourceError::CorpusAmber { pending })
            }
            HeadCorpusDigestReadiness::Ready { digest, .. } => digest,
        };

        // The entries behind that digest: the anchored, distribution-safe
        // rows, unpaginated. `list_content_anchor_inventory` filters on
        // exactly the predicates the digest folds (anchored + distribution
        // safe), so the two derivations describe one relation — pinned by
        // `mint_corpus_digest_equals_the_db_head_corpus_digest`.
        let (rows, _total) = list_content_anchor_inventory(&mut conn, &self.app_ctx, 0, i64::MAX)
            .map_err(|e| SnapshotSourceError::Storage(e.to_string()))?;

        Ok(SnapshotInputs {
            entries: rows
                .into_iter()
                .map(|r| HeadSetEntry {
                    id: r.id,
                    head_digest: r.dht_anchor_hash,
                })
                .collect(),
            corpus_digest,
            signer_agent_cid: self.signer_agent_cid.clone(),
            // Honest: no edge writer exists until T19's standing projector.
            edge_set: SignerEdgeSet::inert(),
        })
    }
}

/// Test double: hands back inputs the caller constructed. Not `cfg(test)` —
/// same rule as `MockCommitmentFetcher`, so integration tests and other
/// modules can compose against it.
pub struct FixedSnapshotSource {
    inputs: SnapshotInputs,
}

impl FixedSnapshotSource {
    #[must_use]
    pub fn new(inputs: SnapshotInputs) -> Self {
        Self { inputs }
    }
}

impl SnapshotSource for FixedSnapshotSource {
    fn load(&self) -> Result<SnapshotInputs, SnapshotSourceError> {
        Ok(self.inputs.clone())
    }
}

// ── Signer ──────────────────────────────────────────────────────────────────

/// Why no authorship proof could be produced. This is not a failure the mint
/// treats as fatal — an unsigned snapshot is a legitimate artifact whose
/// provenance claim is honestly absent.
#[derive(Debug, thiserror::Error)]
pub enum SignerUnavailable {
    /// The production state of the world. Storage-side code holds exactly one
    /// private key — the libp2p transport keypair — and that is a DIFFERENT
    /// identity namespace from the `uhCAk…` agent cid a snapshot names as its
    /// signer (see the crate CLAUDE.md identity table; signing with it would
    /// be a provenance claim about the wrong identity). The agent key lives in
    /// the conductor keystore, and no `sign_bytes` extern exposes it. Closing
    /// this gap is a coordinator-zome change, not a storage change.
    #[error(
        "no agent-key signing is reachable from storage-side code: the agent private key \
         lives in the conductor keystore and no sign_bytes extern exposes it"
    )]
    NoAgentKeyReachable,
}

/// Produces a detached [`SnapshotProof`] over a snapshot's canonical bytes.
pub trait SnapshotSigner: Send + Sync {
    /// # Errors
    /// Returns [`SignerUnavailable`] when no signing key is reachable.
    fn sign(&self, canonical_bytes: &[u8]) -> Result<SnapshotProof, SignerUnavailable>;
}

/// The PRODUCTION signer: it refuses, every time, and says why.
///
/// This is the honest seam, not a placeholder that will quietly start working
/// — nothing in this crate can reach the agent key, so anything that claimed
/// to sign here would be claiming a provenance it cannot establish. The
/// consequence is visible on the far side of the wire as
/// [`crate::trust::snapshot::SnapshotAcceptance::UnprovenSigner`], never as a
/// silent success.
pub struct UnavailableSnapshotSigner;

impl SnapshotSigner for UnavailableSnapshotSigner {
    fn sign(&self, _canonical_bytes: &[u8]) -> Result<SnapshotProof, SignerUnavailable> {
        Err(SignerUnavailable::NoAgentKeyReachable)
    }
}

/// Signs with an ed25519 key the CALLER supplies — real crypto, through the
/// canonical [`elohim_epr::proof`] helpers.
///
/// Nothing constructs this in production, because nothing in this process
/// holds the agent's key (see [`UnavailableSnapshotSigner`]). It exists so the
/// verify path can be exercised end-to-end against a key we actually have, and
/// so that the day an agent-key handle DOES become reachable, the signing side
/// is a constructor call rather than a new design.
pub struct LocalKeySnapshotSigner {
    keypair: elohim_epr::AgentKeypair,
}

impl LocalKeySnapshotSigner {
    #[must_use]
    pub fn new(keypair: elohim_epr::AgentKeypair) -> Self {
        Self { keypair }
    }

    /// The public half — the key a verifier must be able to read out of the
    /// snapshot's `signer_agent_cid` for the proof to verify.
    #[must_use]
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.keypair.public_key_bytes()
    }
}

impl SnapshotSigner for LocalKeySnapshotSigner {
    fn sign(&self, canonical_bytes: &[u8]) -> Result<SnapshotProof, SignerUnavailable> {
        let preimage = crate::trust::snapshot::signing_preimage(canonical_bytes);
        Ok(SnapshotProof {
            algorithm: "ed25519".to_string(),
            signature: elohim_epr::proof::sign(&self.keypair, &preimage),
            scheme_version: SNAPSHOT_SCHEME_VERSION,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_production_signer_refuses_and_names_the_gap() {
        let err = UnavailableSnapshotSigner
            .sign(b"anything")
            .expect_err("production signing must refuse, never fabricate a proof");
        assert!(matches!(err, SignerUnavailable::NoAgentKeyReachable));
        assert!(
            err.to_string().contains("conductor keystore"),
            "the refusal must name WHERE the key actually lives: {err}"
        );
    }

    #[test]
    fn the_local_key_signer_produces_a_scheme_v1_ed25519_proof() {
        let kp = elohim_epr::AgentKeypair::from_secret(&[3u8; 32]).expect("seed");
        let signer = LocalKeySnapshotSigner::new(kp);
        let proof = signer.sign(b"canonical").expect("sign");
        assert_eq!(proof.algorithm, "ed25519");
        assert_eq!(proof.scheme_version, SNAPSHOT_SCHEME_VERSION);
        assert_eq!(proof.signature.len(), 64);
        assert!(
            elohim_epr::proof::verify(
                &signer.public_key_bytes(),
                &crate::trust::snapshot::signing_preimage(b"canonical"),
                &proof.signature
            ),
            "the proof must verify over the DOMAIN-SEPARATED preimage"
        );
        assert!(
            !elohim_epr::proof::verify(&signer.public_key_bytes(), b"canonical", &proof.signature),
            "and must NOT verify over the bare bytes — the domain tag is load-bearing"
        );
    }

    #[test]
    fn the_fixed_source_hands_back_what_it_was_given() {
        let inputs = SnapshotInputs {
            entries: vec![HeadSetEntry {
                id: "c:a".into(),
                head_digest: "anc-a".into(),
            }],
            corpus_digest: "sha256:x".into(),
            signer_agent_cid: "uSigner".into(),
            edge_set: SignerEdgeSet::inert(),
        };
        let loaded = FixedSnapshotSource::new(inputs.clone())
            .load()
            .expect("load");
        assert_eq!(loaded.entries, inputs.entries);
        assert_eq!(loaded.corpus_digest, inputs.corpus_digest);
        assert_eq!(loaded.signer_agent_cid, inputs.signer_agent_cid);
        assert_eq!(loaded.edge_set, inputs.edge_set);
    }
}
