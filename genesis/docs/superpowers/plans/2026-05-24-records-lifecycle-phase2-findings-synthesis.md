---
id: records-lifecycle-phase2-findings-synthesis
status: design   # findings synthesis — meta-patterns + revised gap list, planning input for Phase 3 Part D
---

# Records Lifecycle — Phase 2 Findings Synthesis

**Status:** Phase 1 Wave 1 returned 8/8 (7 primitive walkthroughs + 1 frontmatter normalization). This document synthesizes the structured concerns reports into navigable form, identifies meta-patterns, proposes a revised gap list, and lists operator decisions for Phase 3 Part D plan finalization.

**Inputs:** Concerns reports from agents writing Parts A.2 (Event), A.3 (Resource), A.4 (Observation), A.5 (Commitment), A.6 (Attestation), A.7 (FeedbackSignal), A.8 (Links). Each agent did deep architectural composition + flagged bottlenecks, chokepoints, anti-patterns, substrate gaps, cross-spec drift, and unresolved questions.

**Output:** Six meta-patterns. A revised gap list (was 10, now ~20). An operator-decision queue (~20 items) that the Part D plan rewrite must resolve.

---

## The six meta-patterns

The strongest signal from Phase 1 isn't the individual concerns — it's the **shared shapes** across multiple primitives. Six meta-patterns crystallized:

### Meta-Pattern 1 — Substrate-vocabulary drift across surfaces (HIGH IMPACT)

**5 of 7 primitive returns hit this.** Application archetypes reference vocabulary that the substrate hasn't declared. Three (sometimes four) authoritative surfaces drift independently: Rust whitelists/structs, JSON view schemas, pillar manifests, and codegen artifacts.

| Surface | A.2 Event | A.3 Resource | A.5 Commitment | A.6 Attestation | A.7 FeedbackSignal |
|---|---|---|---|---|---|
| Rust whitelist/struct | `stake_class` + `observation_refs` fields missing from struct | `RESOURCE_CLASSIFICATIONS` missing `backup-state` | `REA_ACTIONS` missing `subscribe` | `ATTESTATION_KINDS` missing 6 subtypes + `$ref` sentinel bug | `SIGNAL_KINDS` missing 4 Meta signals (`comment`, `endorse`, `react`, `report`) |
| JSON view schema | n/a | **`economic-resource-view.schema.json` does not exist** | n/a | `proofClass` enum drift (view vs validator) | `forget-request` missing from `p2p/feedback-signal.schema.json` enum |
| Pillar manifest | `event_classified_as` undeclared in any pillar | `governed_by` has no analog after Gap 5 consolidation | `subscribe` referenced in shefa manifest but not in `REA_ACTIONS` | App-layer subtypes (auto-tag, face-cluster, computation) need pillar declaration | `signal_kinds` declarations not cross-checked against whitelist |
| Codegen output | n/a | n/a | n/a | `ATTESTATION_KINDS` const missing entries + `$ref` literal | No CI gate compares whitelist vs schema vs manifest |

**Bottom line:** 4 of the 8 application archetypes reference vocabulary that fails substrate-level validation TODAY. The schema-first IoC discipline (`feedback_schema_first_ioc` memory) has not been applied uniformly. **This is the single highest-leverage substrate move surfaced by Phase 1.**

**Proposed gap (new):** "Schema-first IoC governance across all extensibility surfaces" — a single CI/codegen gate that fails when Rust whitelist, JSON schema, pillar manifest, and codegen output don't agree on the vocabulary. Replaces the per-vocabulary findings above with one structural fix.

### Meta-Pattern 2 — Substrate-floor enforcement is aspirational, not actual

**3 of 7 returns hit this.** The substrate documents constitutional/governance commitments that the validator layer doesn't actually enforce. Operators rely on human discipline to maintain invariants.

