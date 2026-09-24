@wip @federation @serve-routing @act:iii
Feature: Serve-routing prefers the lower-latency capable peer cross-WAN
  As a learner anywhere in the mesh
  I want blob fetches to be routed to the peer with the lowest measured round-trip time
  So that content loads faster when multiple capable peers hold the same blob
  and the protocol rewards peers that invest in low-latency infrastructure.

  # Implementation anchor: Wave-3 doorway-membrane serve-routing
  # (genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md §3 D1).
  # `select_serve_peers` consumes `elohim_peer_fabric::score::rank` which weights
  # attested_rtt_ms at 0.3 (rtt_factor = 1/(1 + ms/100)).
  # As of Sprint 3 (story 3.2), attested_rtt_ms is fed from elohim-storage's own
  # `p2p::transport_paths` EWMA (elohim/elohim-storage/src/services/serve_routing.rs) —
  # a locally-observed, UNSIGNED round-trip measurement (this node's own view of
  # its RTT to a peer), not a peer-signed HealthAttestation. On co-located
  # household nodes every peer's measured RTT is near-zero and effectively
  # equal, so this assertion is still vacuous on household topology — it MUST
  # be tested cross-WAN where latency genuinely discriminates candidates (e.g.
  # local vs remote shem peer). No step definitions exist for these two
  # scenarios yet, and no household-runnable latency-injection fixture exists
  # (no tc-netem harness, no test-only RTT override hook) — building one is
  # out of scope for the RTT-feed slice that landed this sprint. Household
  # coverage (RTT changes ordering; absent RTT never demotes below a known-slow
  # peer; ordering is stable without RTT) is provided by the unit tests in
  # services/serve_routing.rs and the integration tests in
  # tests/api/serve_routing.rs (transport_paths::global() is seeded directly).
  #
  # @requires:shem: remote multi-tenant canvas with geographically diverse peers
  # is required to produce a measurable RTT spread (>50ms delta) between candidates.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the serve-routing feature is active (Wave-3 deployed)

  @wip @requires:shem
  Scenario: Serve-routing picks the lower-RTT peer when both hold the same blob
    # Two capable peers hold the same content blob, but peer A is geographically
    # closer (lower locally-measured RTT) than peer B. The serving node should
    # prefer A.
    Given peer "near-peer" holds blob "<blob_hash>" with measured RTT of "20ms"
    And peer "far-peer" holds blob "<blob_hash>" with measured RTT of "250ms"
    And both peers have equal capability level and headroom
    And both peers are bonded via active replicates-content commitments
    When storage requests the blob via serve-routing with n=1
    Then the selected candidate is "near-peer"
    And the selection reason includes "rtt_factor" in the score log

  @wip @requires:shem
  Scenario: Score degrades gracefully when one peer has no measured RTT
    # A not-yet-sampled peer still participates in serve-routing with a neutral
    # RTT factor (0.5) — it is never excluded for lack of a measurement.
    Given peer "measured-peer" holds blob "<blob_hash>" with measured RTT of "30ms"
    And peer "new-peer" holds blob "<blob_hash>" with no measured RTT recorded
    And both peers have equal capability level and headroom and bonded status
    When storage requests the blob via serve-routing with n=2
    Then both "measured-peer" and "new-peer" are in the selected candidates
    And "measured-peer" ranks above "new-peer" in the score ordering
    And no 503 is emitted (caller does not shed on a partial-signal peer)
