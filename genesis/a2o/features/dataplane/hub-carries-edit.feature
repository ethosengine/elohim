@e2e @dataplane @regression @requires:household-nodes @requires:owned-substrate @concern:hub-carries-edit @act:i
Feature: A write through a doorway reaches an always-on peer at once and a stopped peer on restart

  A write made through a doorway lands on the peer that doorway writes through. Another household
  peer that stays up must hold the same Automerge heads within seconds, by eager delivery, not the
  60-second sync round. A household peer that was stopped while the write happened must reach
  those heads after it restarts, with nothing written to it directly. Household roles: doorway
  "alpha" writes through peer "matthew"; peer "james" stays up; peer "jessica" is stopped. Seam of
  features/federation/local-first-shared-sheet.feature, where the stopped peer is a shut laptop and
  the peer that stays up is the household hub.

  Scenario: The peer that stayed up converges at once and the stopped peer converges after restart
    Given household peer "jessica" is stopped
    And human "Matthew" is logged in on doorway "alpha" with device
    When Matthew creates content titled "Hub carries edit" with tags "e2e,hub-carries-edit"
    Then the content should be created successfully
    And household peer "james" serves the new document at peer "matthew"'s exact Automerge heads within 5 seconds
    When household peer "jessica" comes back
    Then household peer "jessica" serves the new document at peer "matthew"'s exact Automerge heads within 90 seconds
