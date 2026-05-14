@e2e @federation @epr
Feature: EPR Cross-Peer Content Resolution
  As a learner navigating a path
  I want content stewarded by another peer to resolve transparently
  So that learning paths work regardless of stewardship partitioning

  When Matthew authors a guide and Pete on a different peer stewards it,
  Terrance reading the guide on yet a third peer should never need to know
  the topology. The protocol resolves across peers, the body fetches
  across peers, and recognition flows back to the stewards who carry the
  load — all transparent to the reader.

  # Why this matters:
  # The web's CDN model assumes one canonical origin and many caches.
  # The protocol's federation assumes no canonical origin — content lives
  # wherever stewardship has earned its custody. Cross-peer resolution
  # has to feel like local resolution. If a learner notices the seams,
  # the architecture has failed the human-scale promise.

  Background:
    Given the EPR protocol "/elohim/epr/1.0.0" is active between peers
    And the shard protocol "/elohim/shard/1.0.0" is active between peers

  # --- Foundational P2P resolution (verified landed; @wip lifted by Wave 0 audit) ---

  Scenario: Content stewarded by another peer resolves with full body
    # Pete stewards a curriculum module on peer "alpha". When peer
    # "staging" needs it for a learner, the protocol resolves Pete's
    # EPR Head via DHT, fetches the body via the shard plane, and
    # caches it locally — all on the first request, no operator setup.
    Given peer "alpha" has content "fct-module-01-church-dilemma" stewarded by Pete
    And peer "staging" does not have "fct-module-01-church-dilemma" locally
    When peer "staging" requests content "fct-module-01-church-dilemma"
    Then the content is resolved via EPR protocol from peer "alpha"
    And the content body is fetched via shard protocol
    And the content is persisted to local SQLite on peer "staging"
    And subsequent requests return the content without P2P resolution

  Scenario: EPR Heads publish to DHT on ingestion
    # Authoring is publishing. The moment Matthew creates content on his
    # household peer, the EPR Head is anchored on the DHT — making it
    # discoverable by any other peer that has standing to fetch it.
    Given peer "alpha" ingests content "test-concept"
    Then the DHT contains an EPR Head for "test-concept"
    And peer "staging" can discover "test-concept" via Kademlia lookup

  Scenario: Content GET returns 404 when no peer has the content
    # No silent fallthroughs. If nobody has it, the reader gets a clear
    # 404 — not an empty page, not an infinite spinner. Honest absence.
    Given no peer has content "nonexistent-concept"
    When peer "alpha" requests content "nonexistent-concept"
    Then the response is 404 Not Found

  Scenario: Single content create publishes EPR Head
    # The smallest unit of authoring — one POST — carries the full
    # publish flow: notarize, anchor, gossip the head. No batching,
    # no manual sync step.
    Given peer "alpha" creates content "new-concept" via POST /db/content
    Then the DHT contains an EPR Head for "new-concept"

  Scenario: P2P-resolved content is tagged for diagnostics
    # When content arrives via cross-peer resolution rather than local
    # storage, the local record carries that provenance. Operators can
    # see federation flows without it leaking into the user-facing UI.
    Given peer "staging" resolves "cross-steward-concept" via P2P
    Then the local content record has metadata "resolved_via" = "p2p"

  # --- Reach-gated access (the ones the audit confirmed have substrate but step defs may be wip) ---

  @wip
  Scenario: Community-reach guide accessible only to consented collective members
    # Matthew's collective ("local-church") authored a governance guide
    # at community reach. Matthew is a consented member — the guide
    # opens for him with the collective's authorship attestation visible.
    # Frank is outside the collective; for him, the same URL surfaces a
    # respectful "this content belongs to local-church, here's how to
    # ask for membership" page rather than a hard 403 page.
    Given peer "alpha" has content "community-governance-guide" with reach "community"
    And human "Matthew" is a consented member of collective "local-church"
    And human "Frank" has no collective memberships
    When human "Matthew" requests content "community-governance-guide" from peer "alpha"
    Then the content is served successfully
    When human "Frank" requests content "community-governance-guide" from peer "alpha"
    Then the response is 403 with reason "No consented collective membership"

  @wip
  Scenario: Trusted-reach content requires standing relationship with steward
    # Some content is stewarded for a circle of trust — not for everyone
    # in a collective, but for those the steward has explicitly extended
    # trust to. The reach gate enforces that relationship structurally.
    Given peer "alpha" has content "advanced-theology" with reach "trusted"
    And human "Pete" is a steward of "advanced-theology"
    And human "Matthew" has a "trusted" relationship with human "Pete"
    And human "Frank" has no relationships with any steward of "advanced-theology"
    When human "Matthew" requests content "advanced-theology" from peer "alpha"
    Then the content is served successfully
    When human "Frank" requests content "advanced-theology" from peer "alpha"
    Then the response is 403 with reason "No trusted relationship with content steward"

  @wip
  Scenario: Attestation-gated content requires prerequisite mastery
    # Calculus 201 only opens for learners who have demonstrated mastery
    # of Calculus 101. The attestation is on the DHT (notarized);
    # checking it is a graph traversal, not a database query at the
    # publisher. Terrance sees a respectful "build prerequisites first"
    # affordance pointing to the prerequisite path.
    Given peer "alpha" has content "calculus-201" with reach "public"
    And content "calculus-201" requires prerequisite mastery of "calculus-101"
    And human "Matthew" has mastery of "calculus-101"
    And human "Terrance" does not have mastery of "calculus-101"
    When human "Matthew" requests the body of content "calculus-201"
    Then the content body is served successfully
    When human "Terrance" requests the body of content "calculus-201"
    Then the response is 403 with reason "Prerequisite mastery required"

  @wip
  Scenario: Recognition distributes proportionally to stewards on P2P delivery
    # Stewardship is real work and the protocol counts it. When peer
    # "staging" fetches Pete and Terrance's co-stewarded content, both
    # stewards receive recognition events proportional to their declared
    # share. This is shefa — value flowing where work was done.
    Given peer "alpha" has content "economics-primer" stewarded by "Pete" at 60% and "Terrance" at 40%
    When peer "staging" resolves "economics-primer" via P2P from peer "alpha"
    Then recognition events are created for steward "Pete" and steward "Terrance"
    And steward "Pete" receives approximately 60% of the recognition
    And steward "Terrance" receives approximately 40% of the recognition

  @wip
  Scenario: Policy ceiling blocks content above the device's reach level max
    # Terrance's device — a stewarded-child device — has a policy ceiling
    # on reach level. Even when Terrance holds the structural standing to
    # access "intimate" content, the device-side policy refuses, and the
    # refusal is local + visible — no awkward attempt-and-deny round-trip
    # to the server.
    Given peer "alpha" has content "intimate-journal" with reach "intimate"
    And human "Terrance" has a device policy with reach_level_max of 3
    When human "Terrance" requests content "intimate-journal" from peer "alpha"
    Then the response is 403 with reason matching "Reach level .* exceeds maximum"

  # --- Wave 3 additions: cross-peer human moments the substrate enables ---

  @wip
  Scenario: Steward sees recognition land for content delivered cross-peer
    # Pete is on his own peer when Jessica's peer fetches his guide for
    # a learner. Pete's recognition view should reflect the new event —
    # not days later as a batch, not on next sync, but as it happens.
    # Stewardship has to feel responsive or it doesn't compound.
    Given Pete stewards content "ecology-primer" on peer "shem-pete"
    And Jessica's peer "household-jessica" fetches "ecology-primer" via P2P
    When Pete views his recognition feed on peer "shem-pete"
    Then he sees a recognition event for "ecology-primer" delivered to peer "household-jessica"
    And the recognition timestamp matches when peer "household-jessica" fetched the content

  @wip
  Scenario: Cross-peer fetch surfaces transient peer-offline as a soft state
    # Mid-fetch, the source peer goes offline. The renderer should not
    # blank the page or show a stack trace; it shows a "fetching from
    # other stewards" affordance and tries another peer that holds the
    # CID. The reader sees latency, not failure.
    Given peer "alpha" has content "module-X" with multiple stewards "Pete", "Jessica"
    And Terrance's peer "household-terrance" begins resolving "module-X"
    When peer "alpha" disconnects mid-fetch before delivering the body
    Then the renderer surfaces a "fetching from another steward" indicator
    And the resolver attempts a different peer that holds the CID
    And the body eventually delivers without Terrance seeing an error page

  @wip
  Scenario: Identity binding allows cross-peer fetches to attribute reach correctly
    # Matthew's content with reach "trusted" is requested from peer
    # "shem-pete" — but the requester is Matthew himself on his desktop.
    # The cross-peer path needs to know "this fetch is on behalf of
    # Matthew, who has standing here," not just "this is some peer asking."
    # The identity binding (Phase 2B) is what makes that translation possible.
    Given Matthew has an active AgentPeerBinding on peer "household-matthew-desktop"
    And content "private-journal" with reach "intimate" is on peer "shem-pete" stewarded by Matthew
    When peer "household-matthew-desktop" fetches "private-journal" via the EPR-atom protocol
    Then peer "shem-pete" resolves the requesting peer to agent "agent-matthew" via PeerIdentityMap
    And the reach gate accepts the fetch because Matthew is the steward
    And the content body is served
