# Chapter 10 of the resiliency-saga: the resilience card tells the truth. The
# resilience card rendered on elohim.host and on alpha must tell the SAME truth —
# two doorways, one truth — or the card is worthless as a felt-safety signal. This
# closes the saga: everything chapters 1-9 built (upload, co-stewardship,
# convergence, custody, capacity, projector cache) is only meaningful if the card a
# human actually sees reflects it, consistently, everywhere it's rendered.
#
# Proof signal (HTTP, runs in Dataplane Validation): GET
# /api/v1/resilience/elohim-host-landing/household stewardingCollectives is the
# SAME non-zero value on both alpha-A and elohim.host. New glue
# (steps/dataplane/resiliency-saga.steps.ts): no existing step compares a field
# across two peers with a non-zero floor in one assertion.
#
# Vocabulary note: the served ResilienceSnapshotView field is
# `stewardingCollectives` (a count of stewarding collectives of any kind — a
# deliberate rename from the earlier "households" framing). There is no
# `householdsStewarding` field on this route's wire shape — that name belongs to
# the unrelated, test-only HouseholdResilienceView (household_resilience.rs's
# `compute()`, exercised only by elohim-storage's own test suite, never wired to
# this HTTP handler).
#
# Proof signal (rendered, @wip/@browser-only — excluded from Dataplane Validation
# by the CI script's "not @browser-only" tag filter): a runLook capture of the
# rendered /resource/{id} card asserts no HTTP errors and a real screenshot. Kept
# @wip because it launches a real headless browser, which the plain-cucumber-js
# Dataplane Validation stage does not provision — this scenario documents the
# deeper visual proof without blocking the suite on it.
#
# Status today: RED on the numeric compare — upstream chapters this one depends on
# (5 co-steward-agreement, 9 projectors-carry) are themselves born red, so the card
# cannot yet tell a converged truth. The rendered scenario stays @wip.
@e2e @dataplane @concern:saga-10-card-tells-truth @act:i
Feature: Chapter 10 — the resilience card tells the truth
  Two doorways, one truth: the resilience card for elohim-host-landing must report
  the SAME non-zero stewarding picture whether a human views it via alpha or via
  elohim.host. This chapter closes the saga — the felt-safety signal is only honest
  if it agrees with itself everywhere it's rendered.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: Both doorways report the same non-zero stewarding count for elohim-host-landing
    Then peers "alpha-A" and "elohim.host" report the same non-zero stewardingCollectives for "elohim-host-landing"

  @wip @browser-only
  Scenario: The rendered resilience card shows the truth on elohim.host
    When I render the resilience card for "elohim-host-landing" on peer "elohim.host"
    Then the rendered card capture has no HTTP errors and a screenshot
