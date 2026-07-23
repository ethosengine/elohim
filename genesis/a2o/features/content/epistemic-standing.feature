# Epistemic standing — peer review as value flow, status derived never stored.
# Keel laws from genesis/docs/superpowers/plans/2026-07-23-ontology-keel-slice1-verdict-spine-plan.md
# Executable proof today: elohim/epr-rea/src/epistemic.rs in-module tests (fold + classify + cite_gate)
# and elohim/epr/src/verdict.rs (Decision spine). These scenarios capture the laws as regression
# stories for when the standing surface reaches the app.

@concern:epistemic-standing @slice:ontology-keel-1
Feature: A claim earns its standing through witnessed review — and the system knows what is not its to decide

  As a learner encountering a claim in the commons
  I need to see honestly where it stands — emergent, reviewed, contested, canon —
  so that new truth can arrive, be tested by peers, and settle, without any
  authority silently minting or burying it.

  Background:
    Given a claim published as content-addressed EPR with CID "bafyrei-claim"
    And peer reviews are witnessed interactions carrying verbs Affirm, Dismiss, or Cite

  Scenario: Emergent truth is served honestly as emergent
    Given the claim has fewer than the minimum peer reviews
    When anyone asks the cite gate about the claim
    Then the decision is permit
    And the witness declares the status "emergent"
    And the claim is never hidden for being new

  Scenario: Peer review accumulates into reviewed standing mechanically
    Given the claim has at least the minimum number of peer reviews
    And the dissent weight is below the contest ratio
    When the standing fold runs
    Then the derived status is "reviewed"
    And no status column was written anywhere — re-folding the events reproduces it

  Scenario: Contest routes to a referral, never to silence
    Given the claim carries dismissals whose weight meets the contest ratio
    When anyone asks the cite gate about the claim
    Then the decision is refer with reason "contested-evidence" routed to the "community" layer
    And the decision is never refuse and never a timeout
    And the witness carries the affirm and dismiss tallies as positive evidence

  Scenario: A threshold alone can never mint canon
    Given the claim has overwhelming affirmations and zero dismissals
    But no governance act references the claim
    When the standing fold classifies the claim
    Then the derived status is "reviewed" and not "canon"

  Scenario: Canon is conferred only by a referenced governance act
    Given a qahal governance act canonizes the claim with a precedent reference
    And the claim is not currently contested
    When the standing fold classifies the claim
    Then the derived status is "canon"
    And the canonization reference is inspectable in the witness

  Scenario: Contest dominates canonization
    Given a governance act canonizes the claim
    But dismissal weight meets the contest ratio
    When the standing fold classifies the claim
    Then the derived status is "contested" and the gate refers to the community layer

  Scenario: Two peers folding the same reviews reach identical standing
    Given two household nodes each hold the same set of witnessed review events in different orders
    When each node runs the standing fold independently
    Then both derive byte-identical standing
    And neither needed a datacenter or a shared ledger to agree
