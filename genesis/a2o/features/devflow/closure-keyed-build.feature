@concern:push-delivers-within-budget @act:host @wip
Feature: Pay for an edge image only when its build inputs change
  As a developer sharing the integration fleet
  I want branch checks to leave the fleet and its registry tags alone
  And integration builds to reuse images whose complete inputs were already verified
  So unrelated pushes do not consume another build or restart a peer

  Background:
    Given a scratch superproject with declared storage and doorway build inputs
    And a stub registry that records reads and tag writes without network access

  Scenario: a push off dev plans a build, no deploy and no tag
    Given the push branch is "claude/closure-work" instead of "dev" or "main"
    When the orchestrator plans the wave without any commit tag
    Then only edge is selected with run class "build"
    And edge runs quality checks without building release images
    And no publish, deploy or dataplane validation stage runs
    And the hApp, storage and doorway registry tag sets are unchanged

  Scenario: an identical closure is not rebuilt and the hit names the image
    Given an image has the exact closure annotation and a revision on origin/dev
    And a deployed storage peer already has that image and unchanged pod inputs but is NotReady
    When a dev push has identical declared sources, COPY inputs and build recipes
    Then the quality gate and release build both skip using the same oracle result
    And the skip reports the closure prefix and the verified image digest
    And no existing image tag is pushed again
    And a held NotReady storage peer is reported UNSTABLE after one readiness read
    And that held storage peer incurs no rollout wait

  Scenario: a changed Dockerfile, recipe or derivation script is a new closure
    Given a previously verified closure key
    When each Dockerfile, executed recipe and derivation script is changed in isolation
    Then each change produces a different closure key
    And each new image carries its closure, material digest and source revision annotations

  Scenario: a registry that cannot answer builds
    Given a dev push whose registry returns a missing tag, an incomplete annotation or a connection failure
    When the edge pipeline asks whether its closure image is reusable
    Then the oracle reports a miss with its reason
    And the quality gate and release build run

  Scenario: a closure seeded off dev is never a hit on dev
    Given a registry image has an equal closure annotation
    But its revision is not an ancestor of origin/dev
    When the edge pipeline checks reuse for a dev push
    Then the oracle reports a revision-not-on-dev miss
    And the quality gate and release build run
