# Track A — Survival Core Execution Plan (Roadmap)

> **For agentic workers:** This is a ROADMAP that decomposes Track A into 5 sequenced per-gap plans. Each Phase below gets its own detailed bite-sized plan authored **at build-time** (read the live stub files, write complete-code TDD steps) — see §5. REQUIRED SUB-SKILL for each Phase: `superpowers:subagent-driven-development`. Steps use `- [ ]` for tracking.

**Goal:** Make `elohim.host` / `alpha.elohim.host` serve the landing page + core LMS path from a surviving anchor when half the cluster (Shem) is down — by *finishing the already-designed* reciprocal-replication + commons substrate, wiring the genuine gaps. **No new design; no new DHT entry types.**

**Architecture:** Compose on the **shipped Sprint-3 dwelling-hub substrate** (`replicates-dwelling`, constitutional donut, `replication_prioritizer`, `mutuality_audit_service`) + the **Self-Healing P2P Dataplane campaign** (Plan 1 shipped for in-app ingest; Plans 2–3 designed, archived). Three replication classes, separated by *who holds the agency* (see §0).

**Tech Stack:** Rust (Mishpat zome HDK; `elohim-storage` diesel/services; `hc_client` conductor bridge), TypeScript (genesis seeder), Gherkin (a2o).

---

## 0. Framing & vocabulary (lock this)

Three replication classes, by agency. **Track A survival core builds the involuntary classes; the voluntary patronage class is deferred.**

| Class | Agency | Chosen by | Mechanism | Status |
|-------|--------|-----------|-----------|--------|
| **Dwelling** (resilience) | individual / household | the human (kin/family partner) | `replicates-dwelling` reciprocal pact | substrate SHIPPED (Sprint 3); not yet *invoked* |
| **True commons** (collective sensemaking) | **beyond individual/collective** | the **elohim** (commons-grading), *not humans* | `replicates-commons`: involuntary capacity pledge (donut floor) → elohim-graded placement | reserved, zero logic |
| **Social commons** (patronage) | individual, deliberate | the human ("back this creator") | `project-epr` sponsorship (discoverability) + patronage-replication (durability) | `project-epr` mostly built; **DEFERRED** |

**Key principle for true commons:** humans pledge *capacity* (involuntary, the DNA-locked `COMMONS_MIN_FLOOR_PCT = 10%` donut floor — already enforced); **the elohim decide what fills it** via commons-grading (`reach: commons`). Humans never choose the contents. This closes the gap the dwelling-hub spec (§5.4) explicitly deferred ("what fills that reserved capacity is not yet decided"). Landing + core + manifesto are *already* graded `reach: commons` — they are the first commons payload, placed by the substrate, not chosen by any holder.

## 0.1 Open decisions (confirm before Phase 1 / Phase 4)

