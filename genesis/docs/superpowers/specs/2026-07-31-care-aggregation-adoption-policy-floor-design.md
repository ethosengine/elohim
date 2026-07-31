---
title: Care Aggregation on the Deterministic Floor — Adoption-Time Policy Contracts, Reach-Graduated Valueflows, and the Constitutional Homeostat
id: care-aggregation-adoption-policy-floor-design
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-implementation
topic: [care-economy, aggregation, reach, coupling, valueflows, rea, mishpat, bounds, k-anonymity, adoption-policy, graduation-ceremony, observer-protocol, lanes, homeostat, epr]
cites:
  - observer-protocol | The elohim-observer epic this design gives a substrate path: witnessed care → REA story elements → aggregate layers (Individual→Family→Community→Municipal→Global); its ephemeral sensor pipeline stays OUTSIDE the floor by design | path: genesis/docs/content/elohim-protocol/observer-protocol.md
  - observation-event-layer-design | The landed observation substrate (tables, observation-log protocol, graduation path) this design feeds with a care kind; it deferred sensor→story extraction to the elohim plane — kept deferred here | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md
  - elohim-seam-map-concern-routing | Seam placement: aggregation lanes ride the mishpat bounds seam; the reverse link plane is dataplane; adoption policy is SDK-grammar (a manifest you add, composed inward) | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
---

# Care Aggregation on the Deterministic Floor

- **Date:** 2026-07-31
- **Status:** Draft (conversation-derived design; grounded against source the same day)
- **Origin:** A single exploration arc: the elohim-host-landing resiliency card led to the missing omni→claims leg, which led to the coupling dimensions (story+value+governance), which led to the question this spec answers — *can witnessed care in the home aggregate up reach tiers to care-economy dashboards without betraying the intimate contexts it comes from, and is the deterministic floor right for it?*

## 1. Verdict and scope

The deterministic floor — the substrate humans and elohim both act upon — is **structurally right**. Three primitives independently derived in design conversation already exist in the tree as code: the private→k-threshold→public aggregation pipeline (`elohim-storage/src/services/aggregator.rs` + `AttentionTending`/`CollectiveFilterPattern` DHT pair), the tiered rollup operator (`CoverageRollup` in `elohim-storage/src/graph_views/recursion.rs` — aggregate-with-descent, content-addressed `rollup_hash`, `witness_quorum` field), and single-point bounds enforcement (`services/bounds_validator.rs`, 7 checks, every validator delegates). They are respectively **uncalled, unchained, and unaware of aggregation** — the vision is blocked by wiring and one vocabulary gap, not by a wrong floor.

This spec fixes the design for: (§3) the signal-variety taxonomy and aggregation lanes; (§4) adoption-time policy contracts — the distribution mechanism for lanes; (§5) the aggregation pipeline and rollup federation; (§6) reach mechanics the wider arc requires (contextual reach, effective-reach overlay, co-signature, reverse link plane, graduation ceremonies, compelled-graduation front door); (§7) planted constraints; (§8) the homeostat. §9 records the scale failure notes; §10 the elohim judgment-plane assumptions this floor is designed to be acted on by.

## 2. Grounded floor audit (as of 2026-07-31)

**Exists and wired:** observation substrate (`observations` tables, `/api/v1/observations/*`, libp2p `/elohim/observation/1.0.0`, graduation path observations→attestation) — fed only by infrastructure/identity kinds; REA rail (economic events incl. `action='appreciate'`, DHT→SQL projection, mishpat→REA `ReplicationMirror` live); bounds keystone (`bounds_validator.rs`); write-path reach gate (`p2p/reach_authorization.rs` — unauthorized EPRs never enter the network); read-path binary reach check (`reach_visible_to`).

**Exists and inert:** `aggregator.rs` k-anonymous aggregator (k=5, Suppress|Laplace-stub, output type structurally forbids peer identity, test-enforced) — zero non-test callers; `CoverageRollup` multi-level composition — proven only in a unit test, production single-layer, `witness_quorum` hard-coded 0, no transport; `rea_observed_compute.rs` — correct-but-dormant (no `compute-fulfilled` producer); the `commitment_backed`/`by_action`/`mutual_compute` folds — no HTTP consumer (`ReaFacingView` exists only as a comment).

