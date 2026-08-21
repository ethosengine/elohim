# Chapter 9 of the resiliency-saga: projector caches carry the head. A resilience
# projector is only trustworthy if its cached view carries the same replication
# truth the live mesh has already converged on — the household-resilience
# projection must report the co-steward's commitment-backed truth, not silently
# omit it.
#
# Proof signal: GET /api/v1/resilience/elohim-host-landing/household carries
# BOTH commitmentBackedReplication.commonsCommitments >= 1 and
# commitmentBackedReplication.totalPledgedBytes >= 1. The first proves the
# content-tier promise; the second proves the capacity-tier promise authored by
# the explicit steward consent lever. Reused verbatim: resilience.steps.ts's "When I read {string}" +
# "Then the response field {string} is at least {int}" pair supports dotted field
# paths (readPath()) — the flat "within N seconds ... reports FIELD >= N" step
# does NOT (bare `resp[field]` lookup), so this chapter deliberately uses the
# dotted-path pair instead of that flatter step.
#
# The capacity variant of a `replicates-commons` commitment is now reachable
# through POST /api/v1/commitments/capacity. It reuses the existing Mishpat
# Commitment entry/coordinator/signal and is mirrored per commitment into
# `rea_commitments`, where the replication fold reads `commons_bytes`. This
# chapter therefore restores the byte-budget assertion that was previously
# documented as unreachable. Do not weaken either assertion to make it pass.
@e2e @dataplane @concern:saga-09-projectors-carry @act:i
Feature: Chapter 9 — projector caches carry the head
  A resilience projector is only trustworthy if its cached view carries the same
  replication truth the live mesh has already converged on. This chapter proves the
  household-resilience snapshot reports both the co-steward's commitment-backed
  count and a steward's explicit commons byte-budget pledge.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: The household resilience snapshot carries the co-steward's commitment-backed count
    When I read "/api/v1/resilience/elohim-host-landing/household"
    Then the response field "commitmentBackedReplication.commonsCommitments" is at least 1
    And the response field "commitmentBackedReplication.totalPledgedBytes" is at least 1
