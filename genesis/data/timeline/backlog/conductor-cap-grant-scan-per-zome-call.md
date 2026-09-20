---
id: "backlog-conductor-cap-grant-scan-per-zome-call"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Every zome call storage makes costs ~47 000 SQL queries on matthew — the conductor re-reads 15 000 capability grants per call, and storage minted them"
slug: "conductor-cap-grant-scan-per-zome-call"
written: "2026-09-19"
author: "pipeline-shakeout shift pickup (2026-09-19)"
status: "backlog"
priority: "critical"
tags: [conductor, capability-grants, zome-call-cost, trust, alpha-fleet, holochain-fork]
jobs: [elohim-edge, elohim]
cites:
  - genesis/data/timeline/backlog/conductor-admission-saturated-for-hours-after-restart.md
  - elohim/elohim-storage/src/hc_client.rs
  - elohim/elohim-storage/src/closed_chain_fence.rs
  - elohim/elohim-storage/src/signing.rs
---

# The conductor re-reads every capability grant on every zome call

This is the root cause behind `conductor-admission-saturated-for-hours-after-restart`: why matthew's and adam's
conductors hold every admission permit for the full 60 s call timeout, why no head can be authored through either
public doorway, why the app pipeline cannot deliver, and what is pegging the CPU. The operator ruled out raising CPU
limits (2026-09-19) — correctly: all seven conductors are equally throttled and susan answers in a millisecond.

## Measured

Conductor slow-statement log, `elohim-matthew-alpha-conductor-0`, 2026-09-19 19:20–20:20Z, the `CapGrant … ORDER BY
Action.seq` statement:

| rows_returned | elapsed |
|---|---|
| 11 619 | 4.3 s |
| 15 768 | 6.3 s, 7.6 s |
| 18 516 | 8.7 s |

Three distinct counts — one per cell storage talks to. Six-hour counts of that slow statement: matthew 1 500, adam
694 (sampled 1 761 and 6 528 rows), susan 70. The companion statement, the 500-hash `Entry ∪ PrivateEntry` batch:
matthew 18 390, adam 7 320, susan 4.

## Mechanism (read from the fork)

Storage never signs a zome call with the cell's own agent key; it signs with a keypair it registered through
`authorize_signing_credentials`. So `ZomeCallInvocation::verify_grant` (`holochain/src/core/ribosome.rs:402`) always
takes the remote-caller branch → `SourceChain::valid_cap_grant` (`source_chain.rs:815`) →
`DhtStoreRead::valid_cap_grants` (`holochain_state/src/dht_store/reads.rs:2312`), which, **per zome call**:

1. loads every grant row of the matching access class authored by the agent (`get_cap_grants_by_access`);
2. for EACH row: `get_action` (1 query) and `entry_updated_or_deleted_by_author` (2 queries) — the source says
   "stay one-by-one (cap-grant counts are small)";
3. fetches every surviving grant entry in chunks of 500 (`get_entries_by_hashes`, `CHUNK_SIZE = 500`) and
   deserialises all of them;
4. returns them all; the caller then tests each for the one secret it was looking for.

At 15 768 grants that is ~47 000 SQL queries plus 32 batch reads of ~2 s — per call. 32 × 2 s is the 50–60 s hold.
The design assumes a handful of grants; the lookup never uses the secret it was given to find the grant.

## Where the grants come from

`elohim-storage/src/hc_client.rs:451-469` calls `authorize_signing_credentials` for three cells on every connect, and
`closed_chain_fence.rs:271` records that each call "COMMITS a CapGrant". Nothing revokes a superseded grant. Today's
mint rate is small (connects in 12 h: adam 21, susan 11, matthew 7) and does not separate the peers, so the
thousands are accumulated history on the two chains that are never re-keyed — the genesis pair.

**Corrected 2026-09-19 — storage is not the minter, and the second caller is named.** Two claims above were left
open; both now have answers read from source.

*Storage stopped minting per connect on 2026-09-05* (`0fb64726e`, `ddaf54383` — the post-close write fence, Task 32).
Rail 1 of `closed_chain_fence.rs` persists one credential per cell and reuses it forever; `decide`
(`closed_chain_fence.rs:475`) returns `Reuse` before any conductor call. The fence is armed in the composition root
at `main.rs:872`, before anything dials a conductor, and **all three** storage mint sites route through it —
`hc_client.rs:133` (`authorize_signing_credentials_fenced`, the bridge), `signing.rs:193`
(`ConductorSigningClient`), and `reconcile/holochain_app_signal.rs:652` (the signal stream, which reconnects on
every conductor restart). Credentials live at
`<storage_dir>/closed-chain-fence/credentials/<dna39hex>-<agent39hex>.json`, 0600 inside a 0700 directory, and
`STORAGE_DIR=/data` on alpha is a per-pod `openebs-hostpath` PVC (`genesis/orchestrator/manifests/edgenode/alpha.yaml`
`volumeClaimTemplates: storage-data`), so the durability rail 1 depends on is actually present on the fleet. Storage's
ceiling is therefore **one mint per cell per data-dir lifetime**, plus at most one heal per cell per process
(`discard_stale_credentials`, bounded by a `healed` set). Remedy 3 as scoped — "storage stops minting" — was already
delivered, and it did not stop the growth, because storage was never the dominant minter.

*The other first-party caller is the doorway, and it has two unfenced minting paths, neither of which persists
anything.*

1. `doorway/doorway-service/src/services/zome_caller.rs:805` (`connect_endpoint`) authorizes signing credentials for
   **every provisioned cell** of the conductor it dials, on every connect, and a fresh `ClientAgentSigner` is built
   per connection by design (the per-conductor credential crux, module doc lines 32-37). Any transport-dead
   classification clears the socket and the next call reconnects; the same module records observed
   NXDOMAIN/WebSocket-reset churn (lines 228-232). Each churn cycle is one CapGrant per cell on that conductor's
   agent chain.
