# Converge proposal — theme: `doorway`

Generated 2026-05-10 (pilot run). Phase 2 synthesis subagent of `/converge`.

**Canonical plan**: `genesis/docs/superpowers/plans/2026-05-07-doorway-ssr-runtime.md`

---

## Executive summary for operator

Doorway is convergence theme #2 (composite 26.33). Two distinct sub-themes are tangled in the convergence-themes file:

1. **doorway SSR runtime** — the canonical plan. Substrate has been DELIVERED to alpha (per shift `doorway-ssr-deliver-2026-05-07T23-37`: HTTP 200 with hydration markers + render cache MISS/HIT confirmed). The plan has 108 unchecked checkboxes and 0 marked. Same checkbox-hygiene gap as EPR. Multiple follow-on commits added capability-deriver, manifest-driven dispatch, render cache, and SSR concurrency derived from operator-as-pod budget (commit 932dbf44e).
2. **doorway blob registry routing** — separate plan (`2026-04-28-doorway-blob-registry-routing.md`); not investigated this pilot since not the canonical candidate.
3. **iroh pkarr resolver in doorway** — distinct workstream (commits 1054f1e3a, 1fb7fad52, 460f4a0f0, 191fec1c4, 80c3d1d51) that touches doorway but belongs to the `iroh` theme. Will be handled in the iroh proposal.

**Posture**: I am proposing mark-done edits ONLY on the doorway-ssr-runtime plan, conservatively. Of the 108 step boxes I am NOT proposing any edits — too granular for v1 mark-done apply. Instead I am surfacing a structural OPERATOR-CALL: the SSR work shipped a *substrate*, but the followup-followups (concept content rendering, memory monitoring, staging/prod manifest hardening) listed in the sprint-result are still open. The plan needs a Phase-2 section, not a "everything done" mark.

---

## Memorial-tier check

The word "doorway" appears once in `genesis/docs/content/elohim-protocol/protocol-specification.md` and not at all in `manifesto.md`. SSR specifically is a delivery-layer concern, not a vision principle. The principle SSR enables — "external WebFetch can read the protocol's content" (which advances Distributed Architecture by removing the Angular-CSR-only chokepoint) — IS manifesto-tier, but as an *implementation* of Principle 1.

**Memorial flag**: per memory anchors `project_doorway_manifest_driven_routes`, `project_doorway_is_federation_surface_atproto`, and `project_doorway_single_target_no_fanout`, doorway has accumulated multiple foundational principles. These principles are memorial-tier; the SSR plan is tactical; safe to mark-done on the latter without disturbing the former.

---

## Edit proposals

### 1. `surface-question` — Phase 2 followups from the doorway-ssr deliver shift

**Plan**: `genesis/docs/superpowers/plans/2026-05-07-doorway-ssr-runtime.md`

The deliver shift on 2026-05-07 surfaced 4 followups (from sprint-digest.md). The plan has no "Open Questions" or "Phase 2" section to anchor them; they currently live only in `.claude/shifts/doorway-ssr-deliver-2026-05-07T23-37/sprint-result.md` where they will fall out of agent attention as the shift directory ages.

**Manual edit** (operator inserts a `## Open Questions / Phase 2 Followups` section near the end of the plan, before any "Status" block):

```
## Open Questions / Phase 2 Followups (added 2026-05-10 from deliver shift)

- [ ] **Concept content rendering**: Angular route's data resolver doesn't synchronize with SSR; shell renders but `<article>` body and `<title>` aren't populated. Needs `transferState` + `provideClientHydration` + sync-aware data resolver. (Source: doorway-ssr-deliver-2026-05-07T23-37 sprint-result.md)
- [ ] **Memory monitoring on the SSR pod**: limit is now 1Gi, observed working-set during V8 parse is ~200MB. Instrument actual usage (Prometheus/Grafana); see if there's room to tune back down without re-introducing flake. (Same source.)
- [ ] **Staging + prod manifests have the same low-memory problem**: `staging.yaml` and `prod.yaml` carry their own resource sections. Apply the same memory bump + startupProbe pattern when SSR rolls to those environments. (Same source.)
- [ ] **Reinforce `feedback_cascade_halt_masks_failures` memory** with the new evidence from this shift. (Same source — non-plan, but the deliver shift surfaced it as actionable.)
```

- [ ] Accept  <!-- id: 2026-05-07-doorway-ssr-runtime.md:phase-2-followups -->

**Note**: `add-as-outstanding` is the closest v1-supported edit kind, but the plan has no named anchor section (no `## Open Questions`). Recommend operator perform this as a manual insert; alternatively, the `add-as-outstanding` apply could insert these one-by-one after the existing "Done definition" section if there is one — read the full plan before applying.

---

## Edits NOT proposed (and why)

