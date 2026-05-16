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

  # ============================================================================
  # Federation @wip status (post-2026-05-16 walk)
  #
  # Per 2026-05-16-epr-wip-disposition.md (HEAD 487fd5b0b):
  # - 0 of 8 @wip scenarios were lifted in this sprint
  # - 7 routed to doorway-full-facilitator sprint (rows 5–11) — gate is the
  #   federation-epr.steps.ts file, which does not yet exist anywhere in
  #   genesis/a2o/steps/
  # - 1 routed to iroh-phase-12-followon (row 12) — additionally gated on
  #   end-to-end caller-identity through the EPR-atom codec
  #
  # Substrate is confirmed live for all reach / policy / attestation gating
  # at elohim/elohim-storage/src/epr_service.rs:86–189 (handle_resolve).
  # The gap is the BDD glue layer, NOT the protocol gating.
  #
  # Backlog (must-route, not optional): the 5 "verified landed" scenarios
  # above this block (lines 26–61) are running undefined-silently — they
  # have no step-defs in genesis/a2o/steps/ either. Route them to the
  # doorway-full-facilitator sprint as a side effect of authoring
  # federation-epr.steps.ts (the same step-def file that satisfies rows 5–11
  # will cover the 5 above-block scenarios as well).
  #
  # Per-scenario gate conditions are inline above each @wip below.
  # ============================================================================

  # @wip retained: federation step-def layer (federation-epr.steps.ts) not yet built; reach-gate substrate is live.
  # Backlog destination: doorway-full-facilitator sprint
  # Citation: 2026-05-16-epr-wip-disposition.md row 5
  # Gate condition: lifts when federation-epr.steps.ts ships with `peer "X" has content "Y" with reach "<level>"`, `human "Z" is a consented member of collective "W"`, and `human "Z" requests content "Y" from peer "X"` step verbs. The community-reach gate itself is enforced at elohim/elohim-storage/src/epr_service.rs:86–189 — substrate is ready; only the BDD glue is missing.
  @wip
  Scenario: Community-reach guide accessible only to consented collective members
    Given peer "alpha" has content "community-governance-guide" with reach "community"
    And human "Matthew" is a consented member of collective "local-church"
    And human "Frank" has no collective memberships
    When human "Matthew" requests content "community-governance-guide" from peer "alpha"
    Then the content is served successfully
    When human "Frank" requests content "community-governance-guide" from peer "alpha"
    Then the response is 403 with reason "No consented collective membership"

  # @wip retained: federation step-def layer + standing-relationship fixture builder not yet built; trusted-reach substrate is live.
  # Backlog destination: doorway-full-facilitator sprint
  # Citation: 2026-05-16-epr-wip-disposition.md row 6
  # Gate condition: lifts when federation-epr.steps.ts ships with `human "X" is a steward of "Y"`, `human "Z" has a "<reach-level>" relationship with human "X"` step verbs and the relationship fixture builder. Trusted-reach gate enforcement is live at epr_service.rs:86–189; substrate is ready.
  @wip
  Scenario: Trusted-reach content requires standing relationship with steward
    Given peer "alpha" has content "advanced-theology" with reach "trusted"
    And human "Pete" is a steward of "advanced-theology"
    And human "Matthew" has a "trusted" relationship with human "Pete"
    And human "Frank" has no relationships with any steward of "advanced-theology"
    When human "Matthew" requests content "advanced-theology" from peer "alpha"
    Then the content is served successfully
    When human "Frank" requests content "advanced-theology" from peer "alpha"
    Then the response is 403 with reason "No trusted relationship with content steward"

  # @wip retained: federation step-def layer + prerequisite-attestation fixture not yet built; attestation gate is live in substrate.
  # Backlog destination: doorway-full-facilitator sprint
  # Citation: 2026-05-16-epr-wip-disposition.md row 7
  # Gate condition: lifts when federation-epr.steps.ts ships with `content "X" requires prerequisite mastery of "Y"`, `human "Z" has mastery of "Y"`, and `human "Z" requests the body of content "X"` step verbs and the mastery-attestation fixture builder. The attestation gate is enforced at epr_service.rs:133–177; substrate is ready.
  @wip
  Scenario: Attestation-gated content requires prerequisite mastery
    Given peer "alpha" has content "calculus-201" with reach "public"
    And content "calculus-201" requires prerequisite mastery of "calculus-101"
    And human "Matthew" has mastery of "calculus-101"
    And human "Terrance" does not have mastery of "calculus-101"
    When human "Matthew" requests the body of content "calculus-201"
    Then the content body is served successfully
    When human "Terrance" requests the body of content "calculus-201"
    Then the response is 403 with reason "Prerequisite mastery required"

  # @wip retained: cross-peer recognition-event tracking step-defs + steward-share fixture builder not yet built.
  # Backlog destination: doorway-full-facilitator sprint
  # Citation: 2026-05-16-epr-wip-disposition.md row 8
  # Gate condition: lifts when federation-epr.steps.ts ships with `peer "X" has content "Y" stewarded by "A" at N% and "B" at M%`, `peer "X" resolves "Y" via P2P from peer "Z"`, and `recognition events are created for steward "A" and steward "B"` step verbs. Recognition emission on P2P delivery is the substrate behaviour the test will assert; verify against epr_service path.
  @wip
  Scenario: Recognition distributes proportionally to stewards on P2P delivery
    Given peer "alpha" has content "economics-primer" stewarded by "Pete" at 60% and "Terrance" at 40%
    When peer "staging" resolves "economics-primer" via P2P from peer "alpha"
    Then recognition events are created for steward "Pete" and steward "Terrance"
    And steward "Pete" receives approximately 60% of the recognition
    And steward "Terrance" receives approximately 40% of the recognition

  # @wip retained: federation step-def layer + device-policy fixture not yet built; policy-ceiling enforcement is live in substrate.
  # Backlog destination: doorway-full-facilitator sprint
  # Citation: 2026-05-16-epr-wip-disposition.md row 9
  # Gate condition: lifts when federation-epr.steps.ts ships with `human "X" has a device policy with reach_level_max of N` and the device-policy fixture builder. The policy-ceiling check is enforced at epr_service.rs:109–131; substrate is ready.
  @wip
  Scenario: Policy ceiling blocks content above the device's reach level max
    Given peer "alpha" has content "intimate-journal" with reach "intimate"
    And human "Terrance" has a device policy with reach_level_max of 3
    When human "Terrance" requests content "intimate-journal" from peer "alpha"
    Then the response is 403 with reason matching "Reach level .* exceeds maximum"

  # --- Wave 3 additions: cross-peer human moments the substrate enables ---

  # @wip retained: real-time recognition-feed step-defs + cross-peer fetch tracking not yet built.
  # Backlog destination: doorway-full-facilitator sprint
  # Citation: 2026-05-16-epr-wip-disposition.md row 10
  # Gate condition: lifts when federation-epr.steps.ts ships with `Pete views his recognition feed on peer "X"`, `he sees a recognition event for "Y" delivered to peer "Z"`, and the cross-peer fetch event tracking helper. Substrate emits recognition events on delivery; the BDD layer needs to surface them.
  @wip
  Scenario: Steward sees recognition land for content delivered cross-peer
    Given Pete stewards content "ecology-primer" on peer "shem-pete"
    And Jessica's peer "household-jessica" fetches "ecology-primer" via P2P
    When Pete views his recognition feed on peer "shem-pete"
    Then he sees a recognition event for "ecology-primer" delivered to peer "household-jessica"
    And the recognition timestamp matches when peer "household-jessica" fetched the content

  # @wip retained: cross-peer disconnect simulator + multi-steward failover step-defs not yet built.
  # Backlog destination: doorway-full-facilitator sprint
  # Citation: 2026-05-16-epr-wip-disposition.md row 11
  # Gate condition: lifts when federation-epr.steps.ts ships with `peer "X" has content "Y" with multiple stewards`, `peer "X" disconnects mid-fetch before delivering the body`, and `the resolver attempts a different peer that holds the CID` step verbs, plus the disconnect simulator helper. The multi-steward failover behaviour belongs to doorway/storage; verify against the existing P2P fetch retry logic before authoring.
  @wip
  Scenario: Cross-peer fetch surfaces transient peer-offline as a soft state
    Given peer "alpha" has content "module-X" with multiple stewards "Pete", "Jessica"
    And Terrance's peer "household-terrance" begins resolving "module-X"
    When peer "alpha" disconnects mid-fetch before delivering the body
    Then the renderer surfaces a "fetching from another steward" indicator
    And the resolver attempts a different peer that holds the CID
    And the body eventually delivers without Terrance seeing an error page

  # @wip retained: PeerIdentityMap-mediated identity-bound cross-peer fetch step-defs not yet built; AgentPeerBinding substrate is live.
  # Backlog destination: iroh-phase-12-followon
  # Citation: 2026-05-16-epr-wip-disposition.md row 12
  # Gate condition: lifts when (a) iroh Phase 12 caller-identity is fully wired through the EPR-atom request-response codec end-to-end, and (b) federation-epr.steps.ts ships with `Matthew has an active AgentPeerBinding on peer "X"`, `peer "X" fetches "Y" via the EPR-atom protocol`, and `peer "Z" resolves the requesting peer to agent "A" via PeerIdentityMap` step verbs. The reach-gate accepting Matthew-as-steward is already substrate-live; the gap is the libp2p caller-identity threading + the BDD glue around it.
  @wip
  Scenario: Identity binding allows cross-peer fetches to attribute reach correctly
    Given Matthew has an active AgentPeerBinding on peer "household-matthew-desktop"
    And content "private-journal" with reach "intimate" is on peer "shem-pete" stewarded by Matthew
    When peer "household-matthew-desktop" fetches "private-journal" via the EPR-atom protocol
    Then peer "shem-pete" resolves the requesting peer to agent "agent-matthew" via PeerIdentityMap
    And the reach gate accepts the fetch because Matthew is the steward
    And the content body is served
