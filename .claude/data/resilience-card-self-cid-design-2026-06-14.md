# Design — light the EPR resilience card's data path (elohim-storage levers)

Workstream D of the EPR Content Durability Arc. Drives the corrected diagnosis in
`genesis/data/timeline/backlog/resilience-card-self-cid-provide-loop-gate.md`
(app-layer, NOT netpol) to done.

## Phase 0 — live alpha observation (read-only, via observability MCP)

Confirmed on live alpha (2026-06-14):

1. **Provide-loop dormant on every pod** — the "started" variant's `self_cid`
   structured log field never appears in recent logs; the disabled branch is the
   active branch fleet-wide. `SELF_CID` is set in no manifest → `config.self_cid`
   is `None` → the loop never spawns (main.rs:959-1072 `_ =>` disabled arm).
2. **Conductor boot-race / CellDisabled confirmed on jessica** — all three cell
   roles (lamad, infrastructure, reconcile-controller) stuck `CellDisabled`
   through 8+ retry attempts (~10 min). matthew/adam are healthy (0 restarts, no
   CellDisabled in window, live mesh).
3. **In-pod conductor viable on matthew/adam** — P2P mesh live (13+ peers, active
   inventory gossip). Crucially, the **libp2p P2P identity is established at boot
   independent of conductor cell-readiness** (loaded from `identity.key` before
   the HcClient bridge attempt) — so `self_cid` can be derived reliably from the
   peer_id even while the conductor is still booting.

Verdict: reseed viable on matthew/adam once the two elohim-storage levers land;
jessica needs its CellDisabled cleared first (operator/conductor-side, out of
this scope).

## The join-key contract (load-bearing — a mismatch silently empties the card)

`self_cid` MUST equal the value the snapshot joins on:
- custody sweep: `commitment.provider == self_cid` (reconcile/custody.rs:149)
- seeder resolves `provider`/`receiver` from `GET /p2p/status .peerId`
  (genesis/seeder/src/peer-id.ts) — the REAL libp2p peer id (`12D3KooW...`)
- `/p2p/status .peerId` ← `P2PStatusInfo.peer_id` ← `NodeIdentity::peer_id()`
  (the libp2p PeerId derived from `identity.key`)

Therefore `self_cid` derived at startup MUST be `NodeIdentity::peer_id_string()`
for the libp2p backend. `peer_id` depends ONLY on the keypair file
(identity.rs:206/219), not on `agent_pubkey`, and `load_or_generate` is
idempotent on the file — so deriving it early (before the provide-loop spawn at
main.rs:959) reads/creates the same `identity.key` the P2P node loads later at
main.rs:1118. They are guaranteed identical.

## Lever 1 — derive `self_cid` at startup when `SELF_CID` is unset

After the env-parse block (main.rs ~370), when `config.self_cid` is still `None`
AND the libp2p P2P path will be active (`args.enable_p2p &&
transport_backend == Libp2p`), derive it from `NodeIdentity::load_or_generate`
on `config.storage_dir.join("identity.key")` and set `config.self_cid`. Log the
source explicitly (`env` | `derived-libp2p-peer-id`). iroh backend left to env
(its node id is a different identity — a follow-up); on iroh-or-disabled P2P the
loop stays correctly dormant (no `/p2p/status .peerId` for the seeder anyway).

Effect: the provide-loop spawns on every libp2p node automatically →
`replicates-content`/`replicates-commons` authoring → `content:<reach>` provide
rows the snapshot reads.

## Lever 2 — re-anchor backfill (reach-circuit recovery, don't latch)

The seeder's TS circuit (off-limits) latches provenance-only when the in-pod
conductor's cells are CellDisabled during the seed window, leaving content rows
with `dht_anchor_hash IS NULL`. The elohim-storage-side recovery: a **one-shot
re-anchor backfill** spawned after the lamad HcClient bridge connects (cells
enabled). It walks content rows with `dht_anchor_hash IS NULL`, re-authors each
via `conductor_writes::call_create_content` (the exact null-anchor path
`update_via_conductor` already uses), whose `ContentCommitted` projection stamps
`dht_anchor_hash` + re-notarizes reach. Models the existing post-registry
`household_id backfill` (main.rs:2137) and `shard_manifest_backfill` (main.rs:522)
precedents: idempotent, NULL-only, non-fatal, bounded batch + inter-item pace,
acquires the bridge via the late-connect pattern so a cold boot self-heals.

This is "re-stamp the provenance-only rows once cells enable" — fully in
elohim-storage, no seeder change, no netpol.

## Lever 3 — observability flags on `/p2p/status`

Mirror the `ProjectionReconcileState` precedent (main.rs:1097 → builder 1210 →
status render 7045): a small shared `ProvideLoopState` created early, set when
self_cid is derived + when the loop spawns/disables + by the re-anchor backfill,
passed into `p2p_node` via a `with_*` builder, rendered into a new optional
`P2PStatusInfo.provideLoop` field. Surfaces: `selfCidSource`, `active`,
`reanchorPending`/`reanchorCompleted`/`reanchorFailed`/`reanchorCaughtUp`. So
"card dark because loop off / circuit latched" is visible without log-scraping.
Schema-first: add to `p2p-status-view.schema.json`, then Rust + ts-rs.

## p2p-design-gate

ZERO new DHT entities. `self_cid` is the node's existing libp2p identity (Path:
operational config derive). The re-anchor backfill re-authors EXISTING content
entries through the EXISTING `create_content` coordinator fn (no new entry type,
no new commitment kind). `ProvideLoopState` is Category-C operational status
(reconstructed, not persisted, not notarized) — same class as
`ProjectionReconcileStatus`. Gate clean → proceed.

## Verification

- `cargo build` + `clippy -D warnings` + full crate lib test (RUSTFLAGS getrandom
  custom, RUSTC_WRAPPER='', CARGO_TARGET_DIR=/tmp/..., plain cargo test, no pipe).
- `cargo test export_bindings` from elohim-views for the new ts-rs field.
- `cargo test --test schema_contract` for the p2p-status schema change.
- Unit test: self_cid derivation matches NodeIdentity::peer_id_string for a fixed
  identity.key; re-anchor backfill query selects only NULL-anchor rows.
