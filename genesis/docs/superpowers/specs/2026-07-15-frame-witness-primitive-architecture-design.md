---
title: "The Frame / Witnessed-Interaction Primitive — Canonical Home, Interface Contract, and Projection Architecture"
id: frame-witness-primitive-architecture
tier: spec
status: Draft
created: 2026-07-15
maintainers: Matthew Dowell + Opus 4.8
class: process-meta
process_subdomain: governance-substrate
topic: [witnessed-interaction, elohim-epr, ts-rs, projection, codegen, plant-eprfs, capability-contract, epr-element, dependency-injection, runtime-resolution, frame-ontology, dna-hash-neutral, magnitude-algebra, no-hand-rolling, drift-gate]
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-implementation
sovereignty-frame: descriptive  # migrates the sovereignty guard onto the shared primitive; quotes its phrases as detector data, never asserts the apex
refines:
  - genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md
  - genesis/docs/superpowers/specs/2026-07-15-eprfs-witnessed-interaction-primitive-design.md
cites:
  - sense-respond-governance-classifier | the governance instance whose Layer-A/frame-ontology this homes + projects; §8, §10.4B resolve_escalation, §14 | path: genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md
  - eprfs-witnessed-interaction-primitive | the parent primitive whose envelope (object_cid, substrate, action, magnitude) + extractor trait + 3-rung ladder this gives a home + contract | path: genesis/docs/superpowers/specs/2026-07-15-eprfs-witnessed-interaction-primitive-design.md
  - elohim-seam-map-concern-routing | the routing: SDK-seam (envelope type + frame data, compose inward) + T1 carriers + Track-4 element + mod/plugin extractor bank; no bridges, no brit engine change | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - .claude/skills/epr-content-addressing/SKILL.md
---

# The Frame / Witnessed-Interaction Primitive — Home, Contract, Projection

> **Why this spec exists.** The two parent specs designed *what* the primitive does. This one decides *where its deepest,
> most-performant form lives, what single interface/schema contract it exposes, and how that contract PROJECTS to every
> consumer* — so that no agent (Claude, Codex, Gemini) nor a human is ever lost hand-rolling a bespoke interface. A `.py`
> in `.claude/scripts/_lib` is a **projected consumer, never the root.** It also corrects a landmine both parent specs
> shipped (governance-as-a-substrate-member is a DNA-hash-moving change — §2.3).

## 1. Thesis

A **witness** emits an REA-shaped record — `(object_cid, substrate, action, magnitude)` — about a content-addressed
object; peer-sync validates it; it aggregates on the object CID, substrate-denominated. The governance/frame classifier
is **not analogous to but literally the same primitive** as a consumption meter, up to a choice of **magnitude algebra**
(`Vote{sign}` ordinal tally vs `Count{value,unit}` additive monoid). Two of its specializations *already exist* as
`EprKind`s (`FeedbackSignal`, `AttentionTending`). The job is to name their parent, give the classifier a third magnitude,
and stop every consumer from re-implementing the shape. Four commitments, each defended below:

1. **Define once at the deepest performant home** — `elohim/epr` (`elohim-epr`), the protocol's CID/envelope codec
   authority, already ts-rs-projected to the browser.
2. **Project through machinery the repo already runs** — `cargo test export_bindings` (ts-rs → `epr-ts`),
   `pnpm run schema:codegen:{ts,rs}`, and the `plant-eprfs` `project(import(source)) === source` fidelity gate. **No new
   projection mechanism is invented.**
3. **No consumer hand-rolls a bespoke interface** — enforced two ways: provenance gates (generated-matches-source) *and*
   a forbidden-primitive **usage** lint. Honest concession: enforceable for *ingredients*, advisory for *usage* (§4.5).
4. **Detail-up-front is lensed through the live `@capability*` contract system** — the web feedback `epr-element` becomes
   a projection whose frame-chips derive from the ontology and whose gating threads the existing `standing.ts` DSL — never
   a bespoke `<textarea>` form (§6).

## 2. The home + core content-addressed shape

### 2.1 Home decision: `elohim/epr` — new module `src/witness.rs`