2. `doorway/doorway-service/src/conductor/chaperone.rs` (`POST /hc/connect`) calls `grant_zome_call_capability` once
   per role cell per browser session, tagged `chaperone-<role>`. The browser mints a **fresh** keypair and cap secret
   on every connect and keeps them in memory only
   (`app/elohim-library/projects/elohim-service/src/connection/doorway-connection-strategy.ts:384-386`), so no grant
   is ever reused, and the client retries the call up to three times on 502/503.

Path 2 is an unbounded per-browser-session minter pointed at exactly the conductors the two public doorways front,
which is the shape that fits the measurement: 11 619 / 15 768 / 18 516 rows on matthew and adam against susan's
sampled handful, and three distinct counts because each role cell carries its own source chain. Neither doorway path
is in `elohim-storage`, so neither is reachable by a change to the peer binary.

## What does NOT fix it

- **More CPU.** It buys a constant factor on a cost that grows with every reconnect.
- **Deleting old grants.** A deleted grant is dropped in step 2 — after its three queries have been paid. Deletion
  removes only step 3's share.
- **Restarting the conductor.** The grants are on the source chain.

## Remedies, by the operator's three classes

1. **TRUST — storage signs as the agent it is.** A node's own storage is not a remote capability-bearer; it is the
   agent's own process beside the agent's own keystore. Signing zome calls with the agent key (through lair) takes
   `verify_grant`'s author fast path and the grant table is never read. This is the design answer and removes the
   class. Needs a design pass: keystore access from the storage process, the hosted-human cells on pool conductors
   (whose keys the doorway custodies), and the closed-chain fence.
2. **STRUCTURAL, in the fork we own — find the grant by its secret.** Make `valid_cap_grants` an indexed lookup
   (secret → grant) instead of load-all-then-filter, and replace the per-row `get_action` + two modification reads
   with one joined query. Cost per call becomes independent of grant count. Reaches the fleet as a conductor image
   through the submodule pin; helps every caller, including hosted humans.
3. **STOP MINTING — persist and reuse one signing credential per cell** beside the agent key instead of authorizing a
   new one per connect. First-party, small, stops the growth; does nothing for the 15 000 already there.
   **In `elohim-storage` this is done** (the closed-chain fence, 2026-09-05 — see the correction above), and the
   remaining growth is the doorway's, not the peer's. The fence's one durability gap was closed 2026-09-19:
   `write_private` truncated the credential in place, so a process killed mid-write (OOM kill, eviction, RAM guard)
   left a torn file — recoverable on an OPEN cell, which discards and re-mints, but on a CLOSED cell `decide` refuses
   by name and never re-mints, leaving the role unconnected until an operator deletes the file. It now writes a
   sibling temp, `sync_all`s it, and `rename(2)`s over the target, the same shape as
   `runtime_config::set_watched_key`. Carrying the remedy to the doorway means giving `connect_endpoint` the same
   per-cell persisted reuse, and giving the chaperone a per-session grant that is either reused or revoked — a
   browser-held key that is never revoked is the growth, and it is a doorway concern, not a storage one.
4. **BATCH / RATE-LIMIT** apply to the second caller of the same batch read, `source_chain_records`: three zome sites
   still `query(ChainQueryFilter::new().include_entries(true))` over the whole chain (`content_store/src/lib.rs:16205`,
   `imagodei/qahal_coordinator.rs:409`, `imagodei/lib.rs:4853`); `node_registry_coordinator/src/lib.rs:1854` was
   claimed here as already-fixed with an `ActionSeqRange`-bounded read — that claim was wrong, sourced from the
   coordinator's own doc comment, which was itself wrong: on this fork `SourceChain::query` batch-loads EVERY entry
   on the WHOLE chain before `ChainQueryFilter` (including `ActionSeqRange`) applies — no pushdown, so a
   `sequence_range`-bounded `include_entries(true)` still pays for the whole chain. Corrected 2026-09-19: of the
   three sites originally listed, `content_store::get_my_custody_epr_scopes` and
   `imagodei::qahal_coordinator::get_my_household_collective_cids` are now headers-only two-phase reads (this fix
   landed on both, same day); `imagodei::query_my_source_chain` (`imagodei/lib.rs:4853`) legitimately needs every
   entry — it is a full source-chain dump — and is unchanged. The coordinator's own `export_records` window-load and
   `existing_seal`'s witness scan (`node_registry_coordinator/src/lib.rs`) got the same headers-then-materialise
   split the same day. Coordinator-only, so it hot-swaps without moving the DNA hash.

Order that the evidence supports: **2 first** (it is the only one that relieves the two stalled peers without a
re-key), **3 with it** (stops the growth), **4** alongside as the cheap coordinator change, **1** as the design that
retires the problem.

## Missing nodes

- chain / between "storage calls a zome" → "the conductor runs the zome" / missing node "authorising the caller costs
  the same whether the agent has three grants or thirty thousand": probe — conductor slow-statement count for the
  `CapGrant` statement per peer, and `elohim_conductor_admission_hold_ms` on matthew and adam converging on susan's.
  State: **not built**.
- The conductor exports no metrics and only storage pods are profiled, so its cost was invisible until a person read
  its logs. Smallest instrument: a gauge of rows scanned in `valid_cap_grants`, labelled by cell. State: **not built**.
- `elohim_conductor_admission_*` carries no zome / function / caller label, so "which first-party loop calls most"
  cannot be answered from metrics today. State: **not built**.
