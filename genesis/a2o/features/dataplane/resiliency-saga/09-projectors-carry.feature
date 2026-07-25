# Chapter 9 of the resiliency-saga: projector caches carry the head. A resilience
# projector is only trustworthy if its cached view carries the same replication
# truth the live mesh has already converged on — the household-resilience
# projection must report the co-steward's pledged bytes, not silently omit them.
#
# Proof signal: GET /api/v1/resilience/elohim-host-landing/household
# commitmentBackedReplication.totalPledgedBytes >= 1 (HouseholdResilienceView,
# elohim/elohim-views/src/infrastructure.rs). Reused verbatim: resilience.steps.ts's
# "When I read {string}" + "Then the response field {string} is at least {int}"
# pair supports dotted field paths (readPath()) — the flat "within N seconds ...
# reports FIELD >= N" step does NOT (bare `resp[field]` lookup), so this chapter
# deliberately uses the dotted-path pair instead of that flatter step.
#
# Status today: BORN RED — elohim-storage/src/services/household_resilience.rs:131
# hard-codes `commitment_backed_replication: CommitmentBackedReplication::default()`
# with a `// T15: computed` TODO comment. The field is a REAL, measured zero today,
# not a missing/absent one — so this is a true assertion failure, not a pending
# skip, until T15 computes it for real. Do not weaken this assertion to make it pass.
@e2e @dataplane @concern:saga-09-projectors-carry
Feature: Chapter 9 — projector caches carry the head
  A resilience projector is only trustworthy if its cached view carries the same
  replication truth the live mesh has already converged on. This chapter proves the
  household-resilience snapshot reports the co-steward's pledged bytes — born red,
  because the field is still a hard-coded zero (T15 TODO).

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: The household resilience snapshot carries the co-steward's pledged bytes
    When I read "/api/v1/resilience/elohim-host-landing/household"
    Then the response field "commitmentBackedReplication.totalPledgedBytes" is at least 1
