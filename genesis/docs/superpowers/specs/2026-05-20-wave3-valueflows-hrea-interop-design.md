# Wave 3 — `bridges/valueflows`: hREA / VF-GraphQL Interop Design

**Status:** Design (pre-implementation). Implementation plan will be authored next at `genesis/docs/superpowers/plans/2026-05-20-wave3-valueflows-hrea-interop-plan.md` via `superpowers:writing-plans` once this spec is approved.

**Date:** 2026-05-20
**Plan kinship:**
- Implements Wave 3 of `genesis/docs/plans/2026-04-21-rno-lessons-cross-wave-guidance.md` (the "#4 hREA / VF-GraphQL alignment" wave that was deferred at Gate B and required fresh brainstorming).
- Consumes substrate from `2026-05-19-qahal-collective-membership-dht-design.md` (L7 — Collective + Membership entry types that become hREA `Organization` + `AgentRelationship`).
- Builds on the Viewer.* GraphQL surface landed in L6 (`2026-05-19-viewer-symmetry-reciprocity-qahal-substrate.md`).

**Source references:**
- `/projects/research/vf-graphql/` (cloned at `0a52dbe`) — canonical VF schema modules
- `/projects/research/requests-and-offers/` (cloned at `a45374a7`) — canonical R&O hApp; mapping target for compatibility tests
- `.claude/memory/project_doorway_is_federation_surface_atproto.md` — the bridge-pattern precedent (AT Proto / ActivityPub interop pattern)
- `.claude/memory/project_no_sovereignty_stewardship_over_ownership.md` — the ontological commitment driving the Agency + Authority seams
- `.claude/memory/project_socially_derived_security.md`, `project_recovery_grandma_standard.md`, `project_graduated_recovery_authority.md`, `project_elohim_as_counsel.md` — the relational-authority worldview
- `.claude/memory/project_epr_substrate_vs_vf_graphql.md` — "EPR substrate ≠ VF-GraphQL; EPR codec/libp2p = graph primitive; VF-GraphQL is app-layer." This spec implements that distinction.

## 1. Strategic frame

**The cooperative substrate stance.** Per the cross-wave guidance: "we are not graduating R&O — we are preparing elohim to be worthy of being graduated into." Wave 3 honors that by making elohim's REA layer credibly hREA-intelligible AND by honoring Lynn Foster's VF-GraphQL / hREA work as first-class upstream. R&O remains a parallel project we cooperate with, not a project we subsume by default. If absorption happens, it happens because the flywheel made it the obvious choice — not because we built parallel.

**The bridge-as-flywheel pattern.** Doorway is the bridge of web2 → elohim P2P. `bridges/valueflows` is the bridge of VF/hREA → elohim EPR-REA. Both bridges absorb external protocols, project our canonical substrate into their shape, and surface enough of elohim's distinctive value (extensions, structured error reasons, social-context fingerprints) that mutual benefit compounds. The standing argument for graduation builds with usage; we don't argue for it — we make it obvious.

**The bridge speaks both worlds honestly.** VF-shaped requests are answered in VF shape; underneath, the substrate stays elohim. The translator's job is to make the seams *legible* — to clients (via structured error payloads, extension fields, denial reasons), to operators (via the learning ledger), and to upstream maintainers (via the upstream-contribution inventory that aggregates what we've learned is genuinely worth carrying back to VF/hREA proper). Faithful translation, not absorption masquerading as cooperation.

## 2. Architecture

### 2.1 Top-level layout

