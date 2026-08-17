# Chapter 6 of the resiliency-saga: blobs sync to one head. matthew's device and
# adam's mesh must converge on the SAME canonical served head for
# elohim-host-landing, not merely "a" head each — two peers each green over
# DIFFERENT heads is a false-green (see served-projected-head.feature and
# notary-authority.feature's "resolves the same canonical head across peers" step
# for the same distinction at the notary layer).
#
# Stage split (2026-08-16, operator directive): the CI mesh leg is two
# stages — (1) quiesce/bootstrap, which waits out post-deploy churn and
# proves the SUBSTRATE ITSELF reached a caught-up/converged state, and (2)
# this suite, which runs only after stage 1 passes and proves agent ACTIONS
# flow through the already-quiesced substrate within bounded windows. This
# chapter previously duplicated stage-1 predicates here: a scenario asserted
# alpha-A's whole-node `/health p2p.caughtUp`, the per-sweep
# `elohim_projection_reconcile_converged` gauge, and the
# `elohim_projection_heal_outcomes_total{outcome="healed"}` counter — none of
# which name a specific piece of content, so a red there could never say
# WHICH flow broke. Those same substrate-wide predicates are gated pre-suite
# by scripts/ci/fleet-quiesce-gate.sh (bounded wait for post-deploy churn)
# and defended against regression by scripts/ci/post-deploy-saga-probe.sh's
# own "converged=1 with divergent>=1" honesty fence (the exact check a prior
# scenario here duplicated — see the removed-scenario note below). This
# chapter now proves only the FLOW: alpha-A's own declared head becomes
# alpha-A's own served head (first scenario — local, unconditional, no
# polling needed because it runs after stage-1 quiesce already settled it),
# extended across peers by the cross-node scenario at the bottom.
#
# Proof signal (cross-node): the served head for elohim-host-landing matches the
#   declared head on BOTH alpha-A and elohim.host (reused verbatim from
#   served-projected-head.feature's Track-4 T4-2 step).
#
# Status today: GREEN locally. The cross-node scenario needs the full alpha fabric
# to be meaningful (comparing two independently-operated federation doorways), so it
# is tagged @requires:alpha-cluster-6peer.
@e2e @dataplane @concern:saga-06-heads-converge
Feature: Chapter 6 — blobs sync to one head
  A visitor reaching elohim-host-landing (the household's public landing page) through
  a different doorway than matthew's must see the SAME page matthew published, not a
  stale or half-propagated copy served with no warning that anything diverged — two
  peers each green over DIFFERENT heads is a false-green nobody is told about.
  Convergence means every peer that serves elohim-host-landing serves the SAME head,
  not merely "a" head. This chapter proves the declared head becomes the served head
  on matthew's own peer, then extends the same served-head-matches-declared-head
  proof across the federation.

  Terms this chapter leans on: a peer's DECLARED head is the version it announces
  as canonical for a piece of content; its SERVED head is the version its doorway
  actually delivers to a visitor — this chapter proves the two are the same, on one
  peer and then across peers. "alpha-A" is matthew's storage conductor, the peer
  that publishes elohim-host-landing; "elohim.host" is the second doorway-serving
  peer that must agree with it. "EPR" names a content entry by its protocol slug.
  These scenarios run only after the stage-1 quiesce gate has passed — post-deploy
  churn has settled and the substrate reports caught-up — so they prove
  content-level flow on a settled substrate, never substrate bootstrap itself.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: a locally declared head becomes alpha-A's own served head
    Then the served head for EPR "elohim-host-landing" matches the declared head on peer "alpha-A"

  # REMOVED (2026-08-16, stage-1/stage-2 split — see the header note above).
  #
  # A scenario here previously asserted `elohim_projection_reconcile_divergent`
  # with label "content" >= 1 AND `elohim_projection_reconcile_converged` >= 1
  # in the same breath — a fence added 2026-07-31 against the converged gauge
  # being "fixed" by quietly suppressing the divergence count (history: the
  # dominant divergence class is a heal FORBIDDEN to move a row that already
  # carries a different declared head — fills-never-moves — so matthew logged
  # 6071 `refused_declared` outcomes against 8 `healed` in 12h; convergence had
  # to learn to exclude only ADJUDICATED divergence, never all divergence, or
  # it could never reach 1). That fence asserts a property of the substrate's
  # OWN bootstrap-quiesce gauge computation, not any one piece of content's
  # flow — a stage-1 concern with an existing stage-1 home:
  # scripts/ci/post-deploy-saga-probe.sh runs the identical
  # "converged=1 with divergent>=1" honesty fence as a one-shot post-deploy
  # probe, independent of this suite. No flow-form translation preserves this
  # scenario's intent (it is inherently about the gauge's own arithmetic, not
  # about any content id propagating) — forcing one here would just re-hide
  # the same global predicate behind a different step name, so it is deleted
  # rather than converted. The adjudicated share remains separately readable
  # as `elohim_projection_reconcile_divergent_refused{stream}` for an operator
  # who wants to subtract it out by hand:
  #   elohim_projection_reconcile_divergent - elohim_projection_reconcile_divergent_refused

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
  # OBSERVATIONAL, not causal: no step here triggers a restart. The restart +
  # adopt-before-author sweep are stage-1 preconditions (deploy/quiesce cycle);
  # this scenario proves the state those mechanisms must leave behind.
  Scenario: elohim.host's declared head converges with alpha-A's
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

  # These three prove REGISTRATION, not firing: >= 0 asserts the label series
  # exists and is queryable from boot (pre-touched at registration), so an
  # operator can alert on it without tailing Loki. Proving a COUNT requires
  # inducing each failure class, which would pollute the quiesced stage-2
  # substrate — deliberately deferred to fault-injection scenarios.
  Scenario: the chain-contention failure counter is registered and queryable
    Then labeled metric "elohim_content_witness_reauthor_failed_total" with label "class" "chain_head_moved" on peer "alpha-A" >= 0

  Scenario: the stale-anchor collision counter is registered and queryable
    Then labeled metric "elohim_content_witness_reauthor_failed_total" with label "class" "already_exists" on peer "alpha-A" >= 0

  Scenario: the sweep-abandoned counter is registered and queryable
    Then metric "elohim_content_witness_sweep_abandoned_total" on peer "alpha-A" >= 0

  @requires:alpha-cluster-6peer
  Scenario: alpha-A and elohim.host serve the same converged head for elohim-host-landing
    Given peer "elohim.host" at "elohim.host"
    Then the served head for EPR "elohim-host-landing" matches the declared head on peer "alpha-A"
    And the served head for EPR "elohim-host-landing" matches the declared head on peer "elohim.host"
