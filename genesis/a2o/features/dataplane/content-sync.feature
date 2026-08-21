@e2e @dataplane @concern:content-sync
Feature: Content-sync plane convergence
  When someone authors a content node on one peer, everyone reading from a different peer
  must see the same node — nobody should have to know which machine holds the "real" copy.
  The content-sync plane is what makes that true: each authored node becomes an Automerge
  CRDT document, and the libp2p sync engine gossips it to the other peers. If the plane
  stalls, two people on two peers read two different libraries and neither can tell.

  These scenarios ask the peers themselves whether that is holding right now: is the plane
  producing documents at all, does a real document carry committed changes, and does a
  second peer hold that same document at the same point in its change history.

  (Producer path, for the reader who needs to fix it: EventBus → project_content_doc →
  apply_changes writes each authored node under hAppId="elohim"; lit 2026-06-27.)

  The author-then-converge variant is @wip until the dataplane test runner receives an
  explicit operator write grant for POST /db/content (see a2o/CLAUDE.md "Authorized writes
  on shared alpha"). Until then the live scenarios ask a peer which documents it holds
  instead of naming one. A document id is scoped to the deployment that created it, so an
  id copied from one deployment simply does not exist in another — and "no such document"
  and "the producer is dead" answer the heads probe identically. Letting the peer name the
  document keeps the claim honest on whichever substrate the suite is pointed at.

  Background:
    # Both values are peer ALIASES resolved by resolvePeerUrl() (see
    # src/framework/dataplane/surfaces.ts), not hostnames: "alpha-A" → E2E_DOORWAY_ALPHA,
    # "elohim.host" → E2E_DOORWAY_B / E2E_DOORWAY_BETA. On the deployed fleet those are two
    # doorways in front of two storage peers; on a household mesh they are the two local
    # doorways. Either way the pair must share ONE P2P network, or convergence is untestable.
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: The sync plane has produced at least one document on alpha-A
    # Confirms the content-projection producer is running and the sync table is non-empty.
    # A total >= 1 means at least one content node has been authored and projected into
    # an Automerge document — the minimum evidence that the sync plane is live.
    Then /sync/v1/elohim/docs list on peer "alpha-A" has at least 1 document

  Scenario: A content node named by alpha-A has committed changes there
    # Diagnostic rung between "the table is non-empty" and "it reached the other peer":
    # this exercises the PER-DOCUMENT heads surface, so a failure here indicts the
    # producer on alpha-A and clears the gossip layer of suspicion. Non-empty heads mean
    # that Automerge document has at least one committed change, not merely a row.
    When a content sync doc is selected from peer "alpha-A"
    Then the selected sync doc is present on peer "alpha-A"

  Scenario: The same content doc has converged onto elohim.host
    # The claim the plane exists to keep: elohim.host holds the document alpha-A named,
    # at the SAME point in its change history — equal head sets, not merely two non-empty
    # ones. Two peers each sitting on a different head is a DIVERGED document, which is
    # the failure this scenario is here to catch; presence alone would pass it.
    When a content sync doc is selected from peer "alpha-A"
    Then the selected sync doc has converged onto peer "elohim.host"

  @wip
  Scenario: Author a content node and confirm it converges on a second peer within 30 s
    # Blocked: POST /db/content in API mode requires an explicit operator write grant.
    # See a2o/CLAUDE.md "Authorized writes on shared alpha". Until the dataplane runner
    # receives that grant, convergence is validated against documents the peers already
    # hold (see the non-@wip scenarios above) rather than one this suite authored.
    #
    # When unblocked: create fixture content as Matthew, poll alpha-A /sync heads until
    # present, then poll elohim.host to confirm cross-peer propagation within 30 s.
    #
    # Doorway "alpha" and peer "alpha-A" are the same deployment seen from two sides: a
    # human logs in at the DOORWAY (the gateway that fronts a storage peer), and the sync
    # surfaces are read from the PEER behind it. Both resolve from E2E_DOORWAY_ALPHA.
    Given human "Matthew" is logged in on doorway "alpha" with device
    When Matthew creates content titled "Dataplane sync probe" with tags "e2e,sync"
    Then the content should be created successfully
    And /sync doc matching the created content id is present on peer "alpha-A" within 5 s
    And /sync doc matching the created content id is present on peer "elohim.host" within 30 s

  @wip @requires:shem
  Scenario: Cross-tenant content doc converges via shem relay within 60 s
    # "shem" is the remote multi-tenant cluster — peers belonging to households OTHER than
    # the one alpha-A serves. Its P2P network is a separate island, so a document only
    # crosses when a relay peer bridges the two.
    #
    # Needs a shem relay peer bridging two tenant P2P islands. Holding until the shem
    # relay bridge is wired in the dataplane Wave 3 arc. The @requires:shem tag is a
    # cluster-state substrate gate; the scope reconciler will hold this scenario if shem
    # becomes unavailable. The @wip guard prevents it from failing the live suite now.
    Given human "Matthew" is logged in on doorway "alpha" with device
    And human "Pete" is logged in on doorway "shem" with device
    When Matthew creates content titled "Cross-tenant sync probe" with tags "e2e,shem"
    Then /sync doc matching the created content id is present on peer "shem" within 60 s
