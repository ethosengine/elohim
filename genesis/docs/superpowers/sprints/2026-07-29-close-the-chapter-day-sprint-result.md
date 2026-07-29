---
title: "Close-the-chapter day sprint — sprint result (2026-07-29)"
id: close-the-chapter-day-sprint-result
tier: sprint-result
status: Closed
created: 2026-07-29
maintainers: Matthew Dowell + Claude Fable 5
sprint: resiliency-saga
topic: [resiliency-saga, arc-convergence, card-truth, sweettest-flake, agent-bindings, activity-vs-truth]
---

# Close-the-chapter day sprint — result

**Objective:** close another saga chapter off the overnight's 6/10 board.
**Outcome: 8/10 recorded green (edge #1262)** — ch02, ch06, and ch10 all
flipped in one day, ch05+ch09 confirmed stable (3 consecutive runs), and the
operator's morning complaint — elohim.host's resilience card showing zeros —
is live-verified cured: both doorways serve identical truth ("Held by only 1
household — invite another to help hold these").

## The day's arc

1. **Morning verdicts read** (Haiku observers): genesis #1386 = 2/3 affirmed
   (formation cure worked; matthew behind the UUID ceiling). Edge #1256's
   validation predated the cure — live probes showed alpha-A already ahead of
   the recorded board (ch05 finish line passing, stewardingCollectives=1).
2. **ch05/ch09 were already cured** — the 2026-07-26 mirror namespace fix was
   never verification-recorded; today's probes + fresh validations recorded it.
3. **ch07 decomposed** (two stacked causes): the custody gauge is
   request-triggered (only call site = GET /api/v1/weave — nothing ever called
   it; scenario now fires the GET itself) and the fold honestly reports
   unknown=32 (no shard_manifests rows; no custody-blob commitments). Codex
   landed the manifest producer (123fd4bd5); legacy rows need a legitimate
   re-distribution + commitment seed to flow — ch07 stays honest-red.
4. **agent-bindings 7/7 was a costume** (ci-failure-triage): first-reachable-wins
   URL walk sent every human's bindings to adam's saturated conductor (museum
   trap #1119, second instance → museum row #12). Fix live-confirmed in
   genesis #1388: 7 bindings, 4 humans succeeded, 1 adam-residual. ~50
   mis-provenanced DHT bindings documented (rust-architect decision pending);
   read-only inventory tool landed (429f3fa6c).
5. **The substrate red — adam's cold-arc heal deadlock** (rust-architect +
   probe): kitsune2 resets every cell's storage arc to Empty on restart; until
   gossip promotes it back, every get_links takes the network path (60s
   timeouts × 30-fanout), and the heal loop's own abandoned retries keep the
   fetch queue busy — starving the very gossip that would converge the arc.
   Probe: ONE arc-to-full promotion in 2h across ~28 agents. The prior backlog
   prescription (target_arc_factor<1) would have made it permanent — corrected.
   Cures landed (74fbdf2d7 + 82719df4e after adversarial review caught a
   write-gate regression and a conductor-wide timeout risk): synthetic-timeout
   retry classification + per-leg circuit; resolve_content_head_local (new
   extern, heal-loop-only — HTTP authority gate keeps Network); timed-out
   resolves route into the verified carried-record adoption arm; adam gossip
   knobs back to defaults. Post-deploy: the get_links channel-dropped error
   class is GONE (was ~every 20s for hours).
6. **ch02's activity-vs-truth trap** (measurement cause 4, README): the proof
   gauges structurally read 0 after the cure succeeds (per-sweep overwrite;
   created=0 forever once filled). Finish line re-aimed at the durable
   /db/humans rows (live-verified); sweep-liveness kept as a presence station.
   Recorded green in #1260/#1262.
7. **Sweettest flake killed for real** (ci-investigator correlation +
   epr-meta invariant hook): content_visible_across_agents failures track
   sweettest's own shard packing (2-3 conductor shards on one node at 95-99%
   CPU — hostpath-PVC pinning), NOT co-running pipelines; AND the fixed
   content id self-poisoned every nextest retry (dna #1357 class). unique_id +
   120s poll deadline → DNA #1381 and #1382 green consecutively.
8. **Harbor retention bites** (operator cleanup, 131GB quota): the storage
   Dockerfile's hardcoded edgenode digest pin 404'd edge #1261. Repointed to
   the live dev-latest digest; operator added a keep-10 retention policy;
   structural cure (same-run --build-arg) canonicalized in backlog.

## Board at close (edge #1262)

GREEN: 01, 02, 03, 05, 06, 08, 09, 10 · RED: 04 (deploy-window flap — green
live and in #1259/#1260; the validation races the doorway restart), 07
(honest data gap: shard-manifest resolution + custody-blob commitment —
producer shipped, legacy rows await re-distribution).

## Evening close addendum (21:45Z)

- Genesis #1388 stands as the bindings-fix confirmation (7 bindings / 4
  succeeded / 2 skipped / 1 adam-residual vs the prior 0-of-7). #1390 UNSTABLE
  (same shape); #1391 ABORTED mid-run by executor churn (bindings partial
  before abort: 6/3/2/2 — fix still holding; formation stage never reached;
  ABORTED ≠ regression, no successor build was dispatched).
- B at close: cure ACTIVE (get_links channel-dropped class gone since the
  hot-swap; gossip knobs restored) but the content-anchor drain has NOT yet
  completed — caughtUp=false, anchor ~3232, no arc-to-full promotions in the
  first ~30min post-restart. The next session's first read is unchanged:
  adam's heal-complete lines for healed>0 + arc promotions, then B's caughtUp
  flip. If no promotions appear after a few hours, the remaining suspect is
  the per-space initiate serialization under 28 agents (the structural
  provisioning ceiling — operator decision documented in
  backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md).

## Next sprint (pre-authored): the-last-two-chapters

Target: ch07 custody-witnessed + ch04 doorway-serves stability → 10/10
measurable board (ch04 flap cured, ch07 data supplied).

1. **ch07 data supply** — drive a legitimate re-distribution + custody-blob
   commitment seed so the shard-manifest producer (123fd4bd5) populates
   shard→blob resolution and an active `custody-blob` commitment names the
   custodian; `derive_class` then yields stocked>=1 and the (already-firing)
   weave sweep records it. Design constraint: real write path
   (self_stewardship / re-upload), NOT fixture backfill — legacy rows stay
   unguessed by design. Check the p2p-design-gate before adding any new
   entity or route.
2. **ch04 flap cure** — the validation's GET / races the doorway restart
   (green live, red in the deploy-window runs #1257/#1262). Add a bounded
   doorway-ready wait (poll /health before the raw GET) in the ch04 step
   path — measurement hardening, not an assertion change.
3. **Carry-through watches**: B caughtUp flip (ch06 cross-node un-pends,
   card diversity grows); genesis formation stays 2/3 until the operator's
   UUID migration.

## Residue / next

- B's content-anchor drain: cure active, error class gone; arc convergence +
  anchor drain expected over the following hours — watch heal-complete lines
  for healed>0 and arc-to-full promotions across adam's agents.
- ch04's validation timing (wait-for-doorway-ready before the GET /) is a
  small a2o hardening candidate.
- ch07 data path: a legitimate re-distribution + custody commitment seed.
- Operator ceilings unchanged: matthew captured-UUID chain migration
  (unblocks 3/3 affirm + matthew's discovery legs + the ch02 station's
  per-member counts); elohim-host-landing head direction decision;
  mis-provenanced bindings disposition (rust-architect).
- Delegation pattern proven: four Codex lanes (sweettest coverage, ci-observer
  mapping, bindings inventory, custody manifests) rode the shared worktree and
  were gated by the pre-push hook — the agent-agnostic backlog discipline is
  live (see backlog entries tagged `delegable`).

## Watch-outs recorded

- Jenkins result≠null while building=true (catchError stamps UNSTABLE early)
  fools lastBuild monitors — poll the building flag for true completion.
- An empty-commit push still aborts a same-job build in flight; wait for the
  WHOLE wave (all three jobs) before any push.
- ci-observer MCP log reads time out on genesis-size logs — public
  consoleText + grep is the reliable fallback.
- Container bindgen needs BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/clang/21/include
  for datachannel-sys (stdbool.h not found otherwise).
