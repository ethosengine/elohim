@e2e @dataplane @concern:content-sync
Feature: Content-sync plane convergence
  The Automerge CRDT content-sync plane produces sync documents on a storage node and
  gossips them to peer nodes via libp2p. The producer was lit 2026-06-27: the EventBus
  content-projection path (EventBus → project_content_doc → apply_changes) writes each
  authored node as an Automerge document under hAppId="elohim", and the libp2p sync engine
  replicates it across peers. These scenarios confirm the plane is live and that a known
  document has converged to identical presence on two alpha peers.

  The author-then-converge variant is @wip until the dataplane test runner receives an
  explicit operator write grant for POST /db/content (see a2o/CLAUDE.md "Authorized writes
  on shared alpha"). The live scenarios assert on already-present node:e2e-* docs confirmed
  on alpha on 2026-06-29 (total: 14 documents).

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: The sync plane has produced at least one document on alpha-A
    # Confirms the content-projection producer is running and the sync table is non-empty.
    # A total >= 1 means at least one content node has been authored and projected into
    # an Automerge document — the minimum evidence that the sync plane is live.
    Then /sync/v1/elohim/docs list on peer "alpha-A" has at least 1 document

  Scenario: A known content node has non-empty sync heads on alpha-A
    # Locks in a specific node:e2e-* doc confirmed present on alpha-A on 2026-06-29.
    # Non-empty heads prove the Automerge document has at least one committed change.
    Then /sync doc "node:e2e-45cef93f-339c-4297-9e40-fc6669f27757" is present on peer "alpha-A"

  Scenario: The same content doc has converged to non-empty heads on elohim.host
    # Cross-peer convergence evidence: elohim.host is the alpha-b federation peer sharing
    # the same P2P network. If the same docId has non-empty heads on both peers, the
    # libp2p gossip engine has replicated the Automerge document across the boundary.
    Then /sync doc "node:e2e-45cef93f-339c-4297-9e40-fc6669f27757" is present on peer "elohim.host"

  @wip
  Scenario: Author a content node and confirm it converges on a second peer within 30 s
    # Blocked: POST /db/content in API mode requires an explicit operator write grant.
    # See a2o/CLAUDE.md "Authorized writes on shared alpha". Until the dataplane runner
    # receives that grant, authoring-and-convergence is validated via the node:e2e-* corpus
    # already present on alpha (see the non-@wip scenarios above).
    #
    # When unblocked: create fixture content as Matthew, poll alpha-A /sync heads until
    # present, then poll elohim.host to confirm cross-peer propagation within 30 s.
    Given human "Matthew" is logged in on doorway "alpha" with device
    When Matthew creates content titled "Dataplane sync probe" with tags "e2e,sync"
    Then the content should be created successfully
    And /sync doc matching the created content id is present on peer "alpha-A" within 5 s
    And /sync doc matching the created content id is present on peer "elohim.host" within 30 s

  @wip @requires:shem
  Scenario: Cross-tenant content doc converges via shem relay within 60 s
    # Needs a shem relay peer bridging two tenant P2P islands. Holding until the shem
    # relay bridge is wired in the dataplane Wave 3 arc. The @requires:shem tag is a
    # cluster-state substrate gate; the scope reconciler will hold this scenario if shem
    # becomes unavailable. The @wip guard prevents it from failing the live suite now.
    Given human "Matthew" is logged in on doorway "alpha" with device
    And human "Pete" is logged in on doorway "shem" with device
    When Matthew creates content titled "Cross-tenant sync probe" with tags "e2e,shem"
    Then /sync doc matching the created content id is present on peer "shem" within 60 s
