---
title: "The Weave Epic — recursive capacity, tier-capability, compute contracts, private-replica encryption (an index over four arc subsystems seeded by the operational-weave lens)"
id: weave-epic-arc-design
status: Draft
class: protocol-canonical
domain: D5
topic: [weave, operational, recursion, vsm, councils, tiers, capability, compute-contracts, rea, sharding, encryption, dataplane, epic-index]
cites:
  - operational-weave-facing-lens-design | the lens this epic seeds from (#0); its hand-written aggregate() is what #1 replaces with CoverageRollup descent | sha256:fc432fea065dca00 | path: genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md
  - recursive-architecture-design | canonical home for #1; its CoverageRollup keystone (recursion.rs) is built-but-unconsumed and #1 wires it in | sha256:053f260af9989d4b | path: genesis/docs/superpowers/specs/2026-06-14-recursive-architecture-design.md
  - tiered-quilt-stewardship-design | the tier canon; §6 already names #2 storage-capability as earned-witnessed (not self-declared) | sha256:9f9c6a1c391712b3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
  - genesis/docs/architecture/rea-compute-commitment-primitive.md
  - rea-economic-facing-lens-design | #3 lands its two named non-goals: the delegates-compute bridge + the observed realized-compute event | sha256:b83ead21be13bbaa | path: genesis/docs/superpowers/specs/2026-06-19-rea-economic-facing-lens-design.md
  - mutual-storage-replication-dwelling-hub-design | the First REA Compute-Commitment Instance; #3 inherits its bilateral/self-directed dual | sha256:1acbeeec8b7a3956 | path: genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md
  - epr-durability-replication-arc-plan | #4 build home (new Private-Replica Encryption workstream); owns the real distribute_shards path | sha256:f263ed845af2f916 | path: genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md
  - dht-is-a-notary-not-a-byte-store | the binding constraint: capacity/rollup aggregation is gossip+projection, never a DHT entry | sha256:a1d408ef2478b288 | path: genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md
informed-by:
  - genesis/docs/superpowers/specs/2026-06-14-recursive-architecture-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
# Mixed-env epic (CLAUDE.md scope convention): NO doc-level requires_env so the Wave-A
# household-nodes-testable work stays fair-game. The ONLY blocked leg is #4's LIVE delivery
# (tagged @requires inline in #4's Dependencies — gated on the conductor-leak fix + an X25519 reader-key substrate,
# NOT on a cluster capability). Every Slice-0 in this index is DB-free / single-host / household-nodes.
---

# The Weave Epic — arc index

> **One line:** the operational-weave *lens* (a read-only projection of the per-cluster weave) is
> already specced and seeds an epic of four subsystems — recursive VSM aggregation, a tier-capability
> registry, REA compute contracts, and private-replica encryption. This document is the **index**: it
> does not re-design those subsystems from scratch (most of them already have canonical homes), it
> *places* each one — compose-home, P2P-gate entity verdict, approaches, the one operator fork, the
> dependency edges, and the unblocked Slice-0 — and fixes the wave sequence.

## Provenance & framing

Surfaced 2026-06-20 from the operator's framing that the operational-weave lens is "more foundational"
than the sibling facing lenses because it underwrites: recursive hub aggregation (Stafford Beer's Viable
System Model / "Freedom Machine"), agent-legible storage/bandwidth/capacity forecasting, weave-tier
capability ("which tiers can a peer serve?"), REA compute contracts for whole apps, and sharding/striping
with privacy encryption for the resiliency epic.

**The headline the P2P design gate produced:** across all four subsystems the *only* genuinely-new DHT
entry type is #4's `KeyEnvelope`. Everything else reuses an existing entry, action discriminator,
attestation subtype, or is a pure operational fold. This is a **compose-don't-fork** epic — and #1's
keystone (`CoverageRollup`) is already implemented and sitting unconsumed in the tree.

**Binding canon honored throughout** (do not re-litigate):
- *DHT is a notary, not a byte store* (`2026-06-01-dht-is-a-notary-not-a-byte-store.md`). Capacity /
  inventory / rollup aggregation is **gossip + local projection**, never a DHT entry. ContentLocation-on-DHT
  was tried and lost.
- *iroh + libp2p are permanent dual transport*, not a cutover.
- *Sharding already ships* — `elohim/elohim-storage/src/sharding.rs` (≤16MB single, 16–64MB chunked,
  >64MB `rs-4-7` reed-solomon), `shard_protocol.rs`, transport-neutral `shard_service.rs`, `blob_store.rs`.
- *`agent_cid` (`uhCAk…`) is the canonical identity*; libp2p `12D3Koo…` / iroh `NodeId` resolve TO it via
  `AgentPeerBinding → peer_identity_bindings → peer_transport_manifest`. Never raw-string-compare across
  namespaces (caused the all-zeros resilience card). ⚠ The binding is **self-asserted / unsigned today**
  (`STAGE1_SIGNATURE_SENTINEL`) — do not consume it for *economic attribution* until a cross-signed proof
  lands. `custodian_metrics`, `shard_locations.peer_id`, and seeder-written `rea_commitments.provider` are
  already `agent_cid`-keyed, so the near-term folds join on `agent_cid` directly and do **not** depend on
  the binding.

## Map

| # | Sub-project | Composes into (extend, do not fork) | P2P-gate verdict | Wave |
|---|---|---|---|---|
| **0** | Operational-weave lens (capacity *eyes*) | the charter — already specced | Operational-C, **0 new entry types** | A (now) |
| **1** | Recursive aggregation (VSM councils) | recursive-architecture §2.1 + the lens; **`CoverageRollup` built, unconsumed** (`recursion.rs`) | council = `{"kind":"council"}` on the **existing `Collective` entry**; rollup = Operational-C BLAKE3 recompute. **0 new entry types** | B |
| **2** | Tier-capability registry | the lens (sibling fold) + tiered-quilt §6 (names it `storage-capability`) | measured = Operational-C fold; earned = elohim `attestation:storage-capability` Content **subtype-string**. **0 new entry types, 0 new tables** | A (measured-only) |
| **3** | REA compute contracts (whole apps) | rea-compute-commitment-primitive + rea-economic facing | **existing** Mishpat `Commitment` `delegates-compute` action + **existing** EconomicEvent new action; reward = **existing** `appreciation`. **0 new entry types** | A (base) / B (extension) |
| **4** | Private-replica encryption | epr-durability-replication-arc plan (new workstream) | `ShardManifest` field-add + **1 new `KeyEnvelope` entry**; per-household-keyed DEK, encrypt-then-RS | C (Slice-0 anytime; live blocked) |

---

## #1 — Recursive weave aggregation (VSM councils)

**Compose-into.** `2026-06-14-recursive-architecture-design.md` §2.1 (the `CoverageRollup` aggregate-with-descent
operator — *is* Beer's VSM rollup) and the operational-weave charter. The keystone is **already implemented and
lib-wired but unconsumed**: `elohim/elohim-storage/src/recursion.rs` (`CoverageRollup::rollup` at `:216`, BLAKE3
`rollup_hash` at `:258`, `descend()` at `:273`; N=1 tests pass; `pub mod recursion` at `lib.rs:108`; no caller
outside the module). Two section-adds, no new spec: (a) the lens replaces its hand-written per-level
`aggregate(triptychs)` with `CoverageRollup` descent; (b) the recursive-architecture spec gains the operational
`WeaveView` rollup as its third named caller. This also cures the concrete `rows.len()` descent-erasure at
`graph_views/shefa/resilience_snapshot.rs:29,38`.

**Entities (P2P gate).**
- **CouncilRollup** — Operational-C. `required \ covered` recomputed-on-read from the MEMBER_OF/STEWARDS graph +
  gossip capacity projections; content-addressed by `rollup_hash = BLAKE3(scope, domain, covered, sorted
  constituents)` (consilience-as-agreement: two peers with the same children compute the same hash). No table,
  no `dht_anchor_hash`. HTTP: extend `GET /api/v1/weave?scope=<council_cid>&domain=…` (no new POST).
- **council-layer node** — Notarized-A, **reuse**: a new `{"kind":"council","parentLayerCid":<cid>}` charter value
  on the **existing imagodei `Collective` entry** (`kind` is JSON in the free-text `charter` field, not a typed
  column — zero schema change). MEMBER_OF (council ∈ region) is a Derived-Link-A2 over existing `epr_edge`. A
  `Council` entry type would burn imagodei headroom for nothing.

**Approaches.** (A) recursion-shaped from the start via `CoverageRollup` descent — **recommended** (keystone
built; cures the `rows.len()` debt; answers "build recursion-ready now vs retrofit" definitively — retrofitting
descent onto a flat `aggregate()` rewrites every fold's return type). (B) flat per-level now, recurse later —
re-mints the debt at N levels. (C) notarize rollups as a `CoverageRollupAttestation` DHT entry — the §2.4 GATED
fork, explicitly not taken preemptively; rejected for v1.

**The fork (per-plan decision point).** Does an operational capacity metric (free/used/stewarded) map onto a
`CoverageDomain` set-difference deficit, or is capacity a scalar that doesn't fit? **Recommendation — both, by
metric type:** byte-coverage (does the council hold its RS-required byteset) rides `CoverageRollup` descent
natively; raw free/used capacity is a scalar sum aggregated alongside it. Capacity-as-ratio is not keyspace
coverage; don't force it into `CoverageDomain`.

**Dependencies.** Lens #0 Slices 1–3 (the placement/capacity loaders + gauges the rollup folds over). Sibling:
recursive-architecture Wave 0 (`trait Governor` lift) + the conductor-signal msgpack-decode fix (§3.1 — a dropped
`holo_hash` poisons every rollup signal); Wave 2 (shefa-builder callers) should precede the operational caller.
One seeded `{"kind":"council"}` collective with MEMBER_OF edges (a seed row, no schema change). `household-nodes`
suffices for 2-level rollup; cross-region councils need `shem`.

**Slice-0 (unblocked, DB-free).** A pure proof that `CoverageRollup::rollup` composes **transitively** at N=2:
`rollup(region, [rollup(councilA), rollup(councilB)])` asserts (1) `region.covered == A.covered ∪ B.covered`,
(2) `region.descend()` reaches both councils, (3) an atom trapped in A's deficit is reachable by walking
`constituents` from the region rollup down. Proves "aggregation preserves descent" using only `recursion.rs` —
no loaders, gauges, DHT, or seed.

---

## #2 — Peer tier-capability registry

**Compose-into.** The operational-weave charter — a `tier_capability` fold sibling to `tier_occupancy`, reusing
`CustodianCapacityRow`/`ComputeTriptych` and the `agent_cid` join; new `TierCapabilityView`. And tiered-quilt §6
("Capability attestations subsume misreporting"), which **already names this `storage-capability`** and makes it
earned-witnessed, not self-declared — use that name; do not mint a parallel `servable-tier`. The registry is a
**read that `CommitmentFactory` consults** before binding a `custody-quilt`/`delegates-compute` commitment; it
does not mint commitments.

Three signals fold into a per-tier verdict: **declared** (the existing custody-quilt commitment's pledge
`tier_floor` — no new entity), **earned-witnessed** (`storage-capability` attestation, B2), **measured**
(`custodian_metrics`/`ComputeTriptych`, C).

**Entities (P2P gate).**
- **TierCapabilityView** — Operational-C, keyed on `agent_cid`. A pure fold over existing tables; **zero new
  tables, zero new entry types** (mirrors the resiliency facings). HTTP: a `tierCapability` field on
  `GET /api/v1/weave` (or `GET /api/v1/custodians/capability/{agent_cid}`).
- **storage-capability attestation** (earned branch only) — B2. The peer's claim "I can serve tier-N" is gameable
  if self-declared; its *effect* is peer-verifiable (peers who drew at warm latency witness it). ⚠ Correction to a
  common assumption: the imagodei `Attestation` entry type was **removed** (Stage C.2); attestations now ride the
  **elohim-DNA `Content` entry** as `attestation:*` subtypes (allow-listed in `generated_attestation_kinds.rs`,
  declared in `elohim/sdk/domains/infrastructure/manifest.json`). So this is a **new subtype string**, not a new
  entry type, not an imagodei entry. CID-addressed; uniqueness anchor
  `attestation:storage-capability:{subject_cid}:{period_start}:{tier}`.

**Approaches.** (A) measured-only fold — lands now, zero new substrate, immediately feeds `CommitmentFactory`,
but a peer with unproven capacity reads as capable. (B) earned-witnessed — what the canon says contracts *should*
rely on, gameable-resistant, but needs draw-probe instrumentation + witness emission + decay. (C) both,
declared-cross-checked-against-measured — richest signal (flags `capability-overclaim`), same dependency cost as B.

**The fork (per-plan decision point) — landing-cost.** Ship **measured-only** (pure Category-C fold, zero new
DHT/manifest) or include **earned-witnessed `storage-capability`** (new infra-manifest subtype + witness-emission
+ decay, blocked on the draw-probe instrumentation the quilt epic defers to its Wave 4)? **Recommendation —
measured-only now**, stub the `storage-capability` seam; it plugs into the same fold signature as an added arg later.

**Dependencies.** Lens #0's fold framework (`CustodianCapacityRow`/`ComputeTriptych` loaders — the capability fold
is a sibling of `node_capacity`). Earned branch: tiered-quilt Wave 4 (BreachScanner/draw-probe) + the
`storage-capability` manifest-subtype landing. NOT dependent on the unsigned `AgentPeerBinding` (inputs are
already `agent_cid`-keyed). `custodian_metrics` rows are live gossip projections (provable in-repo with hand-built
rows; live-alpha lighting is operator-owned, `requires_env: observability`).

**Slice-0 (unblocked, DB-free).** `servable_tiers(measured: CustodianCapacityRow, pledge_floor: Option<Tier>) ->
TierCapabilityView` — hand-built inputs, assert per-tier classification (free ≥ warm-budget ⇒ serves
`stocked-warm`; below ⇒ `stocked` only) + `TierCapabilityView` shape + ts-rs codegen. No DB/seed/DHT/observability.

---

## #3 — REA compute contracts for whole apps

**Compose-into.** `genesis/docs/architecture/rea-compute-commitment-primitive.md` (add a "whole-app compute
instance" row to its generalization table) and the `2026-06-19-rea-economic-facing-lens-design.md` (land its two
named non-goals: the mishpat→rea `delegates-compute` bridge and the observed-side realized-compute event that
makes `mutual_compute` stop being intent-only). Inherits the bilateral/self-directed dual from the
mutual-storage-replication dwelling-hub spec.

**Core insight that removes a crypto problem:** deterministic compute ⇒ content-addressed output, so **the output
CID *is* the proof** — any peer recomputes the closure → same CID → verification needs no new cryptography.

**Entities (P2P gate).**
- **delegates-compute commitment, whole-app scope** — Notarized-A, **reuse**. `delegates-compute.schema.json` +
  `Mishpat::Commitment` already ship; "whole app" rides as a `head_ref` + `closure_rule` in `bounds.epr_scope`
  (the EPR-closure ref from epr-acquisition-slice2b §6.7), **not** a new entity. **cid = entry_hash** (return
  entry_hash, not action_hash — `bounded_by`/`graduate`/`revoke` all key on it).
- **compute-fulfillment EconomicEvent** — Notarized-A, **existing** EconomicEvent entry + a **new action**
  (`compute-fulfilled`) carrying `output_cid`. `bounded_by` is a `metadata_json` annotation, not a column
  (no CID-as-FK); self-authored under the fulfiller's `agent_cid`.
- **compute-fulfillment-progress projection** — Operational-C (like `EprPullStatusView`); recomputable, keyed by
  commitment cid; `GET /api/v1/compute/{commitment_cid}/fulfillment`.
- **Reward is not a new entity** — routes through the existing `appreciation` EconomicEvent (provider→receiver
  mutual credit), `bounded_by` the commitment. `StewardshipAllocation` stays orthogonal (per the rea-lens spec).

**Approaches (verification strength).** (1) optimistic recompute (trust-and-debit; CID-mismatch → FeedbackSignal)
— **recommended**, cheapest, structurally ready for quorum, no new crypto. (2) N-of-M quorum (majority `output_cid`
wins) — crowdsource-native, costlier. (3) cryptographic proof — defer (matches the storage spec's deferral).

**The fork (per-plan decision point) — accountability/reward shape.** Single accountable delegate (bilateral,
Z.D-style, one reward recipient, reciprocity-audited) vs flat commons crowdsource (self-directed,
graduate-on-first-fulfill, reward splits N-ways). No canon default — storage shipped *both* as separate
operator-chosen sprints. **Recommendation — defer to build; lean bilateral-first** as the proving ground. Branches
the data model (recipient field vs self-directed), reward split, and whether a reciprocity-audit service runs.

**Dependencies.** #2 tier-capability (who *can* fulfill / quorum size). Lens #0's `commitment_backed` read +
`MishpatCommitmentView`. The `bounded_by`-is-annotation emitter wiring. Live shefa `appreciation` routing. Do not
attribute reward via the unsigned `AgentPeerBinding`; join on `agent_cid`.

**Slice-0 (unblocked, lights alpha today at the base).** The **base** — rea-economic `commitment_backed` — already
folds real CI-wired `seed-provide-rows` data and renders `commitmentBackedCollectives=1` on live alpha; it runs in
Wave A, parallel to the lens. The **extension** Slice-0: one deterministic closure → fulfiller emits a
`compute-fulfilled` EconomicEvent (new action) `bounded_by` an existing delegates-compute commitment, `output_cid`
recorded → the lens's `realized_value_flow`/`mutual_compute` fold stops being intent-only. Testable DB-free over
hand-built events; no #2, no quorum, no reward needed.

---

## #4 — Private-replica encryption

**Compose-into.** A new **"Workstream: Private-Replica Encryption"** in
`genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md` (that arc owns the real
`distribute_shards`/`shard_locations`; the resilience-card lighting plan Sprint 4 defers to it). NOT the facings
read-lens (Category-C, no-crypto). Concretely: a `ShardManifest` field-add (`sharding.rs:32`), one new
`KeyEnvelope` DHT entry, and reuse of the dryoc X25519 seal pattern from `services/sealed_against_self.rs`.
**The blob/shard plane is plaintext today** (`BlobStore::store` → `fs::write(&blob_path, data)`) — this is
greenfield crypto, the one net-new dependency in the arc.

**Entities (P2P gate).**
- **ShardManifest encryption marker + plaintext_cid** — Notarized-A, field-add to the **existing** manifest entry:
  `encryption: "none"|"dek-x25519-v1"` + `plaintext_cid` (canonical, for links/requests/dedup) alongside the
  ciphertext `shard_hashes`. Encrypt-then-RS means shard hashes address ciphertext; `plaintext_cid` preserves
  content identity. `GET /blob/{plaintext_cid}` resolves manifest → reconstruct → decrypt (no new route).
- **KeyEnvelope** — Notarized-A, **the one genuinely-new entry type**. A published DHT entry whose contents are
  `crypto_box_seal(dek, reader_x25519_pk)` — safe to publish, useless without the reader's secret. Keyed by
  `(plaintext_cid, reader_agent_cid)`; kept separate from the immutable manifest and updatable so rotating the
  reader set never churns the content address. **Wraps go to authorized READERS, not per-replica/custodian —
  custodians hold opaque bytes and get no key.**

**Approaches (the E2EE-storage fork, threat-narrowed).** (1) convergent (DEK = hash(plaintext)) — CID-stable,
cross-household dedup survives, but **disqualified for private content** (cross-household dedup is a
confirmation/learn-the-file attack — "confirmation is reading"). (2) random per-object DEK — max privacy, zero
dedup. (3) per-household-keyed DEK = `KDF(plaintext_cid, household_secret)` — **recommended**: keeps
within-household dedup idempotent, opaque + leaks nothing cross-household; encrypt-then-RS over ciphertext keeps
`rs-4-7` and `blob_fetch`'s sha256 verify valid (it checks transported bytes).

**The fork (per-plan decision point) — recoverability vs sovereignty.** Does the mishpat recovery quorum get a
`KeyEnvelope` (total household key loss is governance-recoverable, mirroring `sealed_against_self`'s 2-of-2 posture)
or is it household-readers-only (sole sovereignty; lost keys = permanent loss despite redundant custody)?
**Recommendation — defer to build; it is a values call**, leaning toward the quorum wrap. Branches whether
`KeyEnvelope` carries a quorum row.

**Dependencies (live path blocked).** `@requires:` an **X25519 reader-key substrate** — Holochain agent keys are
ed25519; `crypto_box_seal` needs X25519, and nothing sources a per-reader X25519 pubkey from `agent_cid` yet
(`sealed_against_self` takes the key as given but never sources it). `@requires:` the unresolved **conductor-leak
fix** — real `distribute_shards` over a healthy mesh is Sprint-4 work blocked on the OOM site that is **still
OPEN** (jemalloc canary prepped, not landed). `humans.agent_pub_key` population is security-gated. These gate
**live delivery only**.

**Slice-0 (unblocked, single-host).** A local round-trip in a new `private_replica.rs`: plaintext → random DEK
encrypt → RS-encode the ciphertext → drop ≤parity shards → reconstruct → decrypt → assert the result hashes to
`plaintext_cid`; DEK sealed/unsealed via dryoc with generated keypairs (the `sealed_against_self` test pattern).
Proves encrypt-then-RS-over-ciphertext and the two-CID model end-to-end on one host — no mesh, no DHT, no identity
resolution, no leak dependency.

---

## Sequence

- **Wave A (parallel, now — all household-nodes testable):** #0 operational-weave lens (its 4 slices) ∥ #2
  measured tier-capability fold ∥ #3 base (`commitment_backed`, already lighting real alpha data). Each of #4's
  Slice-0 and #1's Slice-0 are pure proofs that can also run any time.
- **Wave B:** #1 recursive rollup (consume `CoverageRollup`, replace the lens's flat `aggregate()`; precede with
  the recursive-architecture Wave 0/2 + the conductor-signal msgpack fix) + #3 extension (compute-fulfillment
  EconomicEvent emitter → lens `mutual_compute`/`realized_value_flow` light).
- **Wave C:** #4 private-replica encryption — Slice-0 lands anytime; the live path waits on the conductor-leak fix
  and the X25519 reader-key substrate.

## Forks summary (each re-confirmed at its piece's `/plan`)

| Fork | Recommendation | Locked? |
|---|---|---|
| #1 capacity vs `CoverageDomain` | both, by metric (byte-coverage descends; capacity is a scalar sum) | per-plan |
| #2 measured-only vs earned-witnessed | measured-only now; stub `storage-capability` | per-plan |
| #3 bilateral vs commons crowdsource | defer; lean bilateral-first (Z.D proving ground) | per-plan |
| #4 quorum-recovery vs household-sovereignty | defer; values call, leans quorum-wrap | per-plan |

## Non-goals

- This index does not implement anything; each piece gets its own `/plan` → sprint cycle, composing into the
  named home (never a parallel spec).
- No new DHT entry type except #4's `KeyEnvelope`. Any proposal to mint `Council`, `CapacityAttestation`, or a
  capacity table on the DHT is an anti-pattern this index explicitly forecloses.
- Live-alpha lighting (real seed paths, healthy mesh) is operator/security-owned and distinct from each Slice-0's
  in-repo green proof.
