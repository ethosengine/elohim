# Rung 1 of the upgrade-velocity debt snowball (operator-set 2026-08-31).
# Backlog arc: genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md
# Vehicle: POST /admin/coordinators/sync (per-peer) + scripts/ci/fleet-coordswap.sh
# (rolling driver). Proven on the local mesh 2026-08-31: upgrade → revert →
# upgrade, 3 peers, ~40s/peer/pass, conductor PIDs unchanged throughout.
@e2e @dataplane @concern:coordinator-hot-swap @requires:multi-node
Feature: Coordinator logic rolls across peers without anyone restarting

  A steward — the person responsible for a peer's deployed code — ships a
  fix to how content is served, not to what the community has witnessed. Integrity code (the constitution: what counts as valid) is
  untouched; only coordinator code (the ministry: how peers serve and query)
  changes. Coordinator fixes are the most frequent change in this system, so
  they must not cost a network-wide restart: every peer adopts the fix in
  place — same agent key, same cells, same shared data — no reboot, no
  re-key, no window where the network is degraded. Integrity changes are the
  one thing this path must never carry: they change the network's identity
  (its DNA hash) and belong to a separate, deliberate DNA-lineage migration
  where the network agrees on how it evolves.

  Two tools carry the rollout. The STATUS SWEEP asks every peer, without
  changing anything, whether the coordinator code it is running differs from
  the shipped bundle ("drift"). The ROLLING DRIVER applies the bundle to one
  peer at a time and re-checks that peer for zero drift before touching the
  next — so a bad bundle or a wedged peer halts the rollout instead of
  spreading.

  A peer does not only run its own app. It may also host people who reach the
  network through a gateway instead of running a peer of their own: each
  hosted person has their own installation of the app on that peer, with their
  own agent key, created on the day they joined. An app is made of several
  named parts (its roles: learning, identity, governance and so on), and each
  role carries its own coordinator code. A rollout that updates only the
  peer's own app leaves every hosted person on the code their app was created
  with, while the peer itself looks healthy. What they then write in the old
  format, peers on the fix refuse. So the fix has to reach every app on the
  peer, and the peer has to say which apps and which roles it reached.

  Background:
    Given a mesh whose conductor-hosted cells run the installed hApp bundle
    And a steward has rebuilt the bundle with a coordinator-only fix
    And the rebuilt bundle's DNA hash is IDENTICAL to the installed one
    And the rebuilt bundle's coordinator code differs from what peers run

  @wip @concern:coordinator-hot-swap
  Scenario: a coordinator fix reaches every peer while the network keeps serving
    Given the status sweep reports coordinator drift on every peer
    When the steward applies the bundle through the rolling driver
    And each peer's re-check confirms zero drift before the next peer is touched
    Then every peer serves a function that only the new coordinator code provides
    And no conductor process restarted during the rollout
    And every declared content version, stored file, and governance tally is byte-identical to before

  @wip @concern:coordinator-hot-swap
  Scenario: a peer that refuses mid-roll halts the rollout instead of spreading
    Given one peer's operator has configured that node to reject coordinator updates
    When the steward applies the bundle through the rolling driver
    Then the rollout stops at the refusing peer
    And the driver names which peers were updated and which were never touched
    And the untouched peers still run the old coordinator code

  @wip @concern:coordinator-hot-swap
  Scenario: an integrity-touching bundle is refused before any peer swaps
    Given a rebuilt bundle whose DNA hash DIFFERS from the installed one
    When the status sweep checks the bundle against a peer
    Then the peer reports that the bundle belongs to a different DNA lineage
    And no swap happens on any peer
    And the steward is pointed at the DNA-lineage migration path instead

  # Found 2026-10-04 on the alpha fleet: the rollout had covered one app per
  # peer, the peer's own.
  @wip @concern:coordinator-hot-swap
  Scenario: people hosted on a peer receive the fix along with the peer itself
    Given a peer that also hosts several people, each with their own app on that peer
    And every hosted person's app was created before the fix and runs the old coordinator code
    When the steward applies the bundle to that peer
    Then the peer reports the outcome for each app and each of its roles by name
    And every hosted person is served by a function that only the new coordinator code provides
    And what a hosted person writes after the rollout is accepted by peers on the fix
    And each hosted person keeps the same agent key and the same data as before
    And any app or role whose DNA hash differs from the bundle's is named in the report and left unchanged