**Missing entirely:** a signal-variety taxonomy (nothing anywhere types completion vs timing vs dyad-content for policy purposes); care observation kinds and care economic events; cross-node aggregate federation (view federation moves *rows* — the P1 reconciliation stream — never aggregates); participation-rate metadata; any care-economy dashboard (existing dashboards are spatial-governance and compute-TODO stubs).

**Load-bearing defects:** `humans.household_id` NULL rows silently dropped from every household-keyed aggregate (`household_resilience.rs` `IS NOT NULL`); `replicates-dwelling` mirrors write a hub id into `provider` that can never join `humans` (namespace divergence observed-not-rejected); `in_scope_of` keeps only the first scope value at projection.

## 3. Signal-variety taxonomy and aggregation lanes

**Taxonomy** — schema-rooted like `substrate-signal.schema.json`: a closed, DNA-notarizable enum of signal varieties, first cut: `completion-count`, `engagement-duration`, `timing-pattern`, `sequence-trace`, `content-response` (free-text/answer payloads), `presence-coincidence` (who-was-together). Each variety names what a flow atom *reveals*, independent of its EprKind. One atom may carry several varieties; the taxonomy exists so policy can address them separately.

**Lanes** — per-variety aggregation policy: `aggregate-k-anonymous(k, window)`, `aggregate-suppressed` (never leaves origin reach), `aggregate-count-only`, `aggregate-after-graduation-only`. Lanes attach at the **mishpat bounds plane**, not the envelope: the commitment `bounds` object is schema-extensible (`additionalProperties: true` today) and `epr_scope` already scopes commitments to EPR sets. Lane enforcement is **check 8 of `bounds_validator.rs`**, evaluated at aggregation time by the aggregator — the single choke point once wired. No envelope change; no CID moves.

## 4. Adoption-time policy contracts (steering decision 2026-07-31)

The lanes' distribution mechanism. A commons EPR (a Gottman module, a curriculum cluster) carries an **adoption policy**: a policy atom authored by the content's steward/council at the content's reach, coupled to the content CID. It declares, in taxonomy terms, the aggregation social contract of using the material — e.g. *"completion-count aggregates k≥5 monthly; timing-pattern suppressed; content-response after-graduation-only"* — plus the notice channel (*"ratified harm-review claims render in-context"*).

**Adoption ceremony:** pulling the EPR into a private context (a love-map) presents the policy; acceptance mints a **household-reach mishpat commitment referencing the policy atom's CID**. That commitment IS the lane configuration the aggregator enforces for this household's flows on this content. The pattern is the `.epr-meta` compose-gate generalized: directory-local, locally-executing pre/post guards — here running at the *emission boundary* of the household node. Policy checks execute on the deterministic floor, in the household, before any signal leaves; upstream is never trusted with enforcement.

**Lifecycle:**
- **Revocation** — the existing mishpat `revoke` verb on the adoption commitment. Effect is future-only: emissions stop; contributions already inside published k-anonymous aggregates are irrecoverable by construction (§7).
- **Re-adoption** — a new commitment referencing the *same* policy CID: free, no ceremony burden. The unmodified policy already earned its standing as part of the content's commons reach.
- **Modified adoption** — accepting on *altered* terms mints a **new policy atom** (different CID) whose mint-reach is the household. A modified policy has no standing beyond its authors: it cannot claim the commons policy's benefits (recognition routing, aggregate inclusion on the original terms), and if it wants any wider standing — a collective adopting the variant, a fork redistributing the content under new terms — it climbs the earned-reach ladder like any atom. Anti-enclosure runs both directions: the commons cannot impose un-negotiable surveillance (adoption is refusable per-variety), and an adopter cannot strip the aggregate tithe while keeping the commons' distribution standing.

**Two guards on the ToS analogy:** policies are schema-rooted taxonomy instances — machine-comparable lane declarations, never prose EULAs (a policy that cannot be rendered as a per-variety table is invalid); and commons policy atoms are themselves council-ratified, TTL'd documents on the re-ratification clock (§8) — adhesion terms stay accountable.

**The epistemic timestamp (steering decision 2026-07-31).** Adoption is also evidence. The adoption commitment pins the policy CID *and a content-addressed digest of the claim-state actually visible at the adopting node at adoption time* — not merely what globally existed, but what had reached here. When material adopted in good faith later draws ratified harm claims, the record proves the couple adopted under clean standing ("we didn't know, and couldn't have") — hindsight without retroactive guilt, mishpat's ex-post-facto prohibition made structural, protecting adopter and author alike. **Bare adoption is a legible posture:** an EPR with no adoption policy and no governance state is presented as such ("ungoverned — no council review, no steward warranty") and defaults to the most-protective lanes (nothing aggregates). The absence of governance is itself recorded evidence — at adoption (you knowingly adopted unreviewed material) and in hindsight (the commons had not yet looked; there was no warning to heed).

