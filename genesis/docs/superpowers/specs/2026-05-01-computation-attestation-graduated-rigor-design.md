---
title: Computation Attestation & Graduated Proof Rigor
status: Draft
created: 2026-05-01
related:
  - genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md
  - genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md
  - genesis/plans/2026-04-01-elohim-token-epr-native-minting-design.md
  - genesis/research/habermas-machine-2024.md
  - genesis/research/habermas-legacy.md
---

## The recognition

The protocol's existing qahal-deliberation + elohim-as-counsel + mishpat-validator loop is already a Habermas-Machine-shaped artifact — but pluralistic (per-evaluator standing, no single optimization target), constitutionally bounded (floors that resist majority override), and graduated (capability scales with stewardship). DDS arrives at a flatter version of the same problem: one Analyzer, one result, one chain anchor, one trust source. We arrived through our own design path and are noticing the convergence now.

The lesson from Habermas Machine and DDS is not *be the consensus generator* or *bridge to Ethereum*. It is **make every algorithmic mediator answerable to those it mediates**. This spec defines the peer-native primitive that lets any consumer — Polis-style clusterer, LLM-mediated synthesizer, mishpat validator — be auditable through one contract, with proof rigor that *scales with demand*, not by default.

## The primitive

A computation attestation records that some agent ran some algorithm on some inputs and produced some output. It is an EPR variant. Trait surface is loose; proof primitives are pluggable; the contract is concrete enough to implement.

```rust
trait ComputationAttestor {
    fn attest(
        scope: AttestationScope,
        inputs: InputSet,
        algorithm: AlgorithmId,
        output: OutputCommitment,
        agent: AgentId,
    ) -> Attestation;

    fn verify(attestation: &Attestation) -> VerificationResult;
}

trait ProofTier {
    /// Implementation-defined judgment from context to proof class.
    /// Implementations weigh stakes, spread, consensus deficit, and provability;
    /// the trait does not enumerate signals.
    fn required_proof(context: &AttestationContext) -> ProofClass;
}

trait AttestationProof {
    fn class(&self) -> ProofClass;
    fn verify(&self, attestation: &Attestation) -> VerificationResult;
}
```

`AttestationContext` carries the minimum observables: `source_epr_id`, `algorithm_id`, `computer_agent_id`, `scope`, optional `input_merkle_root`, optional `output_hash`. Implementations derive everything else (standing, contestation, coupling depth, audience reach) from DHT and storage queries against these handles. The context type does not encode policy.

`ProofClass` is opaque. Concrete proof primitives — agent-signature, deterministic re-execution, zkML, multi-attestor confirmation — plug in as `AttestationProof` implementations. Provenance is anchored to ELOHIM token's existing chain-agnostic `SettlementBridge`; `verify_provenance(merkle_root)` already exists, this primitive consumes it.

## The gradient

Proof rigor scales with demand, not with default. The narrative names four stations along a continuum; the trait does not depend on them.

**Witness** — the lightest. Computer-agent attests the computation happened; signs the inputs and output. *Trust cheapens compute*: when an agent has standing and the topic is uncontested, witness is enough. This is the default. Most computations live here.

**Audit** — anyone can re-execute. Inputs are Merkle-rooted, algorithm + version are pinned, output is hashed. The cost of contesting falls to the contester; the cost of proving falls to whoever cares. Used when consensus deficit appears (FeedbackSignal::Correction firings, polarized contestation) or stakes rise (governance-input shading toward governance-binding, downstream coupling deepens).

**Proof** — cryptographic. zkML or equivalent. Verification is asymmetric: expensive to prove, cheap to verify. Used when stakes are high *and* contestation is real *and* the algorithm is too non-deterministic for re-execution. Today this is feasible for arithmetic clustering and vote tallying; LLM consensus generation is research horizon (zkML 2026+).

**Confirmation** — Proof + multi-attestor + mishpat-quorum. Used when constitutional floors are touched (tending-immune or standing-immune content classes per the standing-policy and tending-policy manifests). The highest-stakes computation in the protocol; the place where majority-overriding-rights gets stopped, and therefore the place where verification must be ironclad.

The station is judged at the moment of attestation by signals across four categories:

- **Stakes** — standing-impact severity, governance criticality (informs vs. binds), constitutional-floor proximity, subject vulnerability (per the stewardship philosophy)
- **Spread** — downstream coupling depth, audience reach, token-mint magnitude
- **Consensus deficit** — contestation rate (FeedbackSignal::Correction firings), polarization markers, prior-attestation track record of this algorithm
- **Provability** — what proof primitive can the algorithm even support today

Provability matters last. It can ceiling a station (an LLM-mediated synthesis cannot reach Proof today regardless of stakes), but it should never floor one — high stakes with low provability is a research signal, not a ship blocker.

**Default is Witness.** Provability is opt-in via demand. If someone disputes a low-station attestation, they have FeedbackSignal::Correction, OpinionStatement, GateDecisionChallenge — vectors to build the case for higher rigor. The protocol does not pre-pay compute for unrequested proof.

## Three walkthroughs

