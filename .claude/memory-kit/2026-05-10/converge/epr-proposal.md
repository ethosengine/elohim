# Converge proposal — theme: `epr`

Generated 2026-05-10 (pilot run). Phase 2 synthesis subagent of `/converge`.

**Canonical plan**: `genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md`

---

## Executive summary for operator

The convergence-themes file ranks `epr` #1 with composite 35.29 and 16 contributing plans. Investigation reveals two distinct realities:

1. **EPR Phase 3.5 (trust-compute gradient) is functionally COMPLETE on dev.** Every task T1–T20 has a matching `feat(epr-3.5): T<n>` commit on dev (some with follow-up `fix(epr-3.5): T<n> review` commits). Yet the plan has 117 unchecked checkboxes and 0 marked. Pure checkbox-hygiene gap; the work shipped weeks ago (latest task: 3c435d56d on T20 aunt-and-rage-bait integration).
2. **A successor plan, `epr-light-up`, has been driving forward in parallel**, with commits T01–T48 visible (e.g., 516524204 T48 lint pass, 20e408b54 T35 dashboard topology). I could not locate a dedicated plan file for `epr-light-up` in `genesis/docs/superpowers/plans/` — search may have missed it under a different filename (see "Search-shaped findings" below).

**Posture**: I am proposing mark-done edits ONLY for the 16 Done-definition checkboxes in the canonical plan. I am NOT proposing anything for the 117 step-level checkboxes (too granular; each requires per-step evidence verification; out of scope for a pilot conservativeness budget). I am also NOT proposing any merges, removes, or surface-questions for this theme — see safeguards.

---

## Memorial-tier check

EPR mechanics (FeedbackSignal, AttentionTending, standing_view, gradient) are NOT cited by name in `genesis/docs/content/elohim-protocol/manifesto.md` (verified — only the generic word "gradient" appears once at line 221, in a different context). The principles they implement (Graduated Intimacy + Values Alignment + Living Memory) ARE manifesto principles. So the plan's *deliverables* are tactical, not memorial — safe to mark-done.

However, **the brainstorm artifact** (`genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md`) and **bootstrap manifests** (T17 standing/tending policy) are foundational-adjacent. They encode the §2.8 constitutional floors. These should NOT be archived even if the plan is closed. Memorial flag is for the *floors*, not the plan that shipped them.

---

## Edit proposals

### 1. `mark-done` — close the Done-definition checklist on EPR Phase 3.5

**Plan**: `genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md`

The 16 checkboxes in the `## Done definition` section (lines 1062-1079) all have matching commits on dev. Evidence below.

**Edit** (one entry per checkbox, all with the same evidence anchor format):

```edit
target: - [ ] FeedbackSignal EPR kind shipped (4 variants) with integrity validator + coordinator + sweettest
evidence: commits 5b1a960f0 (T01 schema) → 0ca51f43f-style T4-T8 chain; sweettest at elohim/holochain/dna/elohim/sweettests/feedback_signal.rs; HDI validator at elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-1 -->

```edit
target: - [ ] AttentionTending EPR kind shipped, `Visibility::Private`, with integrity validator + coordinator + cross-agent privacy verified in sweettest
evidence: T5+T9 commits; entry file at elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attention_tending.rs (verify Visibility::Private flag)
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-2 -->

```edit
target: - [ ] CollectiveFilterPattern EPR kind shipped (k-anonymous; no peer identities)
evidence: T6 commit; entry file at elohim/holochain/dna/elohim/zomes/content_store_integrity/src/collective_filter_pattern.rs
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-3 -->

```edit
target: - [ ] Edge-local predecessor map populated on every send; sealed-against-self at rest via dryoc 2-of-2
evidence: commits 67a720869 (T12 back-prop service), T10 (predecessor_records migration), T11 (sealed_against_self.rs); 135690a93 fix follow-up
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-4 -->

```edit
target: - [ ] Hop-by-hop back-prop walk (Primitive 2) wired into FeedbackSignal ingest path
evidence: commit 67a720869 (T12); 69d0f4d7a fix (self-filter + skip-on-unseal-failure)
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-5 -->

```edit
target: - [ ] Gossip-flood notification (Primitive 3) layered on existing `/elohim/epr-atom/1.0.0` protocol
evidence: commit 65ce94f48 (T13 gossip-flood service); 7dc99e214 fix (signal_cid wiring)
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-6 -->

