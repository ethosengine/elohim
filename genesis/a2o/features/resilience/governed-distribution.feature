@e2e @resilience @governance @local @wip @concern:blob-durability @dataplane @act:i
Feature: Governed auto-distribution via a revocable compute delegation
  As Matthew dogfooding a keyless Eclipse Che peer-client
  I want my content writes to drive distribute_shards only while a bounded, revocable
  delegates-compute grant authorizes them
  So that "developing the p2p-dataplane requires participating in it" is a real,
  governed loop — not an ungoverned bypass

  # Harvested from the Che keyless peer-client Slice-1 op-gate spine
  # (plan: 2026-06-26-che-keyless-peer-client-slice1-governance-spine-plan.md).
  # The offline spine (seed factory, gated seed endpoint, authorize-operation gate,
  # doorway pre-dispatch op-gate, deploy-posture) landed + whole-branch-reviewed clean.
  # These scenarios are the BEHAVIORAL acceptance layer (plan Task 6) — they need a live
  # hc:start:seed M/J/J household stack + a Matthew dev credential (a fixture precondition,
  # NOT @requires:shem — the household is itself a multi-node cluster). Step defs unwritten → @wip.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And elohim-storage is reachable at "E2E_STORAGE_URL"
    And the doorway runs the delegates-compute op-gate in "enforce" mode
    And human "Matthew" is logged in on doorway "alpha" holding only a portal credential (no on-device key)

  # --- Capability proof: the bridge that holds -----------------------------

  @regression
  Scenario: A bounded grant lets keyless Che drive governed distribution
    Given an active "delegates-compute" grant from "Matthew" to "Matthew" with scope "orchestrate-node", a finite rate, and a finite ttl
    When "Matthew" posts a blob-backed "commons"-reach content item "governed-alpha" through the doorway
    Then the write is authorized and forwarded to storage
    And within 30 seconds "/api/v1/resilience/governed-alpha/household" reports "stewardingCollectives" >= 1
    # Constraint: distribute_shards is the node's own side-effect, authorized per-request at the doorway.

  # --- Revocation regression: deny the NEXT request ------------------------

  @regression
  Scenario: Revoking the grant denies the next distribution request
    Given an active "delegates-compute" grant from "Matthew" to "Matthew" with scope "orchestrate-node"
    And "Matthew" has successfully posted one governed content item
    When the "delegates-compute" grant is revoked
    And "Matthew" posts another blob-backed content item through the doorway
    Then the doorway returns 403
    And the response body is a generic authorization error with no internal commitment detail
    # Constraint: the JWT is NOT the revocation surface — the op-gate consults the commitment
    # per-request, so revocation denies the NEXT request (in-flight fan-out completes).

  # --- Observe is a non-blocking shadow stage (guards a fixed bug) ---------

  @regression
  Scenario: Observe mode forwards an ungranted write instead of blocking it
    Given the doorway runs the delegates-compute op-gate in "observe" mode
    And there is no active "delegates-compute" grant for the requester
    When a blob-backed content write reaches the doorway without a verified credential
    Then the write is forwarded to storage (not blocked)
    And the doorway logs a "would-deny (no credential)" observation
    # Regression: observe must NEVER block — it shadows the would-deny decision so an operator can
    # measure enforce blast-radius safely. A prior build 403'd the no-credential case in observe too,
    # corrupting exactly the anonymous-write class the shadow stage exists to measure (fixed: 0fa7ee17c).

  # --- Bounded-minimum guard: no unbounded delegation ----------------------

  # HELD (2026-08-21): explicitly unit-covered in seed-delegates-compute.test.ts;
  # this a2o row duplicates it for the governance principle only.
  @regression
  Scenario: An unbounded compute delegation is refused at grant time
    When a "delegates-compute" grant is requested with a wildcard epr scope "*" but no finite rate and no finite ttl
    Then the seed factory rejects the grant before it reaches storage
    And the rejection names the missing bound
    # Constraint: wildcard scope demands BOTH an explicit finite rate_per_hour (>=1) AND
    # rotation_ttl_days (>=1); a reach ceiling outside the {commons, community} allowlist additionally
    # requires reach_elevation_acknowledged=true. (Unit-covered in seed-delegates-compute.test.ts;
    # this scenario carries the governance principle: the protocol refuses unbounded delegation.)
    # Operational parameters: rate_per_hour>=1, rotation_ttl_days>=1, reach allowlist {commons, community}.
    # Informs: operator grant presets + the genuinely-DHT-notarized delegates-compute follow-up (D1=A).

  # --- Deploy-posture honesty: refuse an incoherent Che-facing boot --------

  @regression
  Scenario: A Che-facing doorway refuses to boot in an insecure posture
    Given a doorway configured as Che-facing with the op-gate in "enforce"
    When it is started with development mode enabled, or with a JWT secret shorter than 32 characters
    Then the doorway refuses to boot
    # Constraint: CHE_FACING=1 requires gate=enforce AND dev_mode off AND jwt_secret >= 32 chars —
    # dev_mode disables auth (forge-any-performer) and a weak HS256 secret forges any performer.
    # Operational parameter: JWT_SECRET minimum length = 32. (Unit-covered in config.rs validate().)
