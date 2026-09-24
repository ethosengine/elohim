# A submodule pin move is gated by the pinned commit's own attestation and re-selects
# its direct consumers. Driven the way `steps/devflow/run-plane.steps.ts` drives the
# `epr` CLI: every scenario mints its OWN scratch superproject (one gitlink, two
# manifests, a fake `gh` on GH_BIN) and runs the real gate-runner against it with
# GATE_ROOT, so nothing here reads or writes this repository's manifests or ledgers.
#
# Habit: genesis/orchestrator/.epr-meta/pin-attestation.habit.md
# Spec:  genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md
#
# The tags are suite-routing labels, not behaviour: @e2e and @devflow route this file
# into the devflow suite; @concern:pin-attestation joins each scenario to the habit
# check that claims it. The propagation scenario needs the `rakia` binary and carries
# @requires:rakia-cli — steps/devflow/rakia-cli.guard.ts skips it, never fails it, when
# the binary is absent.

@e2e @devflow @act:host
Feature: A submodule pin is gated by its own attestation

  A superproject pins an external component as a git submodule. Moving that pin is a
  change the superproject's own tests cannot judge: only the component's own CI knows
  whether the pinned commit passed. An "attested" gate therefore runs no local recipe;
  it reads the named check the component's CI recorded at the pinned commit, on the
  upstream repository the manifest names (here "o/comp", the component's owner/name on
  the provider — the gitlink "comp" is where it is checked out).

  Every read is recorded as one pin-attestation observation with two fields: the
  conclusion the check reached, and the evidence tier of the read — "witnessed" when the
  gate actually read the upstream, whatever it concluded; "claimed" when the upstream could
  not be read and the gate passed on trust, saying so. Absent, pending or red checks refuse
  the pin; an unreachable upstream never blocks the floor.

  Selection is a separate question: which projects must the local gate run when a pin
  moves? The rakia oracle is the build-manifest planner that answers it by walking declared
  step dependencies; the local gate keeps one hop from the change, never the whole chain.

  Background:
    Given a scratch superproject with a component pinned as the gitlink "comp"
    And the component declares an attested gate on "o/comp" check "ci"
    And a consumer step "app:build" depends on "comp:ci" and a deeper step "app:deep" depends on "app:build"

  @concern:pin-attestation
  Scenario: A green upstream check passes the pin
    Given the upstream check at the pinned commit concluded "success"
    When the gate runs for project "comp"
    Then the gate exits 0
    And the gate printed "attested: o/comp@" followed by "ci success"
    And one pin-attestation observation was recorded with tier "witnessed" and conclusion "success"

  @concern:pin-attestation
  Scenario: A red upstream check refuses the pin
    Given the upstream check at the pinned commit concluded "failure"
    When the gate runs for project "comp"
    Then the gate exits 1
    And the gate printed "a pin without its attestation is not a green pin"
    And one pin-attestation observation was recorded with tier "witnessed" and conclusion "failure"

  @concern:pin-attestation
  Scenario: An absent upstream check refuses the pin
    Given the upstream has no run of the check at the pinned commit
    When the gate runs for project "comp"
    Then the gate exits 1
    And the gate printed "has no run at"
    And one pin-attestation observation was recorded with tier "witnessed" and conclusion "absent"

  @concern:pin-attestation
  Scenario: A still-running upstream check refuses the pin until it concludes
    Given the upstream check at the pinned commit is still running
    When the gate runs for project "comp"
    Then the gate exits 1
    And the gate printed "not yet concluded"
    And one pin-attestation observation was recorded with tier "witnessed" and conclusion "pending"

  @concern:pin-attestation
  Scenario: A pin to a commit the upstream has never seen refuses, it is not an outage
    Given the upstream has never seen the pinned commit
    When the gate runs for project "comp"
    Then the gate exits 1
    And the gate printed "has no run at"
    And one pin-attestation observation was recorded with tier "witnessed" and conclusion "absent"

  @concern:pin-attestation
  Scenario: An unreachable read passes on the floor and says so
    Given the upstream cannot be read
    When the gate runs for project "comp"
    Then the gate exits 0
    And the gate printed "attested: claimed —"
    And one pin-attestation observation was recorded with tier "claimed" and conclusion "unreachable"

  @concern:pin-attestation @requires:rakia-cli
  Scenario: A pin move selects the component and its one-hop consumer, and nothing deeper
    When selection runs for the changed path "comp" with the rakia oracle
    Then the selected projects are "comp, app"
    And project "app" was selected because of "upstream: comp:ci"
