# Reach is the audience an Elohim Protocol Resource (EPR) has earned. A storage peer may hold scoped bytes
# and still refuse them to a caller whose transport key proves no standing.
# Each scenario compares that caller's answer for a present scoped identifier
# with its answer for a same-shaped identifier that does not exist. The answer
# must be indistinguishable, so the refusal reveals neither bytes nor existence.
#
# The background proves the fixture bytes and a derivable head exist on the
# exact peer the stranger dials. Each byte-serving scenario also exercises its
# own authorized path, so a green comparison cannot hide a broken plane.
@e2e @dataplane @concern:reach-enforced-p2p @act:i @wip
Feature: Listed P2P protocol fields reveal no more for a scoped subject than for an unknown one
  Reach is the audience an Elohim Protocol Resource has earned. The exact-peer
  fixture control proves that James holds the real scoped bytes and can derive
  their head, so a negative answer cannot pass merely because the fixture is
  absent from the peer being probed.

  A head is the signed address of the resource version a peer can serve. James
  derives the scoped head locally, but reach forbids publishing it into the
  public Kademlia routing plane. An open head on that plane proves the lookup
  works while the scoped EPR identifier correctly resolves like an unknown one.

  Matthew authored and stewards the fixture for an audience that does not
  include this stranger. The fixture's local agent represents Matthew on the
  storage peer. Preserving Matthew's audience choice protects both the resource
  and its existence within the protocol fields measured here.

  A stranger is a probe that creates a fresh transport key for this scenario.
  The key is absent from the steward's roster and from every identity binding.
  Request fields that merely name a known agent cannot turn that transport
  stranger into the named person.

  Most planes scope an EPR identifier or content address. On view federation,
  the scoped subject is Matthew's local agent identifier: its Cluster view is
  the set of storage peers and roles visible to that agent. It is available to
  Matthew's transport binding but not to a stranger who merely writes that
  identifier into a request.

  Two answers are indistinguishable when their protocol result kind, refusal or
  status code, payload bytes, and address or provider metadata match. Timing and
  packet-length side channels are not measured by these scenarios. Shape-matched
  controls use the same protocol namespace, identifier codec, and request form as
  their scoped counterpart. The probe constructs them per plane: valid EPR
  identifiers for resolve and Kademlia, content hashes for blob, shard, and atom,
  document identifiers for sync, and agent identifiers for Cluster views.

  Each byte-serving scenario places its authorized positive control beside the
  stranger comparison. Every named path is exercised separately, and failure of
  any control fails that scenario rather than being hidden in an aggregate setup.
  An open fixture has no audience restriction, so the stranger may retrieve it.

  Sync is the exception to the positive retrieval paths: audience narrowing
  removes the wider derived document from James's sync store while the canonical
  fixture bytes and derivable head remain on that same peer. Rejecting stale
  re-ingress must leave that projection absent rather than recreate it.

  Background:
    Given household storage peer "james" holds scoped reach fixture "reach-witness-scoped"
    And peer "james" serves the fixture bytes and derives its head for the fixture's steward
    And a reach probe with a fresh ephemeral transport key dials peer "james"
    And the probe key is absent from every household roster and identity binding
    And every unknown control is shape-matched, syntactically valid, and absent from peer "james" and its public lookup planes
    And the probe compares result kind, refusal or status code, payload bytes, and address or provider metadata

  @regression
  Scenario: EPR resolve hides a scoped head from a transport stranger
    Given Matthew retrieves the scoped fixture from peer "james" over single EPR resolve
    When the stranger resolves the scoped fixture and an unknown EPR over the EPR resolve protocol
    Then the EPR resolve answers are indistinguishable
    And neither answer contains the scoped fixture's head or bytes

  @regression
  Scenario: EPR batch resolve hides the scoped slot without rejecting eligible slots
    Given Matthew retrieves the scoped fixture from peer "james" over batch EPR resolve
    And peer "james" holds open reach fixture "reach-witness-open" with no audience restriction
    When the stranger batch-resolves the scoped fixture, an unknown EPR, and the open fixture
    Then the batch slots for the scoped and unknown identifiers are indistinguishable
    And the batch contains no head or bytes for the scoped fixture
    But the batch returns the open fixture's head

  @regression
  Scenario: The libp2p blob protocol hides a scoped blob from a transport stranger
    Given Matthew retrieves the scoped fixture's blob from peer "james" over the libp2p blob protocol
    When the stranger fetches the scoped fixture's blob and an unknown blob over the libp2p blob protocol
    Then the libp2p blob answers are indistinguishable
    And neither answer contains scoped fixture bytes

  @regression
  Scenario: The iroh blobs protocol hides a scoped blob from a transport stranger
    Given Matthew retrieves the scoped fixture's blob from peer "james" with iroh blobs get
    And Matthew retrieves the scoped fixture's blob from peer "james" with iroh blobs get-many
    When the stranger fetches the scoped fixture's blob and an unknown blob with iroh blobs get and get-many
    Then the iroh get answer for the scoped blob is indistinguishable from its get control
    And the iroh get-many answer for the scoped blob is indistinguishable from its get-many control
    And no iroh blobs answer reveals scoped fixture bytes or existence

  @regression
  Scenario: The shard protocol hides scoped manifests and bytes from a transport stranger
    Given Matthew receives the scoped fixture's Have answer from peer "james"
    And Matthew receives the scoped fixture's GetManifest answer from peer "james"
    And Matthew receives the scoped fixture's GetContent answer from peer "james"
    When the stranger sends Have, GetManifest, and GetContent for the scoped fixture and matching unknown controls
    Then every shard answer for the scoped fixture is indistinguishable from its unknown-fixture control
    And no shard answer reveals the scoped manifest, availability, or bytes

  @regression
  Scenario: A stale wider sync document cannot undo Matthew's audience narrowing
    Given peer "jessica" retained the fixture's wider sync document from before Matthew narrowed its audience
    When peer "jessica" attempts to project that stale document back to peer "james"
    Then peer "james" rejects that stale projection
    And the excluded sync projection remains absent from peer "james" after stale re-ingress

  @regression
  Scenario: Sync hides an audience-narrowed document from a transport stranger
    Given the excluded sync projection is absent from peer "james" after audience narrowing and stale re-ingress rejection
    When the stranger synchronizes the scoped document and an unknown document from peer "james"
    Then the sync answers for the scoped and unknown documents are indistinguishable

  @regression
  Scenario: The EPR atom protocol hides a scoped atom from a transport stranger
    Given Matthew retrieves the scoped fixture's atom from peer "james" over the EPR atom protocol
    When the stranger fetches the scoped fixture's atom and an unknown atom over the EPR atom protocol
    Then the EPR atom answers are indistinguishable
    And neither answer contains the scoped atom

  @regression
  Scenario: View federation does not trust an agent identifier named by a stranger
    Given Matthew's transport binding receives a nonempty Cluster view for the fixture's local agent
    When the stranger requests a Cluster view naming the fixture's local agent and an unknown agent
    Then the Cluster answers for the local and unknown agents are indistinguishable
    And neither answer reveals the local cluster or peer topology

  @regression
  Scenario: A public Kademlia lookup finds no scoped head
    Given the same stranger retrieves an open head from the Kademlia routing plane being probed
    When the stranger looks up the scoped fixture's EPR identifier and a shape-matched unknown EPR identifier on the public Kademlia plane
    Then the Kademlia answers are indistinguishable
    And neither public lookup contains or locates the scoped fixture head
