# Chapter 1 of the resiliency-saga (see resiliency-saga/README.md for the full arc).
# Narrative: matthew boots a device — the conductor comes up, and the doorway process
# in front of it reports itself connected and healthy. This is the ground floor every
# later chapter stands on: nothing else in the saga is checkable if the conductor
# isn't up.
# Proof signal: GET /health conductor.connected=true, plus the peer's own healthy
# verdict (healthy:true, status:"online").
# Status today: GREEN — stable, already-verified baseline infrastructure.
@e2e @dataplane @concern:saga-01-device-awakens @act:i
Feature: Chapter 1 — the device awakens
  Before matthew can upload anything or host anyone, his device's conductor must be
  up and the doorway process in front of it must report itself healthy. This chapter
  proves the floor: conductor connectivity and peer health, the precondition every
  later chapter in the saga silently assumes.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: The conductor is connected and the peer reports healthy
    Then peer "alpha-A" /health conductor.connected is true
    And peer "alpha-A" is healthy