```
/projects/elohim/
├── doorway/
│   └── doorway-service/         web2-protection runtime; scaling shield
│       └── (consumes: bridges/atproto, bridges/activitypub,
│                      bridges/web2-routes, bridges/oauth-rp, ...)
│
├── elohim/
│   ├── elohim-storage/          native protocol runtime
│   │   └── (consumes: bridges/valueflows)
│   ├── qahal-authority/         NEW crate — relational-authority gate library
│   │                            consumed by any bridge that absorbs external writes
│   └── holochain/dna/
│       ├── elohim/              existing — REA primitives, EPR
│       ├── lamad/               existing
│       ├── imagodei/            existing + NEW VfBinding entry type
│       ├── mishpat/             existing
│       ├── node-registry/       existing
│       ├── qahal/               future per L7 — Collective, Membership
│       └── hrea/                NEW — Lynn's canonical bundle (version-pinned)
│
├── bridges/                     NEW top-level dir; pluggable bridge crates
│   └── valueflows/
│       ├── valueflows-bridge/       library crate: vf-translator,
│       │                            hREA projection, learning ledger
│       ├── valueflows-types/        Rust types for the bridge
│       └── valueflows-tests/        VF conformance + seam + R&O compatibility
│
└── ...
```

### 2.2 Runtime separation principle

**doorway** = web2 scaling shield. Reasons: needs to protect the fragile P2P substrate from public-web flood, OAuth, SSR, manifest-driven HTTP routes, web2 federation (AT Proto / ActivityPub).

**elohim-storage** = native protocol surface. Hosts `/api/v1/graphql` (native Viewer.*) and `/api/v1/vf-graphql` (the VF-GraphQL surface from this spec). Speaks the protocol; not where web2 traffic is absorbed.

**bridges** = pluggable libraries. Each bridge is a Rust crate. Each runtime consumes the bridges that match its job. `valueflows` lands in elohim-storage because it's protocol-shaped interop; AT Proto / ActivityPub land in doorway because they're web2 federation.

### 2.3 Endpoint placement and proxy path

```
R&O client over public web
   │
   ▼ HTTPS
doorway-service                ← web2 scaling shield: rate-limit, DDoS, TLS termination
   │ (treats /api/v1/vf-graphql as opaque API traffic — no translation here)
   ▼
elohim-storage
   │ consumes bridges/valueflows
   │ which mounts /api/v1/vf-graphql at the storage's HTTP router
   │ which calls qahal-authority on writes
   │ which emits EPR atoms
   │ which projects to hREA DNA
   ▼
hREA DNA + elohim DNAs + imagodei DNA
```

Native peers (Tauri shell, future native peers) connect to elohim-storage directly, bypassing doorway. Same bridge, conditional protection layer.

## 3. The three ontological seams

The substantive design content. Each seam is where VF/hREA's worldview meets elohim's, and where translation has semantic cost — these are where the upstream-contribution candidates emerge.

### 3.1 Seam 1 — Agency: sovereign key holder ↔ socially-constituted participant

**VF/hREA worldview:** `Agent.id` is an `AgentPubKey`. The key holder is the locus of agency. Identity = cryptographic identity.

**Elohim worldview:** `AgentPubKey` is a *device key*, not an identity. A Human exists through participation in social context. A Human may have multiple cryptographic Agents (devices). An ElohimAgent (AI advocate) may act on behalf of a Human.

**Bridge mechanism:** `VfBinding` entry type in imagodei DNA records `{vf_agent_pubkey, elohim_human_cid, registering_agent_cid, registered_at_block_height, initiation}`. The binding maps an hREA `AgentPubKey` to the underlying elohim Human. Crucially, **the binding does not authorize anything** — it's just identity resolution. Authority comes from seam 2.

**Cells per Human, not per device.** When a Human first interacts with VF, the binding handshake provisions a single hREA cell whose AgentPubKey *is* the VfBinding's `vf_agent_pubkey`. Subsequent VF writes from any of the Human's cryptographic Agents flow through that one hREA cell, preserving VF's one-Agent-one-Person semantics at the projection layer. Multi-device fan-out is surfaced as a `Person.elohimAgentKeys[]` extension field — visible to elohim-aware clients, invisible to stock hREA clients.

**ElohimAgent as new VF Agent subtype.** AI advocates acting on behalf of Humans surface as either (a) extension path: `Elohim implements Agent` (preferred), or (b) compatibility path: `AgentRelationship { subject: Human's Person, object: ElohimAgent's synthesized Person, relationship: "actsOnBehalfOf" }`. Translator chooses based on client capability; learning ledger records which clients use which path.

