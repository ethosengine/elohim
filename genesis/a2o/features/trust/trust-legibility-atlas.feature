@e2e @trust @trust-legibility @concern:trust-legibility @dataplane @act:i
Feature: Interim trust-earning states declare why trust is not yet earned
  As a person looking at a surface that is withholding something
  I want the surface itself to tell me why trust has not been earned yet,
  with a live gauge and the event that would earn it
  So that honesty never reads as brokenness and every dark state is
  diagnosable from what I can see — not from operator-only probes

  # Spec: genesis/docs/superpowers/specs/2026-07-18-trust-legibility-atlas-design.md
  # Contract: NEVER a bare zero, NEVER a bare 503 — every withheld render
  # carries (why, gauge, earn-path). gapKind is the in-repo proof pattern.
  #
  # Canonical fixture: evolution-of-trust on the household floor
  # (matthew/jessica/james + doorway A/B). All nine states below were observed
  # live in one operator session on 2026-07-17; each was diagnosable only via
  # Loki / internal APIs at the time. This suite makes the surfaces speak.
  #
  # @browser legs run under E2E_DEVICE_MODE=playwright and screenshot each
  # state — the suite doubles as the demo deck.

  Background:
    Given the content "evolution-of-trust" exists on the household floor
    And Matthew's household is the canonical steward fixture

  # ── 1. contracts-short (the conforming model — regression-guard it) ──────

  @wip @regression
  Scenario: A card with no provide commitments names contracts-short with its window
    Given no provide commitment matches "content:commons" for "evolution-of-trust"
    When the household resilience card is read
    Then the placement gap declares gapKind "contracts-short"
    And the gap carries firstSeenAt and lastSeenAt
    And the declared earn-path names "a provide commitment lands"

  # ── 2. peers-unavailable — say WHICH and WHY, not just that ──────────────

  @wip
  Scenario: Committed-but-unavailable providers are broken down by cause
    Given provide commitments exist whose providers are not accepting
    When the household resilience card is read
    Then the placement gap declares gapKind "peers-unavailable"
    And the availability summary reports counts for committed, online, stale-key, and offline providers
    And the declared earn-path names "providers heartbeat online under live keys"

  # ── 3. anchor-divergence — the bare catching-up 503 learns to speak ──────

  @wip
  Scenario: A fail-closed catching-up shed declares anchor divergence with a live gauge
    Given the storage node's peer anchors disagree with its local chain
    When any read is attempted through the doorway
    Then the response status is 503 with a Retry-After
    And the shed body declares reason "anchor-divergence"
    And the shed body carries the divergentAnchor gauge and a since timestamp
    And the declared earn-path names "identity-lineage-bridge"
    # Observed 2026-07-18: divergentAnchor 2091→2177 climbing while the surface
    # said only {"status":"catching-up","retryAfter":30} for hours.

  # ── 4. namespace-dead join — silent exclusion becomes a counted exclusion ─

  @wip @regression
  Scenario: Rows excluded by identity-namespace mismatch are counted, not vanished
    Given a provide commitment whose provider is a libp2p transport id "12D3KooW…"
    When the household resilience card is read
    Then the card details report excludedProviders with reason "namespace" and a count
    And the declared earn-path names "provider re-authored under an agent key"

  # ── 5. rekey-fossil drift — distinguishable from mere offline ────────────

  @wip
  Scenario: A fossil-key provider reads as stale-key, not offline
    Given a provider whose peer_statuses row has been frozen for days under a superseded key
    And the same steward's live key is online and pool-member
    When the availability summary is read for "evolution-of-trust"
    Then the summary classifies that provider as "stale-key"
    And the declared earn-path names "stale-for-self heal"

  # ── 6. reach-not-earned — the 403 that explains itself ───────────────────

  @wip
  Scenario: An anonymous read before commons reach is earned explains the earning
    Given "evolution-of-trust" has not yet earned commons reach
    When an anonymous reader requests the content
    Then the response status is 403
    And the body declares reason "reach-not-earned" with the content's current reach
    And the declared earn-path names the reach-earning attestation required

  # ── 7. observe-would-deny — shadow verdicts visible before enforce ───────

  @wip
  Scenario: Op-gate shadow verdicts are readable, not log-only
    Given the delegates-compute op-gate runs in observe mode
    And a performer without a grant writes content through the doorway
    When the op-gate diagnostics surface is read
    Then it reports a would-deny count for that performer with a reason
    And the declared earn-path names "delegates-compute grant seeded"

  # ── 8. head-divergence — false-green becomes self-evident ────────────────

  @wip
  Scenario: Two doorways on divergent heads self-report their disagreement
    Given doorway A and doorway B are notarized over different canonical heads
    When each doorway's health surface is read
    Then each reports its dnaHash, bootstrap id, signal id, and declared head
    And the two declared heads visibly differ
    # Tier-C self-report (fractal-federation §Tier-C) — this scenario is its
    # acceptance test. Without it, two greens on two DHTs read identical.

  # ── 9. diversity-degraded — the quiet XOR fallback announces itself ──────

  @wip
  Scenario: Household-blind placement declares its degraded strategy
    Given salvage placement runs while no candidate has a known household
    When the placement report is read
    Then it declares strategy "xor-degraded" with reason "households-unknown"
    And it carries the count of household-blind candidates as the gauge
    And the declared earn-path names "humans projection populated"

  # ── Browser legs: the atlas as demo deck ─────────────────────────────────

  @wip @browser
  Scenario: Each interim state renders its why to a human
    Given each atlas state can be fixtured over "evolution-of-trust"
    When the affected surface is rendered in a browser for each state
    Then a screenshot is captured per state showing the declared reason
    And no rendered state shows a bare zero or bare error without its why