| Source | Where the gap is | What's at risk |
|---|---|---|
| A.6 Attestation | Floor F2/F4/F6 marked TODO(C.3); ACCEPT-all stubs | Any agent can issue `attestation:mastery` for any subject in any concept domain without holding `attestation:steward` |
| A.4 Observation | Schema validation doesn't reject `retention_class: wisdom` on `subject_kind=sensor` observation_kinds | Manifest author could declare `lamad:sensor-biometric` with `retention_class: wisdom` and the protocol would accept it — directly violates `observer-protocol.md` Part VIII "store video beyond 3-second processing window: forbidden" |
| A.8 Links | LINK_ARCHITECTURE.md deprecation checklist incomplete; ~50 `*By*` query-index links unretired | Every sprint that adds a structural link burns the 256-cap further with no relief; `*By*` links violate DHT-as-notary principle |

**Proposed gap (new):** "Substrate-floor validator backfill" — close the gap between aspirational documentation and validator code. Affects integrity zomes, manifest schema validator, and link-type triage.

### Meta-Pattern 3 — Event-sourcing at scale needs snapshot/aggregation primitives

**4 of 7 returns hit this.** Read-side cost growth on long-lived high-volume event streams is unrelieved.

| Source | Where the cost grows | Currently mitigated by |
|---|---|---|
| A.2 Event | Graduation evaluator throughput at hub scale (single tokio task per pillar) | Spec describes future split to separate elohim-graduator service (open question) |
| A.3 Resource | Balance materialization: 10yr × 50tx/day = 182k events per resource; `SUM(quantity_delta)` is non-trivial | Nothing — no snapshot/checkpoint primitive exists |
| A.6 Attestation | Graduation evaluator rate-ceiling missing — policy bug could orders-of-magnitude over-issue Attestations | Manifest validation doesn't include `max_attestations_per_subject_per_day` |
| A.7 FeedbackSignal | Standing-curve re-derivation O(signals-per-author) per signal arrival → hub serialization | Signal-Aggregate Commitment is named but NOT wired; depends on Gap 4 + Gap 10 |

**Proposed gap (new):** "Checkpoint / snapshot / aggregate-subordination as substrate primitives" — formalize the release-valve pattern (Commitment with `action="checkpoint"` or `"aggregate-subordinate"`) that lets high-frequency primitives shed accumulated cost without losing provenance.

### Meta-Pattern 4 — Structural constraint debt accumulating

**A.8 Links sharpest, but A.3 also flags it.**

- **Link type budget:** 225/256. Was 255, brought down to 225 via DNA split (incomplete). ~50 `*By{Attribute}` deprecation candidates not retired. This spec adds 2 more (Gap 1) → 227/256. **No runtime mechanism reclaims slots; only DNA migration.**
- **Entry type budget:** ~78/100 in elohim DNA (from earlier inventory). Gap 5 retirement frees 1 slot.
- **Validator type budget:** Floor enforcement code is the bottleneck; ACCEPT-all stubs are easier than wiring real floors.

**Proposed gap (new):** "Structural constraint reclamation prerequisite" — before any new structural type additions land, the existing deprecation checklists must be closed.

### Meta-Pattern 5 — Hard-cutover retirement gaps are migration landmines

**A.3 explicit, applies broadly.**

- Gap 5 (StewardedResource consolidation): retiring the entry type before derived views land = capacity-planning + household-resilience + node-stewardship dashboards all go dark simultaneously
- Gap 6 (DoorwayHeartbeat retirement per observation spec Stage 6): same pattern — graduation evaluator must work before heartbeats stop being DHT entries

**Proposed sequencing discipline:** Every "retire X" gap must have a "wire derived views / replacement infrastructure" sub-task that lands FIRST, with green tests, before the retirement commit.

### Meta-Pattern 6 — Cross-DNA / cross-pillar coordination ambiguity

**A.4 + A.8 surfaced this.**

- A.8: `GovernanceActionChild` link from elohim → mishpat (governance actions in mishpat DNA, link in elohim) requires `call(CallTargetCell::OtherRole("mishpat"), ...)` bridge — not documented
- A.4: Hub-graduation failover when issuing-hub offline — no spec for secondary takeover
- A.8: Meta archetype's "Membership / Relationship link" — which zome owns this? Imagodei's `AgentToRelationship`, lamad's `ContentToRelated`, or new social-graph link types?

