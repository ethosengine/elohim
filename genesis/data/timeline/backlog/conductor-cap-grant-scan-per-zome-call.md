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

## Delta 2026-09-20 — remedy 3 carried to the doorway: both minters are now bounded

Remedy 3 ("stop minting — persist and reuse") was already delivered in `elohim-storage` (the closed-chain
fence, 2026-09-05). It is now delivered for the two doorway paths named above, which were the ones actually
growing. Neither path's auth posture, JWT validation, grant function scope, or caller set changed; only how
often a grant is authored.

**Path 1 — `zome_caller.rs::connect_endpoint` is bounded by a process credential cache.**
`doorway-service/src/services/signing_credentials.rs` holds one `ClientAgentSigner` per
(conductor admin address, installed app id) for the life of the process. `connect_endpoint` now decides
`Reuse | Mint` per cell (`decide_credential`, the same decide-before-you-author split as
`closed_chain_fence::decide`) and calls `authorize_signing_credentials` only for cells with no cached
credential. Nothing is written to disk — the doorway keeps no secret at rest — so a doorway RESTART re-mints
once per cell. Reconnect CHURN, which was the unbounded term, now mints nothing. Heal: a cap-grant-shaped
zome error (`looks_like_a_rejected_cap_grant`, a copy of storage's `CAP_GRANT_REJECTION_MARKERS`) discards
that cell's credential ONCE per (conductor, role) per process and drops the socket so the next call re-mints;
a second rejection surfaces unchanged. **Ceiling: one grant per cell per doorway process, plus at most one
heal per (conductor, role) per process — held under concurrency**, see below.

**Path 2 — the chaperone grants once per DEVICE, not once per page load.**
The browser now PERSISTS its signing keypair and cap secret per (doorway origin, agent) and presents the same
ones on every later connect
(`app/elohim-library/projects/elohim-service/src/connection/chaperone-credential-store.ts`). `localStorage`,
not IndexedDB: `elohim-service` compiles without the DOM lib and has no IndexedDB abstraction, and the one
precedent for client-side persistence there is `localStorage`. The credential record rides the key the
consuming app ALREADY writes after every connect and ALREADY removes on sign-out
(`holochain-signing-credentials`), so "clear on sign-out" is true today with no app change; a sibling
scope record keyed by (origin, agent) + signing-key fingerprint keeps one doorway's or one human's credential
from being presented as another's. Server side,
`doorway-service/src/conductor/grant_memory.rs` remembers a SHA-256 fingerprint of
(conductor, cell dna, cell agent, signing public key, cap secret) — one-way, no secret at rest — and
`handle_hc_connect` skips `grant_zome_call_capability` for a cell it has already granted to that exact
device. Recording happens only after a grant lands, so a `CellMissing`/`CellDisabled`/failed cell is retried
rather than skipped, and the browser's 3× 502/503 retry cannot multiply grants. Memory is in-process and
bounded (FIFO, 100 000 fingerprints): a doorway restart therefore costs at most one grant per device per role
cell, not one per page load. **Ceiling: one grant per (device, cell) per doorway process — held under
concurrency**, see below.

**Both ceilings are enforced, not merely intended.** `decide` … conductor round trip … `record` is a
check-then-act pair straddling an `.await`; two callers with the same identity would otherwise both observe
"not done" and both author a chain write. Since the device credential is now persistent, that is the COMMON
case — two browser tabs opening together carry byte-identical key material, hence one fingerprint — and on
the doorway's own side a primary connect can race the half-open re-probe. `doorway-service/src/keyed_lock.rs`
provides one `tokio::sync::Mutex` PER IDENTITY (fingerprint for the chaperone, conductor|app|role for the
credential cache), taken before `decide` and held across the round trip, so the second caller waits and
re-decides into `Skip`/`Reuse`. Distinct identities never contend (a wedged conductor for one human cannot
stall another's connect); the key map is reclaimed when its last holder and waiter drop; a cancelled request
releases its key via `Drop` and records nothing. The wait is bounded by the crate's per-conductor-call
deadline, and **expiry degrades to the pre-lock behaviour** — the caller proceeds unserialized, risking one
duplicate grant, never a failed connect and never a skip for a grant that did not land. Proven by tests that
use real `tokio::spawn` concurrency with a stub that parks inside the grant window, not by sequential loops.

Revoke-on-session-end was considered and REJECTED, for the reason this document already records: a deleted
grant is still dropped in step 2 — after its three queries are paid — so deletion only removes step 3's
share, and grant+delete per session doubles the writes on every hosted human's chain.

**Heal across the pair.** If the conductor loses a grant the doorway remembers (reinstall, or a database
restored from an older snapshot), the browser's zome calls come back unauthorized; the client discards its
stored credential and reconnects ONCE with a fresh keypair
(`DoorwayConnectionStrategy.healSigningCredentials`, bounded per strategy instance). That keypair is unknown
to `grant_memory`, so it is granted. One round trip, no loop on either side.

### The measurement that would show it

The mint COUNT per day is the measure, not the total — the ~15 000 accumulated rows stay until remedy 2
(indexed lookup) or a re-key, and this delta does not touch them.

- **Doorway-side, immediate.** `Chaperone: connection established` now carries `cells_granted` and
  `cells_skipped`. On a doorway serving returning browsers, `cells_skipped` should dominate within one
  session of a client rollout; `cells_granted` should fall to roughly (new devices + re-installs) × role
  cells per day. `Signing credentials ready` on the Path-1 side carries `minted` and `reused`: `minted`
  should be non-zero only on the first connect after a doorway restart.
- **Conductor-side, the real proof.** On matthew and adam, take the source-chain CapGrant action count (or
  the daily delta of the `CapGrant … ORDER BY Action.seq` statement's `rows_returned` in the slow-statement
  log) at the same hour on consecutive days. Before: it climbs with page loads and reconnects. After: it
  should be flat apart from new devices. susan, which fronts no public doorway, is the control — its rate
  should be unchanged.
- **The symptom to watch.** `elohim_conductor_admission_hold_ms` on matthew/adam does NOT improve from this
  change alone (the existing rows still cost ~47 000 queries per call). It improves when remedy 2 lands, or
  after a re-key. Reading a flat hold-time as "the fix didn't work" would be the wrong conclusion: the claim
  here is that the GROWTH stopped, and the mint count per day is the only thing that shows it.

### Still open after this delta

- **Remedy 2 (indexed `valid_cap_grants` in the fork)** remains the only thing that relieves the two stalled
  peers without a re-key. Unchanged in priority.
- **The 15 000 existing rows** are untouched. Deleting them buys only step 3.
- **Client heal wiring.** `looksLikeCapGrantRejection` + `DoorwayConnectionStrategy.healSigningCredentials`
  are implemented, exported and tested, but nothing in `app/elohim-app` calls them yet — the zome-call error
  path lives outside the library. Until that one call site exists, a conductor that loses a grant leaves that
  browser's calls failing until the human signs out (which clears the credential) or the doorway restarts
  (which forgets the grant and re-grants). Owner: the Angular layer; scope: one error-path branch.
- **The deadline fall-through is a deliberate, narrow duplicate-grant window.** If a first caller wedges
  past the per-conductor-call deadline, a second caller for the same identity proceeds unserialized and may
  author one duplicate grant. Failing the connect instead would turn a slow conductor into an outage, and
  skipping would leave the human with no grant at all; a bounded duplicate is the least-bad expiry. It
  disappears with remedy 2.
- **Server-side grant memory is in-process.** Promoting it to the doorway's MongoDB records would make a
  doorway restart free rather than one-grant-per-device. Strictly additive; `GrantMemory`'s API assumes
  nothing about residence.

**Cost to delivery, 2026-09-20 (genesis #1577, the first genesis run to reach seeding since the stall):** the seed of fixture humans failed for exactly the two conductors this item names — `Matthew … Request timed out in 60000 ms: call_zome` and `Adam … Request timed out in 60000 ms: call_zome; node+steward: not attempted (conductor unresponsive)` — and succeeded for the rest. A first-pass summary read these as a content problem; the console lines say otherwise. Until the per-call cost is bounded, genesis cannot seed the two heaviest chains, so app + genesis stay undeliverable on alpha regardless of what else is green. Separate and NOT this item's cause: Eve's seed failed with `CellDisabled(CellId(DnaHash(uhC0kRGwtzMN…AdFr), AgentPubKey(uhCAkhsVVjku…--Ks))` (×5; once for agent `uhCAkYCeIZu5…DY5v`) on `elohim-eve-alpha-conductor:4445` — a cell the conductor holds but has disabled, on the same conductor whose readiness timed out edge #1462. It needs its own read (why disabled, and whether a roll re-enables it) before it is filed as anything.

## Delta 2026-09-20 — remedy 2 is written, reviewed, and waiting on a mesh proof

The indexed lookup exists: `elohim/holochain-conductor` branch `perf/cap-grant-single-query`, commit
`28fc8ad6d`, on top of the pinned `25dd2d0be`. `valid_cap_grants` is one joined statement per access class
instead of `3N + ceil(N/500) + 1`. At matthew's measured 15 768 grants: 47 338 statements / 3 396 ms →
2 statements / 103 ms at the SQLite layer (33×; production gains more, since sqlx pays a pool acquire and an
async hop per statement). No schema change and no migration, so it reaches a node as a conductor image and
nothing else — no DNA hash move, no re-key. It is still linear in LIVE grants; the flat version is an indexed
`secret` column (measured 2.8 ms), which needs a Rust-side backfill because the secret lives only inside the
entry blob.

Two things the review changed, both worth keeping in mind when reading the code. (1) The first cut trusted
the denormalised `Action.entry_hash` column and never decoded `action_data`, which inverted fail-closed into
fail-open: the old path returned `Err` (deny-all) on a corrupt action, and `entry_hash` sits on the B-tree
leaf while `action_data` spills to overflow pages, so one bad block corrupts the blob and leaves the row
readable. Every live grant's action is decoded again, with the same decoder. (2) Revocation of an
UNRESTRICTED grant — the class that authorises anyone with no secret — and revocation attempted by a FOREIGN
author against a grant's public entry hash had no test anywhere; both are now pinned, and mutation-checked.

**The superproject pin has NOT moved, on purpose.** Moving it is a deploy intent (the conductor image builds
from that SHA and the edge rolls onto it). Order: build the conductor from `28fc8ad6d`, run it under the
household mesh, confirm zome calls authorise and the `CapGrant … ORDER BY Action.seq` slow statement is gone,
THEN move the pin. The fleet confirms; it does not discover. What the fleet should then show: the slow
statement disappears from matthew's and adam's conductor logs, `elohim_conductor_admission_hold_ms` on both
converges on susan's, and genesis seeds Matthew and Adam (the #1577 failure above) — that last one is the
delivery-level proof.

Filed separately, NOT caused by this change (identical before and after): on the cap-grant read a public
`Entry` row outranks a same-hash `PrivateEntry`, and `cache_chain_ops` inserts a network-supplied
`(hash, blob)` pair. If that hash is not re-derived from the content before insert, a peer could plant a blob
of its choosing under a victim's grant entry hash and influence an authorisation decision without owning the
node. Unverified; needs a bounded read of the op-integration path.
