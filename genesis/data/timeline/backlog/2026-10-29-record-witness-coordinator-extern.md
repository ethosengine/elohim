---
id: "backlog-record-witness-coordinator-extern"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Land the content_store coordinator extern get_record_witness_facts, so the content chrome's witness can read green"
slug: "record-witness-coordinator-extern"
written: "2026-10-29"
author: "claude (rust-architect, porting chrome-trust-edge layer 2 to port/chrome-trust-edge)"
status: "backlog"
priority: "medium"
relatedNodeIds:
  - "elohim/elohim-chrome-asset/.epr-meta/content-chrome-trust-edge.habit.md"
  - "genesis/data/timeline/backlog/fork-receipt-target-fixed-republish-cost.md"
tags: ["chrome", "content-chrome-trust-edge", "dna", "coordinator", "witness", "deferred-port"]
shift_objective: |
  GET /api/v1/epr/{id}/chrome reads witness "green" for a record that a conductor other
  than its author holds as valid, on a storage peer with ELOHIM_CONTENT_CHROME on. Done when
  the extern below is in the elohim DNA's content_store coordinator, the DNA hash is
  unchanged, the zome's sweettest covers it, and a household run of the content chrome's
  "amber until witnessed, green after" scenario passes.
---

## Why this is deferred

The 2026-10-23 snapshot of the content chrome's second layer (`47ba8da76`, branch
`feat/chrome-trust-edge`, also `refs/held/feat/chrome-trust-edge`) added a coordinator
extern, `elohim/holochain/dna/elohim/zomes/content_store/src/record_witness.rs`. The port to
`port/chrome-trust-edge` lands the storage side (`services::content_witness`) behind the
content-chrome switch, which is off by default, and leaves the zome change out because it
touches a DNA. Until the extern lands, `content_witness::check` gets an error from the call,
reads `WitnessEvidence::Unread`, and the facing says `amber`. Nothing reads green from
silence, so this is the honest reading of a missing fact, not a defect.

`elohim-storage/src/chain_write_gate.rs` already classifies `get_record_witness_facts` as a
read (`CRATE_READ_FNS`), because storage calls it by name and the drift test requires every
called name to be classified.

## Exact plan

1. Copy `record_witness.rs` from `47ba8da76` into
   `elohim/holochain/dna/elohim/zomes/content_store/src/` and add `pub mod record_witness;`
   to that zome's `lib.rs` (the snapshot put it after `pub mod republish_epr;`). The file is
   193 lines: `RECORD_OP_TYPES` (both the fork's `CreateRecord`/`UpdateRecord`/`DeleteRecord`
   and stock 0.7's `StoreRecord` spellings), `RecordWitnessFacts`, and one
   `#[hdk_extern] get_record_witness_facts(ActionHash)`.
2. Keep it coordinator-only. The DNA hash covers integrity zomes and modifiers only, so a
   coordinator change does not move it. Confirm before pushing: the hash of the packed DNA
   (with the role's `network_seed` and properties, as `just mesh preflight` hashes it)
   must equal the current one. If anything in the change touches
   `content_store_integrity`, stop: that moves the DNA hash and needs the operator.
3. It reaches running conductors through `happ_manager::sync_coordinators` and the
   conductor's `update_coordinators` hot swap (gated by `ALLOW_COORDINATOR_UPDATE`), with
   no reinstall and no new agent key. On the household, `just build` in
   `elohim/holochain/dna/elohim`, then `just mesh start` repacks the stale coordinator.
4. Re-read the extern before landing: it must stay read-only (one local receipt read, one
   local `get_details`, no network await, no commit, no link walk). That is what
   `CRATE_READ_FNS` asserts about it.
5. Add a sweettest in `elohim/holochain/tests/sweettest/`: author a record on one agent,
   then `exchange_peer_info` and `await_consistency` with a second agent, and assert that
   the second agent reads `held_valid: true` and `authored_by_caller: false`. Do not use
   `#[ignore]`: CI runs the sweettests with `--run-ignored all`.
6. Measure on the household with `ELOHIM_CONTENT_CHROME=1`: a record authored by one peer
   reads amber, then green once another conductor holds it. The fork's fixed receipt
   target of 5 (see `fork-receipt-target-fixed-republish-cost`) is why the extern counts
   valid receipts and does not read `receipts_complete`.