```edit
target: - [ ] Standing computation replaced — `Standing::evaluate(evaluator, subject, conn)` reads standing_view; placeholder deleted; per-evaluator pluralism preserved
evidence: commit a5cf75ed2 (T14 standing_view + projector + Standing::evaluate); 126fc5014 fix (pluralism isolation)
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-7 -->

```edit
target: - [ ] Tending lifecycle (TTL + re-tending + expiry sweep) wired into reconciliation controller
evidence: commit 36517d26a (T15); 71cf78b82 fix (ttl overflow + dead index)
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-8 -->

```edit
target: - [ ] k-anonymous local-peer aggregator emits CollectiveFilterPattern post-threshold
evidence: commit db93815c7 (T16); 609218c05 fix (clippy gates + dead helpers)
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-9 -->

```edit
target: - [ ] Constitutional floor sub-schemas extend standing-policy + tending-policy manifest payloads
evidence: T3 + T7 commits (search `git log --oneline --grep "epr-3.5: T3\|T7"` to confirm); schemas at elohim/sdk/schemas/v1/standing-policy-floor.schema.json and tending-policy-floor.schema.json
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-10 -->

```edit
target: - [ ] Bootstrap default manifests seed first-run via `bootstrap_manifests.rs`
evidence: commit 333fa6356 (T17 bootstrap default manifests)
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-11 -->

```edit
target: - [ ] Author-side compose-time StandingQuery API ships (HTTP route declared in app manifest)
evidence: commit 984d48154 (T18 author-side compose-time StandingQuery API)
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-12 -->

```edit
target: - [ ] Cross-peer test harness primitive shipped; Phase 3's `#[ignore]` on `cold_fetch_resolves_manifest_from_peer` lifted
evidence: commit 7cde2097b (T19 lift Phase 3 cold-fetch ignore via existing harness)
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-13 -->

```edit
target: - [ ] End-to-end aunt-and-rage-bait integration test passes on the new harness
evidence: commit 3c435d56d (T20 aunt-and-rage-bait end-to-end integration); 4ae145778 fix (2-of-2 negative sealed-decrypt assertion)
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-14 -->

```edit
target: - [ ] All Phase 3 quality gates still pass: clippy, schema:test/validate/check-dna, schema-codegen verify, sweettest-check
evidence: OPERATOR-CALL — implied by the successful follow-on epr-light-up sprint advancing to T48 without rolling back gates, but I did not run the gates today. Operator may want to verify before checking this box.
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-15 -->

```edit
target: - [ ] Local merge to dev with `--no-ff` merge commit; no PR
evidence: OPERATOR-CALL — the commit chain on dev includes all T1-T20 commits, but I did not verify whether they were merged via --no-ff or rebased. Operator can confirm with `git log --merges --grep "epr-phase-3-5"`.
```
- [ ] Accept  <!-- id: 2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md:done-16 -->

---

## Edits NOT proposed (and why)

### NOT proposed: mark-done on the 117 step-level checkboxes

There are 117 individual `- [ ]` step boxes across Tasks 0–21. Per safeguard #5, marking these as done would require per-step evidence verification (e.g., does `feedback-signal.schema.json` actually validate all 4 variants? Did `cargo test wire::feedback_signal` actually run green?). This is mechanical work but multiplies the surface area for wrong-mark. The 16 Done-definition checkboxes are the operator-meaningful summary; checking them after the operator confirms is the right granularity for v1 of `mark-done` apply. Recommend: leave the step boxes alone; let the agentic developer who picks up `epr-light-up` continuation flip them naturally as they re-touch each file.

### NOT proposed: archive the canonical plan

Even with the Done-definition closed, this plan is the source-of-truth pointer for `epr-light-up`'s continuation work and for the spec callouts in the brainstorm artifact. Archive only after `epr-light-up` either lands its own canonical plan file OR completes and is itself archive-ready. Per safeguard #7, default to preservation when uncertain.

### NOT proposed: surface-question on the `seed-conductor-identities pre-flight` open question

The convergence-themes file lists one open question for this theme (Sprint Result `/deliver light-up-the-topology` third pass): "**conductor-identities pre-flight**: confirm seed-conductor-identities is needed on alpha (some humans may not have Agent EPR yet) — verify via `imagodei.list_agents` zome query on each pod." This is a **deployment-time op-question**, not an EPR-substrate question. It belongs in the topology / orchestrator theme proposals, not here. (TANGENTIAL.)

### NOT proposed: anything for `epr-light-up`