| Candidate | Verdict | Defense (tree-verified) |
|---|---|---|
| **`elohim-epr` (chosen)** | **Home** | *The* canonical-bytes/CID authority (`cid.rs`: CIDv1, dag-cbor `0x71`, sha2-256; `envelope.rs::canonical_bytes` alphabetical-key `BTreeMap<String,Ipld>`). Already owns `EprKind`/`Reach`/`Coupling`/`Signature` and the **only live multi-language bridge** (`#[ts(export, export_to="../../sdk/epr-ts/src/generated/")]`). In-monorepo, protocol-owned. Two members already live here. |
| **`brit-epr::elohim`** | Rejected as home; kept as graduated projection | `brit` is a submodule fork of gitoxide; `brit-epr::BritCid` is a *parity re-impl* of `elohim-epr`'s codec. **`brit-epr/Cargo.toml` declares `elohim-protocol = ["dep:elohim-epr"]`; `elohim/epr/Cargo.toml` has NO brit dep** — homing the shape in brit is a **Cargo cycle**. |
| **New crate `elohim-witness`** | Rejected | Duplicates CID machinery (a third parity surface) and does **not inherit** ts-rs → `epr-ts` — forcing a bespoke TS bridge, the exact anti-goal. |
| **A DNA entry as root** | Rejected as root; correct at rung 3 | The primitive must exist at the **local-witness rung before notarization** (offline ticks); a DNA-only identity cannot exist pre-notary. The envelope *projects to* a DNA carrier only at rung 3. |

**Dependency invariant:** `elohim-epr` is the hub; `brit-epr`, `eprfs`, `elohim-storage`, the DNA zomes, and the `.claude`
tooling are **all leaves that consume it.** Verified: `EprKind::` is consumed only in `elohim-storage/src/{p2p,api,services}`
+ brit — **no integrity zome depends on `elohim-epr`**, so adding an `EprKind` variant is **DNA-hash-neutral**.

### 2.2 The witnessed-interaction envelope — the parent