**Upstream-contribution candidates from this seam:**
1. `Person.elohimAgentKeys[]` — "an Agent may have multiple cryptographic identities across devices" (extension field, candidate for VF adoption)
2. `Elohim implements Agent` — "AI advocates as first-class economic actors" (new VF Agent subtype, candidate for VF adoption)

### 3.2 Seam 2 — Authority: signature ↔ relational standing

**VF/hREA worldview:** Authority = signature on the entry. The fact that you signed something is sufficient permission.

**Elohim worldview:** Authority = signature + qahal standing + reach + relational network + witnessed mandate. A key alone authorizes nothing. Per `project_no_sovereignty_stewardship_over_ownership` and `project_socially_derived_security`, individual autonomy is a byproduct of community and institutional resilience you exist in.

**Bridge mechanism — `qahal-authority` crate:** every VF write passes through `qahal_authority::evaluate_write_authority(human_cid, requested_act, requested_reach, target_audience)` BEFORE the hREA coordinator is called.

```rust
// In elohim/qahal-authority/ (new crate)
pub fn evaluate_write_authority(
    human_cid: &str,
    requested_act: &Act,
    requested_reach: Reach,
    target_audience: Option<&[HumanCid]>,
) -> AuthorityDecision;

pub enum AuthorityDecision {
    Authorized { social_context_fingerprint: Vec<u8> },
    Denied { reason: AuthorityDenialReason },
    PendingAttestation { sponsor_candidates: Vec<HumanCid> },
}

pub enum AuthorityDenialReason {
    InsufficientReach { requested: Reach, available: Reach },
    NoRelevantQahalMembership { required_collective_kinds: Vec<String> },
    StandingBelowThreshold { current: f64, required: f64 },
    TargetRejectedReciprocal { target_human_cid: String, reason: String },
}
```

The library is consumed by ANY bridge that absorbs external writes — doorway's OAuth-grant flow uses it, valueflows uses it, future bridges use it. Uniform relational-authority semantics across all external interfaces.

**Denial reasons are structured and surfaced.** A 403 response carries `extensions.elohim_authority_denial.{check_failed, required_remedy}`. The VF client learns what social context is missing — not "unauthorized" but "you need a community-reach qahal membership to publish at this reach level."

**Upstream-contribution candidates from this seam:**
1. `Commitment.reach: ReachLevel` — privacy classification at REA-flow level (could be a VF field; clients ignore if not understood)
2. `AgentRelationshipRole` extensions for stewardship relationships beyond VF's open-vocabulary baseline
3. Structured authority-denial response shape as a VF protocol extension

### 3.3 Seam 3 — Truth substrate: signed-DHT-entry ↔ socially-witnessed EPR atom

**VF/hREA worldview:** A signed DHT entry IS the authoritative truth. The signature is the receipt.

**Elohim worldview:** EPR atoms are content-addressed facts witnessed by the social network. The atom carries author identity, social context at authoring, witnessing agents, and projection targets. **EPR is canonical; hREA writes are projections.**

**Bridge mechanism:** every successful VF write commits in this order:
1. EPR atom commits to elohim DNA (canonical truth)
2. `VfProjectionExpected` marker entry commits alongside, status `Pending`
3. hREA projection write fires synchronously to the Human's hREA cell
4. On success → `VfProjectionExpected` flips to `Projected{hrea_entry_hash}`
5. On failure → `VfProjectionExpected` stays `Pending`; reconciliation worker retries
6. After M retries → flips to `Failed{reason}`; observability alert emits

