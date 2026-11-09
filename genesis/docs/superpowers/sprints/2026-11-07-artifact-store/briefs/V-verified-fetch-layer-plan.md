---
id: artifact-store-plan-v-verified-fetch-layer
status: proposed
habits: [blob-durability, dataplane-convergence, sprint-lands-each-closure-once]
---

# Plan V — one verified-fetch layer under every byte path (proposed)

Proposed 2026-11-09 by the storage-closure shift (session f1769d1c), on the steward's request ("write that
plan"). Not dispatched. Slots **after the E4 storage closure lands and before E4b** flips the device to one
payload store. It serves `blob-durability` (one verify-before-write path to harden), `dataplane-convergence`
(two engines that answer the same question differently are a divergence source), and
`sprint-lands-each-closure-once` (one closure per adapter).

## 1. The problem, anchored

Fetching bytes by CID from holders is implemented twice, each with its own budgets, racing, verification,
resume and accounting:

| Engine | Where | Callers |
|---|---|---|
| eprfs sibling fetch (F2) | `elohim/eprfs/eprfs-peer/src/fetch.rs` (`fetch_artifact`, `fetch_artifact_until` :596/:615; daemon fetch queue :1291) over the tier ALPN (`eprfs-peer/src/tier.rs`) | `epr artifact fetch` (`epr-cli/src/artifact.rs`), the daemon's queue (`daemon.rs:1246`) |
| storage blob racing | `elohim/elohim-storage/src/p2p/blob_fetch.rs` (`FetchBudget` :55, `race_fetch_budgeted` :151, `verify_fetched_bytes` :504, `finalize_fetch_success` :360) and `p2p/blob_swarm.rs` (`race_fetch_with_swarm`, the libp2p+iroh plane race) | content reads, shard draws, custody heal; release adoption already reuses it (`services/release_adoption/artifact_pull.rs:63`, `RaceFetchPuller` :258) |

There are two more byte paths beside them: iroh-blobs (`p2p_iroh/blob_store.rs`, a separate copying `FsStore`;
`pull_core.rs`) and the HTTP `/blob/{cid}` route. E4 converges the store
(`crates/elohim-blob-store`). This plan converges the verb that fills it.

Consequences today: a budget, swap-refusal or trickling-holder fix lands in one engine and not the other; the
R1.4 FsStore question reads as "a second store?" rather than "a good holder adapter?"; libp2p/dual/iroh are
mode branches in two codebases; the same test intent (`oversized_manifest_refused_before_any_chunk_moves`,
swap refusal, resume) exists as two suites.

## 2. Target shape

A new crate `crates/elohim-fetch` (own workspace and lockfile, publishable like `crates/elohim-address`),
beside `crates/elohim-blob-store`. No new DHT entry type, table, HTTP route or wire message: it consolidates
existing verbs over existing records (p2p design gate to be run at V0 to confirm).

1. **`trait Holder`** — one device or plane that may answer by CID: `ask_manifest(artifact) -> Answer`,
   `ask_chunk(artifact, cid) -> Answer`, where `Answer` is bytes or a typed refusal (`OutsideStanding`,
   `NotUnderRoot`, `Unavailable{why}`, `NoBytePath`). A holder never asserts integrity; the engine verifies.
2. **Adapters** (each in its owning crate, implementing the trait): eprfs tier (eprfs-peer), storage libp2p
   request-response and iroh-blobs (elohim-storage), HTTP `/blob` (storage, for doorway-backed reads). A
   transport mode with no adapter is a typed gap (`unavailable(no tier path: <mode>)`), never a fallback.
3. **The engine** — `fetch(artifact, holders, budget, store) -> FetchOutcome`: budget checked before any
   allocation (`stall_deadline`, `total_deadline`, `manifest_size_cap`); manifest decode and
   `record.output == artifact`; per-chunk race across holders with cancel; each chunk verified against its CID,
   then the whole blob against BLAKE3, **before** write into the one `BlobStore` (shared instance); resume of
   missing chunks only; a trickling holder abandoned on deadline and the chunk re-raced within budget; an
   accepted record whose bytes cannot be fetched is a typed miss (`OutputNotFetchable`), never a verdict (I1, I4).
