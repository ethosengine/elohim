@e2e @dataplane @wip @concern:operator-runtime-surface @requires:multi-node
Feature: Operator verbs are commitment-gated protocol acts, not cluster surgery
  RED-FIRST. These scenarios ARE the schedulable red for spine node
  `operator-runtime-surface`. They assert what "operating a peer" must come to mean, and
  pass unchanged the moment it does.

  WHY @wip IS ON THIS FILE (remove it deliberately, as an operator decision):
    The edge Dataplane Validation stage selects on
    `@dataplane and not @wip and not @browser-only` (scripts/ci/run-dataplane-validation.sh).
    Without @wip these 4 scenarios join the byConcern rollup immediately as a new red
    concern — which CHANGES WHAT A CI-GATED MEASURE COUNTS. That is an operator call under
    the blob-durability precedent recorded in genesis/manifests/spine.yaml, not something a
    session takes unilaterally. (The stage is advisory — catchError→UNSTABLE — so the cost
    is a red concern in the rollup, not a broken build. It is still the operator's call.)
    Run it locally meanwhile:
      pnpm exec cucumber-js --tags '@concern:operator-runtime-surface'
    Drop @wip when the operator wants this counted.

  WHAT THE CODE ALREADY TAUGHT US (do not lose this — it reshapes the work):
    This node is NOT unbuilt. The commitment-gated authorization primitive exists and is
    fail-closed today:
      elohim/elohim-storage/src/services/operation_authorization.rs  (authorize_operation —
        finds the active `delegates-compute` grant for (performer, capability) in
        mishpat_commitments, then runs the shared 7-check bounds_validator; any
        BoundsViolation is a fail-closed deny)
      elohim/elohim-storage/src/api/authorize_operation.rs           (the route)
    What is MISSING is not the gate. It is that the operator VERBS never consult it:
      POST /admin/ssr-bundle/refresh      (doorway/doorway-service/src/server/http.rs)
      POST /admin/steward-peers/refresh   (same)
    are plain doorway routes on the operator-seat class. And per
    `project_che_opgate_slice1_plan_ready_held`, every doorway deploy runs DEV_MODE=true,
    so even the built gate is not enforcing anywhere.

    So the cure is WIRING + a deploy posture, not construction. Anyone picking this up
    should read operation_authorization.rs FIRST and resist rebuilding what is there.

  THE CLASS THIS KILLS:
    (a) kubectl-only operation — `restart-doorway-epr.sh` and friends. An operator's
        authority should be a bounded, audited, revocable REA commitment
        (`project_rea_compute_commitment_primitive`), never an admin key or cluster access.
    (b) adam-invisibility — a peer observable only through OUR cluster's Loki is not a
        sovereign peer. A peer must be able to answer for itself.

  The @requires:multi-node tag is a fixture precondition (not a cluster-state gate), so the
  scope reconciler ignores it; these run whenever the alpha endpoints are reachable.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: a commitment holder can drive a reconcile and the peer attests to it
    # The positive half — a cure may not simply refuse everything. The holder's authority
    # is the commitment, not an API key, and the peer records WHICH commitment it acted
    # under so the act is auditable after the fact.
    Given "matthew" holds an active "delegates-compute" commitment over peer "elohim.host"
    When "matthew" requests a reconcile on peer "elohim.host" through the doorway
    Then the request is accepted
    And peer "elohim.host" performs the reconcile
    And peer "elohim.host" attests the commitment cid it acted under

  Scenario: a caller without a commitment is refused
    # Fail-closed. Today both admin verbs answer on the operator-seat class without ever
    # consulting authorize_operation, so this is the scenario that is honestly red.
    Given "james" holds no "delegates-compute" commitment over peer "elohim.host"
    When "james" requests a reconcile on peer "elohim.host" through the doorway
    Then the request is refused
    And peer "elohim.host" performs no reconcile

  Scenario: a revoked commitment stops working immediately
    # Revocability is the whole point of preferring a commitment over a key: a key must be
    # rotated everywhere, a commitment is revoked once on-chain and the authority is gone.
    Given "matthew" held a "delegates-compute" commitment over peer "elohim.host"
    And that commitment has been revoked
    When "matthew" requests a reconcile on peer "elohim.host" through the doorway
    Then the request is refused

  Scenario: a peer serves its own runtime telemetry
    # The symmetry gap. adam has zero Loki streams, so today it is legible only through the
    # operator's observability cluster — which makes "operating your own node" false for
    # anyone outside our infrastructure. A peer must answer for itself.
    When peer "elohim.host" is asked for its runtime status directly
    Then peer "elohim.host" reports its own sync, reconcile and peer counts
    And the answer does not traverse the operator's observability cluster
