# Chapter 5 of the resiliency-saga: adam co-stewards via a rea-agreement. matthew's
# content needs more than one steward to survive a single device loss — adam's
# co-stewardship is a Mishpat-notarized Commitment (action="replicates-commons" or
# "replicates-dwelling", per mishpat_projection.rs parse_replicates_commons /
# parse_replicates_dwelling), projected into elohim-storage's rea_commitments table
# as an active row (household_resilience.rs filters exactly these two action values
# for its commitment-backed collectives count).
#
# Proof signal: GET /api/v1/commitments?action=replicates-commons&state=active
# (elohim-storage/src/http.rs handle_db_... via db::rea_commitments::list_commitments,
# which filters on rea_commitments.action.eq(action) AND rea_commitments.state.eq(state)
# — an exact, already-wired HTTP surface, proxied by the doorway's generic "/api/"
# service-path prefix) reports at least one row within 60 seconds.
#
# New glue (steps/dataplane/resiliency-saga.steps.ts): a polling commitment-count
# step against this exact surface (no existing step polls /api/v1/commitments with
# a retry loop — resilience.steps.ts's "I list active {string} commitments" reads
# once, with no retry budget).
#
# Status today: BORN RED — no such commitment has been notarized and projected on
# alpha yet; this chapter is the loop's work queue entry for wiring the co-steward
# agreement flow end to end. Do not weaken this assertion to make it pass.
@e2e @dataplane @concern:saga-05-co-steward-agreement
Feature: Chapter 5 — adam co-stewards via a rea-agreement
  matthew's content needs more than one steward to survive a single device loss.
  adam's co-stewardship is a Mishpat-notarized Commitment, projected into
  elohim-storage's rea_commitments table as an active row. This chapter is born red:
  the agreement has not yet been notarized and projected on alpha.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: An active replicates-commons commitment names a co-steward
    Then within 60 seconds doorway "alpha-A" has at least one active "replicates-commons" commitment