```rust
// In elohim DNA
#[hdk_entry_helper]
pub struct EprAtom {
    pub author_human_cid: String,
    pub act_serialized: Vec<u8>,           // VF mutation contents, deterministic encoding
    pub device_key_used: AgentPubKey,
    pub social_context_at_authoring: SocialContext,
    pub canonical_projection_target: Option<ProjectionTarget>,
    pub witnessed_by: Vec<WitnessSignature>,
    pub block_height: u64,
}

pub enum ProjectionTarget {
    Hrea { dna_hash: DnaHash, target_type: String },
    AtProto { ... },          // future
    ActivityPub { ... },      // future
}

#[hdk_entry_helper]
pub struct VfProjectionExpected {
    pub epr_atom_cid: String,
    pub target_dna: DnaHash,
    pub status: ProjectionStatus,
    pub last_attempt_at_block_height: Option<u64>,
    pub retry_count: u32,
}

pub enum ProjectionStatus {
    Pending,
    Projected { hrea_entry_hash: EntryHash },
    Failed { reason: String, terminal_at_block_height: u64 },
}
```

**Reconciliation worker** in elohim-storage scans `VfProjectionExpected{status: Pending, age > 60s}` every ~30s; retries hREA writes with exponential backoff up to M=5 attempts; flips to `Failed` after exhaustion.

**Read consistency during pending projection windows:** if a VF query is for an entry that's `Pending` (committed in EPR but not yet in hREA), the bridge can synthesize the VF response on the fly from the EPR atom contents. **Deferred to Wave 3b** unless usage data shows the gap matters. For Wave 3, eventually-consistent semantics via reconciliation are acceptable.

**Audit clarity:** EPR atoms are canonical, so "what really happened" is always answerable from elohim's substrate regardless of hREA projection state. The flywheel benefit: VF clients see best-effort hREA semantics; elohim peers see canonical EPR.

**Upstream-contribution candidate from this seam:**
- Social-context fingerprint as a VF response extension (`extensions.elohim.socialContext`) — could become `EconomicEvent.socialContext: SocialContext` in VF proper

## 4. Component design

### 4.1 `bridges/valueflows` crate structure

```
bridges/valueflows/
├── valueflows-bridge/
│   ├── src/
│   │   ├── lib.rs              # mount(router) -> Router entry point
│   │   ├── routes.rs           # /api/v1/vf-graphql endpoint handler
│   │   ├── handshake.rs        # /api/v1/vf-graphql/bindings handler
│   │   ├── translate/
│   │   │   ├── mod.rs          # TranslationPoint instrumentation
│   │   │   ├── agent.rs        # Person, Organization, ElohimAgent translation
│   │   │   ├── proposal.rs     # Proposal + Intent translation (R&O hot path)
│   │   │   ├── commitment.rs   # Commitment + EconomicEvent + Agreement
│   │   │   └── resource_spec.rs # ResourceSpecification translation
│   │   ├── reconciler/
│   │   │   ├── mod.rs          # background worker for VfProjectionExpected
│   │   │   └── retry.rs        # exponential backoff
│   │   ├── extensions/
│   │   │   ├── mod.rs          # extensions.elohim.* response composition
│   │   │   └── opt_in.rs       # SDL directive + X-Elohim-Extensions header
│   │   └── ledger/
│   │       ├── mod.rs          # learning-ledger schema + writes
│   │       └── report.rs       # M5 ledger aggregation → upstream-inventory + R&O compat
│   ├── tests/                  # unit tests for translation functions
│   └── Cargo.toml              # consumed-by: elohim-storage
├── valueflows-types/           # shared types (TranslationPoint, etc.)
│   ├── src/lib.rs
│   └── Cargo.toml
└── valueflows-tests/
    ├── tests/
    │   ├── seams/              # one file per ontological seam
    │   ├── conformance/        # VF conformance via vf-graphql/tests
    │   └── rno_compat/         # R&O compatibility smoke
    └── Cargo.toml
```

### 4.2 The learning ledger schema

Every translation invocation writes a row:

