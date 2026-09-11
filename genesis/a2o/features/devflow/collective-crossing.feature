@e2e @devflow @concern:dev-system-equilibrium @act:household @requires:household-nodes @wip @design
Feature: A repository collective crosses onto the network and its local relationships become witnessed ones
  As a person who stewards a repository collective and the agents that work in it
  I want the collective, its members and a finding they agreed to widen to be published into
  a wider space that real peers hold
  So that what was a declaration on my disk becomes a relationship other people can verify,
  correct and be paid against, without the finding changing its identity on the way

  # This story is a specification for the network slice. It is held (@requires:household-nodes)
  # until the local mesh lane can run it; nothing in the local slice claims it.
  #
  # Vocabulary a cold reader needs:
  #   - A collective is first declared by a file in the repository (`.epr-meta/collective.json`)
  #     with member affiliations logged beside it. On the network the same collective is an
  #     entry in a shared space and each affiliation is a membership attached to that entry.
  #     Crossing is minting the network records from the local ones; nothing is translated.
  #   - A finding's CID (content identifier) is a hash of its bytes. Widening a finding into a
  #     larger space changes who holds it, never its CID.
  #   - The elohim are the protocol's witnesses across spaces; a crossing is recorded by them.
  #   - A steward affiliation minted from a local declaration is pending until another steward
  #     counter-signs it; that counter-signature is called sponsorship. Only a sponsored steward
  #     may approve anything on the network.
  #   - A household mesh is a connected group of peers that share one household space.
  #   - The elohim witness record is written automatically by the protocol whenever anything
  #     crosses between spaces; no one requests it and no scenario step triggers it.
  #   - An agent identity is a small record of what an agent is made of: its model, its persona
  #     package and any tuning layer. Each run of that agent is a separate session record that
  #     points at the identity. Contributions name the identity as author and the session as
  #     provenance. The ElohimAgent contributor in the Background is one such identity.
  #   - A DID document is a standard, per-request description of an identity assembled from
  #     network truth by the DID bridge, the protocol's resolver for that standard. Its subject
  #     is the identity; its controllers are who may act for it. For an agent identity the
  #     controllers are its stewards.
  #   - Locality says who may see a record: private (its author only), repository (members of
  #     the collective), household (peers of the household space). Widening moves a record to a
  #     larger locality; a record that cites a smaller-locality passage cannot widen past it.
  #   - A value event is an inbound economic record, such as a payment or a grant, denominated
  #     in some unit, received by the collective as a whole.
  #   - Contribution share is a computed report over recorded work; it is never stored.
  #   - A collaboration agreement is a network-published contract among a collective's members
  #     that says how received value is distributed. Until one exists, contribution shares are
  #     informational only and no value moves.
  # Contract: the crossing refuses to widen anything that depends on evidence the wider space
  # may not see, and refuses to settle value before affiliations are counter-signed. It never
  # rewrites local history; local records remain readable and are what the network records cite.

  Background:
    Given a household mesh with two peers
    And a repository collective declared locally on the first peer
    And the local affiliations name one Person steward, who is the repository's git author, and one ElohimAgent contributor
    And the second peer's household has its own steward on record

  Scenario: The collective and its affiliations are minted onto the network from the local declaration
    When the steward publishes the collective into the household space
    Then the space holds a collective entry whose charter equals the local declaration's charter
    And each local affiliation becomes a membership on that entry with the same member, kind and role
    And the local declaration and affiliations are unchanged and cited by the network records

  Scenario: A finding widens with its CID unchanged, its holders changed and a witness record written
    # Approving a finding for a locality is a distinct local act covered by the agent-provenance story.
    Given a local finding approved for repository locality by a steward other than its author
    When the finding is published into the household space
    Then the second peer holds the finding under the same CID
    And the elohim witness record names the finding's CID, the source collective and the destination space

  Scenario: A finding that depends on local-only evidence does not cross
    Given a local finding whose receipt cites a passage marked private locality
    When the steward attempts to publish the finding into the household space
    Then the crossing is refused naming the private passage
    And no record for that finding appears on the second peer

  Scenario: A minted steward cannot approve until counter-signed
    Given the minted Steward membership for the Person steward is pending sponsorship
    When that steward approves widening a finding on the network
    Then the approval is refused as pending sponsorship
    When the second peer's steward counter-signs the membership
    Then the same approval is accepted

  # Once crossed, identities must be resolvable by standard means. DID resolution proves the
  # crossing produced records a stranger can verify, which is the "verify" half of the promise.
  Scenario: A crossed agent identity resolves as a DID document controlled by its steward
    Given a contribution on the network authored by the ElohimAgent contributor's identity
    When that identity's DID is resolved through the DID bridge
    Then the document's subject is the agent identity
    And the document's controllers include the Person steward's identity

  Scenario: An ended session resolves as deactivated while its identity persists
    Given a contribution on the network whose provenance is a session of the ElohimAgent contributor
    When that session ends
    And the session's DID is resolved through the DID bridge
    Then the document resolves as deactivated
    And the contributor's identity DID still resolves with its steward as controller

  Scenario: Contribution share is reported but nothing settles before an agreement exists
    Given recorded work on the network by the Person steward and the ElohimAgent contributor, both members of the collective
    When the collective receives a value event of 1000 units, such as a grant
    Then a contribution-share report names each member's computed share
    And no economic event moves value to any member
    And the report states that settlement awaits a collaboration agreement on the network
