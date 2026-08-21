@e2e @qahal @lens-market @requires:household-nodes @wip @act:i
Feature: Plural Mishpat lenses over a shared resource
  As a community holding a shared concept under governance
  I want several schools of thought to offer their own reading of it, side by side
  So that we sense-make from many valid perspectives instead of being forced into one

  The protocol treats justice (mishpat) as plural: many policies may govern one EPR
  at once, each a co-valid lens authored by a different collective. No lens is the
  single truth by fiat. How often a lens is exercised — by DISTINCT members — earns
  it standing (affinity), and rising disagreement (contention) is read as a call to
  develop a fresh lens, never as grounds to crown a winner. This is the "win-win
  plural narrative": a Georgist land-value reading and a Beerian viability reading
  can both be true in their own context, side by side, without collapsing into one.
  Charter: genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md (§8).

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And a shared EPR "epr:lamad-spa" under community governance

  @regression
  Scenario: Two schools author lenses over the same resource — both surface, no collapse
    Given the "georgist" school authors a lens governing "epr:lamad-spa"
      | telos | tax unimproved land value, not labor |
    And the "beerian" school authors a lens governing "epr:lamad-spa"
      | telos | keep the whole system viable          |
    When a member opens the lens market for "epr:lamad-spa"
    Then both lenses are shown side by side
    And neither is presented as the single authoritative reading
    And each lens names its school and what it steers toward

  Scenario: Affinity ranks lenses by the distinct members who exercise them
    Given the "georgist" and "beerian" lenses both govern "epr:lamad-spa"
    When 2 distinct members exercise the "georgist" lens
    And 1 member exercises the "beerian" lens
    And a member opens the lens market for "epr:lamad-spa"
    Then the "georgist" lens ranks above the "beerian" lens
    And a member exercising the same lens twice counts only once

  @regression
  Scenario: An un-notarized lens never enters the market (fail-closed)
    Given a lens governing "epr:lamad-spa" has no notarized provenance
    When a member opens the lens market for "epr:lamad-spa"
    Then that lens does not appear in the market

  Scenario: A malformed lens is surfaced but flagged, never silently dropped
    Given a notarized lens governing "epr:lamad-spa" has an unreadable telos
    When a member opens the lens market for "epr:lamad-spa"
    Then that lens is still listed
    But it is flagged as degraded and excluded from ranking
    And the rest of the market renders normally

  Scenario: Rising contention is a call for renewal, not a verdict
    Given the lenses governing "epr:lamad-spa" draw sharply split judgements
    When a member opens the lens market for "epr:lamad-spa"
    Then the market reports high contention
    And the contention is framed as a prompt to develop a fresh lens
    And no existing lens is declared the winner by the contention alone

  Scenario: An unknown resource yields an empty but valid market
    Given no lens has been authored for "epr:unknown-thing"
    When a member opens the lens market for "epr:unknown-thing"
    Then the market renders with no lenses
    And it is not an error
