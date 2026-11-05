@e2e @resilience @dataplane @act:i @wip @concern:addresses-are-declared-profiles @requires:household-nodes @requires:owned-substrate
Feature: A household holding respects safety and consent without rewriting its history
  Matthew shares a handbook through a channel, a named stream of releases.
  Jessica's peer keeps a copy for him. James reads it through that peer.

  A holding is Jessica's promise to Matthew to retain a particular copy for a
  bounded period. Because the promise is between the two of them, changing its
  retention period takes both of their agreement. Its proof is a signed,
  content-addressed record of what she promised and the evidence of that
  holding; the proof is separate from the handbook's payload bytes.

  Each channel elects a head: the one qualifying release its readers rely on
  as current. Adopting a release means a peer takes it as that current head.

  Four questions have separate answers: what lifecycle action was taken,
  whether a release currently qualifies for election, whether this requester
  may receive its bytes, and how long the peer should retain a copy.
  A channel's usual serving rule cannot override quarantine, refusal or
  withdrawal of the subject's consent.

  The household has appointed a safety steward. Only the safety steward may
  quarantine or refuse a release on a household peer; Matthew, as the subject, alone may
  withdraw his consent. Quarantine is a temporary safety hold that blocks
  adoption, serving and redistribution. Refuse is a terminal safety decision requiring local payload
  destruction. NOT_ELIGIBLE means a release does not currently qualify for
  election; it is not a refusal or an instruction to destroy its payload.

  A content address identifies particular bytes. Changing a retention
  obligation does not change those bytes or give them another address.

  These scenarios concern the named household peers and their observed
  decisions. They do not promise recall of copies elsewhere.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And peer "matthew" at "matthew"
    And peer "jessica" at "jessica"
    And peer "james" at "james"
    And Matthew has shared "Household handbook" with Jessica and James
    And peer "jessica" has an active holding of "Household handbook" with a verifiable proof
    And the channel's default permits serving that release to James
    And the holding's retention obligation has not expired

  Scenario: Quarantine overrides a channel that normally serves James
    Given James can fetch "Household handbook" from peer "jessica"
    When the household's safety steward quarantines that release on peer "jessica" until tomorrow
    And James requests the same release from peer "jessica"
    Then the response supplies no handbook payload bytes
    And the response names "QUARANTINED" and the quarantine's expiry
    And peer "jessica" reports adoption and redistribution blocked by that quarantine
    And the channel's serve default remains enabled
    And the existing retention obligation and holding proof remain recorded

  Scenario: Matthew's consent withdrawal retires the holding but preserves its proof
    Given Matthew is the subject whose consent authorizes this replication and serving
    And Jessica has recorded the content address of the holding proof
    When Matthew withdraws that consent for peer "jessica"
    And peer "jessica" processes the verified withdrawal
    And James requests "Household handbook" from peer "jessica"
    Then Jessica's holding is reported as "RETIRED" with reason "consent-withdrawn"
    And the response supplies no handbook payload bytes
    And peer "jessica" reports no replication or serving authority under the withdrawn consent
    And Jessica can retrieve and verify the original holding proof at its recorded content address
    And that proof is shown as historical evidence rather than an active custody promise
    And the channel's serve default has not restored the withdrawn authority

  Scenario: Refusing a release is distinguishable from finding it ineligible
    Given Jessica retains two earlier handbook releases called "Unsafe edition" and "Old edition"
    And the channel now qualifies only releases published within the last year
    And "Old edition" was published two years ago
    And no safety restriction or consent withdrawal applies to "Old edition"
    And its retention obligation and James's serving authorization remain valid
    When the household's safety steward refuses "Unsafe edition" on peer "jessica"
    And peer "jessica" evaluates "Old edition" for election
    Then "Unsafe edition" has named outcome "REFUSED" with its safety reason
    And peer "jessica" confirms the local payload of "Unsafe edition" has been destroyed
    And "Old edition" has named outcome "NOT_ELIGIBLE" with its failed qualification rule
    And peer "jessica" still retains "Old edition" under its existing obligation
    And James can still fetch "Old edition" under his current serving authorization
    And neither outcome elects a replacement head

  Scenario: Shortening retention changes the obligation rather than the content address
    Given Jessica has recorded the payload content address of "Household handbook"
    And its holding has 30 days of retention remaining
    And the release currently qualifies for election
    When Matthew and Jessica authorize a retention change to 7 days from now
    And peer "jessica" records that change
    Then the holding reports the newly agreed retention deadline
    And James can fetch byte-identical handbook content at the original payload content address
    And the release remains eligible and the holding remains active
    And James's serving authorization is unchanged
    And the channel's elected head is unchanged