**Proposed gap (new):** "Cross-DNA coordination patterns" — document the `call(CallTargetCell::OtherRole(...))` bridge pattern for link creation across DNA boundaries; specify hub-graduation failover for community-scoped Attestations.

---

## Revised gap list

The original 10 gaps from the brainstorming inventory:

```
 1.  Link types: EprToEvent, EprToResource                    (substrate)
 2.  parent_epr_cid: Option<Cid> on Event and Resource        (substrate)
 3.  Surface (re-elevation) operation                         (lifecycle op)
 4.  Submerge canonical signal reconciliation                 (lifecycle op)
 5.  EconomicResource ← StewardedResource consolidation       (cleanup)
 6.  Observation spec implementation + Stage 6 cleanup        (prerequisite)
 7.  Elohim-authoring pattern (domain-specialized agents)     (pattern)
 8.  Dissolution semantics (close/revive lifecycle)           (lifecycle op)
 9.  Bridge pattern for legacy systems                        (interop)
10.  Reach-mutation Events                                    (lifecycle op)
```

After Phase 1 findings, the revised gap list (changes marked):

```
 1.  Link types: EprToEvent, EprToResource                    UNCHANGED, with addenda:
       — ADDENDUM: explicit relationship to existing `ContentToResource` (replace or coexist?)
       — ADDENDUM: SQL adjacency tables `epr_event_edges` + `epr_resource_edges` must
                    ship as part of this gap (not separate); designate canonical projection
                    target (CozoDB graph_views vs Diesel)
 2.  parent_epr_cid: Option<Cid> on Event and Resource        UNCHANGED, with addendum:
       — ADDENDUM: field + link must ship as a pair (field without link = incomplete graph
                    binding; projection-lag silently misses subordinates)
 3.  Surface (re-elevation) operation                         REFRAMED:
       — RESOLVE: update_entry (action hash, same CID) vs event-sourced state machine
                  (never mutate entry) — A.3 raised this unresolved
       — RESOLVE: who can author surface (custodian lineage, elohim with stewardship
                  commitment, mishpat governance — A.3 + brainstorm overlap)
 4.  Submerge canonical signal reconciliation                 EXPANDED:
       — EXPAND: shelf_destination vocabulary must cover memory-lifecycle's 7
                 destinations, not just infrastructure (peer-cellar, external-archive)
       — RESOLVE: cancellation flow (who authors, what happens to in-progress fulfillment)
 5.  EconomicResource ← StewardedResource consolidation       REFRAMED:
       — PREREQUISITE: derived views for `total_capacity_value`, `total_used_value`,
                       `available_value`, `allocations_json`, `trends_json` MUST ship
                       before retirement
       — ADDENDUM: `governed_by` field needs a landing (currently no analog in EconomicResource)
       — ADDENDUM: `data_quality` field needs a landing (Monarch data-confidence loses signal)
 6.  Observation spec implementation + Stage 6 cleanup        UNCHANGED, with addendum:
       — PREREQUISITE: graduation evaluator must work before heartbeats retire
                       (same hard-cutover pattern as Gap 5)
 7.  Elohim-authoring pattern                                 UNCHANGED, with addenda:
       — ADDENDUM: graduation-evaluator architecture decision (sharding by kind_namespace,
                   rate-ceiling per subject/period, hub-failover)
       — ADDENDUM: bridge stewardship-elohim parallel-author fallback when bridge fails
 8.  Dissolution semantics (close/revive lifecycle)           UNCHANGED, with addendum:
       — ADDENDUM: custody Commitment lifecycle when CID dissolves — who closes outstanding
                   custody Commitments; BreachScanner must skip closed CIDs
 9.  Bridge pattern for legacy systems                        UNCHANGED, with addendum:
       — ADDENDUM: KYC bridge identity-authority chokepoint mitigation (bridge migration
                   without losing existing credentials)
       — ADDENDUM: PII removal from Attestation `metadata_json` (right-to-be-forgotten
                   doesn't reach the JSON blob today)
10.  Reach-mutation Events                                    UNCHANGED, with addendum:
       — ADDENDUM: tied to Signal-Aggregate Commitment (Gap 4) — without Gap 10, no path
                   to subordinate accumulated signals
```