```rust
pub struct TranslationPoint {
    pub at_block_height: u64,
    pub direction: Direction,                  // Read | Write
    pub vf_type: String,                        // "EconomicEvent", "Proposal", ...
    pub elohim_source: String,                  // "hREA::EconomicEvent" | "elohim::EprAtom" | ...
    pub translation_kind: TranslationKind,
    pub semantic_cost: SemanticCost,
    pub ontological_commitment: Option<OntologicalCommitment>,
    pub client_capability: ClientCapability,    // StockVf | ElohimAware
    pub code_location: &'static str,
    pub notes: Option<String>,
}

pub enum TranslationKind {
    IdentityShape,     // identical shape, just route to right DNA
    FieldRename,       // shape identical, names differ
    SemanticBridge,    // genuine domain difference (Reach, ElohimAgent, ...)
    Reconciliation,    // same fact in two DHTs, merge for read
    Sidecar,           // elohim-only data linked to canonical entry
}

pub enum SemanticCost {
    Mechanical,        // shape-equivalent translation; pure routing
    JustifiedDistinct, // real semantic difference → keep distinct
    UnclearYet,        // need more usage to judge
}

pub enum OntologicalCommitment {
    SovereigntyToStewardship,
    KeyAuthorityToSocialAuthority,
    FixedAudienceToReachClass,
    BilateralToRelational,
    IndividualWillToContribution,
    EntryToEprAtom,
}
```

Stored in a dedicated Diesel table `translation_observations` in elohim-storage. End-of-Wave-3 report aggregates and produces:

- **Upstream-contribution inventory**: list of `(TranslationKind::SemanticBridge | Sidecar, SemanticCost::JustifiedDistinct)` rows, grouped by VF type, with the elohim extension fields each touches. This becomes the PR-able list we hand to Lynn.
- **R&O compatibility report**: which R&O UI flows worked end-to-end, which failed, which translation points carried the failures. Feeds back into the upstream-contribution list and into our own substrate refinement.

## 5. Data flow

### 5.1 Write path (the load-bearing flow)

```
VF CLIENT (R&O UI, vf-graphql-holochain, etc.)
   │ POST /api/v1/vf-graphql { mutation: "createProposal(...)" }
   ▼
DOORWAY-SERVICE                              [web2 scaling shield]
   │ rate-limit, DDoS, TLS termination
   │ proxy unchanged to elohim-storage
   ▼
ELOHIM-STORAGE → BRIDGES/VALUEFLOWS :: handle_mutation
   │
   ├─ [1] IDENTITY BRIDGE  (seam: Agency)
   │     resolve caller_pubkey → VfBinding → Human CID
   │     ├─ no binding → 401 + binding_challenge_nonce
   │     │    (client posts to /api/v1/vf-graphql/bindings, retries)
   │     └─ binding ok → continue with Human CID + device_key
   │
   ├─ [2] AUTHORITY GATE  (seam: Authority)
   │     qahal_authority::evaluate_write_authority(
   │         human_cid, requested_act, requested_reach, target_audience)
   │     ├─ Authorized{social_context_fingerprint} → continue
   │     ├─ Denied{reason} → 403 + extensions.elohim_authority_denial
   │     └─ PendingAttestation{candidates} → 202 + sponsor list
   │
   ├─ [3] EPR ATOM EMIT  (seam: Truth substrate — canonical commit)
   │     write EprAtom to elohim DNA with full social-context fingerprint
   │     write VfProjectionExpected{status: Pending}
   │
   ├─ [4] ELOHIM SIDECAR ENTRIES  (only if client opted in)
   │     ReachAnnotation, FeedbackSignal, ContributionRecord
   │     linked to EPR atom CID
   │
   ├─ [5] HREA PROJECTION  (synchronous, with reconciliation backstop)
   │     hREA coordinator::create_proposal on Human's hREA cell
   │     ├─ ✓ success → Projected{hrea_hash}, return 200 with extensions
   │     └─ ✗ failure → Pending stays; return 503 with retry hint
   │
   ▼
RESPONSE { data, extensions.elohim.{eprAtomCid, socialContext, ...} }
```

### 5.2 Read path