**Maria — neighborhood zoning, Polis-style clustering.** Maria's neighborhood debates a zoning change. Forty-seven comments come in over a week; the qahal-running app runs PCA clustering and surfaces three axes of disagreement plus the bridging statements that resonated across clusters. Maria sees herself in cluster 2 and discovers shared ground with people she had assumed disagreed. *Witness* is plenty here — the algorithm is deterministic, stakes are governance-input. Then Tom contests it: he claims the clustering grouped him with people he doesn't agree with. Tom files a Correction. Contestation rate trips a station threshold; the next clustering run on the same deliberation escalates to *Audit* — Merkle-rooted inputs, pinned algorithm version, anyone can re-execute. Tom verifies; Maria's confidence increases regardless. The escalation cost was paid because someone asked.

**Sandra — riverfront rezoning, LLM-mediated consensus.** Sandra's town runs a longer deliberation across two hundred comments. An LLM mediator drafts a consensus statement: *"The riverfront should remain accessible to all residents while supporting modest mixed-use development that prioritizes…"* Sandra reads it and feels her concerns are reflected — but also feels her stronger objection to commercial development was smoothed over. The attestation records which inputs were considered, which model + version + values manifest constrained the draft, which alternative drafts were ranked lower. Sandra surfaces a Correction with evidence. Consensus deficit is real; stakes are high; provability ceilings the station at *Audit*. The protocol cannot yet cryptographically prove an LLM ran faithfully, but it can record what was considered and what was discarded, and let Sandra make her case. Habermas's ideal-speech conditions become *attestable* — was every voice represented in the input set? could the algorithm have privileged some voices? — without claiming to be ideal speech.

**The qahal vote — mishpat validator, binding outcome.** After three weeks of deliberation, Sandra's qahal proposal goes to a vote. The mishpat validator computes standing-projected tally, constitutional-floor checks (does this proposal violate any tending-immune or standing-immune content classes?), and quorum thresholds. Output is a binding governance act. Stakes are at the maximum; the algorithm is deterministic; provability is high. *Confirmation* — Proof + multi-attestor + mishpat-quorum. Any member can independently re-execute the validator against the public REA event log. If the validator was compromised or made an error, the attestation is the evidence. The constitutional floors are where the protocol's deepest commitments live; verification must be ironclad.

## Breadcrumbs

- ELOHIM token's chain-agnostic `SettlementBridge` already exposes `verify_provenance(merkle_root)`. This primitive consumes it. No new bridge.
- Tier 2 elohim-discernment minting is a *sibling* of computation attestation — judgment provenance vs. computational provenance. They share the provenance hash anchor; an elohim's reasoning trace is itself a candidate input to a downstream attestation.
- Federation is a *sibling spec*: `2026-05-01-atproto-lexicon-projection-doorway-design.md` defines the doorway projection adapter that translates this primitive into `org.dds.result.*` records for AT Protocol consumers. Signing is doorway-as-relying-party; opportunistic did:plc per peer is deferred. AT Proto interop lives at doorway by architectural memory.
- FeedbackSignal::Correction is the consensus-deficit signal. Contestation rate against a prior attestation is a primary trigger for tier escalation on subsequent computations.
- Habermas Machine (DeepMind 2024) and Habermas's *Theory of Communicative Action* are reference, not adoption — see `genesis/research/habermas-machine-2024.md` and `habermas-legacy.md`. We pay homage; we do not import the single-optimization-target frame.
- DDS-WG's Analyzer + commitment record schema is the shape we lifted: `{deliberation_uri, scope, input_hash, algorithm, output_hash, analyzer_did}`. The substrate (Ethereum + AT Protocol PDS-as-truth) we did not.
- zkML maturity (EZKL, Lagrange, Giza) determines what *Proof* covers today. Vote tallying and clustering are feasible; LLM verification is research horizon.
- Archival substrates (Arweave, Filecoin, Logos) are doorway-projection candidates for cold-path persistence of attestations — see `genesis/research/README.md` "The Archival Problem."

## Open questions

- Should `ProofTier::required_proof` return a single class, or a tier *floor* so a high-standing computer can voluntarily over-attest? Defer.
- Constitutional-floor manifests already encode immune content classes. The link between *floor proximity* and *Confirmation* should be explicit in the manifest schema; needs a small extension. Defer to manifest schema work.
- LLM-mediated computation inputs are not just data but also prompts and system context. The `InputSet` type needs to accommodate "everything that shaped the output" — beyond Merkle-rooted comment lists. Defer to walkthrough refinement.
- Relationship between this primitive and any existing `AttestationSignal` EPR — sibling, subtype, supersession. Worth a brainstorm before coding.
- Per-evaluator standing projection (Phase 3.5) generates per-evaluator outputs. Does each projection get its own attestation, or do we attest the projection function once and let projections be derivations? Coordinate with the Phase 3.5 plan.
- "Polarization markers" as a consensus-deficit signal needs an operational definition. Heuristic candidates: Correction-rate slope, OpinionStatement clustering distance, FeedbackSignal::Quarantine prevalence. Defer to first implementation.