**New gaps surfaced by Phase 1:**

```
11.  Schema-first IoC governance across all extensibility surfaces  (META — see Meta-Pattern 1)
       Single CI/codegen gate that fails when Rust whitelist, JSON schema, pillar manifest,
       and codegen output don't agree on the vocabulary. Fixes 4 application-archetype
       breakage points + 1 codegen bug ($ref sentinel) in one structural move.

12.  Substrate-floor validator backfill  (META — see Meta-Pattern 2)
       Close Floor F2/F4/F6 ACCEPT-all stubs (attestation_validator.rs Task C.3);
       add retention_class manifest validator;
       complete LINK_ARCHITECTURE.md deprecation checklist;
       close attestation_validator.rs Floor 5 temporal completeness (Task C.2).

13.  Checkpoint / snapshot / aggregate-subordination primitive  (META — see Meta-Pattern 3)
       New Commitment action verbs: `checkpoint` (balance snapshot for high-frequency Resources)
       and `aggregate-subordinate` (Signal-Aggregate Commitment wiring per A.7).
       Defines the read-side release valve for event-sourcing at scale.

14.  Schema-first prerequisites — missing view schemas
       Create `economic-resource-view.schema.json` (Gap 0 prerequisite for any Resource HTTP route)
       Reconcile `proofClass` enum drift between attestation-view.schema.json and validator
       Reconcile `signal_kind` vocabulary across SIGNAL_KINDS / JSON schema / manifests

15.  Standing-curve view contract + policy declaration
       `standing_scores` SQL view definition + manifest-declared standing-curve policy
       (`decay_rate`, `vouch_recovery_fraction`, `debit_soft_weight`, `debit_firm_weight`)
       Update frequency policy (real-time vs eventual-consistency staleness SLA)

16.  Cross-DNA coordination patterns  (META — see Meta-Pattern 6)
       Document `call(CallTargetCell::OtherRole(...))` bridge pattern for cross-DNA link creation
       Specify hub-graduation failover for community-scoped Attestations
       Disambiguate "Membership / Relationship link" zome ownership

17.  Multi-oracle / confirmation-class attestation pattern
       Specify the multi-attestor chain format for `proof_evidence.class = confirmation`
       (mentioned in compute-attestation spec but deferred); affects Monarch's
       attestation:price-feed centralized-oracle chokepoint and AWS-compute verification

18.  Recurring Commitment scheduler stagger discipline
       Analog of TierController's `blake3(cid || peer-id || epoch) % stagger_window`,
       applied to billing-cycle fulfillment to prevent thundering-herd on creator
       projection nodes for high-patron-count Patreon-shape applications

19.  Care-class / compute-class isolation in FeedbackSignal
       Add `signal_class` field on FeedbackSignal (analog of `resource_classified_as` on
       Commitment) so quarantine of bad-compute providers doesn't violate care-class
       isolation per project_compute_commitments_bounded

20.  agent-private encryption key derivation on device migration
       (Was in observation spec open questions; surfaces here because A.4 confirms
       it's an unresolved design decision that blocks the M5 auth-portal convergence)
```

**Total: 20 substrate gaps for Part D.** Roughly doubles the original list. Most new gaps are structural-debt closure (Meta-Patterns 1, 2, 4) or design decisions deferred by canonical specs (Meta-Patterns 3, 5, 6).

---

## Operator-decision queue

These are decisions only the operator can make. Phase 3 Part D rewrite cannot proceed without resolution on the architecture-shaping ones. Less-load-bearing ones can be deferred to subsection-by-subsection review.

### Architecture-shaping (must resolve before Part D rewrite)

