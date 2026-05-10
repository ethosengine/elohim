/**
 * Recovery cross-stack step definitions — gate #5 of iroh Phase 11 cutover.
 * (genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md line 514).
 *
 * Framework: Cucumber-JS 11 + tsx (a2o convention).
 * Tag: @recovery-cross-stack — run with: pnpm exec cucumber-js --tags '@recovery-cross-stack'
 *
 * Wire-interaction contract (locked by 2026-05-10-iroh-recovery-e2e.md Task 2):
 *   - Send invitation:   P2PHandle::publish_recovery_invitation routes through DualGossipPublisher
 *                        (elohim/elohim-storage/src/p2p/mod.rs:2403 arm P2PCommand::PublishRecoveryInvitation)
 *   - Receive invitation: gossip subscribe on both stacks — permanent dual per spec line 513
 *                        (classify_topic("recovery.invitation") => TopicTransports::Dual)
 *   - Submit witness:    imagodei zome fn submit_intimate_witness (Holochain Track 1)
 *                        (elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:2709)
 *   - Commit rotation:  imagodei zome fn commit_key_rotation (Holochain Track 1)
 *                        (elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:2506)
 *   - Send revocation:  P2PHandle::publish_recovery_revocation routes through DualGossipPublisher
 *                        (elohim/elohim-storage/src/p2p/mod.rs:2469 arm P2PCommand::PublishRecoveryRevocation)
 *   - Per-peer transport selection: PeerTransportManifest from Plan 1
 *                        (elohim/elohim-storage/src/p2p_iroh/peer_map.rs)
 *
 * Recovery step catalog (from plan Task 1):
 *   | Recovery invitation send (publish)     | gossip publish on recovery.invitation      |
 *   | Recovery invitation wire schema        | RecoveryInvitation struct, MessagePack      |
 *   | RecoveryRequest DHT commit             | create_recovery_request zome fn             |
 *   | Witness submission                     | submit_intimate_witness zome fn             |
 *   | Attestation gathering                  | recovery_witnesses table projection         |
 *   | Reassembly (key rotation commit)       | commit_key_rotation zome fn                 |
 *   | KeyStewardship (share-holder roster)   | KeyStewardship DHT entry                   |
 *   | Revocation send (publish)              | gossip publish on recovery.revocation       |
 *   | Revocation wire schema                 | RecoveryRevocationMessage struct            |
 *   | Self-revocation (coordinator)          | create_self_revocation zome fn              |
 *   | Pending-recovery view (HTTP)           | GET /api/v1/account/pending-recovery        |
 *   | Vote-on-recovery write (HTTP)          | POST /api/v1/account/recovery/:id/vote      |
 *
 * Step bodies remain 'pending' until the Rust integration test (Task 7) is
 * green and Plans 1+4 land; the cucumber scenarios call into a thin HTTP
 * helper exposed by the MultiStackFixture's admin port.
 *
 * Rust integration test (lower-level, no Holochain conductor overhead):
 *   elohim/elohim-storage/tests/iroh_recovery_cross_stack.rs
 *
 * MultiStackFixture (5-node cross-stack harness):
 *   elohim/elohim-storage/src/p2p_iroh/multi_stack_fixture.rs
 */

import { Given, When, Then, DataTable } from '@cucumber/cucumber';

import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// Background steps
// ---------------------------------------------------------------------------

Given(
  'a 5-node cross-stack fixture is running with these share-holder profiles:',
  async function (this: E2EWorld, _table: DataTable) {
    // TODO(plan-1): wire MultiStackFixture admin URLs into E2EWorld state.
    // MultiStackFixture::new([("Ben","iroh"), ("Cara","iroh"), ("Dan","libp2p"),
    //   ("Evan","libp2p"), ("Faye","dual")]) via fixture helper binary.
    // Each node's admin port is stored in this.parameters.crossStackFixture.
    return 'pending';
  },
);