4. **One accounting emitter** — the custody `Use` (fetch) / `serve-blob` `Use` event shape lives in the crate,
   so both planes write the same flow events with quantities.
5. **One shared conformance suite** — the engine's tests run against an in-memory holder, and every adapter
   runs the same suite against its transport (swap refusal, oversized manifest, resume, trickling holder,
   unauthorized CID, never-held-twice across every lane).

## 3. Slices (one closure each; dispatch by tier)

| Slice | Content | Rolls alpha? | Gate | Leg |
|---|---|---|---|---|
| **V0 design gate** | p2p-design-gate answers; trait + error vocabulary + `FetchOutcome` drafted against both engines' current types; decide whether storage's `FetchBudget` or F2's budget is the base | no | read-only | Codex astra design leg, then orchestrator ruling |
| **V1 crate + engine + eprfs adapter** | `crates/elohim-fetch` with the engine and conformance suite; eprfs-peer `fetch.rs` becomes an adapter + caller; F2 behaviour byte-identical (its tests still pass unchanged) | no (eprfs only) | crate gate + `just gate eprfs` | Opus implementer; astra review |
| **V2 storage libp2p + iroh adapters** | `blob_fetch.rs`/`blob_swarm.rs` race logic replaced by adapters over the engine; release adoption keeps calling through `RaceFetchPuller`, now backed by the engine | **yes** (storage) | `just gate elohim-storage` + household `blob-durability` lane + `epr-app-deliverability` | Opus implementer; astra review; household receipt |
| **V3 HTTP `/blob` adapter + iroh ruling input** | doorway-backed reads as a holder; the R1.4 receipts re-read as adapter conformance, which is the input to the FsStore ruling | yes (storage) | as V2 | Sonnet + astra |
| **V4 delete** | the duplicated budget/race/verify code in both crates removed in the closure that replaces it (never both live); the second test suites folded into the conformance suite | with V2/V3 | — | — |

Then **E4b** (one payload store per device by default, `EPR_PAYLOAD_STORE` exported by hc-mesh, unmarked
directory migration) lands on a single fetch path, and **F3** (epr releases over the artifact store) and
**F5** (release-plane custody) arrive as callers, not new engines.

## 4. Done when

- One fetch engine in the tree; `rg 'fn race_fetch|fn fetch_artifact'` shows only the engine plus thin adapters.
- The conformance suite passes against every adapter, and a never-held-twice test spans every lane
  (tier, libp2p, iroh, HTTP) on one device.
- Household lanes `blob-durability` and `epr-app-deliverability` green on V2/V3; F2r (sibling fetch of the
  storage binary) re-run green through the engine.
- A one-line DELTA in `elohim/elohim-storage/.epr-meta/blob-durability.habit.md` and
  `genesis/orchestrator/.epr-meta/sprint-lands-each-closure-once.habit.md` per landed slice.

## 5. Risks and open questions

- **Behaviour drift in storage's race.** `race_fetch_with_swarm` carries Q3/Q4 swarm-pivot latency rules
  (`blob_swarm.rs`, "Returning on the first Manifest…"); the engine must keep them or V2 regresses read
  latency. Measure before/after on the household.
- **Budget semantics differ** between the two engines today; V0 picks one and names what the other loses.
- **iroh `FsStore`** still copies bytes; as an adapter it may need to stream into the shared store rather than
  hold its own copy — that is the R1.4 ruling, not decided here.
- **Licence.** Both new crates draw on AGPL storage code; epr-cli (Apache-2.0) links them. The steward's
  licence ruling (raised by E4) must precede V1.
- **Wall-clock.** Two storage closures (V2, V3) each roll alpha; bundle with other storage work where possible.
