---
title: "Reach Ontology/Vocabulary Split — guiding principles for the 5-way reconciliation"
id: reach-ontology-vocabulary-split-spec
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: superseded-by-implementation — graduate once the reconciliation sprint lands the canonical vocabulary, verdict surface, and fixture harness
created: 2026-07-22
domain: D2
topic: [reach, ontology, vocabulary-drift, verdict, authorization, announcement, epr-rea, mishpat, trust]
cites:
  - genesis/research/ontology-systems-survey-reach-reconciliation-2026-07-22.md
  - genesis/research/letter-to-rea-practitioners-observed-presence-2026-07-22.md
  - genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
  - imagodei-profile-page-viewer-lens-design | Imagodei Profile-vs-Page | sha256:05caf5687b42f4ba | path: genesis/docs/superpowers/specs/2026-06-22-imagodei-profile-page-viewer-lens-design.md
  - elohim/sdk/schemas/v1/enums/reach.schema.json
  - elohim/elohim-storage/src/services/reach_earning.rs
  - elohim/elohim-storage/src/p2p/reach_authorization.rs
---

# Reach Ontology/Vocabulary Split — canonical guiding principles

> **Purpose.** The next sprint that picks up the reach-vocabulary reconciliation (resilience README roadmap item 13 + `reach-vocabulary-frontend-strand.md`) reads THIS document to know the guiding principles. It does not re-litigate them; it plans against them. Method/provenance: 2026-07-22 session — ontology-systems deep-research survey (adversarially verified) + operator-adjudicated thesis. Research grounding: the companion survey; audience-facing rationale: the REA letter (both in `genesis/research/`).

## 0. Thesis (one line)

**Reach is not a property of content; it is the standing verdict of a governed conversation between edges.** Store declarations and witnessed events; derive everything else at the evidence-holding edge; every verdict carries its explanation and its freshness; every observer may announce and be measured by said-vs-did variance; the intimate edge — not the datacenter — is where intelligence governs.

## 1. The split: one ontology, many projected vocabularies

The five drifted "reach" vocabularies are ontological compression of orthogonal axes. The reconciliation does NOT pick a winner among five — it demotes each to its honest role:

| Vocabulary (today) | True axis | Disposition |
|---|---|---|
| Schema 8 (`private…commons`, DNA-notarized) | **Declared reach floor** (relational sensitivity, content-scoped, no viewer term — that is a feature) | **CANONICAL declared vocabulary.** Stays the schema enum; source-of-record `elohim/sdk/schemas/v1/enums/reach.schema.json`; DNA `CORE_REACH_LEVELS` remains its notarized twin. |
| Rust services 8 (`personal…district/public`, `reach_earning.rs`) | none (stale brainstorm descendant) | **DIES.** Migrate to schema 8. |
| TS geographic 8 (`private/invited/local…bioregional/regional/commons`, 4 sites) | **Locality/placement** (dataplane: replication, eviction, caching) | **RENAMED out of "reach"** → a `locality`/`placement` vocabulary. Single TS edit point: `elohim/sdk/storage-client-ts/src/protocol-core.model.ts`; the other 3 sites re-export. Not part of the reach enum. |
| Resilience Part-V 5 (`household…organization/commons`) | **Custody** (who holds/replicates for whom) | **RENAMED** → custody vocabulary anchored on `CustodianCommitment` / `Mishpat::Commitment` lineage. |
| `VALID_REACH_LEVELS` 6 (`…federated…`, holochain.model.ts) | none (false "matches Rust" claim) | **DIES.** |

**Drift-prevention law (ValueFlows .ttl lesson, verified):** exactly ONE generative source-of-record per vocabulary; every other appearance is a generated projection (schema.json → codegen → ts-rs/Rust constants) or an explicit re-export. A vocabulary value hand-typed in a second place is a defect. Add a schema-contract test that fails when Rust/TS/DNA disagree with the schema.

## 2. Declared floor vs derived verdict (the two-layer law)

- **Declared reach** = the author's Knowledge-level policy on a content unit. Small, closed, ordinal, DNA-validated at the integrity zome, cheap, no viewer term. It is a *commitment*, not a computation.
- **Effective reach** = a **derived verdict**, computed where the evidence lives:
  `verdict(content, viewer?, announcement?, freshness) → { Allowed | Blocked | Pending, evidence, explain? }`
  — generalizing the existing `ReachVerdict`/`StandingEvidence` shape (`reach_earning.rs`) and the reach_authorization pre-auth stage. The verdict is never stored as truth; caches of it are Category-C operational projections, reconstructable.
- **Composition law (hard):** derived layers may only **narrow, never widen**. The declared floor and the key envelope are sovereign; no standing score, lens, negotiation, or inference may override them. Deny-overrides, always. A hard denial is still explained (evidence: "declared private; no consented edge; no key").

## 3. The serving-cost bathtub (reach is also a performance contract)

Each declared level maps to a serving class — this mapping is part of the canonical vocabulary's meaning:
- **commons/public:** viewer-independent verdict → compute once, serve statically (cacheable, crawlable; the existing `commons`/non-`commons` seam at `validate_project_epr_commitment` is the cliff).
- **middle bands (community/familiar/trusted):** group-scoped verdicts → materialized relationship tuples + indexed check (Zanzibar shape), background reconciliation absorbs graph-walk cost. This is where trust-compute concentrates.
- **intimate/private/self:** gate is **cryptographic possession** (key envelope), not evaluation; per-viewer disclosure-fold cost is affordable because N is small. Doorway cannot leak what it cannot read.

