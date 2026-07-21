# Phase 1 of the contributor-presence commons-stewardship re-grounding
# (genesis/docs/superpowers/specs/2026-07-21-contributor-presence-commons-stewardship-design.md):
# gertrude — the +1 live human lifecycle on shem — steps into her stewardship.
# A claim is an REA Intent that OPENS A NEGOTIATED SETTLEMENT, never an atomic
# transfer. The system convenes and witnesses the settlement, then execution
# happens in the standing shape: deterministic floor (identity binding via
# membership stamp, presence state transitions, claimed_root chain-root
# anchoring, event emission), elohim ceiling (the convening + negotiated
# backfill — judgment ceremonies), execution into the flow (settlement lands as
# append-only REA events, so every settlement is appealable by construction).
#
# CEREMONY-STUB CONVENTION: scenarios assert a ceremony was CONVENED, was
# WITNESSED, and PRODUCED RECORDED EVENTS — never outcome content. Settlement
# semantics are deliberately uninterpreted (spec section 8): record facts,
# defer valuation.
#
# CI DISCIPLINE: all scenarios land PENDING (steps in
# steps/contributor-presence.steps.ts return 'pending'), never failed. They
# un-pend at spec Stage 3 (claim floor + ceremony stubs) and Stage 4 (live
# gertrude legs on shem). Live-conductor legs carry scenario-level
# @requires:shem so the feature stays live while shem is down (the ceremony
# record scenarios are household-testable once implemented).
@e2e @auth @concern:contributor-presence @wip
Feature: Gertrude claims her presence — a convened, witnessed, negotiated settlement
  As a person whose life the commons has been holding in trust
  I want my claim to open a negotiation that the community convenes and witnesses
  So that I step into my stewardship on terms that honor both what accrued and who I am

  Gertrude has lived in the substrate as a commons-stewarded contributor
  presence: witnessed by her household steward, her value held by the
  elohim-commons — never by the individual witness. Claiming is "I step into
  my stewardship of all this": the presence moves stewarded -> claiming ->
  claimed on the deterministic floor, while the terms of the backfill are
  negotiated at the ceiling within the steward web she exits INTO — bounded by
  the dignity floor and the accumulation ceiling, which are boundary
  conditions, not the answer.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And "Gertrude" exists as a commons-stewarded contributor presence with a steward-authored witnessed ascription

  @requires:shem
  Scenario: Gertrude joins the network with her own conductor and files a claim intent
    # The +1 conductor on shem (in resource budget). Filing the claim is an REA
    # Intent — it opens the negotiation; nothing transfers here.
    Given a conductor is provisioned for "Gertrude" on the shem canvas
    When "Gertrude" files a claim intent on her contributor presence
    Then her contributor presence on peer "alpha-A" is in state "claiming"
    And a claim intent event is recorded for her presence

  Scenario: The claim convening is convened and witnessed
    # Ceremony-stub: assert convened, witnessed, events recorded — NEVER what
    # the convening decided. The tending stewards and the commons pool are
    # parties; the witness who ascribed her residency participates.
    Given "Gertrude" has filed a claim intent on her contributor presence
    Then a claim convening for her presence is recorded as convened
    And the claim convening is recorded as witnessed by her tending stewards
    And the claim convening produced recorded events

  @requires:shem
  Scenario: The deterministic floor executes the settlement mechanically
    # Never judgment at the floor: identity binding via the membership stamp,
    # state transition to claimed, and claimed_root anchored to her identity
    # chain-root cid (never a raw key) so the claim survives every rotation.
    Given the claim convening for "Gertrude" has concluded with a witnessed settlement
    When the deterministic floor executes the settlement
    Then her contributor presence on peer "alpha-A" is in state "claimed"
    And her identity binding is stamped through the membership stamp
    And her presence claimed_root is anchored at her identity chain-root

  Scenario: The negotiated backfill is recorded as append-only events
    # Ceremony-stub: the backfill NEGOTIATION is ceiling work (judgment); what
    # this scenario pins is only that its outcome landed as REA events —
    # append-only facts any future settlement story can recompute over.
    Given the claim convening for "Gertrude" has concluded with a witnessed settlement
    Then the negotiated backfill for her presence is recorded as append-only events
    And no backfill event asserts a valuation semantic

  @requires:shem
  Scenario: The household card flips Gertrude from witnessed to verified
    # The observable the whole arc exists for: with her own key bound, the
    # household-resilience holder relation counts her verified — and stops
    # counting her witnessed.
    Given "Gertrude" has completed her claim on peer "alpha-A"
    Then the household holder relation on peer "alpha-A" counts "Gertrude" as a verified holder
    And the household holder relation on peer "alpha-A" no longer counts "Gertrude" as a witnessed holder

  Scenario: A settlement is appealable through correcting events
    # Appealability is structural: corrections are NEW events appended to the
    # flow — the original settlement events are never mutated or deleted. The
    # appeal judgment itself is ceiling work (ceremony-stub).
    Given a witnessed settlement is recorded for "Gertrude"
    When an appeal of the settlement is convened and witnessed
    Then the appeal outcome is recorded as correcting events appended to the flow
    And the original settlement events remain unmutated in the record