## 5. Aggregation pipeline and rollup federation

1. **Care kind** — declare the observation kind (kind is a free string; pillar-manifest mechanism) and the first care EconomicEvents (`appreciate` is the live bridge).
2. **Wire the aggregator** — `aggregate_and_emit` gets its production caller at the household node; output pattern entries publish to DHT. Real differential-privacy noise replaces the Laplace stub when the below-threshold mode needs it.
3. **Chain the rollups** — compose `CoverageRollup` multi-level (transitivity already unit-proven); populate `witness_quorum` over a gossip topic so upper-tier rollups are peer-attested; **federate aggregates-as-atoms**. Rollup atoms are content-addressed EPRs carrying: the aggregate, the descent hashes, the witness quorum, and **participation-rate metadata** (k consenting of M eligible) — the consent-equilibrium sensor born inside the rollup shape.
4. **Identity coherence precondition** — fix the household_id NULL class and the hub-id-in-provider namespace divergence before any dashboard is trusted; at scale these are silent aggregate corruption, not gaps.
5. **Consumers** — `ReaFacingView` (the named sibling deliverable) becomes real; care-economy dashboards read rollup atoms at their tier, refreshed at the cadence of witness (§7).

## 6. Reach mechanics the arc requires

- **Contextual reach** — reach values as references to qahal collectives (the dyad is the smallest collective), unifying dyad→household→collective→commons in one mechanism. The largest substrate change in this spec; everything else assumes it quietly.
- **Effective-reach overlay** — effective reach = mint-reach ∧ latest ratified reach-transition claim (council demotion/restoration). Read-side computation; atoms never mutate; holders never lose their copies (museum discipline — content couples to its critique, never disappears).
- **Co-signature** — joint artifacts (shared maps, graduation bills) need multi-party proofs or a countersignature-claim pattern; this is where the co-authored/single-authored consent line is *enforced*: co-authored atoms graduate only with all authors; **single-authored witness about one's own lived experience graduates on the author's sole authority** — dyad governance cannot gag a member's own voice.
- **Reverse link plane** — process participation is class A2 (derived via link): flow atoms reference the resource CID; the EPR surface enumerates reach-visible reverse edges from DHT-anchored links (SQL as cache). Coupling legs stay constitutive, singular, mint-time.
- **Graduation ceremonies** — private flows enter wider contexts only by consent-bearing re-authorship: draft bill at origin reach → co-signatures → collective witnesses and accepts under its own governance → credit claims mint at the collective's reach. Mishpat's `graduate` verb is the prior art.
- **Compelled graduation (the front door)** — k-of-n custody quorum across independent trust domains (composition partly chosen by the subject at enrollment — duress contexts choose jurisdiction-spanning quorums); warrant-as-EPR with dereferencing evidence legs; **preservation and disclosure are separate acts with separate burdens** (preservation cheap and content-addressed; disclosure quorum-gated to councils, never petitioners); sealed orders carry unseal TTLs; the exceptional-access ledger is commons-visible in aggregate. A recovery-capable identity system *has* exceptional access; this governs it in the open rather than pretending otherwise.

## 7. Planted constraints (constitutional, not engineering)

1. **Long-tail darkness** — below-k content contributes nothing (or noise). Mitigate with longer accumulation windows, never lower k. The participation-rate gauge makes the darkness measurable.
2. **Consent-veto bias** — joint-artifact aggregation is veto-able; the worst-harm cohorts are therefore systematically under-sampled. The non-participation rate is itself k-anonymous signal, carefully handled, never de-anonymized.
3. **Aggregate irrevocability** — a contribution inside a published k-anonymous aggregate cannot be clawed out. Revocation is future-only, and adoption ceremonies say so.
4. **Raw sensorium never touches the floor** — the observer epic's ephemeral pipeline (frame → REA story → frame destroyed) is an elohim-plane obligation the substrate can never verify. The floor carries structured stories only; trust in destruction is earned at the judgment plane (witnessed discernment, alignment scenarios, lineage diversity), not encoded in bytes.
5. **Consilience does not shard** — `witness_quorum` means upper-tier rollups are attested, i.e. deliberated. Planetary dashboards refresh at the cadence of witness, not of SQL.
6. **Un-witnessed space is a right** — household perception bounds are governed commitments; what was never perceived can never be compelled. Data minimization protects both species at once.