1. **`stake_class` placement** — entry field on EconomicEvent (visible at audit time without manifest read) OR manifest-only resolution (cleaner, no field drift). (A.2)
2. **Countersignature for `transfer` Events** — substrate invariant (integrity zome enforces two-party signing) OR convention (coordinator-layer policy)? (A.2)
3. **Graduation-evaluator architecture** — single tokio task per pillar today; need decision on sharding (by kind_namespace, by provider, by EPR), rate-ceiling (per-subject/period), and hub-failover model. (A.2, A.4, A.6)
4. **Surface re-elevation CID continuity** — `update_entry` (new action hash, same CID) vs event-sourced state machine (never mutate entry, lifecycle state derived). (A.3)
5. **Balance-view action-direction logic** — manifest-declared lookup table projected into storage, OR hardcoded service classification, OR coordinator-side per-event direction field? (A.3)
6. **Multi-oracle / confirmation-class price-feed format** — specifies the `proof_evidence` blob shape for `class=confirmation`; affects Monarch oracle-elohim chokepoint. (A.6)
7. **FeedbackSignal vs Observation boundary decision rule** — formal statement: "does this social move require reach-coupling at authoring time? if yes → FeedbackSignal; if no → Observation." (A.4, A.7)
8. **Signal-Aggregate Commitment threshold** — time-based (signals older than 90 days), count-based (more than N signals on target), or standing-curve-based (after curve crystallizes)? (A.7)
9. **Application-layer vs pillar-layer attestation subtype manifest split** — protocol extensibility governance decision; affects auto-tag, face-cluster, computation. (A.6)
10. **`shelf_destination` vocabulary for Gap 4** — must cover memory-lifecycle's 7 socio-institutional destinations, not just infrastructure. (A.5)

### Implementation-detail (can be resolved subsection-by-subsection)

11. PII removal flow for Attestation `metadata_json` under right-to-be-forgotten (A.6)
12. `agent-private` encryption key derivation on device migration (A.4)
13. Diversity threshold governance process after alpha calibration (A.4)
14. Cancellation flow — who can cancel, what happens to in-progress fulfillment, custody-quilt handoff (A.5)
15. Custody Commitment lifecycle when CID dissolves — sequencing of mishpat vs steward-elohim authorship (A.5)
16. Squelch on-DHT vs off-DHT — cheaper and more private off-DHT, but loses provenance (A.7)
17. Standing-curve view update frequency — real-time vs eventual-consistency staleness SLA (A.7)
18. `resource_classified_as_json` vs `resource_conforms_to` for custody payload (A.5)
19. `delete_link` ReconcileController projection (right-to-be-forgotten edge-remove) (A.8)
20. Cross-DNA link creation for `GovernanceActionChild` — bridge call pattern (A.8)
21. `epr_atoms` table schema confirmation (A.3)

---

## Recommendations for Phase 3

### A. Restructure Part D into prerequisite waves

The original Part D was 10 gaps with implicit ordering. Phase 1 findings reveal that several gaps have **hard prerequisites** that must land before they can:

```
Wave A (prerequisites — must land first):
  Gap 11 (schema-first IoC governance) — fixes all 4 application-archetype breakage
  Gap 12 (substrate-floor validator backfill) — closes Floor F2/F4/F6 + retention validator
  Gap 14 (missing view schemas) — economic-resource-view.schema.json + proofClass + signal_kinds
  Gap 6 (Observation spec implementation + Stage 6) — strict prerequisite already named

Wave B (substrate primitives — depends on Wave A):
  Gap 1 + Gap 2 (link types + parent_epr_cid + SQL adjacency tables)
  Gap 5 (EconomicResource consolidation + derived views before retirement)
  Gap 13 (checkpoint / snapshot / aggregate-subordination primitive)

Wave C (lifecycle operations — depends on Wave B):
  Gap 3 (surface re-elevation) — with CID-continuity decision made
  Gap 4 (submerge canonical signal) — with shelf_destination vocabulary expanded
  Gap 8 (dissolution semantics) — with custody Commitment lifecycle clear
  Gap 10 (reach-mutation Events)

Wave D (patterns + interop):
  Gap 7 (elohim-authoring pattern) — with graduation-evaluator architecture decided
  Gap 9 (bridge pattern) — with PII removal flow + KYC migration clear
  Gap 16 (cross-DNA coordination) — bridge call patterns
  Gap 17 (multi-oracle confirmation-class)
  Gap 18 (recurring Commitment stagger)
  Gap 19 (FeedbackSignal signal_class field for care/compute isolation)
  Gap 20 (agent-private key derivation)

Wave E (validator + policy):
  Gap 15 (standing-curve view + policy declaration)
```

