---
id: "backlog-algedonic-phase2-network-phase3-dedupe"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Algedonic phase 2 (network) + phase 3 (dedupe/burn-down) — the arc after local-first"
slug: "algedonic-phase2-network-phase3-dedupe"
written: "2026-08-10"
author: "claude (phase-1 Task 6 capture, operator-directed)"
status: "backlog"
priority: "medium"
tags: [algedonic, feedback-signal, epr-rea, ci-cd, network, dedupe, research-derived]
cites:
  - algedonic-feedback-signal | Algedonic Feedback Signal | sha256:3e9c6fd495dfb854 | path: genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md
  - algedonic-phase1-epr-local-first | Algedonic Phase 1 | sha256:a70d7eab56d40189 | path: genesis/docs/superpowers/plans/2026-08-10-algedonic-phase1-epr-local-first-plan.md
  - algedonic-slice1-delivery-flow | Algedonic Slice 1 | sha256:54eeb412f9a85e2a | path: genesis/docs/superpowers/plans/2026-08-10-algedonic-slice1-delivery-flow-plan.md
  - vision-gap-limit-governor-stub | Vision-Gap STUB | sha256:14ea8f3e81cd87c8 | path: genesis/docs/superpowers/plans/2026-06-14-vision-gap-limit-governor-stub.md
  - .claude/memory/project_ghost_declaration_deadlock_batch3.md
  - elohim/epr-rea/src/epistemic.rs
---

# Algedonic phase 2 + phase 3 — the arc after local-first (2026-08-10)

Phase 1 (`algedonic-phase1-epr-local-first`) closed the loop at the EPR level, local-first: the
devspace producer → stock/limit → addressed signal → consumer pattern is typed, validated, and
demonstrably closed with real pain (`elohim/epr`, `epr-rea`, epr-meta measure mint, habits
renderer). This is the single re-surfacing point for what comes next — phase 2 graduates the
pattern to the network (CI/CD-produced signals), phase 3 burns down what the pattern subsumes.
**Fold new algedonic-arc concerns here — do not mint siblings.**

## Phase 2 — network (graduate the local-first pattern past the devspace)

