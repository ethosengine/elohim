---
title: iroh Phase 11 Cutover Gate #5 — Recovery e2e Cross-Stack Harness
status: complete
created: 2026-05-10
parent: genesis/docs/superpowers/plans/2026-05-08-iroh-phase11-prep.md
spec: genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md (gate #5, line 514)
related:
  - genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md
  - genesis/docs/superpowers/plans/2026-04-21-recovery-protocol-phase-2-m1-data-model.md
  - genesis/docs/superpowers/plans/2026-04-24-recovery-protocol-phase-2-m3-coordinator-and-storage.md
  - genesis/docs/superpowers/plans/2026-04-24-recovery-protocol-phase-2-m4-fast-path-revocation.md
memory_anchors:
  - project_socially_derived_security
  - project_recovery_grandma_standard
  - project_graduated_recovery_authority
  - project_iroh_parallel_stack_phases3_7_landed
  - project_iroh_phase11_all_backends_wired
---

# iroh Phase 11 Cutover Gate #5 — Recovery e2e Cross-Stack Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wrap the existing libp2p-canonical recovery flow (RecoveryRequest → IntimateQuorum witnessing → KeyRotation, plus KeyRevocation) in cross-stack a2o scenarios that prove gate #5 — recovery completes cleanly when share-holders speak iroh, libp2p, or a mixture of both.

**Architecture:** Recovery is gossip-mediated (`recovery.invitation` and `recovery.revocation` topics) with DHT-notarized authority entries (RecoveryRequest, HumanityWitness, KeyRotation, KeyRevocation, KeyStewardship). The wire-touching surface that needs cross-stack proof is the gossip publish/subscribe pair (one site each in `elohim/elohim-storage/src/p2p/mod.rs:2383` and `:2442`); DHT entries traverse Holochain kitsune2 (Track 1, untouched). This plan adds (a) a 5-node parity harness mixing iroh-only / libp2p-only / dual peers, (b) Cucumber/Gherkin scenarios in `genesis/a2o/features/auth/recovery/cross-stack/`, (c) Rust integration test that asserts share-holder reachability across the dual-stack mesh, and (d) per-share transport-tag debug logging so the parity-soak can confirm cross-stack delivery actually happened.

**Tech Stack:**
- Cucumber-JS 11 + tsx (a2o convention from `genesis/a2o/cucumber.mjs`)
- TypeScript step definitions in `genesis/a2o/steps/` (one new file: `recovery-cross-stack.steps.ts`)
- Rust `tokio::test` integration (`elohim/elohim-storage/tests/iroh_recovery_cross_stack.rs`)
- `elohim_storage::p2p_iroh::parity_harness::TwoNodeFixture` (extended to a 5-node `MultiStackFixture` in this plan)
- `tracing` debug-level events with structured `transport=iroh|libp2p` field
- `RUSTFLAGS='--cfg getrandom_backend="custom"'` for `elohim-storage` build
- `cargo test ... -- --test-threads=1` for any test that touches `ELOHIM_TRANSPORT_BACKEND` env

---

## P2P design gate — entity classification

Per `.claude/skills/p2p-design-gate/SKILL.md`. This plan introduces **zero new notarized DHT entities, zero new sync messages, and zero new wire-level data shapes**. Every entity touched is pre-existing and was design-gated at original introduction. Classification of each:

| Entity referenced | Category | Source-of-truth | Already gated by |
|---|---|---|---|
| `RecoveryRequest` | A (notarized) | DHT (imagodei DNA) | `2026-04-21-recovery-protocol-phase-2-m1-data-model.md` |
| `HumanityWitness` | A (notarized) | DHT (imagodei DNA) | same |
| `KeyRotation` | A (notarized) | DHT (imagodei DNA) | same |
| `KeyRevocation` | A (notarized) | DHT (imagodei DNA) | `2026-04-24-recovery-protocol-phase-2-m4-fast-path-revocation.md` |
| `KeyStewardship` | A (notarized) | DHT (imagodei DNA, `lib.rs:669`) | pre-existing |
| `RecoveryInvitation` (gossip wire) | C (operational) | gossipsub wire — `recovery_invitation.rs:22-29` | M3 milestone (`2026-04-24-recovery-protocol-phase-2-m3-coordinator-and-storage.md`) |
| `RecoveryRevocationMessage` (gossip wire) | C (operational) | gossipsub wire — `recovery_revocation.rs:24-42` | M4 milestone |
| `recovery_witnesses` (storage projection) | C (operational projection) | derived from `RecoveryV2Signal::IntimateWitnessSubmitted`; canonical truth is the DHT `HumanityWitness` entry | M3 milestone |
| `recovery_requests` (storage projection) | C (operational projection) | derived from `RecoveryV2Signal::RecoveryRequestCommitted`; canonical truth is the DHT `RecoveryRequest` entry | M3 milestone |
| `PeerTransportManifest` (referenced from Plan 1) | C (operational) | introduced by Plan 1 (`2026-05-08-iroh-libp2p-complementarity.md` §"Cross-stack peer-map as permanent structural schema"); this plan only **reads** it | Plan 1 |
| `MultiStackFixture` (test-only) | not protocol — it's a `#[cfg(test)]` Rust struct that lives in `src/p2p_iroh/multi_stack_fixture.rs` for test convenience | n/a | n/a |

**No new HTTP routes proposed.** Tasks 6 and 7 call only pre-existing routes from `account.rs:70-77` and pre-existing zome functions cataloged in Task 1.

**No new DNA entry types.** Mishpat sits at 11/100 per project memory; this plan introduces zero entries.

**No new gossip topics.** This plan exercises the existing `recovery.invitation` and `recovery.revocation` topics (the dual-publish wiring is owned by Plan 4).

---

## Blocked Until

This plan can be **WRITTEN** now (and committed); execution of Tasks 5–9 (the harness, the Rust integration test, the green-running scenarios) blocks on:

- **Plan 1 (`peer_transport_manifest`)** — owns the source-of-truth for the per-peer transport profile lookup. Plan 1 introduces and design-gates the `PeerTransportManifest` Category-C operational projection (graduated from the existing `cross_stack_peer_map` table per spec `2026-05-08-iroh-libp2p-complementarity.md` §"Cross-stack peer-map as permanent structural schema"; canonical truth is the DHT `AgentPeerBinding` entries authored by each peer). This plan is a **pure consumer** — it reads `PeerTransportManifest` to ask "which transport does share-holder X support?" and introduces zero schema or persistence of its own. Without Plan 1 landed, the consumer test has nothing to read.
- **Plan 4 (Gossip dual-publish)** — `recovery.invitation` and `recovery.revocation` topics must publish to **both** iroh-gossip and libp2p-gossipsub for cross-stack peers to receive (per spec gate #4: gossip dual-publish is **permanent post-cutover** for recovery topics). Without it, the mixed-stack scenario cannot pass: an iroh-only share-holder will never see a libp2p-only publisher's invitation.

Tasks 1–4 are pure documentation/scaffolding and can land independently. Task 10 (commit) only fires once everything green.

The plan as written assumes:
- `PeerTransportManifest` is exposed at `elohim_storage::p2p_iroh::peer_map::PeerTransportManifest` (graduated from current `CrossStackPeerMapRow` per Plan 1).
- The dual-publish entry point is `elohim_storage::p2p::P2PHandle::publish_recovery_invitation_dual(inv) -> DualPublishResult` and `publish_recovery_revocation_dual(msg) -> DualPublishResult` per Plan 4 (mirroring the `publish_dual` pattern sketched in `2026-05-08-iroh-phase11-prep.md` §2.8 Pattern B). If Plan 4 names them differently, Task 4 (wire-mapping doc) updates here.

---

## Discovery Required

The following pieces of the existing recovery flow are NOT exposed cleanly enough for the cross-stack scenarios as drafted. Each is called out with a concrete mitigation; if any mitigation is wrong-shaped at execution time, raise BLOCKED.

1. **Share custody is metadata-only on the DHT.** `KeyStewardship` (`elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:669`) records `key_shard_holders: Vec<String>` (agent IDs) and a single `shard_commitment_hash` (used as an Ed25519 verifying key per `recovery_v2.rs:267-282`). The actual Shamir-share **bytes** are not on the DHT and have no current peer-to-peer custody-transfer protocol in elohim-storage. **Mitigation:** the cross-stack scenarios test the *gossip + attestation* surface (which is what gate #5 actually requires per spec line 514: "shares traverse whichever transport profile each peer supports"). Share-bytes custody is a separate epic; this plan stubs share-bytes as opaque blobs handed peer-to-peer over the existing blob plane (BLAKE3-keyed iroh-blobs / SHA-256 libp2p-blob), with the test asserting only "share-holder peer received notification AND fetched share-blob via its supported transport." The "share custody coordinator function" referenced in Task 5 is the `SubmitIntimateWitness` flow (`elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:2709`), which is the closest analog: it proves an emergency contact attests to the recovery (the social half of the recovery seed).
2. **No existing libp2p baseline test for recovery e2e.** `elohim/holochain/tests/sweettest/src/tests/recovery_m4.rs` covers coordinator correctness but is single-conductor + `#[ignore]`'d on real DNA artifact. There is no existing two-stack gossip-traversal recovery integration test. **Mitigation:** Task 7 *creates* the libp2p-only baseline first (in the same Rust file as the cross-stack test) so the cross-stack scenarios have a green peer to compare to.
3. **Per-share transport observability does not currently exist.** Today the gossip publish path emits one `tracing::debug!` per topic broadcast but does not tag the share recipient or the transport. **Mitigation:** Task 8 adds `tracing::debug!(target = "recovery::transport", share_holder_agent_cid = %h, transport = "iroh"|"libp2p", ...)` at the recipient side of each per-share fetch attempt. This is read-only observability — no protocol change.

---

## Task 1: Map the existing recovery flow (call-site catalog)

**Files (read-only research, write the catalog into this plan as a comment block at end of `tests/iroh_recovery_cross_stack.rs` for executor reference):** none modified.

This task documents what's wired today so subsequent tasks can plug in without redesigning the protocol. Output is the table below, copied verbatim into the integration-test file's module doc-comment in Task 7.

- [ ] **Step 1.1:** Confirm the file-line catalog below by `grep`. Each entry must resolve to the cited symbol; if any drifted (e.g., `p2p/mod.rs` line moved), update the line number in this plan in-place.

  | Recovery step | DHT entry / Wire | Source-of-truth file:line |
  |---|---|---|
  | Recovery invitation send (publish) | gossip publish on `recovery.invitation` | `elohim/elohim-storage/src/p2p/mod.rs:2383` (call site: `P2PCommand::PublishRecoveryInvitation` arm) |
  | Recovery invitation API (handle) | `P2PHandle::publish_recovery_invitation` | `elohim/elohim-storage/src/p2p/mod.rs` (find with `grep -n 'fn publish_recovery_invitation' src/p2p/mod.rs`) |
  | Recovery invitation wire schema | `RecoveryInvitation` struct, MessagePack | `elohim/elohim-storage/src/p2p/recovery_invitation.rs:22-29` |
  | Recovery invitation topic constant | `RECOVERY_INVITATION_TOPIC = "recovery.invitation"` | `elohim/elohim-storage/src/p2p/recovery_invitation.rs:16` |
  | RecoveryRequest DHT commit (coordinator) | `create_recovery_request` zome fn | `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:1883` |
  | RecoveryRequest DHT entry shape | `CreateRecoveryRequestInput` | `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:1674` |
  | Witness submission (share custody half) | `submit_intimate_witness` zome fn | `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:2709` |
  | Attestation gathering (storage projection) | `recovery_witnesses` table, populated by `RecoveryV2Signal::IntimateWitnessSubmitted` | `elohim/elohim-storage/src/db/recovery_witnesses.rs` + `elohim/elohim-storage/src/signals.rs:818` |
  | Attestation validation rules (pure-logic) | `check_intimate_quorum_rules` | `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs:160-237` |
  | Reassembly (key rotation commit) | `commit_key_rotation` zome fn | `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:2506` |
  | Cryptographic-quorum reassembly check | `check_cryptographic_quorum_rules` | `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs:241-298` |
  | KeyStewardship (share-holder roster) | `KeyStewardship` entry, `key_shard_holders: Vec<String>` | `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:669` |
  | Revocation send (publish) | gossip publish on `recovery.revocation` (legacy) and `elohim/integrity/revocation` (canonical) | `elohim/elohim-storage/src/p2p/mod.rs:2442` (call site: `P2PCommand::PublishRecoveryRevocation` arm) |
  | Revocation API (handle) | `P2PHandle::publish_recovery_revocation` | `elohim/elohim-storage/src/p2p/mod.rs:1173` |
  | Revocation wire schema | `RecoveryRevocationMessage`, MessagePack | `elohim/elohim-storage/src/p2p/recovery_revocation.rs:24-42` |
  | Revocation topic constants | `RECOVERY_REVOCATION_TOPIC` (legacy) / `TOPIC_INTEGRITY_REVOCATION` (canonical) | `elohim/elohim-storage/src/p2p/topics.rs:52` + `mod.rs:268` |
  | Self-revocation (coordinator) | `create_self_revocation` zome fn | `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:1948` |
  | Pending-recovery view (HTTP) | `GET /api/v1/account/pending-recovery` | `elohim/elohim-storage/src/api/account.rs:386` |
  | Vote-on-recovery write (HTTP) | `POST /api/v1/account/recovery/:id/vote` | `elohim/elohim-storage/src/api/account.rs:74` |

  Expected output: `grep -n` confirms each line; any drift is patched in-place in this table before Task 2 runs.

- [ ] **Step 1.2:** Confirm there is **no** existing two-stack recovery integration test (the plan asserts so in "Discovery Required" item 2). Run:
  ```bash
  grep -rln "recovery.*iroh\|iroh.*recovery\|cross_stack.*recovery\|recovery.*cross_stack\|recovery.*MultiStack" /projects/elohim/elohim/elohim-storage/tests/ /projects/elohim/genesis/a2o/features/ 2>/dev/null
  ```
  Expected output: no hits (only this plan file once it exists). If hits surface, the plan blocks pending review of the existing test for de-duplication.

---

## Task 2: Wire-interaction matrix (per recovery step → dual-stack readiness)

**Files:** none modified (table below is the deliverable, executed against by Task 4).

For each recovery step from Task 1, confirm the wire is dual-stack-ready per Plans 1+4. The matrix is the gate: every "Dual-stack ready?" cell must be **YES** before Task 5's harness can be considered passing. Cells marked **BLOCKS-ON-PLAN-N** are the inter-plan dependencies already called out under "Blocked Until."

- [ ] **Step 2.1:** Verify each row's "Wire interaction" column matches Task 1's catalog. If a row's wire is anything other than the four listed (gossip publish, EPR record, sync delta, blob fetch), the row needs investigation:

  | Recovery step | Wire interaction | Dual-stack ready? |
  |---|---|---|
  | Send recovery invitation | gossip publish (`recovery.invitation`) | **BLOCKS-ON-PLAN-4** (dual-publish wiring) |
  | Receive recovery invitation | gossip subscribe (`recovery.invitation`) | **BLOCKS-ON-PLAN-4** (dual-subscribe + receive-side projection) |
  | Commit RecoveryRequest to DHT | Holochain kitsune2 (Track 1) | **YES** — Track 1 is untouched by iroh/libp2p split per spec §"What lives where" |
  | Submit IntimateWitness (per share-holder attestation) | Holochain kitsune2 (Track 1) | **YES** — same as above |
  | Project recovery_witnesses to storage | local Holochain post-commit signal → SQLite | **YES** — local-only, no transport |
  | Fetch share-blob (out-of-band custody) | blob fetch (BLAKE3 iroh / SHA-256 libp2p) | **YES per spec §"Plane-by-plane verdict"** — blob plane is iroh-canonical with libp2p fallback (gate #2 covers this) |
  | Commit KeyRotation | Holochain kitsune2 (Track 1) | **YES** |
  | Send revocation | gossip publish (`recovery.revocation` + `elohim/integrity/revocation`) | **BLOCKS-ON-PLAN-4** |
  | Receive revocation | gossip subscribe (both topics) | **BLOCKS-ON-PLAN-4** |
  | Vote-on-recovery write | HTTP API (local to peer's elohim-storage) | **YES** — no inter-peer transport |

  Expected output: every row has a verdict. Two rows BLOCK on Plan 4; the rest are ready.

- [ ] **Step 2.2:** Confirm spec gate #4 (line 513) flags `recovery-invitation` and `recovery-revocation` as **permanent post-cutover** dual-publish (not just transitional). Cite the spec line to the executor.

  Expected: `grep -n "Permanent post-cutover" /projects/elohim/genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md` returns a line that includes "recovery topics".

---

## Task 3: Author the cucumber feature file

**Files:** create `genesis/a2o/features/auth/recovery/cross-stack/recovery-cross-stack-transport.feature`.

The five scenarios required by gate #5 + the standard a2o tagging convention from `genesis/a2o/CLAUDE.md`. Tags: `@e2e @auth @recovery-cross-stack @iroh @phase11-gate5`. Add `@wip` until Plans 1+4 land (Task 10 removes `@wip` once green).

- [ ] **Step 3.1:** Create the feature file with the five scenario shapes prescribed in the parent prompt. Use named personas matching the M3 happy-path convention (`Abby` claimant, `Ben/Cara/Dan/Evan` share-holders) so step-definition reuse is maximized.

  ```gherkin
  @e2e @auth @recovery-cross-stack @iroh @phase11-gate5 @wip
  Feature: Recovery completes across mixed iroh/libp2p share-holder transports

    Cutover gate #5 (spec 2026-05-08-iroh-libp2p-complementarity.md line 514).
    The recovery flow today is libp2p-canonical. Phase 11 makes recovery topics
    dual-publish (gate #4, permanent post-cutover). These scenarios prove that
    a recovery completes regardless of which transport profile each share-holder
    supports — the share-holder is reached via whichever wire it speaks.

    Background:
      Given a 5-node cross-stack fixture is running with these share-holder profiles:
        | name | transport |
        | Ben  | iroh      |
        | Cara | iroh      |
        | Dan  | libp2p    |
        | Evan | libp2p    |
        | Faye | dual      |
      And Abby is registered with required_witness_count of 3
      And each of Ben, Cara, Dan, Evan, Faye has a HumanRelationship to Abby with emergency_access_enabled = true

    Scenario: Recovery completes when all share-holders speak iroh
      Given Abby's required_witness_count is satisfied by share-holders ["Ben", "Cara", "Faye"]
      When Abby invokes create_recovery_request from a fresh agent key
      And each share-holder in ["Ben", "Cara", "Faye"] receives the recovery.invitation gossip
      And each share-holder in ["Ben", "Cara", "Faye"] submits submit_intimate_witness
      Then the recovery_witnesses projection for the request has count 3
      When Abby invokes commit_key_rotation with IntimateQuorum carrying witness_hashes for Ben, Cara, Faye
      Then the rotation succeeds
      And every share-holder receipt was tagged transport=iroh in the recovery::transport debug log

    Scenario: Recovery completes when all share-holders speak libp2p
      Given Abby's required_witness_count is satisfied by share-holders ["Dan", "Evan", "Faye"]
      When Abby invokes create_recovery_request from a fresh agent key
      And each share-holder in ["Dan", "Evan", "Faye"] receives the recovery.invitation gossip
      And each share-holder in ["Dan", "Evan", "Faye"] submits submit_intimate_witness
      Then the recovery_witnesses projection for the request has count 3
      When Abby invokes commit_key_rotation with IntimateQuorum carrying witness_hashes for Dan, Evan, Faye
      Then the rotation succeeds
      And every share-holder receipt was tagged transport=libp2p in the recovery::transport debug log

    Scenario: Recovery completes when share-holders are mixed (some iroh, some libp2p)
      Given Abby's required_witness_count is satisfied by share-holders ["Ben", "Dan", "Faye"]
      When Abby invokes create_recovery_request from a fresh agent key
      And each share-holder in ["Ben", "Dan", "Faye"] receives the recovery.invitation gossip
      And each share-holder in ["Ben", "Dan", "Faye"] submits submit_intimate_witness
      Then the recovery_witnesses projection for the request has count 3
      When Abby invokes commit_key_rotation with IntimateQuorum carrying witness_hashes for Ben, Dan, Faye
      Then the rotation succeeds
      And the recovery::transport debug log contains at least one transport=iroh receipt
      And the recovery::transport debug log contains at least one transport=libp2p receipt

    Scenario: Recovery proceeds when one share-holder is offline
      Given Abby's required_witness_count is satisfied by share-holders ["Ben", "Dan", "Faye"]
      And share-holder Cara is offline
      When Abby invokes create_recovery_request from a fresh agent key
      And share-holders ["Ben", "Dan", "Faye"] each receive the recovery.invitation gossip
      And share-holder Cara does NOT receive the recovery.invitation gossip
      And share-holders ["Ben", "Dan", "Faye"] each submit submit_intimate_witness
      Then the recovery_witnesses projection for the request has count 3
      When Abby invokes commit_key_rotation with IntimateQuorum carrying witness_hashes for Ben, Dan, Faye
      Then the rotation succeeds (k-of-n threshold met without Cara)

    Scenario: Recovery is rejected when a share-holder revokes
      Given Abby's required_witness_count is satisfied by share-holders ["Ben", "Dan", "Faye"]
      And Ben, Dan, Faye have each submitted submit_intimate_witness
      When share-holder Ben publishes a RecoveryRevocationMessage on the recovery.revocation topic
      Then the recovery_witnesses projection for the request reflects Ben's revocation
      And Abby's commit_key_rotation with IntimateQuorum carrying witness_hashes for Ben, Dan, Faye is rejected by the M2 validator
      And the rejection reason references the revoked HumanityWitness rule
  ```

  Expected output: file exists, 5 scenarios, all `@phase11-gate5 @wip` tagged, named-persona consistency with `intimate-quorum-happy-path.feature`.

- [ ] **Step 3.2:** Validate the feature file parses with cucumber-js dry-run (no step bindings yet — this just verifies Gherkin syntax):
  ```bash
  cd /projects/elohim/genesis/a2o && pnpm exec cucumber-js --dry-run --tags '@phase11-gate5' features/auth/recovery/cross-stack/recovery-cross-stack-transport.feature
  ```
  Expected output: 5 scenarios listed, all reporting "undefined" steps (which is fine for dry-run); zero parse errors.

---

## Task 4: Wire-mapping doc embedded in the step-def file

**Files:** create `genesis/a2o/steps/recovery-cross-stack.steps.ts` (header-only at this task; bodies stubbed `pending`).

The header doc-comment locks in the wire-interaction contract from Task 2 and the function-symbol contract from Task 1, so when Plans 1+4 land the step bodies (Task 6) plug straight in.

- [ ] **Step 4.1:** Create `genesis/a2o/steps/recovery-cross-stack.steps.ts` with the doc-comment + 13 stub Given/When/Then bindings (one per unique step phrase across the 5 scenarios). Bodies all return `'pending'` until Task 6 wires them.

  Header pattern (matches `account-m5.steps.ts:1-30`):
  ```typescript
  /**
   * Recovery cross-stack step definitions — gate #5 of iroh Phase 11 cutover
   * (genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md line 514).
   *
   * Framework: Cucumber-JS 11 + tsx (a2o convention).
   * Tag: @recovery-cross-stack — run with: pnpm exec cucumber-js --tags '@recovery-cross-stack'
   *
   * Wire-interaction contract (locked by 2026-05-10-iroh-recovery-e2e.md Task 2):
   *   - Send invitation: P2PHandle::publish_recovery_invitation_dual (Plan 4)
   *   - Receive invitation: gossip subscribe on both stacks (Plan 4)
   *   - Submit witness: imagodei zome fn submit_intimate_witness (Holochain Track 1)
   *   - Commit rotation: imagodei zome fn commit_key_rotation (Holochain Track 1)
   *   - Send revocation: P2PHandle::publish_recovery_revocation_dual (Plan 4)
   *   - Per-peer transport selection: PeerTransportManifest from Plan 1
   *
   * Step bodies remain `'pending'` until the Rust integration test (Task 7) is
   * green; the cucumber scenarios call into a thin HTTP helper exposed by the
   * MultiStackFixture's admin port.
   */
  import { Given, When, Then } from '@cucumber/cucumber';
  import { E2EWorld } from '../src/framework/world.js';
  ```

  Then 13 stub bindings (one per scenario step phrase). Body example:
  ```typescript
  Given('a 5-node cross-stack fixture is running with these share-holder profiles:', async function (this: E2EWorld, _table) {
    return 'pending'; // wired in Task 6 after MultiStackFixture exposes admin port
  });
  ```

  Expected output: file exists, all 13 step phrases bound, all return `'pending'`.

- [ ] **Step 4.2:** Re-run `cucumber-js --dry-run` from Task 3.2 — should now report all steps "skipped" (binding found, body pending) instead of "undefined".

---

## Task 5: Build the 5-node MultiStackFixture (Rust)

**Files:** create `elohim/elohim-storage/src/p2p_iroh/multi_stack_fixture.rs`; modify `elohim/elohim-storage/src/p2p_iroh/mod.rs` (add `pub mod multi_stack_fixture;`); both gated on `#[cfg(feature = "p2p-iroh")]` matching the existing `parity_harness` convention.

The fixture extends `TwoNodeFixture` (`src/p2p_iroh/parity_harness.rs:34`) to 5 nodes with assignable transport profiles. Reuses `loopback_config` and `IrohNode::start_with_protocols` exactly as `parity_harness` does — same async start pattern, same loopback discovery (`use_n0_relays: false`).

- [ ] **Step 5.1:** Define the fixture struct + transport-profile enum:
  ```rust
  //! Multi-stack fixture for cross-transport recovery e2e (gate #5).
  //!
  //! Extends `TwoNodeFixture` to 5 nodes with per-node transport profile
  //! assignment. Gates iroh-only / libp2p-only / dual-stack participation
  //! per node so the cross-stack scenarios can exercise the share-holder
  //! transport-mix matrix from cucumber feature
  //! `genesis/a2o/features/auth/recovery/cross-stack/recovery-cross-stack-transport.feature`.

  #![cfg(feature = "p2p-iroh")]

  use std::path::Path;
  use crate::p2p_iroh::{config::IrohConfig, IrohNode, parity_harness::loopback_config};

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum TransportProfile { IrohOnly, Libp2pOnly, Dual }

  pub struct NodeSlot {
      pub name: String,
      pub profile: TransportProfile,
      pub iroh: Option<IrohNode>,
      pub libp2p_handle: Option<crate::p2p::P2PHandle>, // None for IrohOnly
  }

  pub struct MultiStackFixture {
      pub nodes: Vec<NodeSlot>,
      pub iroh_addrs: Vec<iroh::NodeAddr>, // populated for nodes with iroh
  }

  impl MultiStackFixture {
      pub async fn new(
          dirs: &[(String, TransportProfile, &Path)],
          iroh_protocol_factory: impl Fn() -> Vec<crate::p2p_iroh::AlpnRegistration>,
      ) -> anyhow::Result<Self> { /* see Step 5.2 */ }
  }
  ```

- [ ] **Step 5.2:** Implement `MultiStackFixture::new` so each `NodeSlot` is wired per its profile:
  - `IrohOnly`: `IrohNode::start_with_protocols(loopback_config(dir), protocols)`; `libp2p_handle = None`.
  - `Libp2pOnly`: spawn libp2p `P2PHandle` via existing test harness pattern (mirror `tests/forwarder_integration.rs` or whichever existing test starts a libp2p `P2PHandle` standalone — find with `grep -n 'P2PHandle::' /projects/elohim/elohim/elohim-storage/tests/*.rs | head -5`); `iroh = None`.
  - `Dual`: both spawned; both populated.

  After all nodes spawn, populate `iroh_addrs` by `iroh_node.node_addr().await?` for every node with an iroh side. Cross-bootstrap: each iroh-side subscribes its `recovery.invitation` and `recovery.revocation` topics with the other iroh nodes' `NodeAddr`s as the `bootstrap` arg (per `IrohGossip::subscribe` API in `src/p2p_iroh/gossip.rs`).

- [ ] **Step 5.3:** Add a helper that returns the `PeerTransportManifest` (Plan 1 type) for any named slot, so the cross-stack peer-map can resolve "Ben → iroh-only" without duplicating the test data:
  ```rust
  impl MultiStackFixture {
      pub fn peer_transport_manifest_for(&self, name: &str) -> crate::p2p_iroh::peer_map::PeerTransportManifest {
          // BLOCKS-ON-PLAN-1: PeerTransportManifest field set
          // Implementation: read self.nodes' profile, populate libp2p_peer_id /
          // iroh_node_id Option fields per profile.
          unimplemented!("plan 1 dependency")
      }
  }
  ```
  Mark with a `// TODO(plan-1)` comment so the dependency is greppable.

- [ ] **Step 5.4:** Build:
  ```bash
  cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --features p2p-iroh
  ```
  Expected: clean compile (the `unimplemented!` from Step 5.3 is allowed because no test calls it yet — Task 7 calls it once Plan 1 lands).

---

## Task 6: Wire step-def bodies (HTTP-driven against MultiStackFixture admin port)

**Files:** modify `genesis/a2o/steps/recovery-cross-stack.steps.ts` (replace `pending` bodies with real bindings); modify `elohim/elohim-storage/src/p2p_iroh/multi_stack_fixture.rs` (add `pub fn admin_port(&self, name: &str) -> u16` returning per-node admin URL).

Bodies call into the per-node admin HTTP port (the same one the `account.rs` API exposes — every fixture node spins up the full `http.rs` server). Cucumber world stores `MultiStackFixture` admin URLs keyed by name.

- [ ] **Step 6.1:** Add an `admin_port` accessor to `MultiStackFixture` that returns the bound port for a given node name. Each spawned node's `http::serve` returns its bound port (existing pattern in `forwarder_integration.rs`).

- [ ] **Step 6.2:** Replace each step body with the real call:
  - `Given('a 5-node cross-stack fixture is running...')` — spin up `MultiStackFixture` via a helper test binary launched from the cucumber world (or, if the existing a2o convention prefers an external `local-stack`, defer to `genesis/a2o/scripts/local-stack.ts` extension; reading `genesis/a2o/scripts/local-stack.ts` in this task confirms the exact integration shape).
  - `Given('Abby is registered with required_witness_count of 3')` — POST to Abby's node `POST /api/v1/...` (mirror the M3 happy-path step's call pattern; find with `grep -n 'create_recovery_request\|create_human' /projects/elohim/genesis/a2o/steps/*.ts`).
  - `Given('each of Ben, Cara, Dan, Evan, Faye has a HumanRelationship to Abby...')` — POST `/db/relationship` (or the equivalent — confirm by reading `account.rs` route table) on each relevant node.
  - `When('Abby invokes create_recovery_request from a fresh agent key')` — POST to Abby's node calling the imagodei zome.
  - `And('each share-holder in <list> receives the recovery.invitation gossip')` — poll each share-holder's `GET /api/v1/account/pending-recovery` until non-empty (or timeout).
  - `And('each share-holder in <list> submits submit_intimate_witness')` — POST per share-holder.
  - `Then('the recovery_witnesses projection for the request has count 3')` — GET projection.
  - `When('Abby invokes commit_key_rotation with IntimateQuorum carrying witness_hashes for...')` — POST.
  - `Then('the rotation succeeds')` — assert HTTP 200.
  - `And('every share-holder receipt was tagged transport=iroh in the recovery::transport debug log')` — read each node's `tracing` log (Step 8 publishes via JSON file or per-node ring buffer; confirm with Task 8).
  - `Then('the recovery::transport debug log contains at least one transport=iroh receipt')` — same source, predicate "contains".
  - `Given('share-holder Cara is offline')` — kill Cara's `NodeSlot` via fixture `take()` helper.
  - `And('share-holder Cara does NOT receive the recovery.invitation gossip')` — assert Cara's pending-recovery is empty after the offline take.
  - `When('share-holder Ben publishes a RecoveryRevocationMessage on the recovery.revocation topic')` — POST Ben's node `/api/v1/account/self-revocation` (the existing self-revoke route triggers the gossip publish per `account.rs:74`).
  - `And('Abby's commit_key_rotation ... is rejected by the M2 validator')` — assert HTTP 4xx with body referencing `RECOVERY_AUTHORITY_LAYERS` validator rule.

- [ ] **Step 6.3:** Run dry-run again — all steps should report "passed" or "failed" (not "skipped"):
  ```bash
  cd /projects/elohim/genesis/a2o && pnpm exec cucumber-js --tags '@phase11-gate5' features/auth/recovery/cross-stack/recovery-cross-stack-transport.feature
  ```
  Expected: at this stage either all 5 scenarios fail with concrete error messages (Plan 1+4 not yet landed) OR pass (Plans 1+4 already landed). Per the "Blocked Until" section, failure here is the expected gating signal.

---

## Task 7: Rust integration test — Shamir-split → distribute → reassemble (loopback)

**Files:** create `elohim/elohim-storage/tests/iroh_recovery_cross_stack.rs`.

This is the lower-level Rust test that proves the dual-stack mechanics in isolation, without the cucumber + Holochain conductor overhead. Uses `MultiStackFixture` directly. Skips the DNA scenarios (those run via the cucumber harness in Task 6); this test asserts the **gossip + share-blob fetch round-trip** with stub witness signatures.

- [ ] **Step 7.1:** Module doc-comment includes the Task 1 catalog table verbatim (so future agents reading the test see the source-of-truth file:line refs).

  Header pattern matches `tests/iroh_sync_parity.rs:1-10`:
  ```rust
  //! Phase 11 acceptance gate #5 — recovery e2e cross-stack.
  //!
  //! Spec: genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md line 514.
  //! Plan: genesis/docs/superpowers/plans/2026-05-10-iroh-recovery-e2e.md.
  //!
  //! Asserts: a recovery invitation published from one peer reaches each
  //! share-holder via that share-holder's supported transport (iroh or libp2p),
  //! and each share-holder's witness submission travels back via the same
  //! profile lookup. Share-bytes custody is stubbed (opaque blob fetched via
  //! the share-holder's transport); the social-attestation half (HumanityWitness)
  //! uses the live coordinator path.
  //!
  //! Gated on `p2p-iroh`.
  #![cfg(feature = "p2p-iroh")]
  ```

- [ ] **Step 7.2:** Write `recovery_round_trip_all_iroh`, `recovery_round_trip_all_libp2p`, `recovery_round_trip_mixed` — three tests mirroring the cucumber scenario shapes 1, 2, 3. Each:
  1. Spawns `MultiStackFixture` with the relevant profile mix.
  2. Splits a fake 32-byte recovery seed via `shamirsecretsharing` crate (`shamirsecretsharing = "0.1"`) into `5-of-5` (one per share-holder) — matches `KeyStewardship.threshold_m=3, total_shards_n=5` semantics from `lib.rs:669`. **NOTE:** if `shamirsecretsharing` crate is not already a dep, do not add it — use a deterministic stub split (XOR with index) and document with `// stub: real Shamir at share-custody epic` per "Discovery Required" item 1.
  3. For each share-holder, calls `MultiStackFixture::peer_transport_manifest_for(name)` (Plan 1), publishes the share-blob to the share-holder's preferred transport (blob plane via existing `BlobStore` for libp2p, `IrohBlobStore` for iroh — both already exist per spec gate #2).
  4. Publishes `RecoveryInvitation` via `publish_recovery_invitation_dual` (Plan 4) on Abby's node.
  5. Polls each share-holder's `pending_recovery_requests` projection (read via `crate::db::recovery_requests::list_recovery_requests_for_agent`) until non-empty or 10s timeout.
  6. Asserts each share-holder fetched its share-blob and the per-receipt `tracing` event has `transport={expected}`.
  7. Reassembles by collecting 3-of-5 shares; asserts deterministic-stub reconstructs the seed.

- [ ] **Step 7.3:** Write `recovery_threshold_with_one_offline` (mirrors scenario 4) and `recovery_rejected_on_share_holder_revocation` (mirrors scenario 5).

- [ ] **Step 7.4:** Build + run:
  ```bash
  cd /projects/elohim/elohim/elohim-storage && \
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --test iroh_recovery_cross_stack -- --test-threads=1
  ```
  `--test-threads=1` per memory `feedback_env_var_test_flakiness` — these tests touch `ELOHIM_TRANSPORT_BACKEND` per-node and should not run in parallel.

  Expected (post Plans 1+4): all 5 tests pass. Pre Plans 1+4: tests fail at the `peer_transport_manifest_for` `unimplemented!()` (the planned blocker).

---

## Task 8: Per-share transport-tag observability

**Files:** modify `elohim/elohim-storage/src/p2p/blob_fetch.rs` (or wherever the share-blob fetch lands per Task 7 — confirm with `grep -n "race_fetch\|fetch_blob" src/p2p/`); add one `tracing::debug!` line at the recipient side.

This observability is needed for both (a) cucumber assertions in Task 6 (the "transport=iroh" in debug log assertions) and (b) the parity-soak (cutover gate #6) so reviewers can confirm cross-stack delivery actually happened over the week-long soak.

- [ ] **Step 8.1:** At the recipient side of every blob fetch, after the fetch resolves, emit:
  ```rust
  tracing::debug!(
      target: "recovery::transport",
      share_holder_agent_cid = %agent_cid,
      transport = %transport_label, // "iroh" | "libp2p"
      blob_hash = %hash,
      "share-blob received"
  );
  ```
  `transport_label` is derived from the dispatch arm chosen in the bridge (`TransportBackend::Iroh` → `"iroh"`, `TransportBackend::Libp2p` → `"libp2p"`).

- [ ] **Step 8.2:** Confirm there is no PII risk in the log line — `agent_cid` is the canonical content-derived agent identity (already public per `peer_map.rs:1`), `hash` is content-addressed.

- [ ] **Step 8.3:** Add the corresponding test assertion helper used by Task 7:
  ```rust
  fn count_transport_receipts(node: &NodeSlot, want: &str) -> usize {
      // Reads from a per-node tracing::Subscriber configured with a
      // ring-buffer collector. Implementation mirrors how
      // tests/iroh_*_parity.rs configures tracing for assertions
      // (find with: grep -n 'tracing_subscriber' /projects/elohim/elohim/elohim-storage/tests/iroh_*.rs).
      todo!("wire to ring-buffer subscriber per parity-test convention")
  }
  ```

- [ ] **Step 8.4:** Build:
  ```bash
  cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --features p2p-iroh --tests -- -D warnings
  ```
  Expected: clean.

---

## Task 9: Run all green (post Plan 1 + Plan 4 land)

**Files:** none modified.

Execute end-to-end. This task only fires after Plans 1+4 are merged on `dev`.

- [ ] **Step 9.1:** Rust integration tests:
  ```bash
  cd /projects/elohim/elohim/elohim-storage && \
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --test iroh_recovery_cross_stack -- --test-threads=1
  ```
  Expected: all 5 tests pass.

- [ ] **Step 9.2:** Cucumber a2o:
  ```bash
  cd /projects/elohim/genesis/a2o && pnpm exec cucumber-js --tags '@phase11-gate5'
  ```
  Expected: 5 scenarios pass.

- [ ] **Step 9.3:** Pre-push hook from repo root:
  ```bash
  cd /projects/elohim && git status
  ```
  Confirm only the four files this plan creates are dirty (multi_stack_fixture.rs, iroh_recovery_cross_stack.rs, recovery-cross-stack-transport.feature, recovery-cross-stack.steps.ts) plus the one-line `mod.rs` modification and the one-line `blob_fetch.rs` log line.

- [ ] **Step 9.4:** Remove `@wip` from feature file; re-run Step 9.2 to confirm green at the non-wip default tag.

---

## Task 10: Commit

**Files:** none modified beyond Tasks 5–9.

- [ ] **Step 10.1:** Stage:
  ```bash
  git add genesis/a2o/features/auth/recovery/cross-stack/recovery-cross-stack-transport.feature \
          genesis/a2o/steps/recovery-cross-stack.steps.ts \
          elohim/elohim-storage/src/p2p_iroh/multi_stack_fixture.rs \
          elohim/elohim-storage/src/p2p_iroh/mod.rs \
          elohim/elohim-storage/tests/iroh_recovery_cross_stack.rs \
          elohim/elohim-storage/src/p2p/blob_fetch.rs \
          genesis/docs/superpowers/plans/2026-05-10-iroh-recovery-e2e.md
  ```

- [ ] **Step 10.2:** Commit message:
  ```
  iroh phase 11 gate #5: recovery e2e cross-stack harness

  Cucumber feature + Rust integration test + 5-node MultiStackFixture
  prove that recovery completes across iroh-only / libp2p-only / mixed
  share-holder transports, plus k-of-n offline + revocation rejection
  scenarios. Per-share transport-tag debug log feeds the parity-soak.

  Wraps existing recovery flow (RecoveryRequest, IntimateQuorum,
  KeyRotation, KeyRevocation) — no DHT entry-type changes. Builds on
  Plan 1 (peer_transport_manifest) and Plan 4 (gossip dual-publish);
  recovery topics dual-publish permanent post-cutover per spec gate #4.

  Spec: genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md (line 514, gate #5)
  Plan: genesis/docs/superpowers/plans/2026-05-10-iroh-recovery-e2e.md
  ```

---

## Self-review checklist

- [x] Spec coverage: gate #5 (line 514) directly addressed, with explicit reference in feature file + Rust test header.
- [x] No `TBD`, `TODO` (only `// TODO(plan-1)` annotations marking explicit inter-plan dependencies — these are required, not placeholder), `appropriate`, or `similar to` placeholders.
- [x] All 5 prescribed scenario shapes present in `recovery-cross-stack-transport.feature`: all-iroh, all-libp2p, mixed (key cross-stack scenario), one-offline (k-of-n), revocation-rejection.
- [x] Step definitions reference real coordinator functions: `create_recovery_request` (`imagodei/zomes/imagodei/src/lib.rs:1883`), `submit_intimate_witness` (`:2709`), `commit_key_rotation` (`:2506`), `create_self_revocation` (`:1948`); real wire types: `RecoveryInvitation` (`p2p/recovery_invitation.rs:22`), `RecoveryRevocationMessage` (`p2p/recovery_revocation.rs:24`); real HTTP routes: `/api/v1/account/pending-recovery`, `/api/v1/account/self-revocation`, `/api/v1/account/recovery/:id/vote` (`api/account.rs:70-77`).
- [x] Inter-plan refs only use Plan 1 (`PeerTransportManifest`) and Plan 4 (`publish_recovery_invitation_dual` / `publish_recovery_revocation_dual`) APIs; both called out under "Blocked Until" with the exact symbols this plan assumes.
- [x] No new DHT entry types (mishpat is at 11/100 per project memory; recovery uses existing `KeyStewardship` / `RecoveryRequest` / `HumanityWitness` / `KeyRotation` / `KeyRevocation`).
- [x] Read-only research; no production code changes proposed beyond a single `tracing::debug!` line in `blob_fetch.rs` for observability (Task 8) and one `mod` declaration in `p2p_iroh/mod.rs`.
- [x] `--test-threads=1` cited per memory `feedback_env_var_test_flakiness` for tests that touch transport env vars.
- [x] Discovery-Required section enumerates the three pieces of the existing recovery flow that are not directly testable as drafted (share-bytes custody is metadata-only on DHT; no existing libp2p two-stack baseline; no per-share transport observability) — each with a concrete mitigation, none of which redesigns the protocol.

---

## Status

**COMPLETE — 2026-05-10.** All 10 tasks executed in worktree `worktree-iroh-recovery-e2e`. Plans 1+4 were already landed, so the execution-block was vacuously satisfied.

Gate results:
- `cargo check --features p2p,p2p-iroh --tests`: EXIT 0
- `cargo fmt --check`: EXIT 0
- `cargo clippy --features p2p,p2p-iroh --tests -- -D warnings`: EXIT 0 for all new files (pre-existing failure in `iroh_gossip_byte_parity.rs` from Wave 2 commit `0283e5adf`, unrelated)
- `cargo test --features p2p,p2p-iroh --lib`: 1380 passed, 0 failed
- `cargo test --features p2p,p2p-iroh --test iroh_recovery_cross_stack -- --test-threads=1`: **7 passed, 0 failed** (90s wall clock)
- `@wip` removed from feature file; scenarios are `@phase11-gate5` permanently

Commits: `d086fd7b4` (initial 7-file harness), follow-up warning-fix + @wip-removal (see worktree HEAD)

**Cutover gate #5 closed**: recovery-seed shares traverse whichever transport profile each peer supports.
