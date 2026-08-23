@e2e @dataplane @regression @requires:household-nodes @requires:owned-substrate @concern:transport-parity @act:i
Feature: Matthew can trust every supported household transport
  Matthew's family should receive the same newly shared writing whether their
  storage peers use libp2p, iroh, or dual mode. The single-plane examples
  independently isolate libp2p and iroh while proving the same fresh-writing
  contract. The dual example separately proves convergence while both planes
  remain active; it does not claim which racing plane delivered the writing.

  Each example creates a fresh Matthew-authored content node after recording
  the mode's sync counters. Equal Automerge heads mean the peers hold the same
  CRDT history and therefore the same document state. The example passes only
  when every peer reaches Matthew's new heads. In dual mode it also requires
  both planes to perform sync work, without misattributing that ambient work to
  the new document.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And peer "matthew" at "E2E_STORAGE_MATTHEW"
    And peer "jessica" at "E2E_STORAGE_JESSICA"
    And peer "james" at "E2E_STORAGE_JAMES"

  Scenario Outline: A fresh family writing converges through the isolated <mode> plane
    Given the household runs in "<mode>" mode with selected planes "<planes>"
    And the household's current transport sync counters are recorded
    And human "Matthew" is logged in on doorway "alpha" with device
    When Matthew creates content titled "Transport matrix <mode>" with tags "e2e,transport-matrix"
    Then the content should be created successfully
    And every household peer serves the new document at Matthew's exact Automerge heads within 30 seconds
    And every household peer completed new sync work on every selected plane

    @transport:libp2p
    Examples: libp2p carries the writing
      | mode   | planes |
      | libp2p | libp2p |

    @transport:iroh
    Examples: iroh carries the writing
      | mode | planes |
      | iroh | iroh   |

  @transport:dual
  Scenario: A fresh family writing converges while both planes remain active
    Given the household runs in "dual" mode with selected planes "libp2p+iroh"
    And the household's current transport sync counters are recorded
    And human "Matthew" is logged in on doorway "alpha" with device
    When Matthew creates content titled "Transport matrix dual" with tags "e2e,transport-matrix"
    Then the content should be created successfully
    And every household peer serves the new document at Matthew's exact Automerge heads within 30 seconds
    And every household peer completed new sync work on every selected plane