| # | Item | What + why | Gate/blocker | Owner shape |
|---|------|------------|---------------|-------------|
| 1 | **Runtime-harvest concern threading** (slice-1 Task 3, held) | `evaluate()` needs a context param so exhaustion findings in `runtime-findings.jsonl` carry the same `@concern` address that `ci-harvest`'s `route()` call gives CI findings — the address book (slice-1 Task 1, landed `8a05236a7`) already exists; only the runtime-harvest call site is unwired. Spec [§5 slice 1](epr:algedonic-feedback-signal). | none — bounded, mirrors the landed ci-harvest wiring | quality-deep / small |
| 2 | **Pre-push `[build:edge]` tag / changeset coherence deny** (slice-1 Task 4, held) | One deny clause in the existing pre-push hook makes measurement-by-deploy unwritable at push time — a `[build:edge]` dispatch tag with no edge-relevant changeset in the commit range gets refused, so "just to measure" can't restart the fleet it's measuring. Spec [§5 slice 1](epr:algedonic-feedback-signal). | needs the hook's existing tag-parse surveyed before the deny clause lands | rust-architect / small |
| 3 | **Live CI validation of the ci-harvest concern + no-measure wiring — re-verify the feedstock, don't assume batch-3** | Slice-1 Task 2 (landed `11f334120`) wired the `ci-no-measure` finding class on the no-measure feedstock as understood at authoring time. That understanding has since shifted: the alpha "~2000 unanchored rows" batch-3 residual was **re-diagnosed as anchored rows with phantom declared heads** (dead incarnations wedging the anti-self-election guard), not a missing-anchor/no-measure gap — cured by ghost-declaration decay, commit `a9f9d781b` (memory: `project_ghost_declaration_deadlock_batch3`). Phase-2 planning must re-verify which no-measure causes are still live against the current fleet state rather than planning against the superseded batch-3 backfill framing. Spec [§5 slice 1](epr:algedonic-feedback-signal). | needs a live CI run + fresh ledger read before scoping | ci-investigator survey → rust-architect if causes remain |
| 4 | **CI/CD emitting typed algedonic signals tracked by EPRs/epr-meta** (network→local-first) | The devspace producer role (epr-meta measure mint, phase-1 Task 4) graduates to CI/CD pipelines themselves as producers — a pipeline stage becomes a first-class algedonic emitter addressed at an `@concern`, not just the sentinel ledgers mirroring it locally. This is the actual "graduate to network" step the phase-1 plan named and deferred. Spec [§5 slice 2](epr:algedonic-feedback-signal); phase-1 plan operator-steering note. | depends on #3 (re-verified feedstock) and slice-2's zome whitelist (phase 3 #6) | rust-architect design pass |
| 5 | **`bound_stock` determinism decision** | `bound_stock` (epr-rea's fold, phase-1 Task 3) sums in event order, not CID order — two peers folding the same events in different arrival order could disagree by one ULP at an exact band edge on whether the threshold was crossed. `elohim/epr-rea/src/epistemic.rs:136` (`ordered.sort_by_key(\|e\| e.event_cid)`) already sorts by CID for exactly this reason; `fold::fulfillment` has the same unfixed property and has never needed determinism because fulfillment ratios aren't consensus-visible the way algedonic pain crossing a band edge is. Decide (sort-by-CID in the fold, or an explicit tolerance band) **before** the network rung, where cross-peer agreement on crossing matters. Ledgered in phase-1 Task 3's report as a known unknown. | design decision only — no blocker | rust-architect decision |

## Phase 3 — dedupe / burn-down (slice-2 spec items this arc subsumes)

| # | Item | What + why | Gate/blocker | Owner shape |
|---|------|------------|---------------|-------------|
| 1 | **Vision-gap limit-governor stub supersession check** | `2026-06-14-vision-gap-limit-governor-stub.md` is cited by the phase-1 plan as the band-edge design source; check whether epr-rea's landed `Bound`/`bound_stock` (phase-1 Task 3) supersedes the stub outright or whether the stub still carries live scope the fold doesn't cover. | needs a read-through diff, not new design | quality-deep survey |
| 2 | **`rate-limit-exceeded` wire-schema alignment to the algedonic evidence shape** | The existing `rate-limit-exceeded.schema.json` predates the algedonic `evidence: {stock, limit, bound_ref}` shape (slice-1 Task 6, landed) — align its fields to the same envelope so consumers don't special-case one kind. Source of truth stays the DHT `FeedbackSignal` entry; this is wire-contract cleanup only. Spec [§5 slice 2](epr:algedonic-feedback-signal). | none — schema-only edit + `pnpm run schema:test` | rust-architect / small |
| 3 | **C15 algedonic-channel minting in the concern canon** | Slice-2 item, unchanged from the spec: mint the algedonic channel as a first-class C15 concern-canon entry. Spec [§5 slice 2](epr:algedonic-feedback-signal). | rides #6 (zome whitelist) for the entry-type leg | rust-architect |
| 4 | **App-manifest `algedonicHandler` field** | Slice-2 item: the app manifest needs a declared handler surface for algedonic kinds, mirroring how other signal kinds are routed. Spec [§5 slice 2](epr:algedonic-feedback-signal). | none standalone; useful once #6 lands | angular-architect / small |
| 5 | **`.epr-meta` ask-policy on EprKind-birth surfaces** | `elohim/epr/src/kind.rs` and `elohim/sdk/domains/*/manifest/` are where new EPR kinds (including future algedonic sub-kinds) are born; they need an `.epr-meta` ask-policy so a new kind can't be minted without the do/do-not canon (phase-1 Task 2) being consulted. Spec [§5 slice 2](epr:algedonic-feedback-signal). | needs the epr-meta authoring pattern (see `elohim-epr-metafile` skill) | rust-architect / small |
| 6 | **Zome `SIGNAL_KINDS` whitelist + kind-gates + `CounterEvidence` floor routing** | Slice-2's protocol-wiring core: `SIGNAL_KINDS` whitelist extension + kind-gates in `create_feedback_signal` (algedonic requires `evidence` + `bound_ref`), plus `FloorClass::CounterEvidence` routing + property-test extension. This is the DHT-side leg every other phase-3 item assumes. Spec [§5 slice 2](epr:algedonic-feedback-signal). | p2p-design-gate already cleared at spec-authoring (Category A, existing entry type) — needs a zome-change pass | rust-architect shift |

**Sequencing note:** phase-2 #1-#2 are independent, bounded re-pickups of held slice-1 work and can
land any time. #3 (re-verify feedstock) should run before #4 (CI/CD as producer) — no sense
designing the network producer role against a feedstock assumption already known to be stale. #5
(bound_stock determinism) is a standalone decision, cheap to make now, expensive to discover live
once cross-peer signals exist. Phase-3 items 3-6 are the slice-2 spec's protocol-wiring core and
gate on #6 (zome whitelist) as the shared DHT-side leg; items 1-2 are independent cleanup.