I could not locate a plan file for it in `genesis/docs/superpowers/plans/`. The commit chain shows ~48 tasks done, but without a plan file I cannot propose mark-done edits (no checkboxes to flip). **OPERATOR-CALL**: is there a plan file under a different name (e.g., `2026-05-XX-epr-light-up-...`)? If yes, please surface; a follow-up converge cycle can then mark its checkboxes too.

---

## Vision rubric — does completing EPR Phase 3.5 advance the manifesto?

Walked manifesto Part II (Design Principles 1–6):

| Principle | Verdict | Reasoning |
|---|---|---|
| 1. Distributed Architecture as Foundation | YES | Standing-view is per-evaluator (no central scoring authority); FeedbackSignal walks DHT + libp2p; gossip-flood reaches current holders. No new centralization. |
| 2. Graduated Intimacy and Contained Intensity | **YES (substantively)** | This is *the* substrate principle — reach is earned by standing; constitutional floors (§2.8) protect vulnerable classes; AttentionTending is peer-private by default. The whole theme implements this principle. |
| 3. Values Alignment Over Rules Enforcement | YES | Manifest-driven debit weights mean different communities project different standings from the same FeedbackSignal subgraph (pluralism). |
| 4. Community-Driven Governance | SOMEWHAT | k-anonymous CollectiveFilterPattern emission is community-aware, but governance is downstream (mishpat / qahal pillars). Tangential to community governance directly. |
| 5. Wealth as Circulation, Not Accumulation | TANGENTIAL | Standing is reputational, not value-flow. EPR is REA-adjacent but the direct economy work is shefa-VF-GraphQL (Phase 4+). |
| 6. Living Memory and the Lifecycle of Data | YES | Tending TTLs + re-tending + expiry sweep (T15) is exactly the "memory grows where tended, fades where not" implementation. |

**Score**: substantively advances 4 of 6 (1, 2, 3, 6); tangential on 4 + 5. Per the score guide ("9-10: directly advances 3+ principles substantively") = **9/10 vision-alignment** for the *theme*.

But for the *act of marking done*: that's hygiene, not delivery. The substrate is already shipped. Marking the boxes advances corpus legibility (Principle 6's living-memory implementation), not protocol capability.

---

## Search-shaped findings

The TF-IDF + bigrams search ranked `epr` #1 by composite score, and rightly so — it's the highest-signal theme in the corpus. Three notes per the safeguard #4 + the bias awareness section:

1. **`epr` is not yet at DF saturation** but trends that way: 16 plan files mention it, 58 signal items, 7 path-renames. As Phase 3.5 + Phase 4 + Phase 5 continue, `epr` will likely cross the >30% DF threshold within 1-2 cycles and start losing visibility to convergence. **Recommendation**: add an EPR-tier section to the manifesto/epic narrative (with English-translated framing per `feedback_no_hebrew_pillar_names_in_narrative` — call it "Provenance-Linked Records" or similar in user-facing prose) BEFORE that happens. This memorializes the theme upstream so it stays anchored regardless of convergence visibility.

2. **`epr-light-up` is invisible to the search.** It's a real workstream with ~48 commits, but has no plan file under that name in the searched directories. The convergence scan will not surface it as a sub-theme until either a plan file lands OR commits start tagging by a different bigram (e.g., `epr-phase-4`). **OPERATOR-CALL**: pre-author a plan file capturing the in-flight work so it surfaces next cycle.

3. **The brainstorm artifact at `2026-04-30-trust-compute-gradient-brainstorm.md` is the foundational doc**. Search did not surface it under this theme (it lives under `specs/`, not `plans/`). Per safeguard #5, I'm noting this as work outside the search's view: the brainstorm has §2.8 (constitutional floors) and §B (aunt-and-rage-bait scenario) which are referenced in T17 and T20 commits respectively. The search's ranking of *plans* misses this *spec* anchor.

---

## Cross-theme coordination

- **iroh theme**: EPR Announce identity-binding (Plan 4 of the iroh-delivery-master) reads from `peer_transport_manifest`, which Plan 1 of iroh delivers. EPR's substrate work is already shipped; iroh's downstream wiring is what the next cycles touch.
- **doorway theme**: T18 author-side compose-time StandingQuery API exposes an HTTP route declared via app manifest. Doorway is a registry consumer; no doorway-side change needed for this theme.
- **recovery theme**: M5 defender stub depends on the constitutional-floor manifests (T17) being live. They are. Defender stub work could resume.