Given(
  'Abby is registered with required_witness_count of {int}',
  async function (this: E2EWorld, _requiredWitnessCount: number) {
    // TODO(plan-1): POST to Abby's node to create human record.
    // Mirror: intimate-quorum-happy-path.feature Background "Abby is registered via doorway-alpha".
    // HTTP: POST /api/v1/human (or equivalent from account.rs route table).
    return 'pending';
  },
);

Given(
  'each of Ben, Cara, Dan, Evan, Faye has a HumanRelationship to Abby with emergency_access_enabled = true',
  async function (this: E2EWorld) {
    // TODO(plan-1): POST HumanRelationship on each share-holder's node.
    // HTTP: POST /db/relationship (confirm exact route from account.rs route table).
    // emergency_access_enabled = true qualifies them as intimate contacts per
    // check_intimate_quorum_rules (imagodei_integrity/src/recovery_v2.rs:160).
    return 'pending';
  },
);

// ---------------------------------------------------------------------------
// Scenario-level Given steps
// ---------------------------------------------------------------------------

Given(
  "Abby's required_witness_count is satisfied by share-holders {string}",
  async function (this: E2EWorld, _shareHolderListJson: string) {
    // TODO(plan-1): record which share-holders are active for this scenario.
    // _shareHolderListJson e.g. '["Ben", "Cara", "Faye"]' — parse and store in world state.
    return 'pending';
  },
);

Given('share-holder {word} is offline', async function (this: E2EWorld, _name: string) {
  // TODO(plan-1): call MultiStackFixture::take(name) to shut down the named NodeSlot.
  // Admin port: fixture.admin_port(name) — kills iroh + libp2p handles for that slot.
  return 'pending';
});

Given(
  'Ben, Dan, Faye have each submitted submit_intimate_witness',
  async function (this: E2EWorld) {
    // TODO(plan-1): submit witness for Ben, Dan, Faye against the open recovery request.
    // HTTP: POST /api/v1/account/recovery/:id/vote (account.rs:74).
    return 'pending';
  },
);

// ---------------------------------------------------------------------------
// When steps
// ---------------------------------------------------------------------------

When(
  'Abby invokes create_recovery_request from a fresh agent key',
  async function (this: E2EWorld) {
    // TODO(plan-1): POST to Abby's node creating a recovery request.
    // Coordinator: create_recovery_request (imagodei/zomes/imagodei/src/lib.rs:1883).
    // This publishes RecoveryInvitation on "recovery.invitation" via DualGossipPublisher.
    // RecoveryInvitation wire: elohim/elohim-storage/src/p2p/recovery_invitation.rs:22.
    return 'pending';
  },
);

When(
  'each share-holder in {string} receives the recovery.invitation gossip',
  async function (this: E2EWorld, _shareHolderListJson: string) {
    // TODO(plan-1): poll each share-holder's GET /api/v1/account/pending-recovery
    // (account.rs:386) until non-empty or 10s timeout.
    // The DualGossipPublisher fans out to both iroh-gossip + libp2p-gossipsub simultaneously
    // so each peer receives via its preferred transport automatically.
    return 'pending';
  },
);

When(
  "share-holders {string} each receive the recovery.invitation gossip",
  async function (this: E2EWorld, _shareHolderListJson: string) {
    // Same polling logic as above — alias for the offline-scenario wording.
    return 'pending';
  },
);

When(
  'each share-holder in {string} submits submit_intimate_witness',
  async function (this: E2EWorld, _shareHolderListJson: string) {
    // TODO(plan-1): POST per share-holder to submit witness.
    // Coordinator: submit_intimate_witness (imagodei/zomes/imagodei/src/lib.rs:2709).
    // HTTP: POST /api/v1/account/recovery/:id/vote (account.rs:74).
    return 'pending';
  },
);

When(
  "share-holders {string} each submit submit_intimate_witness",
  async function (this: E2EWorld, _shareHolderListJson: string) {
    // Alias for the offline-scenario wording.
    return 'pending';
  },
);

When(
  'Abby invokes commit_key_rotation with IntimateQuorum carrying witness_hashes for {word}, {word}, {word}',
  async function (this: E2EWorld, _a: string, _b: string, _c: string) {
    // TODO(plan-1): POST to Abby's node to commit key rotation with IntimateQuorum.
    // Coordinator: commit_key_rotation (imagodei/zomes/imagodei/src/lib.rs:2506).
    // IntimateQuorum requires threshold m=3 witnesses out of n registered share-holders.
    return 'pending';
  },
);