### NOT proposed: bulk mark-done on the 108 step-level checkboxes

Same reasoning as the EPR proposal: too granular. The substrate delivered is unambiguous (HTTP 200 with `ngh="0"` hydration markers proves render); but flipping 108 individual `- [ ]` boxes correctly requires per-step evidence verification. For a pilot, the surface-question + Phase-2 capture above is the higher-value edit. **OPERATOR-CALL**: if you want, a follow-up cycle can target the per-task acceptance criteria (one `mark-done` per task, ~20 entries) which is far less prone to misclassification than per-step.

### NOT proposed: archive the canonical plan

Substrate shipped, but Phase-2 followups remain unfinished. Archiving now would lose the followups. Per safeguard #7, preservation default applies.

### NOT proposed: dedupe-merge with the doorway-blob-registry-routing plan

Both plans live in `superpowers/plans/`, both have `doorway` in filename, but they cover different concerns (SSR vs blob registry vs pkarr). Per safeguard #2 (convergent-insight respect): the rediscovery of "doorway is the integration point" across multiple plans is *signal that doorway is foundational*, not duplication. Leave both. Operator may want to memorialize "doorway as registry-driven proxy" pattern (per memory `project_doorway_manifest_driven_routes`) in an epic — but that's promotion-upstream, not merge.

### NOT proposed: anything for the 19 open-questions touching this theme

The convergence-themes file shows 19 open-questions touching doorway. I read the top 5 (the ones extracted into the file). Most are alpha-cluster ingress/routing issues (`/lamad/index.html` returning nginx, ingress.yaml apply needed, conductor stale mapping). These are operational ops-questions, not doorway-substrate questions. They belong in topology / orchestrator theme proposals, NOT here. Per safeguard #5, I am declining to force them into doorway-tier proposals; they distort the doorway substrate work.

---

## Vision rubric — does completing the doorway-ssr-runtime plan advance the manifesto?

| Principle | Verdict | Reasoning |
|---|---|---|
| 1. Distributed Architecture as Foundation | YES | Doorway-SSR removes the Angular-CSR-only chokepoint that previously made content unreadable to external WebFetch agents. This is exactly the "no single point of capture" principle — content becomes legible to peer-to-peer consumption (including AI design tools, social card crawlers, search engines). |
| 2. Graduated Intimacy | TANGENTIAL | SSR doesn't itself implement reach gradient; it serves whatever content the registry permits. |
| 3. Values Alignment | TANGENTIAL | SSR is rendering; values are upstream. |
| 4. Community-Driven Governance | TANGENTIAL | Same. |
| 5. Wealth as Circulation | TANGENTIAL | Same. |
| 6. Living Memory | SOMEWHAT | Render-result cache + manifest-driven dispatch are lifecycle-aware. |

**Score**: substantively 1 principle (Principle 1) + somewhat 1 (Principle 6) = **6/10** vision-alignment. Cross-cutting feature, not substrate.

For the act of marking-done / surfacing followups: **5/10** — hygiene work that helps Principle 6 (living memory) by keeping plan state legible.

---

## Search-shaped findings

1. **`doorway` theme bundles three workstreams** (SSR, blob-registry, pkarr) under one keyword. This is the bigram-vs-unigram bias — the unigram `doorway` absorbs all three; the bigrams `doorway ssr`, `doorway blob`, `doorway pkarr` would surface them separately but each individually has fewer signal points. **Recommendation**: future plans use bigram-rich filenames (e.g., `2026-05-XX-doorway-ssr-phase-2.md` not `2026-05-XX-doorway-followups.md`) so they cluster correctly.

2. **The 19 open-questions count is misleading**. Most are ops/topology issues that mention "doorway" because the bug surfaced at the doorway boundary, not because doorway substrate is at fault. Search has no way to distinguish "doorway-as-victim" from "doorway-as-cause." This is search noise; treat the count with skepticism.

3. **`pkarr` work landed in doorway directory but is iroh-theme work.** The seven recent doorway commits I found (1054f1e3a through 80c3d1d51 + 2e4ff1b2e) all relate to gate #10 of the iroh cutover. Search rightly attributes them to doorway (they're in `doorway/`); but for next-action menu purposes they're iroh-tier. I'll cover them in the iroh proposal.

---

## Cross-theme coordination

- **iroh theme**: pkarr resolver in doorway is Plan 5 of `2026-05-10-iroh-delivery-master.md` (gate #10). All commits visible. Iroh proposal will mark gate #10 done.
- **EPR theme**: doorway carries the StandingQuery HTTP route (T18) declared via app manifest. No doorway-side code change needed for that.
- **topology theme**: alpha ingress / nginx welcome / conductor stale mapping issues belong here when topology proposal is written (not in this pilot).