**Wave A is the unblocker** — closing those gaps lets Waves B–E proceed without re-derivation cost. The application archetypes (Wave 2 dispatch) can start landing once Wave A is done because the vocabulary will exist.

### B. Defer Wave 2 (applications full-drafts) until Wave A lands?

Original orchestration had Wave 2 (application archetype full-drafts) dispatch after Wave 1 returns. Phase 1 findings expose that **4 of 8 application archetypes have broken substrate-vocabulary references today**. Dispatching Wave 2 against archetypes that reference vocabulary that doesn't exist would produce drafts that bake in the drift.

**Recommendation:** Defer Wave 2 application archetype full-drafts until after the operator decides whether Gap 11 (schema-first IoC governance) is closing during Part D execution or in parallel sprint. If parallel sprint, Wave 2 can still proceed but must flag the vocabulary that's expected-to-land as `// PLANNED — pending Gap 11`.

Alternative: dispatch Wave 2 with strict scope guards — "do not invent vocabulary; if your archetype needs an undeclared verb/classification/signal_kind, flag it in your concerns report and proceed with the closest existing match."

### C. The "structural-debt" track is its own conversation

Meta-Patterns 1, 2, and 4 collectively reveal that the substrate has been accruing debt faster than it's been retired. Gaps 11, 12, and the LINK_ARCHITECTURE.md deprecation are not records-lifecycle work per se — they're substrate-hygiene work that records-lifecycle exposes. The operator may want to consider whether this debt belongs in Part D of this spec or in a sibling sprint.

**Operator decision:** does the records-lifecycle spec land Gaps 11+12 (the structural-debt closure), or does it cite them as prerequisites and defer to a sibling cleanup sprint? Both paths are defensible:

- **Land here:** records-lifecycle becomes the catalyst that cleans the substrate; Part D grows but the cleanup is opinionated and decisive
- **Defer to sibling:** records-lifecycle stays focused on the lifecycle primitive gaps; structural debt closes via its own cadence (probably faster, since the cleanup is mechanical)

### D. What Part C (composability stress-test) needs

Per operator direction, Part C is deferred to placeholders pointing to dev-sprint measurement. Phase 1 findings reveal Part C should specifically measure:

1. Per-peer working set under 8 simultaneous application participation (the 8B-user math)
2. Graduation-evaluator throughput at hub scale (Wave B + Wave D dependency)
3. Standing-curve re-derivation latency at moderation-active hub (Gap 15)
4. Recurring Commitment fulfillment under thundering-herd (Gap 18)
5. Link adjacency query latency under 500+ subordinates (A.8 concern)
6. DHT entry budget under sustained social velocity (A.7 + Gap 13 release-valve verification)

These become the named measurement scenarios in Part C placeholders.

---

## What changed (and what didn't)

**Original gap count:** 10
**Revised gap count:** 20 (10 reframed + addenda; 10 new)

**Original operator-decision count:** ~5 (informal, in brainstorming)
**Revised operator-decision count:** 21 (10 architecture-shaping + 11 implementation-detail)

**Wave structure:** Original had 10 gaps in implicit order. Revised has 5 waves (A through E) with explicit prerequisites.

**Wave 2 dispatch:** Original plan dispatched applications-full-drafts immediately after Wave 1. Revised recommends operator decision on whether to defer until Gap 11 lands.

**What didn't change:** The eight foundational primitives still compose every application pattern. The substrate's "no new DHT entry types" commitment still holds. The architectural framing (records as positions on a graduation pipeline, not new entry types) is fully validated by Phase 1.

---

## Next steps

1. **Operator reviews this synthesis document** — especially the 10 architecture-shaping decisions
2. **Operator + me revise the Part D plan** ([`2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.md`](./2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.md)) to reflect the new gap list + wave structure
3. **Operator decides on Wave 2 dispatch timing** — now (with scope guards) or after Wave A lands
4. **Phase 3 begins** — operator-led Part D execution per the revised plan
