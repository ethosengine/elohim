# Chapter 3 of the resiliency-saga: matthew uploads elohim-host-landing into his
# eprfs. The EPR atom must resolve on his own peer (alpha-A) with a served head that
# matches what was declared, and a blob actually attached — a bare content record
# with no blob is not an upload, it's a stub.
#
# Proof signal:
#   the served head for EPR "elohim-host-landing" matches the declared head on
#     alpha-A (Track-4 T4-2 — the RUNNING doorway process has actually materialized
#     the head its own content row declares, not merely a stale-but-200 host).
#   EPR "elohim-host-landing" blobHash is non-null on alpha-A (a blob was attached).
#
# Both steps are reused verbatim from steps/dataplane.steps.ts; the same pair is
# used by federation-deploy.feature and served-projected-head.feature as the
# alpha-A (author-peer) baseline.
#
# Status today: GREEN on alpha-A — matthew is the deploy-time author peer, so both
# conditions already hold there.
@e2e @dataplane @concern:saga-03-eprfs-upload
Feature: Chapter 3 — matthew uploads elohim-host-landing into his eprfs
  The upload isn't complete until the EPR record is fully materialized on matthew's
  own peer: a served head that matches what was declared, and a blob attached to it.
  This chapter proves the upload landed before any co-steward or convergence chapter
  can build on it.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: elohim-host-landing resolves with a matching served head and an attached blob
    Then the served head for EPR "elohim-host-landing" matches the declared head on peer "alpha-A"
    And EPR "elohim-host-landing" blobHash is non-null on peer "alpha-A"
