@e2e @resilience @resilience-p1 @concern:reconcile-inventory @dataplane
Feature: Household reciprocity — the M1 custody pair is named, not assumed

  # This feature is the standing flag from the 2026-06-04 terrance-drift RCA:
  # the M1 custody pair drifted for three weeks because no scenario asserted
  # its MEMBERS by name — two persona-rename sweeps (timothy→terrance in the
  # seeder, timothy→jessica in the Jenkinsfile) chose different successors and
  # every gate stayed green. The pipeline echo described a pair the seeder
  # didn't seed; the seeded pair referenced a suspended persona; the
  # acceptance check (cross-pod blob fetch) passed regardless of who held
  # custody. A story that names its people cannot drift silently.
  #
  # Scope note: this is deliberately ONLY the named-pair flag. The full
  # household-reciprocity story (formation ceremony, balancing defaults,
  # discovery, delegates-compute, member-offline continuity) is design-gated:
  # genesis/data/timeline/backlog/qahal-household-collective-first-class.md

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: Matthew and Jessica hold the M1 custody commitments for each other
    When I list active "custody-blob" commitments
    Then an active "custody-blob" commitment exists from "human-matthew-manager" to "human-jessica-spouse"
    And an active "custody-blob" commitment exists from "human-jessica-spouse" to "human-matthew-manager"

  Scenario: The triad mesh — James is in the household's custody, both ways
    When I list active "custody-blob" commitments
    Then an active "custody-blob" commitment exists from "human-matthew-manager" to "human-james-student"
    And an active "custody-blob" commitment exists from "human-james-student" to "human-matthew-manager"
    And an active "custody-blob" commitment exists from "human-jessica-spouse" to "human-james-student"
    And an active "custody-blob" commitment exists from "human-james-student" to "human-jessica-spouse"
