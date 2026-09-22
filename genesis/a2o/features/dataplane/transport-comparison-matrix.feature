@e2e @dataplane @regression @requires:household-nodes @requires:owned-substrate @concern:transport-parity @act:i
Feature: Matthew can trust every supported household transport
  Staleness evidence is aggregate: its sample count and sum advance alongside
  independent document convergence, without claiming unique-document correlation
  or imposing a latency threshold.
  Matthew's family should receive the same newly shared writing whether their
  storage peers use libp2p, iroh, or dual mode. The single-plane examples
  independently isolate libp2p and iroh while proving the same fresh-writing
  contract. The dual example separately proves convergence while both planes
  perform new sync work; it does not claim which racing plane delivered the writing.

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
    And every receiving household peer records new valid origin-to-projected-apply samples for the selected plane
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
  Scenario: A fresh family writing converges while both planes perform new sync work
    Given the household runs in "dual" mode with selected planes "libp2p+iroh"
    And the household's current transport sync counters are recorded
    And human "Matthew" is logged in on doorway "alpha" with device
    When Matthew creates content titled "Transport matrix dual" with tags "e2e,transport-matrix"
    Then the content should be created successfully
    And every household peer serves the new document at Matthew's exact Automerge heads within 30 seconds
    And every household peer completed new sync work on every selected plane

  @transport:staleness @transport:mixed-iroh
  Scenario: In a mixed household the iroh-only receiver has a new writing well inside one sync round
    Matthew's device authors through peer matthew; James stays dual while Jessica, the receiver, runs
    iroh only. Every peer re-syncs with every other on a 60-second round; on top of that, a dual author
    pushes each new change straight to peers that run iroh only. So Jessica should serve the writing
    within 10 seconds, not when the next round comes around (2026-09-22, before the direct push: 38.6s
    on iroh against ~0.9s on libp2p). An origin-to-projected-apply sample is the age of a change, from
    its authoring timestamp to the moment the receiving peer has applied it and serves it.
    Given Matthew and James run in "dual" mode while receiver "jessica" runs in "iroh" mode
    And the household's current transport sync counters are recorded
    And human "Matthew" is logged in on doorway "alpha" with device
    When Matthew creates content titled "Transport matrix mixed iroh" with tags "e2e,transport-matrix"
    Then the content should be created successfully
    And every household peer serves the new document at Matthew's exact Automerge heads within 10 seconds
    And receiver "jessica" records new valid origin-to-projected-apply samples for the "iroh" plane
