---
status: design   # Wave 2 findings addendum — synthesis of application-archetype concerns, planning input
related:
  - 2026-05-24-records-lifecycle-phase2-findings-synthesis.md   # the Phase 2 synthesis this extends
---

# Records Lifecycle — Wave 2 Findings Synthesis Addendum

**Status:** Phase 1 Wave 2 returned 7/7 (Khan-Academy, Google-Drive, Google-Photos, Meta-Facebook, Patreon, Requests-and-Offers, AWS-Compute application archetype full-drafts). This document synthesizes the structured concerns reports into navigable form as an addendum to [`2026-05-24-records-lifecycle-phase2-findings-synthesis.md`](./2026-05-24-records-lifecycle-phase2-findings-synthesis.md).

**Inputs:** Structured concerns reports from 7 parallel rust-architect/general-purpose agents writing application archetype full-drafts. Each agent did deep composition + flagged bottlenecks/chokepoints/anti-patterns/substrate-gaps/cross-archetype-drift/bridge-complexity/unresolved items.

**Output:** 13 extensions to existing D.x subsections + 12 net-new substrate gap candidates + 6 "service X.rs planned but unimplemented" call-outs + 8 meta-patterns spanning multiple archetypes. Operator-decision queue grows by ~15 items.

**Key takeaway:** The substrate's 8 foundational primitives + 20 substrate gaps (after Phase 2) hold up structurally — every archetype composed cleanly without inventing new entry types. But the *services and policy surfaces* that drive the primitives have substantial unfinished work, and the *cross-archetype vocabulary coordination* needs an SDK-boundary canonical home. The architecture's theory is validated; the operational discipline around it needs Wave-2-informed extensions before downstream sprint work can land cleanly.

---

## The 8 cross-archetype meta-patterns

### Meta-Pattern 1 — D.12 needs extensions beyond FeedbackSignal aggregation (5 of 7 returns)

D.12 specified `checkpoint` Commitment for balance snapshots + `aggregate-subordinate` Commitment for FeedbackSignals. Wave 2 found three other places the same shape applies:

- **Content-body aggregation** (Meta + Khan): comment-body Posts (child EPRs with `signal_kind: "comment"`) need their own subordinate pattern. Viral 10k-comment thread = 20k entries with no release valve. *Khan also surfaces this for mastery-session aggregate Posts.*
- **Attestation refresh-on-change** (Patreon): tier-Attestations re-issue monthly even when nothing changed. 10k patrons × 12 cycles = 120k Attestations/year per creator. Need `valid_until` field + refresh-only-on-actual-change discipline. Could collapse 120k → 3k/year.
- **Materialized derived-aggregate primitive for EconomicResource** (Patreon + Drive): D.12 mentions checkpoint/snapshot but isn't wired explicitly to EconomicResource balance derivation. Creator-fund / household-balance / document-state Resources all need this.

### Meta-Pattern 2 — D.20 friction-gradient direction + currency-conversion + non-transfer Event fees (4 of 7 returns)

D.20 specified fee_splits on `transfer` Events with friction-gradient denominated in `currency-USD`. Wave 2 surfaced four under-specified surfaces:

- **Direction ambiguity** (Patreon): does the ratchet apply on **incoming patronage receipt** (receive-side) OR only on creator's **cash-out outflows** (outflow-side)? Two very different economic models.
- **Currency-conversion for non-USD Resources** (AWS): friction-gradient tier table denominated in `currency-USD`; compute earnings are `gpu-hour` Resources. Either extend `attestation:price-feed` OR add distinct `attestation:compute-rate-feed` for the conversion.
- **Custody-blob fees** (Drive): D.20 fee_splits are on `transfer` Events; document-storage is `custody-blob` action, not transfer. Storage fees need separate model — either custody-blob carries fee_splits in payload_json, OR storage modeled as periodic transfer Events from Collective Commons.
- **Per-pillar fee-routing reconciliation** (R&O): each archetype declares its own Bridge Commons + cooperative Commons routing; coordination across pillars needed.

### Meta-Pattern 3 — Cross-peer probes + bilateral coordination (2 of 7 returns, structurally critical)

D.6 first-quorum-wins covers community-scoped Attestations (where multiple peers can race to issue the same Attestation; CRDT dedup wins). But **bilateral coordination** between two specific peers is uncovered:

- **Standing probes** (Meta): when an amplified Post arrives from a stranger, receiver needs the author's standing. No protocol for cross-peer standing probes — needs `standing-probe` request-response codec on libp2p with TTL cache (~1 hour).
- **Job matching** (AWS): bilateral coordination between one consumer Commitment and one provider Commitment. No fallback if either party's matching-elohim is offline.

**New gap candidate**: bilateral coordination protocol layer.

### Meta-Pattern 4 — Per-Attestation-subtype validator framework missing (3 of 7 returns)

Per-Attestation-subtype validators are documentation convention today, not substrate-floor enforced:

- **`ATTESTATION_REACH_CEILING`** (Photos): face-cluster Attestation must not be elevatable above household reach; integrity zome needs a manifest-declared per-subtype max-reach check at `grant-reach` Event validation
- **Confirmation-class confirmer eligibility** (AWS): `attestation:price-feed` has known oracle collectives; `attestation:computation` has no analog; need `compute-verification-collective` EPR type with Membership granting confirmer eligibility
- **Apply-gate authoring authority** (Khan): `attestation:mastery` at apply-gate — autonomous learning-elohim OR explicit learner authorization?

**New gap candidate**: per-subtype validator hook framework declaring `attestation_reach_ceiling`, `confirmer_eligibility_collective`, `authoring_authority_chain` per attestation_kind.

### Meta-Pattern 5 — D.17 stagger discipline needs more patterns (2 of 7 returns)

D.17 covers per-agent stagger for recurring-fulfillment verbs (Patreon-shape monthly billing). Wave 2 surfaced two patterns D.17 doesn't cover:

- **Cancel-storm dampening** (Patreon): cancellation is `immediate`, not recurring. Creator drama → 10k-100k cancellations in minutes → 1000+ Events/sec at creator's projection. Need cancel-shape stagger.
- **Cooperative grouped stagger** (R&O): cooperative-cycle-close needs N member Commitments to **aggregate atomically** THEN supplier-side notification staggers. Structurally different from per-agent stagger.

**D.17.1 follow-up**: cancel-storm dampening + grouped stagger.

### Meta-Pattern 6 — Local-hardware dependency for elohim cognition stratifies access (Photos critical)

The "grandma standard" assumes household-hub-capable hardware. Vision-elohim face recognition needs 100-500MB models; Raspberry Pi class household-hubs lack RAM for concurrent inference. Photos surfaced this; learning-elohim (Khan) and care-stewardship-elohim (value_scanner) likely have similar dependencies.

**New gap candidate**: manifest-declared model-size ceiling + fallback when local compute insufficient. Options: skip Attestation; emit partial Attestation; delegate to a designated collective elohim-node (with explicit consent + reach-scoped access).

**Substrate-policy concern**: if face clustering only works on high-spec hardware, the feature becomes stratified by compute access rather than household need.

### Meta-Pattern 7 — Cross-archetype vocabulary needs SDK-boundary canonical home (7 of 7 returns)

Every archetype surfaced at least one vocabulary collision:
- `signal_kind: "comment"` defined by Drive + Meta + Patreon (different reach-scope semantics, same `signal_class`)
- `signal_kind: "endorse"` defined by Meta (engagement-replacement) + Khan (peer-review trust) — must declare different `signal_class` (Meta=care, Khan=trust)
- `signal_kind: "bid"` / `"dispute"` namespace collision between R&O + Patreon
- `grant-reach` / `revoke-reach` payload_json schemas inconsistent across archetypes
- `aggregation:feedback-signal` classification needs elohim pillar manifest (cross-cutting), not single domain
- `quarantine` (governance) vs `compute-failure-report` (compute) — must not cross-pollute classes
- Action verb namespace: `lamad:path-completion-commit` vs `patreon:tier-fulfillment-commit`

D.10's vocabulary governance gate catches *existence* drift (whitelist vs schema vs manifest) but NOT **payload-schema consistency across archetypes** or **single-canonical-declaration-site discipline**.

**New gap candidate**: SDK-boundary canonical manifest at `elohim/sdk/domains/elohim/manifest.json` declaring cross-pillar shared verbs/signal_kinds/attestation subtypes with payload schemas; per-pillar manifests import rather than re-declare. D.10's gate extended to verify cross-archetype payload consistency.

### Meta-Pattern 8 — Reputation concentration without value concentration (2 of 7 returns)

D.20 friction-gradient prevents accumulation of **value** classes. Wave 2 surfaced that **reputation/standing** accumulation has no analogous friction:

