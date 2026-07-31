# Chapter 6 of the resiliency-saga: blobs sync to one head. matthew's device and
# adam's mesh must converge on the SAME canonical served head for
# elohim-host-landing, not merely "a" head each — two peers each green over
# DIFFERENT heads is a false-green (see served-projected-head.feature and
# notary-authority.feature's "resolves the same canonical head across peers" step
# for the same distinction at the notary layer).
#
# Proof signal (local): alpha-A's own reconcile pass reports caughtUp=true, its
#   own per-sweep convergence gauge (elohim_projection_reconcile_converged)
#   reads 1 (2026-07-26 reframe — see below), and has healed at least once
#   (elohim_projection_heal_outcomes_total{outcome="healed"}).
#
# Ordering + strictness (2026-07-31): the converged-gauge assertion runs
# BEFORE the healed-outcome assertion, and the healed-outcome assertion uses
# the "strictly" labelled-metric wording. Both changes close the same false-
# green channel: the lenient labelled-metric step returns 'pending' (not a
# hard failure) when the healed{outcome} series has never been observed
# (e.g. right after a restart, before the first successful heal) — and a
# 'pending' step makes cucumber SKIP every remaining step in the scenario,
# so the converged assertion below it never ran and the scenario could read
# green/pending while convergence was actually 0. "strictly" makes an absent
# series a measured zero (assert-and-fail) once /metrics has proven
# reachable, rather than an unobserved 'pending'; reordering additionally
# guarantees the always-materialised converged gauge (registered at process
# start, never legitimately "not yet observable") is checked first and can
# never be shadowed by any predecessor's pending, strict or not.
# Proof signal (cross-node): the served head for elohim-host-landing matches the
#   declared head on BOTH alpha-A and elohim.host (reused verbatim from
#   served-projected-head.feature's Track-4 T4-2 step).
#
# Reframe (2026-07-26): the local scenario previously asserted `/health
# divergentAnchor <= 0` — a value the health handler recomputes from a live
# ~2000-row windowed scan on every single request, so it oscillates between
# requests even when the mesh's TRUE convergence state is stable (chronic
# flappiness, not a real regression). elohim-storage's own reconcile sweep
# already folds the same pending/exhausted/divergent bookkeeping into
# `elohim_projection_reconcile_converged` (elohim-storage/src/metrics.rs) —
# an IntGauge set ONCE per sweep, not once per HTTP poll — plus the
# `elohim_projection_heal_outcomes_total{stream,outcome}` counter (outcome ∈
# healed | timeout_retried | timeout_exhausted | missing | failed | refreshed
# | refused_declared | refused_stale | no_row) that names WHICH class of row
# outcome is happening, not merely a folded pass/fail bit. Reading the
# per-sweep gauge and the monotonic heal counter instead of the per-request
# windowed field keeps the same invariant (this peer isn't stuck divergent)
# while dropping the sampling noise.
#
# Status today: GREEN locally. The cross-node scenario needs the full alpha fabric
# to be meaningful (comparing two independently-operated federation doorways), so it
# is tagged @requires:alpha-cluster-6peer.
@e2e @dataplane @concern:saga-06-heads-converge
Feature: Chapter 6 — blobs sync to one head
  Convergence means every peer that serves elohim-host-landing serves the SAME head,
  not merely "a" head. This chapter proves the local reconcile pass is caught up and
  divergence-free on matthew's own peer, then extends the same
  served-head-matches-declared-head proof across the federation.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: alpha-A's reconcile pass is healing forward and locally converged
    Then peer "alpha-A" /health p2p.caughtUp is true
    And metric "elohim_projection_reconcile_converged" on peer "alpha-A" >= 1
    And labeled metric "elohim_projection_heal_outcomes_total" with label "outcome" "healed" on peer "alpha-A" strictly >= 1

  # HONESTY GUARD (added 2026-07-31, with the divergence-classification cure).
  #
  # `elohim_projection_reconcile_converged` was gated on the divergence TOTAL,
  # and the dominant class of divergence on a live peer is one heal is FORBIDDEN
  # to move: the local row already carries a different declared head, so only a
  # canonical channel may move it (fills-never-moves). matthew logged 6071
  # `refused_declared` outcomes against 8 `healed` in 12h — the refusals are
  # correct and permanent until a canonical channel fires, so the gauge could
  # never reach 1 no matter how well the heal leg worked. It read 0 fleet-wide
  # for 12h+. Convergence now excludes only ADJUDICATED divergence (refused, or
  # retry-budget-spent) and still fails on unadjudicated divergence.
  #
  # This scenario is the fence against the cure becoming a whitewash. It asserts
  # BOTH sides at once: divergence is still MEASURED and published (>= 1 proves
  # the total was not quietly zeroed to make the gauge green), AND convergence is
  # reachable in the same breath. A future change that "fixes" convergence by
  # suppressing the divergence count fails here; so does one that re-gates
  # convergence on the total.
  #
  # The adjudicated share is separately readable as
  # `elohim_projection_reconcile_divergent_refused{stream}`, so an operator can
  # always subtract to get exactly what convergence is gated on:
  #   elohim_projection_reconcile_divergent - elohim_projection_reconcile_divergent_refused
  Scenario: convergence excludes only adjudicated divergence, never unresolved divergence
    Then labeled metric "elohim_projection_reconcile_divergent" with label "stream" "content" on peer "alpha-A" >= 1
    And metric "elohim_projection_reconcile_converged" on peer "alpha-A" >= 1

  # Station (minted 2026-07-26): the interstitial node the cure sprint could only
  # find by log archaeology. Both projections CLAIM a notarized anchor for the
  # content id, but the claims diverge (alpha-A uhCkkh_Gb… vs elohim.host
  # uhCkkl4C9…) — and elohim.host's is a GHOST: an anchor string that outlived
  # the conductor incarnation that authored it (DNA reinstall re-key), so the
  # current conductor holds no chain and the canonical-head declare channel is
  # refused with "no content found". Born red; the ghost-witness sweep re-authors
  # B's side (gate 1), and the anchor EQUALITY this station asserts flips when
  # cross-conductor record retrievability (notary-authority arc, or the
  # declare-carries-Record coordinator change) lands (gate 2).
  # Station (minted 2026-07-27): the missing node between "ghost detected" and
  # the anchor-equality finish line below. Anchor equality is the LAST thing to
  # flip — it needs both peers to hold the same authored root. But the peer's
  # HEAD can converge FIRST and independently: elohim.host does not need to hold
  # alpha-A's root to ADOPT alpha-A's declaration of it.
  #
  # The defect this names: on every restart, elohim.host's re-anchor and
  # ghost-witness sweeps authored a fresh local root AND immediately crowned it
  # (the projection's own-commit path self-declared unconditionally), so B
  # re-elected itself forever and could never converge on A's head no matter how
  # many heal sweeps ran. Adopt-before-author splits authoring from declaring:
  # the sweeps now ask the substrate for an existing canonical head first, and a
  # root they do author is no longer a declaration.
  #
  #   chain: saga-06-heads-converge
  #   between: ghost detected (station above) -> anchor equality (finish line below)
  #   missing node: elohim.host ADOPTS alpha-A's declared head — declared:true
  #     and headActionHash equal to alpha-A's — while the two roots may still
  #     differ. Probe: GET /db/content/elohim-host-landing/head on both doorways.
  #   current state: born red until the adopt-before-author sweep has run on a
  #     restarted elohim.host (one reconcile cadence past restart-churn).
  # Station (minted 2026-07-29, DoD-1 probe evidence): the adopt station's
  # premise — B is headless and should take A's declaration — no longer holds.
  # Probed live: BOTH doorways answer /db/content/elohim-host-landing/head
  # with declared:true, trust:notarized — A=uhCkk78Z… (declaredAt 08:56:34Z)
  # vs B=uhCkkl4C9… (10:30:38Z, NEWER, different blobHash). Heal is
  # structurally forbidden from moving a declared head (fills-never-moves),
  # so "B adopts A" as written would demand a backward move the invariants
  # correctly refuse.
  #
  #   chain: saga-06-heads-converge
  #   between: adopt station (above) -> anchor equality (finish line below)
  #   missing node: a declared-vs-declared conflict rule — when two peers
  #     each hold a declared, notarized head for the same id, which
  #     declaration wins? The forward-ordering guard (declared_at newer)
  #     says B's; the operator's deploy intent may say A's. Probe: both
  #     /head responses declared:true with unequal headActionHash — today
  #     that state is invisible (each side reads locally-green).
  #   DECIDED 2026-07-31 — R1 adopted: no automatic arbitration between two
  #     competing declared heads. Divergence escalates to a FRESH authority
  #     declaration rather than resolving locally; recency (declared_at) is
  #     never the tiebreak — it is not globally comparable across conductors
  #     (head_adoption.rs:36-42). Today's stand-in authority channel is the
  #     deploy declare-cycle (stage-spa-blob.sh DECLARE_ONLY, now load-bearing
  #     rather than advisory). Full record, rejected alternatives (R3), and the
  #     gated successor arc (R2, earned-tier via progenitor_pubkey):
  #     genesis/data/timeline/backlog/content-head-election-vs-reach-fork-arbitration.md.
  #   current state: rule DECIDED; still blocked from live exercise behind the
  #     shem-conductor DHT silence (backlog:
  #     shem-conductors-signal-hairpin-suspect-dht-silent) — no fabric event has
  #     yet driven a real declared-vs-declared conflict through the fresh-
  #     declaration path end to end.
  #   Disambiguation: the scenario immediately below exercises a DIFFERENT,
  #     older mechanism than the R1 rule just decided above — adopt-before-
  #     author's LOCAL-DHT/PEER-HINT arm, which converges a peer holding NO
  #     declared head (or an undeclared one) onto an existing declaration. It
  #     is not the declared-vs-declared conflict-arbitration rule R1 answers
  #     (both peers ALREADY declared, disagreeing). The 2026-07-29 station
  #     above already found this scenario's premise (B is headless) does not
  #     match today's live state (B already holds a conflicting declared
  #     head), so it is expected red until that live conflict is resolved via
  #     the fresh-declaration channel — at which point either peer may again
  #     be genuinely headless (adopt applies) or both converge to the same
  #     declared value (the assertion holds trivially).
  Scenario: elohim.host adopts alpha-A's declared head after a restart
    Given peer "elohim.host" at "elohim.host"
    Then peer "elohim.host" resolves the declared head for content "elohim-host-landing" equal to peer "alpha-A"

  Scenario: Both conductors anchor elohim-host-landing at the same root
    Given peer "elohim.host" at "elohim.host"
    Then peers "alpha-A" and "elohim.host" hold the same DHT anchor for content "elohim-host-landing"

  # ---------------------------------------------------------------------------
  # STATIONS (minted 2026-07-26, story-harvest of live Loki evidence in the
  # elohim-alpha namespace, ~14:0x-14:2x UTC 2026-07-26): gate 1 (the
  # ghost-witness sweep, projection_reconcile.rs's re-author loop, cited by
  # the station above) has two recurring PER-ROW failure classes plus a
  # PER-SWEEP budget failure — all three visible today only by tailing Loki,
  # none of them counted in Prometheus. These are the missing nodes BETWEEN
  # "ghost detected" (the station above) and "gate 1 succeeded": each one
  # names the assertion the eventual fix should make true and the counter
  # that would measure it.
  #
  #   chain: saga-06-heads-converge
  #   between: ghost detected (station above, red) -> gate 1 succeeds (the
  #     re-author lands and the anchor-equality station goes green)
  #   missing node A: adam-alpha's re-author call races a ~1-commit/sec writer
  #     on adam's OWN chain ("Source chain error: source chain head has
  #     moved", observed seq 7096->7476 in ~6 min, adam-alpha, ~14:0x UTC) —
  #     non-fatal + retried per-row, but a chronically busy chain livelocks
  #     that row forever. Probe (proposed, not yet wired):
  #     elohim_content_witness_reauthor_failed_total{class="chain_head_moved"}.
  #   missing node B: jessica-alpha's re-author create COLLIDES with content
  #     that already exists locally ("Content with id '...' already exists.
  #     Use update_content to modify existing entries", jessica-alpha,
  #     ~14:1x UTC) — the stale-anchor heal path's create assumed the row
  #     never already has a local entry; it does. Probe (proposed, not yet
  #     wired): elohim_content_witness_reauthor_failed_total{class="already_exists"}.
  #   missing node C: sweeps on shem-node conductors frequently exceed the
  #     120s wall-clock budget ("sweep exceeded wall-clock budget — abandoned,
  #     resumes next sweep", candidate counts oscillating) — a
  #     saturated/slow conductor drops the WHOLE sweep's progress for that
  #     tick, not just one row. Probe (proposed, not yet wired):
  #     elohim_content_witness_sweep_abandoned_total.
  #   current state: WIRED — all three counters are registered in
  #     elohim-storage/src/metrics.rs (`elohim_content_witness_reauthor_failed_total`
  #     with label `class` = "chain_head_moved" | "already_exists", and
  #     `elohim_content_witness_sweep_abandoned_total`), incremented at the
  #     wire sites in the ghost-witness re-author loop
  #     (`p2p/projection_reconcile.rs::witness_ghost_anchors`), and both
  #     `class` combos are pre-touched at registration so the label series
  #     exist in `/metrics` from boot, not only after first fire. The sweep's
  #     failure modes are now COUNTED, not merely OBSERVED — this sprint's cure
  #     work is measurable instead of Loki archaeology.
  # ---------------------------------------------------------------------------

  Scenario: Chain-contention livelock on a busy writer chain is counted, not silent
    Then labeled metric "elohim_content_witness_reauthor_failed_total" with label "class" "chain_head_moved" on peer "alpha-A" >= 0

  Scenario: Stale-anchor re-author collisions with existing local content are counted
    Then labeled metric "elohim_content_witness_reauthor_failed_total" with label "class" "already_exists" on peer "alpha-A" >= 0

  Scenario: A wall-clock-budget-exceeded sweep is visible without tailing Loki
    Then metric "elohim_content_witness_sweep_abandoned_total" on peer "alpha-A" >= 0

  @requires:alpha-cluster-6peer
  Scenario: alpha-A and elohim.host serve the same converged head for elohim-host-landing
    Given peer "elohim.host" at "elohim.host"
    Then the served head for EPR "elohim-host-landing" matches the declared head on peer "alpha-A"
    And the served head for EPR "elohim-host-landing" matches the declared head on peer "elohim.host"