```
VF CLIENT → DOORWAY proxy → ELOHIM-STORAGE → BRIDGES/VALUEFLOWS :: handle_query
   │
   ├─ [a] Parse VF GraphQL document; identify selected types
   ├─ [b] Fetch from hREA DNA (any peer's cell can serve reads)
   ├─ [c] Check VfProjectionExpected for entry status:
   │       ├─ Projected → use hREA entry directly
   │       ├─ Pending → fall back to EPR atom synthesis (deferred W3b)
   │       └─ Failed → not-found with extensions.elohim.eprAtomCid
   ├─ [d] Fetch elohim sidecar entries via link traversal
   ├─ [e] Compose response with extensions.elohim.* if client opted in
   │       (via @elohim SDL directive OR X-Elohim-Extensions header)
   ├─ [f] Log read TranslationPoints to ledger
   │
   ▼
RESPONSE { data: { ...VF fields..., extensions: { elohim: {...} } } }
```

### 5.3 Reconciliation worker (background loop)

Runs in elohim-storage, every ~30s:

```
scan VfProjectionExpected WHERE status = Pending AND age > 60s
for each entry:
    retry hREA projection using stored EPR atom contents
    ├─ success → mark Projected{hrea_entry_hash}
    ├─ failure (retry_count < M=5) → leave Pending, increment retry_count
    └─ failure (retry_count >= M)  → mark Failed{reason}, emit alert
```

## 6. Testing strategy

Eight test classes (per §5 of brainstorm transcript):

1. **Unit tests** for translation functions — per-TranslationPoint input/output, mocked qahal-authority, mocked hREA coordinator
2. **Integration tests** — test conductor with all DNAs including pinned hREA; real VF queries + mutations; verify partial-failure modes
3. **VF conformance** — consume Lynn's `/projects/research/vf-graphql/tests/`; replay queries against our endpoint
4. **Seam tests** — one file per ontological seam; explicit exercises of denial reasons, handshake protocol, extension fields, lazy cell provisioning
5. **R&O compatibility smoke** — point R&O dev instance at our endpoint; run their UI flows (create user → create request → accept offer)
6. **Sweettest coverage** — VfBinding entry + handshake; ElohimAgent; VfProjectionExpected state transitions; cross-DNA flows
7. **Reconciliation worker tests** — state with VfProjectionExpected at each status; verify retry behavior, backoff, Failed transition
8. **Learning ledger validation** — exercise each TranslationKind × SemanticCost × OntologicalCommitment cell; verify ledger query API

Per `feedback_shift_measure_jenkins`: CI-level validation runs on Jenkins. Local-only tests are unit + seam-tests; Sweettest + R&O-compat + VF-conformance require pipeline runs.

## 7. Milestones (M1 — M6)

**M1 — Substrate readiness.** Add hREA DNA to conductor (version-pinned). Create `bridges/valueflows` workspace skeleton. Mount empty `/api/v1/vf-graphql` route on elohim-storage. Ship VF read endpoint for one type (EconomicEvent) end-to-end as a tracer bullet.

**M2 — Identity bridge.** Implement `VfBinding` entry type in imagodei DNA. Build handshake protocol at `/api/v1/vf-graphql/bindings`. Create `elohim/qahal-authority` crate with `evaluate_write_authority` skeleton (authorize-all initially; real gates land in M3). Lazy per-Human hREA cell provisioning.

**M3 — Authority gate + write path for Proposal+Intent.** Full `qahal-authority` implementation with all four denial reasons. EPR atom emit + sidecar entries (ReachAnnotation, FeedbackSignal). hREA projection synchronous. Reconciliation worker. R&O's `createProposal` flow works end-to-end. Seam tests for all three seams green.

**M4 — Remaining VF types.** Mutations for EconomicEvent, Commitment, Agreement, ResourceSpecification. ElohimAgent entry type + dual-path translation (extension + compatibility). Multi-device fan-out as `Person.elohimAgentKeys[]`.

**M5 — Learning ledger reports.** End-of-Wave-3 deliverables generated from ledger data:
- Upstream-contribution inventory (PR candidates for VF/hREA — extension fields, new types, structured error shapes we found genuinely useful)
- R&O compatibility report (which flows work, which fail, which translation points carried the failures)