## 4. Freshness is a first-class verdict term (the clock)

Serving from replicas ⇒ the new-enemy problem: **a revocation must order before what it protects.** Every verdict carries a freshness requirement/attestation. Anchor on the existing amber/green DHT-witnessing signal — derived from `dht_anchor_hash` presence, never stored — and extend it to govern revocation ordering. Zookie-style per-request freshness (caller trades staleness vs latency) is the transferable interface pattern for gossip-propagation delay.

## 5. The observer/announcement slot (design in from v1)

The verdict interface accepts an optional **announcement**: who/what the visitor is, on whose behalf (delegation cited as a `Mishpat::Commitment` — standing + revocation on-chain, composing with the REA compute-commitment primitive), and declared intent.
- **Identity discipline (the 'A'):** announced identity must be **resolvable** and its claims **inspectable**. Internally this already exists in DID-document shape: `agent_cid` is the canonical identifier; `AgentPeerBinding`/`peer_transport_manifest` resolve transport identities to it (never string-compare across namespaces); `ContributorPresence` claim verification (email/dns-txt) is credential-issuance-shaped; Mishpat delegation is a capability credential. **Interop:** the announcement slot should accept a W3C **Verifiable Presentation** as one announcement format (external dispatched agents will arrive carrying DIDs/VCs), mapping issuer-signed claims into verdict evidence. Adopt the DID/VC *mechanics* by reference; the framing stays community-grounded per the identity-ontology guard (claims community/institution-issued and backstopped; recovery survives key loss; a key is custody, not personhood).
- **v1 behavior:** anonymous → commons band (safety-metadata logging only); doorway-session-authenticated → viewer-lens engages (consent-gated `relationship_class × intimacy` per the imagodei viewer-lens spec); announced → negotiable depth.
- **Announcement is REA-native:** announcement = Intent; traffic = witnessed Events; the diff = **fulfillment variance (said-vs-did)** emitted as events feeding standing. Variance never *widens* access (composition law); fresh identities start at the floor so identity-reset cannot launder variance history.
- Anti-surveillance invariant: announcement is a *privilege offered to the visitor* that buys depth — never a toll; anonymity always yields the honest commons surface.

## 6. Where inference lives (the Cyc inversion)

No central/general reasoner, ever. The verdict function stays small enough to run on any node (hub-optional floor): plain code over locally-materialized facts in the hot path — today's engineering rail. But this is a *sequencing* choice, not a ceiling: the verdict/explanation interface must admit richer **local** inference (household elohim-counsel negotiating disclosure) as edge compute grows, without re-centralizing. Facts live at the edges that witnessed them (verify-locally-then-serve); explanations are computed where the evidence and keys live.

## 7. Deliverables the reconciliation sprint owes (Definition of Done)

1. Schema 8 confirmed canonical; Rust `reach_earning.rs` enum + tests migrated; `VALID_REACH_LEVELS` removed; schema-contract drift test added (fails on Rust/TS/DNA vs schema disagreement).
2. Geographic 8 renamed to locality/placement (single SDK edit point + 3 re-exports); Part-V custody vocabulary renamed and re-anchored; resilience README gap-matrix row updated to name all five strands and their dispositions.
3. Verdict surface speced (route + Rust shape) as the generalization of `ReachVerdict`, with `explain` opt-in (metered — tracing has cost) and freshness parameter; announcement slot present even if v1 only distinguishes anonymous/session.
4. **Fixture harness:** offline, deterministic "given these declarations/tuples/commitments, agent A sees exactly {…}" invariant tests for the verdict function (SpiceDB `zed validate` pattern). Ships with the reconciliation, not after.
5. Downstream audit: consumers hardcoding reach strings (`content_store_integrity` validator, doorway `reach_aware_serving`/`access_control`, steward `storage/reach.rs`, `p2p/reach_authorization.rs`) reconciled to the canonical enum or the renamed axis vocabularies. Data-aware migration (SpiceDB lesson): no vocabulary value removed while live rows still carry it — migrate rows first.
6. a2o scenario(s) capturing the composition law (narrow-never-widen; anonymous→commons; revocation-orders-before-serve) as regression stories.

**Out of scope for the reconciliation sprint** (sequenced behind it, do not bundle): T4-4 reach-governed serving enforcement; full negotiation/UMA-style claims flow; variance→standing feedback loop; locality-driven placement engine. The reconciliation makes their interfaces possible; it does not build them.

## 8. Open questions (carry into planning, not blockers)

- Federation of independently-evolved vocabularies is unsolved field-wide (ValueFlows Knowledge-level-extension claim was refuted in verification); AT Protocol lexicon namespacing is the strongest candidate pattern if/when external parties extend our vocabularies.
- Middle-band tuple materialization: which existing projection (relationships table, `AgentPeerBinding`-style) becomes the tuple store, and what reconciles it. Note (from the REA-letter today/tomorrow analysis): `vf:AgentRelationship` is the ontology-native tuple — our `human_relationships` projection (consent-filtered, per the viewer-lens spec) is its closest live analog, making it the leading candidate rather than a new table.
- Whether `Pending` verdicts (evidence not yet propagated) render as amber UX uniformly.
- DID-method mapping: whether `agent_cid` + `AgentPeerBinding` formalize as a `did:` method (DID document = the peer-transport manifest) and whether `ContributorPresence` claim-verification graduates to issuing Verifiable Credentials — interop dividend vs added surface; sequenced with, not inside, the reconciliation sprint.