All types derive `Serialize, Deserialize, TS` with `#[ts(export, export_to="../../sdk/epr-ts/src/generated/")]` — identical
to `Envelope`/`EprKind` today, so `cargo test export_bindings` fans them into `epr-ts` with **zero new codegen path**.
Every `Cid` field carries `#[ts(type="string")]` (as `Envelope`'s already do).

```rust
// elohim/epr/src/witness.rs
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub struct WitnessedInteraction {
    #[ts(type = "string")] pub object_cid: Cid,   // REA Resource; the aggregation anchor (auto-dedup theorem)
    pub substrate: SubstrateSignal,               // attention|compute|storage|bandwidth|energy|time|resource
    pub action: ReaVerb,                          // use|consume|produce|cite|affirm|dismiss
    pub magnitude: Magnitude,                     // algebra-tagged (2.3) — GOVERNANCE RIDES HERE, not in substrate
    pub witness: WitnessId,                       // v1 self-reported {claude|codex|gemini|human}; graduated agent_initial_pubkey
    pub coverage_span: Option<Span>,              // window covered; supports empty-never-projects
    pub issued_at: DateTime<Utc>,
    // wall_clock is advisory/local-only, stripped at sync (fleet-safety); NOT in canonical_bytes.
}
```

Emitted inside an `Envelope` as **one new `EprKind::WitnessedInteraction` variant** (precedented twice), inheriting
`compute_cid`/`reach`/`coupling`/`proof`/`supersedes`.

### 2.3 The magnitude algebra — the axis of specialization (must-fix #1)

**Resolved landmine, binding on both parent specs:** `governance` is **NOT** a `SubstrateSignal` member. Verified:
`elohim/sdk/schemas/v1/enums/substrate-signal.schema.json` enumerates exactly `[attention, compute, storage, bandwidth,
energy, time, resource]` and carries `_dna: {constant: SUBSTRATE_SIGNALS, zome: content_store_integrity}`, validated in
the **integrity zome** (`if !SUBSTRATE_SIGNALS.contains(&sig)`). **The DNA hash covers integrity zomes** — adding a member
moves the hash, dragging in `ALLOW_DNA_REINSTALL`, agent re-key, lineage/migration, and P2P-partition risk on the alpha
pair. Both parent specs listed `governance` as an 8th substrate "additively/cost-free"; **it is the single most expensive
change in the design.** Governance rides as a **magnitude variant over an existing substrate**, never a new resource
dimension:

```rust
pub enum Magnitude {
    Count(f64, Unit),              // additive monoid, ℝ≥0 — consumption (identity 0 = abstain = empty-never-projects)
    Vote(i8),                      // ordinal tally, signed — governance affirm/dismiss
    Classification(Cid),           // frame_ref → the frame atom (§3); FrameClassification is CID-referenced, not embedded
}
```

`Classification` carries a **`Cid` pointing at the frame atom**, not a typed cross-crate embedding — keeping
`WitnessedInteraction` self-contained in `elohim-epr` (no brit type-dependency → no Cargo cycle → the ts-rs cross-crate
`../../../../` import trap is structurally impossible: all `#[derive(TS)]` types stay in one crate, one `export_to`).

### 2.4 `FrameClassification` — split durable-truth from resolution-provenance (must-fix #2)

**Resolved:** the graduated escalation ladder (`compute_source`, `resolved_by_hop`, `bounded_by`) must **NOT** be in the
v1 content-addressed shape. If it is in `canonical_bytes` it is in the CID; when the H3 peer-native leg lands and a record
sets `resolved_by_hop: H3`, the *same conceptual classification* mints a *different CID* and the aggregation anchor moves
under you. Split the durable judgement from its resolution provenance:

```rust
// The durable, content-addressed judgement — what the CID hashes over.
pub struct FrameClassification {
    #[ts(type="string")] pub target_cid: Cid,   // CID of the write/EPR judged (v1: server-minted path digest — honest gap, §9)
    #[ts(type="string")] pub frame_ref: Cid,    // CID of the frame atom (SDK-schema data, §3)
    pub verdict: FrameVerdict,                   // Legitimate | Drift | Abstain (abstain first-class)
    pub confidence: f64,                         // [0,1]; coarse proxy at v1
    pub evidence: FrameEvidence,                 // spans (ORIGINAL content), matched_recall_signal, rubric_answer, reason_ref
}
// Operational/provenance — COUPLED to the classification, NOT in its CID.
pub struct ResolutionProvenance {
    #[ts(type="string")] pub classification_cid: Cid,   // couples to the FrameClassification
    pub classifier_agent: ClassifierAgent,              // v1 self-reported; graduated agent_initial_pubkey
    pub compute_source: ComputeSource,                  // terminal | api-key | peer-native
    pub resolved_by_hop: Hop,                           // H0 | H1 | H2 | H3
    #[ts(type="string")] pub bounded_by: Option<Cid>,   // delegates-compute Commitment CID, present at H3
}
pub enum FrameVerdict { Legitimate, Drift, Abstain }
```

**Schema-checked invariant:** a **detector** (Family-2) frame may only emit `Abstain`, never `Drift`; a **defeater**
(Family-1) may emit `Legitimate`/`Drift`. Suppression is Layer-B's job; a labeling function may only over-fire.

### 2.5 The extractor trait — off the codec hub

The `Extractor` / labeling-function **trait signature does NOT live in `elohim-epr`** (keeps the codec crate pure data,
leaf-friendly). It lives in a thin `elohim-witness-sense` module (or `elohim-storage`/`steward/device`), *importing* the
`elohim-epr` shapes:

```rust
pub trait Extractor: Send + Sync {
    fn tag(&self) -> ExtractorTag;                     // which frame atom / countable substrate it senses
    fn extract(&self, w: &RawSignalWindow) -> Option<Magnitude>;  // None = ABSTAIN; empty-never-projects
}
```

The governance-frame classifier is **one more registered `Extractor`** returning `Some(Magnitude::Classification(_))`. One
registry hosts both the consumption sensors and the classifier — the **mod/plugin seam**, compile-time-composed at v1.

## 3. The single interface/schema contract — three authoritative sources

Mirrors the repo's existing three-source model exactly:

| # | Source of truth | Governs | Projects to (existing machinery) |
|---|---|---|---|
| **S1** | **Rust structs in `elohim-epr::witness`** | the envelope + `Magnitude` + `FrameClassification` + `ResolutionProvenance` | ts-rs `cargo test export_bindings` → `epr-ts/src/generated/*.ts`; consumed by `@elohim/epr` (browser) + every Rust runtime by linking |
| **S2** | **Frame-ontology atoms — plain JSON `elohim/sdk/schemas/v1/frames/*.json`** | the frame vocabulary ("data, not a model prompt"): `id@version`, `family` (defeater\|detector), `polarity`, `linguistic_definition`, `rubric`, `recall_signal`, `cost_class`, `binding`, `cites` | `schema:codegen:rs` → Rust `const` frame tables; `schema:codegen:ts` → frame constants fanned to consumer dirs; provenance via `eprfs-agent compose-graph` |
| **S3** | **DNA integrity-zome whitelist** (`content_store_integrity`: `SIGNAL_KINDS`, `STANDING_IMPACTS`) | the **carrier** notarized taxonomy (rung 3) | already mirrored into `schemas/v1/feedback-signals/` → `schema-enums.ts` (`SignalKind`) |

**S2 is the decisive fork (completeness-critic (d)).** Resolved: **frame atoms are plain SDK-schema JSON, NOT homed in
brit-epr and NOT an `elohim-storage` HTTP view over brit** — so `elohim-storage` reads them as local schema data with **no
`brit-epr` dependency** (avoiding the auth-required Nexus `cargo-internal` read-token that a brit-atom home forces onto the
storage build). brit-epr *consumes* the atoms at graduation (sealing them as `EprMeta` subtrees for CID provenance); it
does not define them. The **hybrid** (D1's schema-data home + the plant-eprfs fidelity gate for authoring/projection) is
the resolution.

**One generator per enum (must-fix #3).** Shared enums (`compute-source`, `rea-verb`, `frame-verdict`, `frame-family`) are
**either** schema-rooted **or** ts-rs-only — never both (dual-sourcing re-creates the recorded `reach-enum-drift`
incident: schema 8 vals vs Rust 8 *different* vals). A conformance test asserts the two never both exist for one name.
`frame-verdict`/`frame-family` are ts-rs-rooted (they are `FrameClassification` field types); `substrate-signal` stays
schema-rooted (DNA-notarized — no ts-rs peer). **Adding a frame is one atom + one cite in S2** (SDK-seam, touches no
engine code); **adding a magnitude algebra touches S1** (a closed-enum edit — the honest cost, §9.A).

## 4. The projection pipeline + the drift/fidelity gate

- **4.1 → TS (web `epr-element` + SDK).** `cargo test export_bindings` from `elohim-epr` → `epr-ts/src/generated/`. **Gate:**
  ts-rs freshness sha256 diff at pre-push. The Lit element imports `WitnessedInteraction`/`FrameClassification`/
  `FrameChipView` from `@elohim/epr` — never a local interface (the `elohim-imagodei-contributor-card → ContributorPresenceView`
  pattern).
- **4.2 → Rust consumers.** `schema:codegen:rs` → enum consts for `elohim-storage`, `steward/node`, the `Extractor`
  registry. **Gate:** codegen `--verify` freshness in pre-push.
- **4.3 → Python (`.claude`/`.codex` hooks).** The atoms' `recall_signal` blocks project into a **generated**
  `.claude/scripts/_lib/generated/frame_ontology.py` (phrase lists, markers — **data, no logic**).
  `_lib/frame_classifier.py` exposes `classify(write) -> FrameClassification` / `triggers(write) -> list[TriggerHit]`, its
  detector params *imported* from the generated module. Authored through the **existing** `plant-eprfs` machinery: a
  `FramePackage` kind under `.epr-meta/elohim/packages/frames/*.json`; `package-projections.mjs` gains a `FramePackage`
  branch (**additive** — it switches on `pkg.kind`). **Gate:** `pnpm run elohim-agent:packages:verify` enforces
  `project(import(source)) === source` byte-for-byte; drift lodges a deduped finding. For verbatim data projection the
  projection CID **equals** source CID — proof it is a mirror, not a fork.
- **4.4 → `brit-epr` provenance (graduated).** New `brit/brit-epr/src/elohim/witness/` on the unmodified `engine`
  chassis. **Gate:** a **hard, non-skippable, raw-CID three-way golden vector** (must-fix #4). The *existing* brit guard
  (`cite_parity.rs`, skip-on-oracle-absent + verdict-label only) is **weaker** than needed: a new kind can serialize to
  three different CIDs across `elohim-epr`/`brit-epr`/`eprfs-core` with every current test green. Before the first
  `WitnessedInteraction` ships: one canonical envelope → assert **identical CID string** in all three engines,
  non-skippable (a fixed constant needs no oracle).
- **4.5 The anti-hand-roll gate — usage, not just provenance (must-fix #5).** Gates 4.1–4.4 verify
  *generated-matches-source*; none verify *used-not-bypassed* — nothing stops a future agent adding a hook with an inline
  `if "self-sovereign" in text` in an unscanned file. Add a **forbidden-primitive lint** (an eslint `no-restricted-syntax`
  + a ruff/grep check that bans the raw ingredients — `.count(` over frame-phrase lists, substring scans on frame vocab —
  *outside* the generated module). **Honest concession stated in the doc:** "no hand-rolling" is a build invariant for
  *ingredients*, **advisory for usage**; semantic Family-2 detectors are model inference and cannot be proven stable.

## 5. The DI + runtime-context-resolution pattern (reuse, don't invent)

**One injection interface, two-stage resolution**, fusing two proven repo mechanisms:

- **Resolve-by-name (which primitive)** = `REFERENCE_VALIDATORS` shape (`epr_meta.py`): string-keyed registry
  (`"epr:witness-<name>"`); **unregistered ref degrades to advisory, never a hard failure.**
- **Resolve-by-context (which backend serves it here)** = `resolve_agent_url()` shape (`elohim_agent.rs`):
  health-probe-then-fallback; the caller never knows which answered.

```rust
pub fn resolve_witness(binding: &BindingRef, ctx: &RuntimeContext) -> Arc<dyn Witness>;
pub trait Witness: Send + Sync {                    // consumers depend on `dyn Witness`, never a concrete backend
    fn observe(&self, env: &WitnessedInteraction) -> Verdict;   // PURE: same envelope → same Verdict
    fn substrates(&self) -> &[SubstrateSignal];
}
// resolution: registry.get(binding) or AdvisoryWitness; then match ctx.compute_source:
//   peer-native → health-probe elohim_node_url; else fall through | api-key → DoorwayWitness | terminal → LocalWitness
```

Config surface (all pre-existing shapes): `ELOHIM_COMPUTE_SOURCE` env / `compute_source` TOML (`TransportBackend` enum
precedent, `#[default] Terminal`); `elohim_node_url`/`elohim_agent_url` (verbatim from `resolve_agent_url`); binding rows
in `elohim/sdk/schemas/v1/manifests/witness-bindings.json`. `PeerWitness` is `#[cfg(feature="p2p-iroh")]`-gated; *which*
backend answers is per-invocation runtime; *which* primitive is data-driven by the binding ref.

**The browser boundary — resolution is server-side only (honest limit).** `resolve_witness`/`resolve_agent_url` is Rust,
server-side; the browser Lit element **cannot** call it. The honest architecture is `browser WitnessClient.observe() stub
→ doorway POST /api/v1/witness/observe → server-side resolve_witness`. **That route does not exist** and (per
`project_doorway_main_route_needs_is_service_path`) needs **both a match arm and `is_service_path`** or the EPR router
shadows it to the SPA bundle. "Every consumer in every language calls exactly one interface" is **false at the browser**
and must not be claimed; one interface *signature* survives, the resolver does not cross the boundary.

**Adding a consumer = one registry line + one binding row + zero interface code.** Behavioral fidelity: attach a **golden
vector** (`{sample_envelope, expected_verdict}`) to each binding row; the per-language conformance test asserts the
registered ctor reproduces it — upgrading "superset of names" to behavioral parity (a name-only test passes a ctor that
silently returns advisory-forever).

## 6. Capability-contract lensing — the feedback `epr-element` as a derived projection

The web feedback component is the **third witness surface** (agent-at-edit-time · async-semantic-recall ·
human-at-feedback-time), sharing one frame bank and one aggregation anchor. Mechanism is **live and verified**:
`@capability*` JSDoc tags extracted by `cem-plugins/capability-contract.mjs`, graded at runtime by `capability/mixin.ts`
(`CapabilityAwareElement` + `ContextConsumer`) + `capability/standing.ts` (`satisfiesRequirement` DSL).

**6.1 Both feedback surfaces must be disposed (resolved).** The Lit `elohim-gate-feedback-trigger.ts` only *dispatches* a
`feedback-submit` CustomEvent; the actual `createComment()`/`IssueReportService.createReport()` calls that **bypass the
notarized `FeedbackSignal`** live in the **Angular `gate-feedback-modal.component.ts`**, which **re-hardcodes the same
`flag|challenge|feedback|report` taxonomy a second time.** The migration must migrate `FrameChipView` into **both** and
**delete the un-migrated Angular `gate-feedback/*` as dead code in the same PR** — else the bespoke governance form the
directive abolishes is left standing on the surface that ships.

**6.2 The derived-chip projection.** `FrameChipView` (ts-rs wire-shape) replaces the hardcoded `DEFAULT_MENU_ITEMS` /
`GateFeedbackType` const: `{frame_ref, frame_id, label, family, polarity, signal_kind, default_standing_impact,
requires_evidence, required_standings, min_lens, cost_class}`. **Atom↔projection field-parity gate:** derive the
drift-sensitive fields *from the atom at projection time* (the CID is the parity guarantee); restrict any lamad-manifest
overlay to editorial fields (`color`, hover copy). Extend the verify gate from "id resolves" to "**atom fields == projected
chip fields**".

**6.3 "How detailed up front" — three orthogonal gates** (all on the live JSDoc mechanism):

| Axis | Tag / source | Gates |
|---|---|---|
| **Surface** | `@capabilityFrames <id\|family:X\|*>` | which frames appear |
| **Depth** | `@capabilityMaxLens` × `chip.minLens` × `profile.lens` | how much of each chip renders |
| **Viewer** | `@capabilityRequiredStandings` ∩ `chip.requiredStandings` | which chips this viewer sees |

The lens gradient maps onto disclosure: `minimal`/`simple` → free-text `<textarea>` only (the honest abstain floor, no
taxonomy shown); `standard` → chips + free-text; `detail`/`debug` → chips + per-chip `evidence_cid` + `Vote{sign}`
intensity. **Detail-up-front is literally the lens axis, operator-dialable** via `ProfileLock.maxLens`.
`@capabilitySubstrate`/`@capabilityMagnitude` are **deferred (YAGNI)** — v1 is governance-only, every chip emits `Vote`.

**6.4 Emission — free-text as first-class abstain.** Chip → `WitnessedInteraction{action: affirm|dismiss, magnitude:
Vote}` carrying `FeedbackSignal`-on-`target_cid`. `[ASSERTED]` at v1 — needs the browser client stub + doorway route +
storage-client method + `Into<>` plumbing (net-new across three layers). Free-text only → the existing `createComment`
(operational, un-notarized — the **human abstain**, mirroring `Extractor::extract() → None`). Empty → no emission
(empty-never-projects).

## 7. Migration of the two live detectors + the signal hook (parity-proven)

- `_p2p_design_gate` → frame atom `frame-p2p-design@1` (`family: detector`; substring scan → `recall_signal`).
- `_sovereignty_ontology_guard` → `frame-sovereignty-apex@1` (`_SOV_APEX_PHRASES` → `recall_signal.phrases`,
  `_SOV_FRAME_MARKER` → `recall_signal.marker`, `_sov_apex_count` → the generated net-new-apex counter).
- `sovereignty-guard-signal.py` → generalized `classifier-signal.py`, keyed on *which* frame fired, importing the **same**
  generated `frame_classifier.triggers()` the PRE side uses — so "the ledger and the gate can never disagree" becomes
  structural, not the hand-maintained `em._sov_apex_count` reuse it is today.

**Parity proof (merge precondition):** a golden-vector harness over a fixture corpus (replayed past writes + existing
`.claude/data/sovereignty-guard.jsonl` landings) asserts `old_detector(write) == new_generated_classify(write).verdict`
bool-for-bool, and the migrated hook's ledger output byte-identical line-for-line. **Honest limit:** parity is *provable*
for the declarative detectors (substring/apex-count); Family-2 semantic detectors are model inference — the gate proves
the *rubric/recall_signal round-trips*, not that the model's *output* is stable.

## 8. v1-buildable vs graduated (honest)

**v1 (buildable now, advisory-weight):** `elohim/epr/src/witness.rs` (`WitnessedInteraction`, `Magnitude`,
`FrameClassification` durable, `ResolutionProvenance` operational, `FrameVerdict`, one `EprKind` variant; ts-rs → `epr-ts`)
+ the **hard three-way golden CID vector**; **S2 as plain schema JSON** for the two existing detectors (proving "two guards,
one engine, zero engine edits"); `.claude` register `epr:validator-frame-classifier` loading the **generated**
`frame_ontology.py`; `epr-element` chips derived + standing-gated + lens-graded, textarea preserved as abstain, **Angular
`gate-feedback/*` disposed**; `compute_source ∈ {terminal, api-key}`; `target_cid` = **server-minted path digest**
(client-side CID computation forbidden); parity harness green (merge gate).

**Graduated (`[ASSERTED]`, not proven):** notarized `FeedbackSignal`-on-CID emission (browser stub + doorway
`/witness/observe` route + storage-client method + `Into<>` — net-new, not v1); the 3-rung ladder with the disinterest
gate; `brit-epr::elohim::witness` provenance; `target_cid`/`witness` → real CIDs / `agent_initial_pubkey`; frame atoms →
`Mishpat::Precedent`; `compute_source = peer-native`; `Magnitude::Count` + full REA aggregation + the consumption
light-runtime registering `Extractor`s into the **same** registry; **governance as a distinct `SubstrateSignal` member —
a named, DNA-hash-moving change, never folded into a v1 schema edit.**

## 9. Open decisions (for the architect)

- **A — Closed `Magnitude` enum vs "open set of algebras."** ts-rs cannot project a trait object, so `Magnitude` is closed
  — a fourth algebra touches the core crate, mildly violating "add capability without touching the engine." Escape hatch:
  `Magnitude::Opaque { schema_ref: Cid, payload: Ipld }` (surrenders type-safety). Unresolved.
- **B — Per-invocation vs boot resolution, per surface.** The agent-gate wants per-call freshness; a page-turn-frequency
  consumption meter wants resolve-once-at-boot (`TransportBackend`'s trait-object shape). Does splitting
  `resolve_witness`/`resolve_witness_cached` leak the backend choice back to the caller the interface promised to hide?
- **C — Abstention magnitude (the one place the unification is genuinely strained).** `Count` has a clean identity (`0` =
  abstain); `Vote{0}` is a *cast neutral ballot*, ≠ *no ballot*. A Family-2 detector's abstention is real evidence ("N
  witnesses looked and declined"), which cannot be modeled as absence — yet empty-never-projects says absence *is* how
  non-interaction is modeled. Does `Magnitude` need `Abstain{observed:true}`, or does abstention ride entirely in
  `Verdict` and never enter the aggregation envelope? (Graduating human abstain to notarized needs a new T1 `signal_kind`
  — DNA-hash-moving.)
- **D — Rust→Python: data-only, or a fourth codegen pipeline?** The ontology projects as pure shared data; the ~40 lines
  of register/vote/abstain *mechanism* stay hand-written-conformant in Python, golden-vector-test-guarded not
  codegen-guaranteed. Build a minimal Rust→Python IDL projector, or accept the tested shell?
- **E — One artifact, three homes.** A frame is simultaneously schema-typed SDK data (§3), a `plant-eprfs FramePackage`,
  and — graduated — a DHT `Precedent`. Strict layering (schema validates shape, package owns content, DHT owns notarized
  existence) is the clean answer but is asserted, not yet proven under an edit touching all three.

## 10. Decomposition seed

**Must-fix-before-build (bind the parent specs too):** (1) governance = `Magnitude::Classification`, never a
`substrate-signal` member; (2) split `ResolutionProvenance` out of the v1-CID'd `FrameClassification`; (3) one generator
per enum; (4) the hard non-skippable three-way golden CID vector; (5) the forbidden-primitive usage lint; (6) dispose the
**Angular** `gate-feedback/*`, not just the Lit element. Gaps: **(G1)** `elohim/epr/src/witness.rs` + the `EprKind`
variant + ts-rs → `epr-ts` + the golden CID vector; **(G2)** S2 frame atoms (`frame-p2p-design@1`, `frame-sovereignty-apex@1`)
+ `schema:codegen:{rs,ts}` + the `FramePackage` branch in `package-projections.mjs` + the verify gate; **(G3)** the
generated `frame_ontology.py` + `frame_classifier.py` + the migrated `classifier-signal.py` + the **parity harness** (merge
gate) + the forbidden-primitive lint; **(G4)** `FrameChipView` + the derived-chip `epr-element` + `@capabilityFrames`/lens
gating + **dispose Angular `gate-feedback/*`**; **(G5)** `resolve_witness` + the binding manifest + golden-vector
conformance; **(G6, graduated)** the doorway `/witness/observe` route + browser client stub + notarized emission +
`brit-epr` provenance + `Magnitude::Count` consumption runtime. G1–G4 are the household-testable spine (advisory,
zero-DNA-move); G5 wires DI; G6 is graduated. The home (`elohim-epr`) + projection reuse (ts-rs + `schema:codegen` +
`plant-eprfs`) are settled and tree-verified; §9.C (abstention magnitude) and §9.D/E are the remaining forks.
