---
name: story-delivery-status-axis
description: Story author-status is orthogonal to delivery-status; the delivery-status gradient (envisioned → backlog → refined → wip → active{alpha,beta,latest-stable} → stable, with regression as orthogonal) is the same lifecycle as backlog status, extended both ends
metadata:
  type: feedback
---

Story `status:` (draft/canonical/retired) and story `delivery_status:` are **orthogonal axes**. Author-status says "the storyteller has finished composing, operator sealed it as canonical narrative." Delivery-status says "the feature this story dramatizes actually runs at some maturity level — its Gherkin scenarios + visual confirmation prove the lived substrate."

**The delivery_status gradient** (most-delivered → least-delivered, with regression orthogonal):

```
stable                              ← held green long enough across releases to be load-bearing
regression                          ← was-stable, now broken (sideways state; can apply anywhere right of wip)
active.latest-stable                ← released and marked stable in its current release-channel
active.beta                         ← released, hardening
active.alpha                        ← released, exploratory
wip                                 ← in active development
refined                             ← definition-of-done complete, ready to pull
backlog                             ← identified, not yet refined
envisioned                          ← idea-stage; lives in manifesto/vision, no backlog entry yet
```

**Key distinction — Stable vs Active.latest-stable**: `active.latest-stable` is "most-recent release marked stable in its release-channel"; top-level `stable` is "held green long enough across releases that we trust it as load-bearing substrate." A feature can be `active.latest-stable` for months before earning top-level `stable`. This gives the substrate a stewardship signal distinct from the release signal.

**Why:** Memory ceremony Run #2 (2026-05-14) flipped `james-son--as-stewardee--stewarded-device-sync.md` from draft → canonical, which authorized 6 memory transitions (3 graduations + 3 memorializations) on the basis of *narrative truth*. The canonical feature file `stewarded-device-sync.feature` does not exist. By the new axis, the story is `status: canonical, delivery_status: undelivered`. We graduated working memory on a story whose feature is still owed. Operator surfaced this asymmetry mid-Wave-4 — all four memory-team agents independently proposed accumulators/schemas for it in Wave 5 retro (four-way convergence).

**Unification opportunity**: this gradient is the same lifecycle as backlog `status:` (proposed → ready → in-progress → done) — extended both ends. `envisioned` lives upstream of `backlog`; `wip → active → stable` lives downstream of `done`. Backlog entries, feature files, and stories should share one lifecycle vocabulary. A story's `delivery_status` is then derived (not authored) from the feature(s) it dramatizes — aggregate via **weakest-link policy** (operator-confirmed 2026-05-14): `min()` over `{canonical feature} ∪ {adjacent_features}` by gradient order, with `regression` on any contributing feature propagating UP to story-level `regression`. Spec lives in LIFECYCLE.md → "Story-level aggregation from feature verdicts".

**How to apply:**
1. **Schema add** to story frontmatter: `delivery_status:` from the gradient above. Auto-poller maintains it; parses linked a2o Gherkin pass/fail + visual-confirmation manifest; never operator-authored.
2. **Cartographer ranking weights**: `regression` UP (repair urgency); `envisioned` UP iff vision×readiness scores high; `wip`/`refined`/`backlog` baseline; `active.*` DOWN-ranked but watch for regression; `stable` DOWN (no more work owed).
3. **Librarian gate**: graduations require `delivery_status >= active.latest-stable` on the linked feature (not just feature-file-exists). The Run #2 james-son flip would have failed this gate — canonical narrative + nonexistent .feature = `delivery_status: undelivered`.
4. **Storyteller disposition matrix update**: `graduate-pending` interstitial gets siblings — `graduated-narratively` (canonical story, feature `<= wip`) vs `graduated-fully` (canonical AND feature `>= active.latest-stable`).
5. **Backlog status enum extension**: extend `status:` on backlog/roadmap entries from {proposed, ready, in-progress, done} to the full gradient. Unify with feature-status and story-delivery-status under one vocabulary registered at `genesis/graphos/vocabulary.md`.

**Where this surfaces in the substrate:**
- Story schema in `genesis/data/stories/CONVENTIONS.md` — add `delivery_status` to frontmatter
- LIFECYCLE.md — add the orthogonal-axis concept to disposition matrix
- Stories INDEX.md — render two axes side-by-side per story row
- Pre-shift librarian check — flag `graduation-delivery-gap` ≥1 (graduated story with nonexistent feature file)

The lesson generalizes beyond stories: any artifact that gets sealed by operator-authority should be audited against its substrate-evidence dimension separately. Narrative truth and substrate truth are different categories of truth.
