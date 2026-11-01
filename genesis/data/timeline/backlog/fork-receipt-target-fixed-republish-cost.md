---
id: "backlog-fork-receipt-target-fixed-republish-cost"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The conductor fork's validation-receipt target is a fixed 5 and a republish never collects a receipt from a peer that already holds the op — so on a three-conductor household no op ever completes, and matthew republishes all 73,533 of its ops on every sweep"
slug: "fork-receipt-target-fixed-republish-cost"
written: "2026-10-16"
author: "claude (rust-architect, measuring the content chrome's witness rule on the household)"
status: "backlog"
priority: "medium"
relatedNodeIds:
  - "genesis/a2o/features/protocol/content-chrome-trust-edge.feature"
  - "memory:project_full_arc_authority_disables_network_get"
tags: [conductor, holochain, kitsune2, publish, validation-receipts, household, performance, perf-scale, perf-io, perf-convergence, trustful-declared, friction-wait, friction-mechanical, custody-everyone, constraint, risk]
cites:
  - elohim/holochain-conductor/crates/holochain/src/conductor/cell.rs
  - elohim/holochain-conductor/crates/holochain/src/core/workflow/publish_dht_ops_workflow.rs
  - elohim/holochain-conductor/crates/holochain/src/core/workflow/incoming_dht_ops_workflow.rs
  - elohim/holochain-conductor/crates/holochain/src/core/workflow/validation_receipt_workflow.rs
  - elohim/holochain-conductor/crates/holochain_integrity_types/src/entry_def.rs
  - elohim/holochain-conductor/crates/holochain_p2p/src/op_store.rs
  - elohim/holochain-conductor/crates/holochain_conductor_api/src/config/conductor.rs
  - elohim/elohim-storage/src/services/content_witness.rs
shift_objective: |
  In the pinned conductor fork (elohim/holochain-conductor), make an op's receipt set able to
  complete on a small network, and let a republish collect the receipts it is still owed:
  (1) mark receipts complete at min(required_validations, remote authorities for the op's basis)
  instead of the fixed required count; (2) when a publish carrying require_receipt reaches a
  peer that already holds the op, have that peer send a receipt anyway (do not drop the flag in
  the existing-op filter). Prove both on the household: receipts_complete rows go from 0 to most
  of matthew's ops, and the per-sweep republish volume on matthew falls from ~73k ops to the
  ops genuinely still waiting.
---

# The receipt target cannot be met on a household, so every op is republished forever

Found while building the content chrome's witness reading (2026-10-16). The chrome first
proposed "green when the record's validation receipt sets are complete". The household
showed that rule can never turn green, and the reason is a substrate cost, not just a
chrome problem.

## What the fork does (pinned `elohim/holochain-conductor` @ 96bcd2bee)

1. **The target is a fixed number.** `cell.rs:655-700` takes the entry def's
   `required_validations`; anything else falls back to `DEFAULT_RECEIPT_BUNDLE_SIZE = 5`
   (`publish_dht_ops_workflow.rs:29`). `RequiredValidations::default()` is also 5
   (`holochain_integrity_types/src/entry_def.rs:5,111-114`). No entry type in our DNAs sets
   `required_validations`. The op is marked complete only when `receipt_count >= required`
   (`cell.rs:695`). Nothing scales the target to the number of authorities.
2. **One receipt per remote conductor, never from yourself.** A conductor signs one receipt for
   all of its local cells (`validation_receipt_workflow.rs:36-47`) and never sends one to an
   author it hosts (`validation_receipt_workflow.rs:126-127`). The `ValidationReceipt` table is
   `PRIMARY KEY ON CONFLICT IGNORE`, so a duplicate does not count twice. On a three-conductor
   household the most any op can collect is **2**.
3. **A republish never collects a missing receipt.** Only a publish asks for a receipt
   (`holochain_p2p/src/spawn/actor.rs:1972` sets the metadata; `op_store.rs:125-145` reads it;
   gossip carries none). When a republish reaches a peer that already holds the op, the op is
   filtered out before its flag is recorded (`incoming_dht_ops_workflow.rs:154-161`). A
   validator also drops the receipt for an author it does not see as recently online
   (`validation_receipt_workflow.rs:76-89`). Either way, an op that missed its receipt on first
   arrival stays without it.
4. **An incomplete op is republished every sweep.** The publish selection takes rows with
   `receipts_complete IS NULL` (`holochain_data/src/dht/inner/chain_op_publish.rs:110,190,279`),
   and the default `min_publish_interval` is 5 minutes (`holochain_conductor_api/src/config/conductor.rs:701-705`).
   matthew's `conductor-config.yaml` does not override it.

## Measured on the household (read-only, 2026-10-16)

Read with a scratch read-only SQL runner over `hc-dbtool`'s `db::open(.., Access::ReadOnly)`,
on matthew's lamad DHT database (`dht-uhC0kUQPSeKEnh5_…`).

- **0 of 73,533 `ChainOpPublish` rows are complete.** Receipt counts per op: 0 → 22,156;
  1 → 2,641; 2 → 48,736. The maximum is 2, as item 2 predicts.
- **The republish load.** At 14:37 UTC, 72,631 of the 73,533 rows had `last_publish_time`
  within the previous ~2 minutes: in effect, the whole authored set goes out again on each
  sweep, to both other conductors, indefinitely. A second sample at 16:47 UTC found the same
  72,565 rows republished 4 minutes earlier (the default sweep is 5 minutes).
  `evolution-of-trust`, seeded 2026-10-04, still carried today's publish time.
- **The permanent no-receipt share.** Of the records matthew's agent created, 2,388 of 7,603
  (31%) have no receipt on their record op, and 4,725 of 15,975 updates (30%). Some of these
  are 283 h old and still at zero, even though jessica's and james's conductors hold them.
- **How fast receipts do arrive when they arrive** (ops authored in the last 24 h): the first
  receipt after 7.5 s on average, 144 s at most. The second after 8.4 s on average, 128 s at
  most. Fresh records: `epr-app-deliverability-mvaduozv-…` got both receipts in 47–49 s;
  `epr-app-deliverability-mva1ubbx-…` got them in 0–1 s.

## Why it matters

- **Cost, `perf-scale`:** republish volume grows with everything the node has ever authored,
  not with what is still unwitnessed. The household pays it per sweep, on every conductor that
  authors. The alpha fleet's ratio of conductors to target differs, but the same fixed target
  and the same lost-receipt path apply there.
- **Honesty:** `receipts_complete` cannot be used as a witness signal on a small network. The
  content chrome therefore derives its witness from "any valid remote receipt, or a non-author
  that holds the record" (`content_store/src/record_witness.rs`, still to land per backlog `record-witness-coordinator-extern`, and `elohim-storage/src/services/content_witness.rs`),
  and has to spend one peer ask on the 31% that never got a receipt. A fix here would let the
  chrome drop that ask for most records.

## Shape of the fix (fork-side; not a DNA change)

- Complete an op at `min(required_validations, authorities for its basis that are not this
  conductor)`. On a full-arc household that is 2.
- In `incoming_dht_ops_workflow`, when a publish with `require_receipt` names an op the store
  already holds, queue a receipt instead of dropping the flag with the op.
- Measure before and after on the household: `receipts_complete` row count, and rows with a
  `last_publish_time` inside one sweep.

Readiness: the fork is ours and pinned by submodule; both changes are in the conductor, so they
reach the fleet through the conductor image, not a DNA-hash move. Open question for the fork
owner: whether the authority count should come from the peer store's arc view at publish time
or be fixed per network.