## 8. Generator functions and the homeostat

Failure *generators* (dynamics that manufacture failure faster than point-fixes drain it) and their counters, most of which ride existing bounds vocabulary (validity windows, rotation TTLs, revoke/graduate verbs):

| Generator | Counter |
|---|---|
| Goodhart collapse (care metrics fund people → metrics get farmed) | reward peer-witnessed outcomes over raw counts; witness diversity; non-deterministic (discernment-based) evaluation |
| Reach oligarchy (earned reach compounds; long tail stays dark) | reach decays without re-ratification — commons standing as TTL'd state |
| Council gerontocracy | sortition from qualified pools; TTL'd mandates; holonic fork-right (sub-commons exit and compete) |
| Constitutional ossification | every governance doc (burden schedules, adoption policies, alignment scenarios) carries a re-ratification clock |
| Value monoculture (one model lineage in every home) | lineage-diversity quorum requirements; commons-funded open lineages |
| Enclosure creep (convenience hubs become de-facto required) | hub-optional floor as *standing tested invariant* — scenarios asserting a lone household can still do everything that matters |
| Individual/commons oscillation (all-private starves sensemaking; all-commons chills and drives exit) | the participation-rate gauge as consent-equilibrium sensor, feeding threshold-adjustment ceremonies |

The homeostat is the stasis-loop discipline constitutionalized: drift accumulates → measured → ceremony drains → baseline resets. Fault-domain diversity is the unit of trust at every plane — the same invariant the resiliency card computes for bytes (`FAULT_DOMAIN_TARGET`) applied to judgment (context- and lineage-diverse council quorums).

## 9. Scale failure notes

Hot-anchor convergence on popular CIDs (the tiered rollup chain is the answer — commons receives rollups, never raw flows); row-federation ceiling (P1 row reconciliation is O(rows); planetary scale federates aggregates-as-atoms only); conductor RAM ∝ corpus (the `sets-authority-arc` mishpat knob is the governed lever); identity-coherence drift as silent aggregate corruption; `in_scope_of` single-value truncation as aggregation-key loss.

## 10. Elohim judgment-plane assumptions

This floor is designed to be acted upon by resident household elohim — assumed future: a household-scale box running a frontier-quality local model over the home's IoT and request flow. Division of labor: **the floor carries facts** (immutable, reach-scoped, k-enforced, identity-absence test-enforced); **the elohim carry discernment** (when/how/whether to render, sealed petitions that disclose judgments without particulars, courtroom accompaniment as witness-with-judgment answering at the scope asked). Elohim are agents in the full REA sense — presences, earned standing, witnessed discernment acts, mishpat bounds, revocability — governed the way persons are, because deterministic verification cannot reach them. Their significant judgments are themselves EPR atoms at appropriate reach. Their capture-resistance is fault-domain diversity: household distribution, context diversity, lineage diversity, staggered governed model-update ceremonies.

## 11. Decomposition seams (the minted ledger)

1. Reverse link plane: reach-filtered flow enumeration from any EPR (A2; probe: nav-context `derived_from` gains a "flows" source).
2. Edge-reach: claims/derivation edges carrying reach independent of their endpoints (open substrate question; nav-context target-reach filtering probe attached).
3. Contextual reach: reach-as-collective-CID.
4. Effective-reach overlay: mint ∧ ratified transition claims.
5. Co-signature proofs + the co-authored/single-authored consent line.
6. Signal-variety taxonomy schema + lane vocabulary (§3).
7. Adoption-policy atoms + adoption/revocation/re-adoption/modification ceremony + the adoption-time epistemic digest (visible-claim-state pinning; bare-adoption legible posture) (§4).
8. Aggregator wiring + care kind + rollup chaining + aggregates-as-atoms federation + participation-rate metadata (§5).
9. Graduation ceremony (voluntary) and compelled-graduation front door (§6).
10. Identity-coherence repairs (household_id NULL class; provider namespace guard; in_scope_of multi-value).
11. Homeostat clocks: reach decay, mandate TTLs, re-ratification cadence (§8).