**M6 (optional) — Apollo Federation.** Compose `/api/v1/graphql` (native Viewer.*) + `/api/v1/vf-graphql` (VF surface) into a single federated subgraph. Clients query both worlds in one round-trip. Optional — decision deferred to end-of-M5 based on whether evidence shows real benefit.

Each milestone ships independently; M6 can be retired if not justified.

## 8. What this spec deliberately does NOT do

- Does not retire or migrate our existing elohim REA primitives. The bridge translates between VF and our substrate; our REA stays in elohim DNAs, hREA holds its canonical types, the translator bridges them.
- Does not engage Lynn Foster / Bob Haugen pre-implementation. Engagement happens at M5 when we have a concrete upstream-contribution inventory in hand and R&O compatibility validated — a more grounded conversation than "we want to support VF, here's what we're thinking."
- Does not implement read-through projection for pending VF queries (deferred to Wave 3b unless usage data demands it).
- Does not unify the two REA primitives. Our elohim REA stays for elohim-specific surfaces (Reach, FeedbackSignal, sidecar entries); hREA primitives live in their DNA for VF-shape clients. The bridge translates between them.
- Does not put VF-GraphQL on doorway. It's a protocol surface; lives in elohim-storage.
- Does not block on Apollo Federation. M6 is optional.

## 9. Open questions (for M5 retrospective)

### 9.1 Read-through projection for pending state

Should the bridge synthesize VF responses from EPR atoms when hREA projection is pending? Cost: duplicate translation code in the read path. Benefit: VF clients never see stale results during reconciliation windows. **Defer decision to M5** when we have evidence on how often pending windows last long enough to matter.

### 9.2 Per-Human cell lifecycle

What happens when a Human stops engaging with VF for an extended period? Do we keep their hREA cell forever, or eventually retire it? Retiring requires migration semantics; keeping is open-ended storage growth. **Defer to Wave 4** with stewardship-lifecycle framing.

### 9.3 ElohimAgent authority delegation

The `AuthorityScope` enum in `ElohimAgent` has three variants (`ReadOnly`, `DelegatedExchange`, `FullSteward`). The Human-side ceremony to grant `FullSteward` is unspecified — needs a real protocol (signed delegation chain? sponsored attestation?). **Defer to a follow-on imagodei spec** since this is identity-side, not interop-side.

### 9.4 Cross-bridge denial coherence

If doorway's OAuth-grant flow and valueflows' VF mutation flow both call `qahal-authority` with conflicting decisions for the same Human, which wins? Probably the more restrictive — but the resolution rule needs to be explicit. **Defer to Wave 4** as part of bridge-substrate maturation.

### 9.5 hREA upstream version churn

How tightly do we pin? What's our migration cadence when Lynn releases? **M5 retrospective input** — by then we'll have experience with at least one version change.

## 10. Implementation handoff

Implementation plan to be authored next at `genesis/docs/superpowers/plans/2026-05-20-wave3-valueflows-hrea-interop-plan.md` via `superpowers:writing-plans`. The plan will decompose M1-M6 into bite-sized tasks with file paths, code sketches, and commit boundaries.

**Sequencing constraints:**
- M1 (hREA DNA + endpoint skeleton) must land before M2-M6 work begins
- M2 (identity bridge + qahal-authority) must land before M3 (real writes)
- M3 must complete (with seam tests green) before M4 (more types)
- M5 is a documentation/analysis milestone that consumes data from M1-M4
- M6 is optional; decision at end of M5

**Out-of-scope additions filed as follow-on work:**
- FU-3: Read-through projection (Wave 3b, gated on M5 evidence)
- FU-4: Apollo Federation composition (Wave 3c, gated on M5 evidence)
- FU-5: ElohimAgent authority delegation ceremony (separate imagodei spec)
- FU-6: Cross-bridge denial coherence resolution (Wave 4 substrate work)

The Wave 3 close gates Gate C in the cross-wave guidance — at M5, with the bridge running, the upstream-contribution inventory in hand, and R&O compatibility validated, the next brainstorm sets Wave 4 framing.