- **R&O**: long-tenured supplier accumulates outsized fulfillment-history Attestations; structurally hard to compete with regardless of fee ratcheting
- **Patreon**: celebrity creators similar pattern — standing concentration without value concentration (value ratchets but reputation accumulates)

**New gap candidate**: reputation-decay or reputation-friction discipline. Options: standing-curve decay rate increases with tenure; reputation rate-ceiling per author per period; reputation-redistribution via Global Commons Allocation Events.

---

## D.x extensions (13 revisions to existing subsections)

These don't add new D.x sections; they EXTEND existing ones with Wave-2-surfaced addenda. Each extension lands as an edit to the existing subsection in `2026-05-24-records-lifecycle-design.md`.

| # | Extension | Surfaced by | Affects subsection |
|---|---|---|---|
| E1 | Content-body aggregation Commitment (extends `aggregate-subordinate` to `content_type: "post"` children of parent Posts) | Meta + Khan | D.12 |
| E2 | Attestation refresh-on-change pattern with `valid_until` field | Patreon | D.12 |
| E3 | Materialized derived-aggregate primitive wired to EconomicResource state | Patreon + Drive | D.12 + D.4 |
| E4 | Cancel-storm dampening stagger (immediate-action verbs) | Patreon | D.17 |
| E5 | Cooperative grouped stagger (aggregate-atomic-then-stagger-out) | R&O | D.17 |
| E6 | Friction-gradient direction clarification (receive vs cash-out) | Patreon | D.20 |
| E7 | Custody-blob fee mechanics + per-pillar fee schedule reconciliation | Drive + R&O | D.20 |
| E8 | Currency-conversion for non-USD Resources (gpu-hour ↔ USD via attestation:price-feed extension) | AWS | D.20 |
| E9 | `subject_cid` generalization to Commitments (cross-jurisdictional compute job dispatch) | AWS | D.9 |
| E10 | `subordinate_reach_policy` field on D.1 subordination links (does child inherit parent's reach mutations?) | Photos | D.1 |
| E11 | `signal_class` single-tag-vs-multi-tag clarification (peer-review = trust+care?) | Khan | D.18 |
| E12 | `cohort-teacher-elohim` as new mentor-facing specialization category (distinct from care/inventory/vehicle) | Khan | D.6 |
| E13 | Community-authored vocabulary delegation governance (childcare-hours, garden-share) | R&O | D.10 |

---

## New substrate gap candidates (12 net-new D.x proposals)

These are new substrate concerns Wave 2 surfaced. Numbered D.21+ following the existing 20 from Phase 2 synthesis. Each warrants its own subsection in a future Part D revision OR a sibling-sprint spec.

### D.21 — Standing-probe federated request-response codec (Meta)

When an amplified Post arrives from a stranger, the receiver needs the author's standing for feed ranking and reach gating. Cross-peer standing probes have no protocol today.

**Design proposal**: a `standing-probe` request-response codec on libp2p plane. Receiver queries a peer in the author's reach cluster; per-receiver standing-cache (TTL ~1 hour) bounds cross-peer query rate. Caches indexed by `(author_cid, signal_class)`.

### D.22 — Bilateral coordination protocol (AWS + Meta)

D.6 first-quorum-wins covers community-scoped Attestations but bilateral coordination (consumer↔provider matching, send-to-specific-peer signal delivery) has no fallback when either party's elohim is offline.

**Design proposal**: bilateral-coordinator-fallback declared in manifest per action verb (`matched-compute`, `accept-subscription`, etc.). Options: marketplace-collective hub-node takes over with explicit delegation; consumer can match directly with degraded UX; matching pauses gracefully with operator-visible signal.

### D.23 — Per-Attestation-subtype validator framework (Photos + AWS + Khan)

Per-subtype validators are documentation convention. Make them substrate-floor.

**Design proposal**: extend manifest `attestation_kinds` declarations with `attestation_reach_ceiling`, `confirmer_eligibility_collective_cid`, `authoring_authority_chain`. Integrity zome enforces at write time.

### D.24 — Local-hardware fallback for elohim cognition (Photos)

Vision-elohim / learning-elohim model-size requirements stratify household-hub access.

**Design proposal**: manifest-declared `model_size_ceiling` per elohim specialization + fallback policy (defer to designated collective elohim-node with explicit consent + reach-scoped delegation).

### D.25 — Reputation-decay / reputation-friction discipline (R&O + Patreon)

Friction-gradient (D.20) covers value concentration; reputation concentration is structurally unbounded.

**Design proposal**: per-signal_class decay-rate options (already partially in D.14 standing-curve policy) plus reputation rate-ceiling per author per period plus optional reputation-redistribution via Global Commons Allocation Events.

### D.26 — Sealed-against-self provenance chain primitive (Meta)

Per `social_medium/epic.md` Part IV: amplification chain (who-passed-it-to-whom) sealed against the next-passer but recoverable through governance handshake. Substrate has signer_pubkey chains on amplify Events but encryption-against-single-peer + multi-party handshake decryption protocol undesigned.

**Design proposal**: Shamir-split of relayChain field with mishpat-governance authorizing the unseal (analogous to D.19 Shamir add-on but applied to provenance chains).

### D.27 — Profile-Collective auto-creation invariant (Meta)

Per D.15 D-2 resolution, asymmetric follow = Membership in followee's profile-Collective. Every Human needs auto-created profile-Collective at signup.

**Design proposal**: imagodei coordinator zome auto-creates `content_type: "profile-collective"` EPR on Human creation; contributor never sees it as a separate concept; followers join as Memberships. Coordinator-zome change only; no new entry type.

### D.28 — Apex-elohim council throughput + failover semantics (Meta + Patreon)

Every commons-tier reach mutation + Global Commons Allocation Event needs apex-council attestation. Council becomes substrate-wide rate-limiter at scale.

**Design proposal**: explicit allocation cadence parameter (batch council attestations on schedule, not per-Event); failover semantics when council member offline; council load-budget model.

### D.29 — Bandwidth / network constraints as Commitment fields (AWS)

`provide-compute` Commitments lack network capacity declaration. Large-job consumers can saturate provider uplink with no substrate mechanism to prevent.

**Design proposal**: `bandwidth_mbps_up` + `max_job_payload_bytes` fields in `resource_classified_as_json` for compute Commitments; validate at match time.

### D.30 — SDK-boundary canonical manifest for cross-pillar vocabulary (7 of 7 returns)

Cross-archetype vocabulary drift surfaced from every Wave 2 return.

**Design proposal**: `elohim/sdk/domains/elohim/manifest.json` becomes the canonical declaration site for cross-pillar shared signal_kinds, action verbs, attestation subtypes, with payload schemas. Per-pillar manifests **import** rather than **re-declare**. D.10's vocabulary governance gate extended to verify cross-archetype payload consistency.

### D.31 — Webhook idempotency for bridge-stewardship-elohim (AWS + Patreon)

At-least-once webhook delivery (Stripe, AWS, Plaid) requires idempotent Event creation. Current Observation tier doesn't specify dedup-key strategy.

**Design proposal**: content-addressed dedup key (BLAKE3 of vendor's event payload) checked at coordinator before creating Event. Bridge spec extends to require dedup-key declaration per bridge type.

### D.32 — Reach taxonomy refinement (R&O + Meta)

R&O "community-scope" (browsable inventory) and Meta "community-scope" (friends-only social) mean structurally different things despite sharing the Reach enum value.

**Design proposal**: refine Reach enum with finer-grained variants (`community-marketplace`, `community-social`, `community-cohort`) OR add a `reach_semantic` discriminator that contextualizes the same enum value per pillar. Operator decision on which approach.

---

## "Service X.rs planned but unimplemented" inventory

Every archetype flagged at least one service that the spec defined but hasn't been built. This is the **operational gap** Wave 2 surfaced — substrate primitives are declared; the services that drive them are next-sprint work.

| Service | Surfaced by | Spec dependency |
|---|---|---|
| `checkpoint_service.rs` | Drive | D.12 |
| Graduation evaluator backpressure path | Photos | D.6 + 2026-05-11 observation spec Stage 5 |
| Creator-fund derived view (`resource_state_service.rs` extension) | Patreon | D.4 |
| Bilateral matching fallback service | AWS | D.6 (new gap D.22) |
| Standing-probe federated codec | Meta | D.14 (new gap D.21) |
| Cohort-graduation evaluator throughput | Khan | D.6 |

**Recommendation**: a dedicated sprint plan for "Records Lifecycle Services Wave" that lands the 6 services these archetypes name. Sequencing depends on which archetypes are prioritized for delivery.

---

## Operator-decision queue additions

15 new operator-decision items from Wave 2 (atop the 21 from Phase 2 synthesis):

### Architecture-shaping (must resolve before Part D' revision)

1. **D.12 extension scope** — land E1+E2+E3 as in-place edits to D.12, OR split into D.12.1/D.12.2/D.12.3 subsections for clarity?
2. **D.20 friction-gradient direction** (E6) — receive-side ratchet OR outflow-only ratchet? Or both, manifest-declared per resource_class?
3. **D.20 custody-blob fee model** (E7) — fees in custody-blob Commitment payload, OR separate periodic transfer Events?
4. **D.20 currency-conversion** (E8) — extend `attestation:price-feed` for compute-rate OR new `attestation:compute-rate-feed`?
5. **D.18 signal_class multi-tag** (E11) — single-signal-multi-class OR N parallel signals per Recognition?
6. **D.32 reach taxonomy refinement** — refine enum OR add per-pillar `reach_semantic` discriminator?
7. **D.26 sealed-provenance chain** — adopt the Shamir-split + mishpat-unseal pattern from D.19, OR design a distinct mechanism?
8. **D.24 elohim-cognition fallback** — defer to collective elohim-node OR skip Attestation OR partial-emit?

### Implementation-detail (resolve subsection-by-subsection)

9. Patreon tier-Attestation refresh cadence (monthly vs only-on-change)
10. Apply-gate Attestation authoring authority (autonomous learning-elohim vs explicit learner) — Khan
11. Mute semantics (DHT-notarized squelch-shape vs purely-operational SQL filter) — Meta
12. Compute-verification-collective bootstrap (how does the first such collective form?) — AWS
13. GPU-hour ↔ USD pricing oracle authority — AWS
14. Short comments inline in FeedbackSignal metadata_json vs child Post EPR — Meta
15. Khan apply-gate dual-mode (autonomous default + explicit override per household preference?) — Khan

---

## Recommendations

### A. Part D' revision pass after operator review of this addendum

The records-lifecycle spec's Part D was content-complete after Phase 3. Wave 2 surfaced 13 extensions + 12 new gap candidates that warrant a Part D' revision. This is operator-led work (same shape as the original Part D drafting); estimated similar scope to the original Wave A-E execution. Possible to dispatch parts of it (the 12 new gaps could be dispatched to rust-architect agents one-per-gap if operator chooses).

### B. Services Wave is a sibling sprint

The 6 unimplemented services (`checkpoint_service.rs`, etc.) are a substrate-implementation sprint, not a spec revision. Sequence after Part D' lands. Each service can be its own implementation plan; some have prerequisite dependencies (standing-probe depends on D.21 spec landing).

### C. Application Wave 2 is content-complete; ready for next layer

All 7 application archetypes have full drafts. Future work on these archetypes is:
- Implementation sprints (build the Angular surfaces + bridge crates + services)
- Story-coverage sprints (each archetype's `genesis/a2o/features/` scenarios)

The architecture spec doesn't need further archetype work; downstream sprint planning is the next move.

### D. Horizon archetypes (YouTube, WordPress, Factory, Bank) inherit Wave 2 findings

The 4 horizon archetypes (`architecture/horizons/`) were already deferred per operator direction. Wave 2's findings make them easier-to-graduate when their time comes — most of the substrate work they'd need is now identified.

### E. The substrate's theory holds

**The most important Wave 2 finding is the absence of one**: no archetype tried to invent a new DHT entry type. All 7 composed cleanly from the 8 foundational primitives. The substrate's architectural commitments (no new entry types; manifest-declared extensibility; reach-gated visibility; floor-permissive authorship; mass-conservation on Events) all held up across 7 distinct stress profiles.

The work remaining is **operational discipline around the primitives**, not **the primitives themselves**. That's a very good place to be after Phase 1 + Phase 2 + Phase 3.

---

## Next steps

1. **Operator reviews this addendum** — especially the 8 architecture-shaping operator decisions
2. **Decide on Part D' revision scope** — land all 13 extensions + 12 new gaps, OR triage into "must-have-pre-launch" vs "post-launch sibling sprint"
3. **Optionally dispatch Part D' as parallel sub-agent work** (one agent per new gap subsection) if scope is large enough to warrant
4. **Sequence the Services Wave** sibling sprint
5. **Schedule story-coverage sprints** for the 7 active archetypes — `a2o/features/` scenarios per archetype
