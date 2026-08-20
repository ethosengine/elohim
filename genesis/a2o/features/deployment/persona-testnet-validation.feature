# SUBSTRATE: this feature is a ONE-BOX harness, not a deployed-fleet target.
# Every scenario inherits a Background demanding 20 concurrent `elohim-node`
# processes driven by steward/node/simulation/spawn-persona-testnet.sh.
#
# Two INDEPENDENT blockers keep it out of CI. Both were measured on 2026-08-20;
# neither is the stale `elohim-node/simulation` path that an earlier revision of
# this header blamed. That path bug was real but unrelated, and is now fixed
# (testnet-manager.ts points at steward/node/simulation; the crate was renamed in
# d35d1d5e3). Fixing it does not make one scenario here runnable:
#
#   1. NO BINARY, NO RUNNER (-> @local). `steward/node` has no build-manifest.json,
#      so no pipeline ever builds the `elohim-node` bin, and no Jenkinsfile or
#      scripts/ci/*.sh invokes the `testnet` cucumber profile. Both e2e gates run
#      cucumber against a deployed doorway URL and spawn no local processes.
#      Verified: the spawn script now runs and stops at "Cannot find elohim-node
#      binary" (exit 1) rather than "No such file or directory" (exit 127).
#   2. NO STEP DEFINITIONS (-> @wip). `cucumber-js --dry-run --tags @persona-testnet`
#      reports 18 scenarios / 128 steps, 128 undefined. Not one step is implemented.
#
# The two tags guard DIFFERENT runners and neither is redundant:
#   @local -> excluded from the deployed API and browser gates, which filter
#             'not @local and not @wip' (genesis/scripts/ci/e2e-verify-*.sh).
#   @wip   -> excluded from `pnpm test:testnet`, this feature's only real home
#             (--profile testnet --tags '@testnet and @e2e and not @wip'), which
#             does NOT filter @local. Drop @wip and the testnet profile picks up
#             128 undefined steps.
# Removing either tag reds a gate without adding one line of coverage; the work
# that earns them back is writing the step definitions and giving `elohim-node` a
# build, in that order.
#
# NOT @requires:shem: shem is the remote multi-tenant canvas and is available
# today; carrying that tag is what silently promoted this feature into the
# deployed API suite when shem flipped back on (commit 41c565f3e).
#
# Known-good foothold for whoever implements this: gen-persona-configs.sh --out DIR
# runs binary-free and emits all 20 persona TOMLs, so the config-generation
# assertions are reachable before any Rust build exists.
@e2e @deployment @p2p @persona-testnet @testnet @local @wip
Feature: Persona Testnet — 20 Humans on One Box
  As the Elohim Protocol,
  I want to simulate 20 real humans from genesis stories on a single machine
  so that I can validate P2P behavior through the lens of trust, not infrastructure.

  Each node IS a person. Matthew's household shares a cluster.
  The economy workers share a cluster. Newcomers discover the network cold.
  Compute budgets are tracked per-persona using shefa vocabulary.

  Background:
    Given a persona testnet is running with 20 humans in 4 clusters
    And compute budget tracking is active

  # ─── Graduated Scaling (1→5 conductors) ──────────────────────────────────

  @scaling @compute
  Scenario: Step 1 — Single conductor seeds Matthew's stewardship content
    Given 1 conductor is running for "Matthew"
    And content is filtered by stewardedBy for "human-matthew-manager"
    When genesis content is seeded to Matthew's conductor
    Then Matthew's conductor has content where he is highest-affinity steward
    And baseline compute is measured
      | metric | unit        |
      | cpu    | millicores  |
      | memory | megabytes   |
      | time   | seconds     |
    And compute usage is recorded as shefa EconomicEvent

  @scaling @compute
  Scenario: Step 2 — Household replication between Matthew and Susan
    Given 2 conductors are running for "Matthew" and "Susan"
    And each conductor is seeded with its stewardship content
    When household replication activates via spouse relationship
    Then Susan can see Matthew's content at neighborhood reach or above
    And Susan cannot see Matthew's private-reach content
    And Matthew can see Susan's content at neighborhood reach or above
    And compute for 2 conductors is within budget
      | resource | budget | unit       |
      | cpu      | 2000   | millicores |
      | memory   | 4000   | megabytes  |

  @scaling @compute
  Scenario: Step 3 — Cross-cluster bridge to Pete
    Given 3 conductors are running for "Matthew", "Susan", and "Pete"
    And each conductor is seeded with its stewardship content
    When cross-cluster discovery activates via congregation_member relationship
    Then Pete can see community-reach content from Matthew's household
    And Pete cannot see family-private content
    And content replication follows trust topology not bulk access
    And compute for 3 conductors is within budget
      | resource | budget | unit       |
      | cpu      | 3000   | millicores |
      | memory   | 6000   | megabytes  |

  @scaling @compute
  Scenario: Step 5 — Five conductors with multi-hop content discovery
    Given 5 conductors are running for "Matthew", "Susan", "Pete", "Terrance", and "Frank"
    And each conductor is seeded with its stewardship content
    When all conductors have discovered their peers
    Then Terrance sees learning content via Susan's learning_partner bridge
    And Frank's economy content is NOT directly visible to Matthew
    And content discovery paths match the relationship graph
    And compute for 5 conductors is within budget
      | resource | budget | unit       |
      | cpu      | 5000   | millicores |
      | memory   | 10000  | megabytes  |
    And per-conductor compute is recorded in shefa vocabulary

  # ─── Cluster Formation ──────────────────────────────────────────────────

  Scenario: Matthew's household discovers each other
    Given cluster "matthews-household" has 5 personas
    When the cluster has been running for 30 seconds
    Then Matthew should see 4 peers in his cluster
    And Susan should see 4 peers in her cluster
    And Sammy's node should be running with guardian constraints

  Scenario: Faith community forms around Pete
    Given cluster "faith-community" has 5 personas
    When the cluster has been running for 30 seconds
    Then Pete should see 4 peers in his cluster
    And Terrance should see Sammy as a cross-cluster mentee

  Scenario: Local economy nodes discover business partners
    Given cluster "local-economy" has 5 personas
    When the cluster has been running for 30 seconds
    Then Frank should see Georgina as a trusted business partner
    And Bub should see Manny as a trusted business partner

  Scenario: Newcomers join with no prior relationships
    Given cluster "extended-network" has 5 personas
    When Maria's node starts with zero relationships
    Then Maria should discover Nancy as cluster primary via bootstrap
    And Maria's peer count should grow from 0 over time

  # ─── Cross-Cluster Bridges ─────────────────────────────────────────────

  Scenario: Trust topology creates cross-cluster sync paths
    Given all 4 clusters are running
    When a document is created on Matthew's node
    Then it should reach Pete within 10 seconds via congregation bridge
    And it should reach Nancy within 10 seconds via neighbor bridge
    And it should NOT reach Maria who has no trust path to Matthew

  Scenario: Susan bridges household to economy through Georgina
    Given all 4 clusters are running
    When Georgina creates a community document
    Then Susan should receive it via community_member relationship
    And Matthew should receive it via Susan's household

  # ─── Sync Correctness ─────────────────────────────────────────────────

  Scenario: CRDT convergence across all 20 nodes
    Given all 20 personas are healthy
    When 5 documents are created on 5 different nodes simultaneously
    Then all 20 nodes should converge within 60 seconds
    And no node should have conflicting document states

  Scenario: Network partition heals correctly
    Given all 20 personas are healthy and synced
    When Pete's node is killed (partition)
    Then the faith community should continue operating with 4 nodes
    When Pete's node is healed (restart)
    Then Pete should resync within 30 seconds
    And no documents should be lost

  # ─── Compute Budget ────────────────────────────────────────────────────

  @compute
  Scenario: Per-persona compute stays within budget
    Given the testnet has been running for 5 minutes
    When the compute budget is checked
    Then no persona should exceed 360 cpu-seconds
    And no persona should exceed 150 megabytes of memory
    And total cluster compute should not exceed 7200 cpu-seconds

  @compute
  Scenario: Sync storm does not blow memory budget
    Given all 20 personas are healthy
    When 100 documents are created simultaneously across all nodes
    Then peak memory per persona should stay under 150 megabytes
    And the compute budget check should pass

  @compute
  Scenario: Economic events record actual compute consumed
    Given the testnet has been running for 5 minutes
    When settlement is triggered
    Then 20 EconomicEvent records should be produced
    And each event should record cpu-seconds and megabytes consumed
    And the ledger should be append-only JSONL

  # ─── Persona Lifecycle ─────────────────────────────────────────────────

  Scenario: Clean startup from genesis personas
    When the persona testnet is started from clean state
    Then 20 configs should be generated from personas.json
    And each config should have the human's ID as node ID
    And each config should have compute budget limits

  Scenario: Clean shutdown preserves economic ledger
    Given the testnet has been running with budget tracking
    When the testnet is stopped
    Then all 20 processes should be terminated
    And the compute ledger should be preserved
    And the economic events file should be valid JSON

  Scenario: Full cleanup removes all testnet artifacts
    Given the testnet has been stopped
    When cleanup is requested
    Then all data directories should be removed
    And all PID files should be removed
    And the compute ledger should be archived before deletion
