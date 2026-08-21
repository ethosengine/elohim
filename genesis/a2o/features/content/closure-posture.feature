# Closure posture — what silence means, declared per axis.
# Laws from genesis/docs/superpowers/plans/2026-07-24-closure-posture-axis-card-plan.md
# Executable proof today: the axis-card registry leg in elohim/sdk/schemas/scripts/test-schema.mjs
# (meta-schema conformance + the totality-law negatives) over
# elohim/sdk/schemas/v1/registries/axes/*.axis.json. These scenarios capture the laws as regression
# stories for when axis cards are read by a counterparty rather than only by our own gates.

@concern:closure-posture @slice:closure-posture-1 @act:host @e2e
Feature: An axis declares what its silence means, so a stranger can read it safely

  As a peer negotiating with an agent whose code I do not share and cannot audit
  I need every gradient axis to say whether absence on it means "unknown" or "no"
  so that I can never mistake a partial view for a permission, nor a missing
  observation for an accusation.

  Background:
    Given a gradient axis is declared by an axis card in the registry
    And every axis has a fact layer and a verdict layer

  Scenario: Absence on the fact layer is unknown, never a negative fact
    Given the axis declares its fact-layer closure as "open"
    And no peer has recorded any observation about a claim
    When the standing over that claim is folded
    Then the result distinguishes "no evidence" from "evidence against"
    And the claim is not treated as refuted for want of a witness

  Scenario: Absence on the verdict layer is a refusal, never a maybe
    Given the axis declares its verdict-layer closure as "closed"
    And no permit has been earned for a requested disclosure
    When the classifier is asked to decide
    Then the decision is refuse
    And the absence is never read as "unknown, therefore possibly allowed"

  Scenario: A closed-verdict axis must name the total function that decides it
    Given an axis card declares its verdict-layer closure as "closed"
    But the card names no classifier
    When the axis registry is validated
    Then validation fails
    And the failure names the missing classifier

  Scenario: An open-verdict axis is not forced to name a classifier
    Given an axis card declares its verdict-layer closure as "open"
    And the card names no classifier
    When the axis registry is validated
    Then validation succeeds
    And the totality law is understood to apply only where a verdict closes

  Scenario: Half a closure declaration is no declaration
    Given an axis card declares a fact-layer closure but no verdict-layer closure
    When the axis registry is validated
    Then validation fails
    And an axis whose posture toward absence is unstated is never admitted

  Scenario: A card points at its vocabulary and never copies it
    Given an axis card names the vocabulary it classifies over
    When the axis registry is validated
    Then the reference resolves to a registered protocol enum
    And the card carries no competing copy of the value list

  Scenario: One axis is known by one name
    Given an axis card is stored at a path bearing the axis name
    When the axis registry is validated
    Then the card's declared axis matches its filename
    And the card's identifier matches its filename
    And a single axis can never accumulate three names

  Scenario: A declared gap is a live obligation, not an excuse
    Given a classifier does not yet satisfy every law its axis declares
    When the axis card is authored
    Then the distance is recorded on the card as a named gap
    And the gap is visible to anyone reading the card
    And the axis is never described as though the distance were already closed
