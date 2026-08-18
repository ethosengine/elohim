@e2e @dataplane @concern:operator-runtime-surface @requires:multi-node
Feature: Operator verbs are commitment-gated protocol acts, not cluster surgery
  THE STORY. Today "operating a peer" means holding OUR cluster's credentials:
  restart scripts call kubectl, admin routes answer whoever reaches them, and a
  peer like elohim.host's is legible only through our monitoring stack. The
  person harmed is the peer operator outside our infrastructure — they cannot
  act on their own runtime, prove why an action was allowed, or watch their own
  node. These scenarios assert the replacement: operator authority is a
  "delegates-compute" commitment — a bounded, revocable, auditable grant
  recorded on the network by a peer's steward, naming who may perform which
  operator verb within what bounds (scope, rate, expiry). A "reconcile" is the
  first gated verb: it asks the peer to run its own state-repair sweep NOW
  (re-checking its local projections against its peers) instead of waiting for
  the next timer tick — the act today's pod-restart scripts crudely stand in
  for. And because operating your own peer includes SEEING it, the last
  scenario asserts the peer serves its own runtime telemetry directly.

  In the scenarios: "matthew" is a steward who holds (or held) such a grant
  over peer "elohim.host"; "james" holds none. The Background registers the
  same doorway pair as the sibling dataplane suites (shared step fixtures);
  peer "alpha-A" anchors that pair even though these scenarios address
  "elohim.host". "Refused" means an explicit authorization refusal (401/403) —
  a missing route or a crash is a failure of the scenario, never a pass.

  RED-FIRST. These scenarios ARE the schedulable red for habit
  `operator-runtime-surface`. They assert what "operating a peer" must come to
  mean, and pass unchanged the moment it does.

  The @wip tag was REMOVED 2026-08-18 (an operator decision this file
  reserved): these 4 scenarios now count in the edge Dataplane Validation
  byConcern rollup (the stage's tag filter excludes only wip and browser-only
  scenarios; advisory — catchError→UNSTABLE). Step definitions are wired
  (steps/dataplane/operator-commitment-gated-verbs.steps.ts) and the slice-1
  cure (storage POST /api/v1/operator/reconcile + doorway per-verb op-gate +
  ALLOW_SEED_DELEGATES_COMPUTE grant lever) is on dev. They go green when the
  deployed fleet carries that cure AND fixture personas can authenticate to
  the doorway API outside a browser session — today they cannot (the "persona
  API-mode auth gap"), so the holder scenarios fail at login before reaching
  the verb.
  Run locally: pnpm exec cucumber-js --tags '@concern:operator-runtime-surface'

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
    # alpha-A is registered because these Given-peer fixtures are shared with the
    # sibling dataplane suites (one doorway pair, byte-identical Background); the
    # scenarios below address elohim.host only.
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: a commitment holder can drive a reconcile and the peer attests to it
    # The positive half — a cure may not simply refuse everything. The holder's authority
    # is the commitment, not an API key ("delegates-compute" = the bounded, revocable
    # grant defined in the preamble; "the doorway" = the web gateway through which
    # external callers reach a peer). The attestation is OBSERVABLE: the 200 response
    # body names the commitment cid the peer acted under plus the id of the recorded
    # economic event (the durable audit row the grant's rate ceiling counts).
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
    # rotated everywhere, a commitment is revoked once on the network and the authority is
    # gone. This scenario deliberately starts from the post-revocation state — the
    # grant-then-revoke transition is the fixture; the assertion is that the SAME holder
    # who succeeded in the first scenario is refused the moment the grant is revoked.
    Given "matthew" held a "delegates-compute" commitment over peer "elohim.host"
    And that commitment has been revoked
    When "matthew" requests a reconcile on peer "elohim.host" through the doorway
    Then the request is refused

  Scenario: a peer serves its own runtime telemetry
    # The symmetry gap: today this peer is legible only through the operator's
    # observability cluster, which makes "operating your own node" false for anyone
    # outside our infrastructure. Directness is proven positively by the previous step —
    # the probe hits the peer's OWN public hostname and the peer's own HTTP surface
    # answers with its sync/reconcile/peer state; the negative step pins that no
    # Grafana/Loki/Prometheus hop is part of that answer path.
    When peer "elohim.host" is asked for its runtime status directly
    Then peer "elohim.host" reports its own sync, reconcile and peer counts
    And the answer does not traverse the operator's observability cluster