- **D1 — Reciprocal pair for the first dwelling pact (Phase 1).** The existing resilience canon (`genesis/docs/content/elohim-protocol/resilience/README.md`) names **Gertrude (shem) ↔ Dowell (on-prem)** as the *minimum-viable backup relationship* — exactly the two-failure-domain pact we need. Your original framing was **Adam (shem) ↔ Matthew (on-prem)** (the genesis content pair). **Recommendation: wire the canonical Gertrude↔Dowell pact first** (it's already the documented recovery relationship and spans the two failure domains), and add Adam↔Matthew as a second pact later. *Confirm: Gertrude↔Dowell, Adam↔Matthew, or both?*
- **D2 — Commons scope for Phase 4.** Recommendation: implement the *minimal* true-commons slice — commons-graded content is placed into the involuntarily-pledged commons capacity (no human choice), floor-via-pledge closes the declaration gap. Defer dynamic "elohim councils deciding the commons need" governance. *Confirm minimal slice vs richer governance.*

## 0.2 Resiliency & projection axes (the HA target — backs the progressive icon story)

Two **orthogonal** axes give the effort a target and back the existing progressive resiliency→projection icon story (`2026-05-29-durability-topology-felt-resilience.md` progressive icon / ambient status / free-used triptych; the `resilience-snapshot` component; reach/delivery/persistence in `2026-05-29-epr-reachability-economics.md`):

- **Resiliency (durability) — this IS the graduation gradient:** `local-personal (1 replica)` → `dwelling-backed (2, a reciprocal pact)` → `collective-backed` → **High-Available = 5 hub-quality replicas** across distinct failure domains. A *hub-quality replica* = a full copy on a stewarded, always-on, WAN-reachable hub (not a transient peer cache). **Grounding (2026-05-31): HA=5 is NEW — reconcile with existing machinery, don't bypass it:** (a) `distribution_view.rs::replica_target_for(reach)` already scales replica *targets* by reach (`Private 2 → Public 16`; this enum lacks `commons` — a reach-vocabulary **drift to fix**); (b) `household_resilience.rs` classifies `protected = ≥3 households stewarding AND ≥2 online peers` (household-count, not hub-quality-replica-count). **OPEN (Phase 4, not Phase 1):** is HA a flat `≥5 hub-quality` floor, the reach-scaled `replica_target_for`, or HA=5-as-floor that reach can raise? → story-harvest decision.
- **Projection/delivery (orthogonal):** `HA → globally-projected (global CDN)` via `project-epr`. **Already specced — reference, don't reinvent:** `2026-05-29-epr-reachability-economics.md §7 "anycast-CDN endgame"` (elohim.host → nearest doorway edge via anycast/GeoDNS; P2P substrate as origin; immutable CIDs cached eternally, mutable heads kept fresh via `project-epr`). An EPR can be HA without being globally projected.
- **Icon today (one axis, not two):** the `resilience-snapshot` component renders only the *durability* axis as 3 states (`protected`/`partial`/`at-risk`). The projection axis data exists (`regionalDistribution` local/regional/global) but isn't yet rendered as a second progressive scale — **that second axis is the future UI evolution these two scales unlock.**

**Consequence:** commons content (landing+core, `reach: commons`) reaches **HA by construction** once the commons floor holds (Phase 4) — every hub holds it → ≥5 hub-quality replicas. Dwelling pacts (Phase 1) give the 2-replica floor for *non-commons* household content. The three Phase-1 hubs (Dowell on-prem + Gertrude + Adam on shem) are the first replicas on the path to 5. The outage-drill success criterion (§2) is really "≥1 of the ≥5 HA replicas is reachable."

## 1. Existing-design index (build on these — do not re-derive)

| Phase | Builds on (existing design/impl) |
|-------|-----------------------------------|
| 1. Writer-caller handshake | `2026-05-29-close-the-gaps-HANDOFF.md` **Gap 0**; `elohim/elohim-storage/src/services/replicates_dwelling_service.rs` (writer exists, no caller) |
| 2. Mutuality wiring | `mutuality_audit_service.rs` (`find_counter`→None, `emit_reciprocity_imbalance`→log-only); `commitment_fetcher.rs`; `2026-05-28-mutual-storage-replication-dwelling-hub-design.md` §6 |
| 3. Seeder fan-out | Self-Healing **Plan 1** (`.claude/archive/2026-05-15/.../2026-04-19-self-healing-plan-1-observable-auto-distribute.md`; `peer_selection.rs`, `placement_gaps`, `household_backfill.rs`); `genesis/seeder/src/seed.ts:459` |
| 4. `replicates-commons` | `2026-05-28-mutual-storage-replication-dwelling-hub-design.md` §5.4; `constitutional_ratio_registry.rs`; `replication_prioritizer.rs` (commons→Skip); commons-grading in `genesis/seeder/src/seed-sqlite.ts` |
| 5. Verifier + reconstruction | Self-Healing **Plan 2 + Plan 3** (`.claude/archive/2026-05-15/.../2026-04-19-self-healing-p2p-dataplane-design.md` §6–§7) |

## 2. Definition of done (the outage drill — end-state)

With Shem comms severed (or the on-prem hub down), an anonymous client `GET`s the landing page + core path (`elohim-protocol`) from a surviving anchor and receives HTTP 200. Current-scope milestones (Phases 1–4) are the steps that make this reachable; Phase 5 (verifier/reconstruction) hardens it; the *routing* half (Track C) is separate.

---

## 3. Sequenced gaps (leverage order)

### Phase 1 — Gap 0: writer-caller handshake *(highest leverage, smallest)*

**Why first:** Sprint 3 shipped the entire commitment mechanism, but `replicates_dwelling_service::create_replicates_dwelling_commitment` **has no caller** — so no pledges exist in alpha and the resilience bar stays dark despite a live mechanism (per the HANDOFF). Authoring the first reciprocal pact makes everything downstream observable.

**Files:**
- `elohim/elohim-storage/src/services/replicates_dwelling_service.rs` (writer — confirm signature)
- Caller site: the bilateral counter-commitment flow (route + seeder/config). Likely a new diagnostics/admin route + a genesis seeding step that authors the pair from config.
- Pair config: the Gertrude↔Dowell (or Adam↔Matthew) hub IDs + scope_filter + ratio_attestation.

**Scope note (transitive vs broad):** set the pact's `scope_filter` **broad** so each hub backs the other's dwelling set (the "family back each other up" intent). Literal *transitive-closure-of-commitments* meta-replication (Adam mirrors Matthew's *entire* committed set, recursively) is **deferred** — broad-scope dwelling coverage + the commons floor (Phase 4) already deliver the survival goal without the cascade/cycle complexity.

**Task breakdown:**
- [ ] Confirm D1 (pair) and read the live `replicates_dwelling_service` writer signature + `ReplicatesDwellingPayload` fields.
- [ ] Author the **provider→recipient** commitment (on-prem hub names shem hub) via the writer; verify it projects to `rea_commitments`.
- [ ] Author the **counter** commitment (shem hub names on-prem hub).
- [ ] Verify both appear in `replication_prioritizer::active_commitments_for_provider` and the resilience/capacity views light up.

**a2o target:** a scenario proving a bilateral dwelling pact is authored and the per-hub capacity/resilience view reflects ≥1 active pledge each direction. (Extends the Sprint-3 dwelling-hub a2o features.)

### Phase 2 — Mutuality wiring *(makes reciprocity real)*

**Why:** `mutuality_audit_service` classifies Matched/Pending/Breached and logs, but `find_counter` returns `None` (can't find the counter), and `emit_reciprocity_imbalance` only logs. So mutuality never actually audits.

**Files:**
- `elohim/elohim-storage/src/services/mutuality_audit_service.rs` (`find_counter`, `emit_reciprocity_imbalance`)
- `elohim/elohim-storage/src/services/commitment_fetcher.rs` (`CommitmentFetcher` trait + `ConductorCommitmentFetcher`)
- Mishpat coordinator: a by-pair query (`get_commitment` by `(action, provider, recipient)`) if not present.

**Task breakdown:**
- [ ] Implement `find_counter(provider, recipient)` — either extend `CommitmentFetcher` with a by-pair method or `hc_client::call_zome` on a Mishpat by-pair coordinator fn.
- [ ] Wire `emit_reciprocity_imbalance` to emit a real `FeedbackSignal` (`reciprocity-imbalance`, debit_weight 8, decay_days 60) via `hc_client`.
- [ ] Drive a sweep with both commitments present → `Matched` (no signal); withdraw one → after `grace_period_days`, `Breached` → signal emitted.

**a2o target:** withdrawn counter past grace → `reciprocity-imbalance` FeedbackSignal naming the breaching hub; both present → `Matched`, no signal.

### Phase 3 — Seeder fan-out (Plan 1 for the seeder) *(immediate 2-replica survival)*

**Why:** Plan 1 (distribute-at-ingest) shipped for *in-app* ingest, but the genesis **seeder** still `PUT`s to a single `DOORWAY_URL` (`seed.ts:459`). So seeded commons (landing+core) lands on one anchor only.

**Files:**
- `genesis/seeder/src/seed.ts` (the single-`DOORWAY_URL` upload path, ~:459)
- `genesis/seeder/src/doorway-client.ts` (`pushBlob`)
- Reuse the shipped in-app placement intelligence (`peer_selection.rs` / `household_backfill.rs`) conceptually; the seeder analog is multi-anchor PUT.

**Task breakdown:**
- [ ] Make the seeder accept ≥2 anchor URLs (on-prem + shem doorways) for commons-graded content.
- [ ] Fan out the commons blob PUTs to both anchors atomically (all-or-nothing per blob; surface partial-failure).
- [ ] Verify post-seed inventory shows landing+core on both anchors.

**a2o target:** seeding commons content lands it on ≥2 anchors atomically (not single `DOORWAY_URL`); kill either anchor post-seed, landing+core still served.

### Phase 4 — `replicates-commons` (true commons, elohim-graded placement)

**Why:** the principled universal guarantee — every hub holds commons-graded content via the donut floor, *placed by the substrate, chosen by no human*. Closes the floor-via-declaration → floor-via-pledge gap (dwelling-hub §5.4).

**Files:**
- Mishpat zome `commitments.rs` (add `replicates-commons` validator alongside `replicates-dwelling`)
- `elohim/elohim-storage/src/services/replication_prioritizer.rs` (commons-tier scoring — today returns `Skip`)
- `elohim/elohim-storage/src/services/constitutional_ratio_registry.rs` (floor-via-pledge: un-backed commons declarations fail bounds_validator)
- Commons-grading is the *elohim decision* input — already real via `reach: commons` (manifesto/landing/core).

**Task breakdown:**
- [ ] Implement the `replicates-commons` action validator (no recipient hub; commons-class; donut-floor bound).
- [ ] Add commons-tier scoring to `replication_prioritizer` — commons-graded advertised blobs → fetch into pledged commons capacity (no recipient match required; gated by `reach: commons`).
- [ ] Upgrade the floor check from declaration to backing pledge (un-backed commons_pct fails bounds_validator).
- [ ] Verify every participating hub fetches+holds commons-graded landing+core without any per-hub content choice.
- [ ] **A-core sliver (unified justification):** once the prioritizer scores across *both* `replicates-dwelling` and `replicates-commons`, add a `blob_held_because` projection (Category C, reconstructable: blob_hash → {justifying commitment refs}) and gate eviction to "no commitment justifies." Note: content-addressed dedup (`BlobStore`) and the three-surface reconciliation (`reconcile/custody.rs`) already exist — this only adds the cross-class justification view + eviction gate.

**a2o target:** every participating hub provably holds commons-graded content per the donut floor; kill any single anchor, landing+core is never lost. A blob justified by *both* a dwelling pact and the commons floor is held once and only evicted when neither justifies.

### Phase 5 — Verifier + reconstruction (Self-Healing Plan 2 + Plan 3) *(hardening)*

**Why:** survive *more* than the per-blob RS-4+3 tolerance and auto-repair on custodian loss. Fully designed in the archived self-healing campaign (§6 verifier, §7 reconstruction); not built (`shard_verifier.rs` / `reconstruction.rs` absent).

**Files:** new `elohim/elohim-storage/src/services/shard_verifier.rs`, `.../reconstruction.rs`; integrate with `reconcile/custody.rs` (built) + `drain_gap_queue`.

**Task breakdown:**
- [ ] Build the verifier sweep (bounded-sample cross-peer Have-check; `shard_verifications` / `verification_breaches`).
- [ ] Build reconstruction orchestration (pull-mode; stampede defense via deterministic arbitration) triggered on shard-diversity insufficiency.
- [ ] Cross-blob shard-diversity placement so the survivors hold a recoverable set.

**a2o target:** the chaos drill — sever Shem (or kill ≥ RS-tolerance custodians), reconstruction restores availability; alpha still serves landing+core.

---

## 4. Out of scope (deferred — designed-for, not this plan)

- **Social-commons / patronage** (`project-epr` durability facet; `replicates-collective`) — voluntary human-chosen class.
- **Dynamic elohim-council commons governance** — beyond commons-grading-as-placement.
- **Track B** (DDNS+ACME crate — in progress on a separate branch) and **Track C** (discovery de-hardcode).
- Encryption envelope / per-recipient key wrapping; proof-of-storage.

## 5. Per-gap detailed plans (authored at build-time)

Per the writing-plans scope-check (multiple independent subsystems → separate plans), each Phase's **complete-code, bite-sized TDD plan** is written when that Phase is picked up for build — by reading the *live* stub files (signatures drift) and producing `genesis/docs/superpowers/plans/2026-MM-DD-track-a-phaseN-<name>.md`. This roadmap fixes the sequence, scope, references, files, and a2o targets; it deliberately does **not** inline code that would be stale by build-time. Phase 1 is the next artifact.
