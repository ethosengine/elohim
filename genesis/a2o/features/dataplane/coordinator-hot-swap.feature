# Rung 1 of the upgrade-velocity debt snowball (operator-set 2026-08-31):
# a coordinator-zome change is the highest-frequency change class in dataplane
# work, and it MUST NOT cost a fleet roll. The conductor's update_coordinators
# hot-swap preserves the agent key, the cells, and all DHT state — the vehicle
# here (storage admin endpoint + rolling driver) makes that reachable per-peer
# over HTTP, k8s-rollout style, on the local mesh and the fleet alike.
# Backlog arc: genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md
@e2e @dataplane @concern:coordinator-hot-swap @requires:multi-node
Feature: Coordinator zomes roll across peers without anyone restarting

  A steward ships a fix to how content is served — not to what the community
  has witnessed. Integrity (the constitution) is untouched; only coordinator
  logic (the ministry) changes. Every peer adopts the fix in place: same
  agent key, same cells, same DHT — no reboot, no re-key, no churn window.

  Background:
    Given a mesh of peers whose conductors run the installed hApp bundle
    And a rebuilt hApp bundle whose DNA hash is IDENTICAL to the installed one
    And the rebuilt bundle's coordinator wasm differs from what conductors run

  @wip @concern:coordinator-hot-swap
  Scenario: a coordinator fix reaches every peer while the network keeps serving
    Given the status sweep reports coordinator drift on every peer
    When the rolling driver applies the bundle peer by peer
    And each peer's re-check confirms zero drift before the next peer is touched
    Then every conductor answers the new coordinator extern
    And no conductor process restarted during the rollout
    And declared heads, blobs, and elections are byte-identical to before

  @wip @concern:coordinator-hot-swap
  Scenario: an integrity-touching bundle is refused by the vehicle
    Given a rebuilt bundle whose DNA hash DIFFERS from the installed one
    When the rolling driver dry-runs the bundle against a peer
    Then the peer reports the lineage mismatch instead of swapping
    And the operator is pointed at the DNA-lineage path, not the hot-swap path
