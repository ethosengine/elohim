# Chapter 6 of the resiliency-saga: blobs sync to one head. matthew's device and
# adam's mesh must converge on the SAME canonical served head for
# elohim-host-landing, not merely "a" head each — two peers each green over
# DIFFERENT heads is a false-green (see served-projected-head.feature and
# notary-authority.feature's "resolves the same canonical head across peers" step
# for the same distinction at the notary layer).
#
# Proof signal (local): alpha-A's own reconcile pass reports caughtUp=true and
#   divergentAnchor<=0 on /health.
# Proof signal (cross-node): the served head for elohim-host-landing matches the
#   declared head on BOTH alpha-A and elohim.host (reused verbatim from
#   served-projected-head.feature's Track-4 T4-2 step).
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

  Scenario: alpha-A's reconcile pass is caught up with zero divergent anchors
    Then peer "alpha-A" /health p2p.caughtUp is true
    And peer "alpha-A" /health divergentAnchor <= 0

  @requires:alpha-cluster-6peer
  Scenario: alpha-A and elohim.host serve the same converged head for elohim-host-landing
    Given peer "elohim.host" at "elohim.host"
    Then the served head for EPR "elohim-host-landing" matches the declared head on peer "alpha-A"
    And the served head for EPR "elohim-host-landing" matches the declared head on peer "elohim.host"
