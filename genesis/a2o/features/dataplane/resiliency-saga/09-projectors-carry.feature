# Chapter 9 of the resiliency-saga: projector caches carry the head. A resilience
# projector is only trustworthy if its cached view carries the same replication
# truth the live mesh has already converged on — the household-resilience
# projection must report the co-steward's commitment-backed truth, not silently
# omit it.
#
# Proof signal: GET /api/v1/resilience/elohim-host-landing/household
# commitmentBackedReplication.commonsCommitments >= 1 — the field this route's
# actually-served type (ResilienceSnapshotView, elohim-storage/src/views.rs) now
# carries. Reused verbatim: resilience.steps.ts's "When I read {string}" +
# "Then the response field {string} is at least {int}" pair supports dotted field
# paths (readPath()) — the flat "within N seconds ... reports FIELD >= N" step
# does NOT (bare `resp[field]` lookup), so this chapter deliberately uses the
# dotted-path pair instead of that flatter step.
#
# Status today: BORN RED — but not for the reason once documented here. The wire
# gap this chapter originally named is closed: `commitmentBackedReplication` now
# rides the served `ResilienceSnapshotView` on the relational
# `household_resilience::snapshot_with_staleness_secs` path (always set there, per
# `elohim/sdk/schemas/v1/views/resilience-snapshot-view.schema.json`). The chapter
# asserted `totalPledgedBytes >= 1`, but that field is UNREACHABLE by design from a
# content-tier commitment: the fold
# (`elohim_facings::folds::replication_commitment::classify`,
# `elohim-facings/src/folds/replication_commitment.rs:182-194`) deliberately pledges
# 0 bytes for a `replicates-content` / `replicates-commons` **content** commitment
# — it names an EPR, not a byte budget, so it is counted, not summed (only the
# **capacity** variant of a commons commitment, or a `replicates-dwelling` pledge,
# contributes bytes; `action="provide"` rows are excluded entirely as a
# double-count guard against the very commitments this fold already counts — same
# file, lines 27-33). No capacity-tier pledge producer exists anywhere in the
# codebase, so `totalPledgedBytes` can never move off 0 from the co-steward
# agreement chapter 5 proves. This chapter now asserts the count truth the fold DID
# promise: `commonsCommitments`, which flips 0 → 1 the moment chapter 5's
# `replicates-commons` commitment is notarized and mirrored into `rea_commitments`
# — the SAME cure this chapter was always waiting on, just measured by the field
# that is actually reachable. Pledged-bytes coverage returns to this saga once a
# capacity-tier pledge producer exists (tracked in this directory's README residue
# list). Do not weaken this assertion to make it pass.
@e2e @dataplane @concern:saga-09-projectors-carry
Feature: Chapter 9 — projector caches carry the head
  A resilience projector is only trustworthy if its cached view carries the same
  replication truth the live mesh has already converged on. This chapter proves the
  household-resilience snapshot reports the co-steward's commitment-backed count —
  born red, because no `replicates-commons` commitment has been notarized for this
  content yet (the same underlying gap chapter 5 names; the served field itself is
  wired and always present on this route).

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: The household resilience snapshot carries the co-steward's commitment-backed count
    When I read "/api/v1/resilience/elohim-host-landing/household"
    Then the response field "commitmentBackedReplication.commonsCommitments" is at least 1