When(
  'share-holder {word} publishes a RecoveryRevocationMessage on the recovery.revocation topic',
  async function (this: E2EWorld, _name: string) {
    // TODO(plan-1): POST the named share-holder's node to trigger self-revocation.
    // HTTP: POST /api/v1/account/self-revocation (account.rs:74 route family).
    // This triggers create_self_revocation (imagodei/zomes/imagodei/src/lib.rs:1948)
    // and publish_recovery_revocation -> DualGossipPublisher on "recovery.revocation".
    // RecoveryRevocationMessage wire: elohim/elohim-storage/src/p2p/recovery_revocation.rs:24.
    return 'pending';
  },
);

// ---------------------------------------------------------------------------
// Then steps
// ---------------------------------------------------------------------------

Then(
  'the recovery_witnesses projection for the request has count {int}',
  async function (this: E2EWorld, _expectedCount: number) {
    // TODO(plan-1): GET /api/v1/account/pending-recovery (account.rs:386) from Abby's node.
    // Assert recovery_witnesses count == expectedCount.
    // Projection sourced from: db/recovery_witnesses.rs + signals.rs:818.
    return 'pending';
  },
);

Then('the rotation succeeds', async function (this: E2EWorld) {
  // TODO(plan-1): assert HTTP 200 from commit_key_rotation POST.
  return 'pending';
});

Then(
  'the rotation succeeds (k-of-n threshold met without Cara)',
  async function (this: E2EWorld) {
    // Same assertion as 'the rotation succeeds' — alias for the offline scenario wording.
    return 'pending';
  },
);

Then(
  'every share-holder receipt was tagged transport={word} in the recovery::transport debug log',
  async function (this: E2EWorld, _transport: string) {
    // TODO(plan-1): read per-node tracing ring-buffer from MultiStackFixture admin port.
    // Assert ALL receipts for this recovery round-trip have transport=={transport}.
    // Log emitted by: elohim/elohim-storage/src/p2p/blob_fetch.rs (Task 8 tracing line).
    // Target: "recovery::transport", field: transport = "iroh" | "libp2p".
    return 'pending';
  },
);

Then(
  'the recovery::transport debug log contains at least one transport={word} receipt',
  async function (this: E2EWorld, _transport: string) {
    // TODO(plan-1): read per-node tracing ring-buffer; assert count > 0 for given transport.
    return 'pending';
  },
);

Then(
  "share-holder Cara does NOT receive the recovery.invitation gossip",
  async function (this: E2EWorld) {
    // TODO(plan-1): assert Cara's GET /api/v1/account/pending-recovery returns empty list
    // after a brief poll window (Cara was taken offline before create_recovery_request).
    return 'pending';
  },
);

Then(
  "the recovery_witnesses projection for the request reflects Ben's revocation",
  async function (this: E2EWorld) {
    // TODO(plan-1): GET /api/v1/account/pending-recovery; assert Ben's witness entry
    // shows revoked status (status != 'active' or absent from valid quorum set).
    return 'pending';
  },
);

Then(
  "Abby's commit_key_rotation with IntimateQuorum carrying witness_hashes for Ben, Dan, Faye is rejected by the M2 validator",
  async function (this: E2EWorld) {
    // TODO(plan-1): POST commit_key_rotation; assert HTTP 4xx.
    // Rejection comes from check_intimate_quorum_rules (imagodei_integrity/src/recovery_v2.rs:160)
    // which validates that none of the included HumanityWitness entries have been revoked.
    return 'pending';
  },
);

Then(
  'the rejection reason references the revoked HumanityWitness rule',
  async function (this: E2EWorld) {
    // TODO(plan-1): assert response body contains reference to revocation rule
    // (e.g. "RECOVERY_AUTHORITY_LAYERS" or "revoked witness" in error message).
    return 'pending';
  },
);
