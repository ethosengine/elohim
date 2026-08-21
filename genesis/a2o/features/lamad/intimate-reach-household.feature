@e2e @lamad @intimate-reach
Feature: Intimate-reach content within a household
  EPR content whose reach is "intimate" belongs to a consented dyad - not to a
  household, not to a steward, not to the commons. Matthew and Jessica keep a
  love map at reach "intimate" (the canonical enum's third rung: private, self,
  intimate, trusted, familiar, community, public, commons; consent is the
  dual-flag mechanism on their relationship, not a reach value). James lives in
  the same household and stewards bytes for the couple's resiliency, but the
  couple scope excludes him: stewardship is storage availability, never read
  access.

  The serving gate enforcing this is live substrate on the P2P RESOLVE PATH
  ONLY: epr_service's check_reach_authorization (intimate branch: mutual,
  dual-consented relationship) is invoked solely from the libp2p/iroh
  transports via handle_resolve - the HTTP routes apply a coarser gate
  (GET /db/content/{id} refuses anonymous but serves ANY authenticated caller;
  /epr-head/{id} is provenance-only). Until HTTP-path reach enforcement lands
  (backlog: http-reach-enforcement-gap), the read/refuse scenarios below keep
  their wip tag - the step defs are real and bind, and fail honestly on this gap.
  At-rest blind custody - shards as amorphous, senseless bits under an
  encryption envelope with per-recipient key wrapping - is the deferred sprint
  named in
  genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md;
  the scenarios capturing it are story-first (@wip @envisioned) until that
  substrate lands.

  None of the runnable scenarios need shem: the household (matthew / jessica /
  james) is itself a 3-node cluster, so intra-household replication proves the
  intimate peer-level story at household scale. Adam's family enters only in
  the cross-household variant, which is correctly held behind @requires:shem.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @wip @act:i
  Scenario: Jessica reads the couple's love map; anonymous visitors cannot
    Given human "Matthew" is logged in on doorway "alpha" with device
    And human "Jessica" is logged in on doorway "alpha" with device
    And humans "Matthew" and "Jessica" have a mutual intimate relationship with dual consent
    And content "love-map-matthew-jessica" is stewarded by "Matthew" at reach "intimate"
    When "Jessica" requests content "love-map-matthew-jessica"
    Then the content is served to "Jessica"
    When an anonymous client GETs content "love-map-matthew-jessica"
    Then the 403 body requiredReach is "intimate"

  @wip @act:i
  Scenario: A household member outside the couple scope cannot read couple content
    Given human "James" is logged in on doorway "alpha" with device
    And humans "Matthew" and "Jessica" have a mutual intimate relationship with dual consent
    And content "love-map-matthew-jessica" is stewarded by "Matthew" at reach "intimate"
    And human "James" has no intimate relationship with "Matthew"
    When "James" requests content "love-map-matthew-jessica"
    Then the request by "James" is refused with status 403
    And the 403 body requiredReach is "intimate"

  @wip @requires:multi-node @act:i
  Scenario: James stewards the couple's resiliency bytes without read access
    Given human "James" is logged in on doorway "alpha" with device
    And content "love-map-matthew-jessica" is stewarded by "Matthew" at reach "intimate"
    And the content "love-map-matthew-jessica" is replicated for resiliency
    And the device of "James" stewards shards for "love-map-matthew-jessica"
    When "James" opens "/shefa/cluster" on his device
    Then the cluster totals show non-zero stewarded bytes for others
    And the resilience view for "love-map-matthew-jessica" lists the device of "James" as a shard location
    When "James" requests content "love-map-matthew-jessica"
    Then the request by "James" is refused with status 403

  @wip @browser-only @act:i
  Scenario: Matthew feels the intimate boundary on the content card
    # Story-first: no content-reach "intimate" badge exists in the app today -
    # the only reach badge (gate-artifact-card) speaks qahal ReachTier
    # vocabulary, a different axis. This scenario is the spec for the missing
    # surface: the reach boundary should be FELT at the point of reading.
    Given human "Matthew" is logged in on doorway "alpha" with device
    When "Matthew" opens the content viewer for "love-map-matthew-jessica"
    Then a reach badge shows scope "intimate"
    And the badge names the consented audience "Jessica"

  @wip @browser-only @act:i
  Scenario: James's steward view shows opaque stewarded bytes, never content
    # Story-first: asserts opacity THROUGH the steward's own surfaces - the
    # peer-topology drill-down shows counts and bytes, and nothing anywhere
    # renders the couple's words to the steward.
    Given human "James" is logged in on doorway "alpha" with device
    And the device of "James" stewards shards for "love-map-matthew-jessica"
    When "James" opens "/shefa/peers" on his device
    Then the hosted-for-others detail shows a count of stewarded items
    And no title or body of "love-map-matthew-jessica" is rendered anywhere
    When "James" opens the content viewer for "love-map-matthew-jessica"
    Then the viewer shows a locked state naming the required reach "intimate"

  @wip @envisioned @requires:multi-node @act:i
  Scenario: Replicated shards are encrypted - senseless bits in the steward's pantry
    # Story-first capture for substrate that does NOT exist yet: RS(4,3) shards
    # are PLAINTEXT today (sharding.rs is erasure coding only - zero
    # encrypt/seal/key-wrap code). The encryption envelope + per-recipient key
    # wrapping is the named follow-up sprint in
    # genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md.
    # When it lands, the natural key seam is the SAME relationship substrate the
    # serve-gate already reads: per-reach content keys wrapped to the consented
    # dyad's imagodei pubkeys (the sealed_against_self wrap pattern, scoped to
    # reach instead of recovery quorum). Until then this scenario must never
    # ride green off the serve-gate's refusal - a serve refusal is not at-rest
    # confidentiality.
    Given the content "love-map-matthew-jessica" is replicated for resiliency
    And the device of "James" stewards shards for "love-map-matthew-jessica"
    When the shard bytes held on the device of "James" are inspected
    Then they are ciphertext with no recoverable fragment of the love map
    And only the keys of "Matthew" and "Jessica" can unwrap the content key

  @wip @envisioned @requires:shem @act:iii
  Scenario: Adam's family stewards the couple's shards across households
    # The cross-household variant of the same story: replication to another
    # FAMILY's stewarded compute. Held behind shem (remote multi-tenant canvas);
    # the intra-household James scenarios above prove the same boundary at
    # household scale without it. Requires the same encryption envelope as the
    # scenario above PLUS cross-household shard placement.
    Given the content "love-map-matthew-jessica" is replicated for resiliency across households
    And a device of the household of "Adam" stewards shards for it
    When the shard bytes held by the household of "Adam" are inspected
    Then they are ciphertext with no recoverable fragment of the love map
    And "Adam" requesting the content is refused with requiredReach "intimate"
