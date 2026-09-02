---
id: ark-s1-station2-custody-plan
status: active
cites:
  - "compute-envelope-tevah | Tevah | sha256:25153362aae54306 | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - "ark-s0-launcher-plan | 2026-09-02-ark-s0-launcher-plan | sha256:12ad4b4fd51225b8 | path: genesis/docs/superpowers/plans/2026-09-02-ark-s0-launcher-plan.md"
---

# Tevah S1 · Station 2 — the custodians Jessica already has hold the witness — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On the ark-launched household mesh, a SIGKILLed conductor's death witness is held by the two custodians who committed to custody Jessica's spool, with the same content digest, within sixty seconds, and each custodian's own storage records that it received the witness from Jessica — station 2 of `@concern:death-witness`, green through `just test mesh '@concern:death-witness and @station-2'`.

**Architecture:** Nothing new at the DHT. Jessica's **storage pulls** the ark spool (`ELOHIM_ARK_SPOOL_PATH`, off when unset) and, row before blob, writes a `content` row at `reach: private` (`content_type: issue-report`, `metadata.kind: death-witness`, `id` = the witness CID) then stores the `.cbor` bytes in the pantry — whose raw-codec CID wraps the **same sha256** the witness's dag-cbor CID carries. Custodians hold a standing `custody-spool` commitment they **authored on their own conductor** (authorship is the counter-signature); a custodian-side `SpoolCustodyAuthor` expands it into one `custody-blob` per advertised witness, the existing custody reconcile pulls the bytes, and the existing `serve-blob` economic event on the custodian is the receipt. The mesh dials the custody sweep and inventory broadcast to 15 s so the 60 s budget is reachable.

**Tech Stack:** Rust (`elohim/elohim-storage`, diesel, tokio; storage's own workspace and pool slot), `elohim-ark-core` (read the witness), TypeScript seeder (`genesis/seeder`, `@holochain/client`), Cucumber a2o (`genesis/a2o`), Bash (`hc-mesh.sh`, `hc-mesh-prologue.sh`).

**Spec:** `genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md` §6 (witness path, `amber-offered`), §7 (C-rows for custody), §12 items 10, 11, 12, 16. **Grounding (2026-09-02, verified against code):** custody rides the **elohim DNA** `Commitment` through `content_store::create_rea_commitment`, which has no action whitelist and whose integrity zome has no commitment validation — `custody-spool` needs **no zome edit**; the mishpat coordinator's exhaustive match governs a different 3-field homonym. The spec's `attestation:custodian-commitment` kind exists and is untouched: provider = author already binds the custodian. Corrections to carry into the spec at Task 7.

## Global Constraints

- **P2P design gate (run 2026-09-02):** `custody-spool` = Notarized (A), agent-scoped composite id `custody-spool-<sha256(provider|receiver|"spool:witness:<ward>")[..16]>`, existing `Commitment` type in `content_store_integrity`, DNA-hash-NEUTRAL with no coordinator edit, authored on the custodian's conductor; the receipt = Ephemeral (C), the existing `serve-blob` `economic_events` row. The witness content row rides `issue-report` + `metadata.kind: death-witness` (spec §7). Zero new tables, routes, entry types, or attestation kinds. Any deviation is a plan amendment, not an executor call.
- **Identity:** one digest, two renderings. `content.id` = the witness CID (`bafyrei…`); `content.blob_hash` = the legacy `sha256-<hex>` marker the custody/inventory planes speak; `content.blob_cid` = the raw-codec CID (`bafkrei…`). `BlobStore::parse_content_address` keys on the multihash digest and accepts all three. Never mint a fourth form.
- **Row before blob** (spec §6.3): a blob with no content row serves to anyone (`blob_reach.rs`). The content row is written first at `reach: private`; a crash between the two leaves an unservable row, never an unguarded blob.
- **Storage pulls; the ark never pushes.** `ark-supervisor` has no network by construction; a localhost POST is refused. The watcher is a std poll loop after the `runtime_config.rs` precedent (`PATH_ENV`), OFF when the env var is unset.
- **Custodian authors, not the ward.** The ward's conductor is the thing that died. `SpoolCustodyAuthor` runs on every peer holding an active `custody-spool` with `provider == self`, through its own live conductor (`SalvageCommitmentAuthor` is the exact precedent, including `resolve_self_agent_cid` → skip the tick when `None`; never write a transport id).
- **Transport:** the receipt exists only on the libp2p fetch path; the mesh runs `dual`, so the drill is pinned to it. The iroh receipt is a named missing node (M7), not designed around.
- **Build environment (storage is its own workspace):** `cd /projects/elohim/elohim/elohim-storage && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev RUSTFLAGS='--cfg getrandom_backend="custom"' cargo <cmd>` ending with `; echo EXIT=$?`. `cargo test --lib <module>` for focused runs; clippy `-D warnings` on the crate. The mesh binary slot is the same path: after a storage change, `cargo build --features "p2p p2p-iroh" --bin elohim-storage` into that slot, then `just mesh storage-restart <peer…>` (the orchestrator does this in the foreground; executors never touch mesh processes).
- **Managed surfaces:** never hand-edit `CLAUDE.md` or `.claude/skills/**` — they are projections of `.epr-meta/elohim/packages/*`; edit the package body and run `pnpm run elohim-agent:packages:project`. This plan touches neither.
- **Commits:** exact-file `git add <paths>` and `git commit -m "…" -- <paths>` (never the directory; another lane commits concurrently). Never push, never `kubectl`, never start/stop mesh processes. Trailer: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.

---

## File structure

```
elohim/elohim-storage/src/services/spool_ingest.rs        NEW  SpoolIngest: poll <spool>/witnesses/*.cbor → content row (private) → BlobStore::store; idempotent on CID
elohim/elohim-storage/src/services/mod.rs                 modify: pub mod spool_ingest; pub mod spool_custody_author;
elohim/elohim-storage/src/config.rs                       modify: ark_spool_path: Option<PathBuf> (env ELOHIM_ARK_SPOOL_PATH), ark_spool_poll_seconds (default 5)
elohim/elohim-storage/src/main.rs                         modify: spawn SpoolIngest when configured; dispatch SpoolCustodyAuthor beside the custody sweep
elohim/elohim-storage/src/blob_store.rs                   modify (tests only): witness_cid_and_blob_address_share_one_digest
elohim/elohim-storage/src/services/spool_custody_author.rs NEW  SpoolCustodyAuthor: custody-spool (provider==self) × advertised death-witness blobs → custody-blob via own conductor
elohim/elohim-storage/src/services/rea_commitment_service.rs modify: deterministic_spool_custody_id + SPOOL_CLASSIFICATION helpers (pure)
genesis/seeder/src/seed-spool-custody.ts                  NEW  each custodian authors custody-spool toward the ward on ITS OWN conductor
app/elohim-app/scripts/hc-mesh-prologue.sh                modify: spool-custody leg after household formation
app/elohim-app/scripts/hc-mesh.sh                         modify: ark mode exports ELOHIM_ARK_SPOOL_PATH=<peer data_root>/ark per storage peer, CUSTODY_SWEEP_SECONDS=15, INVENTORY_BROADCAST_SECONDS=15
genesis/a2o/steps/mesh/death-witness.steps.ts             modify: three station-2 steps
genesis/a2o/features/resilience/death-witness.feature     modify: station 2 loses @wip
elohim/elohim-storage/.epr-meta/runtime-death-witnessed.habit.md   DELTA (Task 7)
genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md   §7/§12 corrections (Task 7)
```

---

### Task 1: `SpoolIngest` — storage picks up the ark spool, row before blob

**Executor:** Codex. **Reviewer:** Opus.

**Files:** create `elohim/elohim-storage/src/services/spool_ingest.rs`; modify `src/services/mod.rs`, `src/config.rs`, `src/main.rs`. Read first: `src/runtime_config.rs` (the `PATH_ENV` poll precedent), `src/services/content_service.rs:111` (`ContentService::create(CreateContentInput)`), `src/db/content_diesel.rs` (`CreateContentInput` fields — find `blob_hash`, `blob_cid`, `reach`, `metadata`, `dht_anchor_hash`), `src/blob_store.rs:194` (`BlobStore::store(&[u8]) -> StoreResult`), `src/blob_reach.rs` (why the row comes first), `elohim/ark/core/src/witness.rs` (`DeathWitness`, decode with `serde_ipld_dagcbor::from_slice`, `cid()`), `elohim/ark/supervisor/src/spool.rs` (the on-disk layout: `witnesses/<cid>.cbor` + `.json`, written tmp+rename so a `.cbor` is complete when visible).

**Interfaces:**
```rust
pub struct SpoolIngestConfig { pub spool_root: PathBuf /* <data_root>/ark */, pub poll: Duration /* default 5 s */ }
pub struct SpoolIngest { /* content: Arc<ContentService>, blobs: Arc<BlobStore>, cfg, seen: HashSet<String> */ }
impl SpoolIngest {
    pub fn new(cfg: SpoolIngestConfig, content: Arc<ContentService>, blobs: Arc<BlobStore>) -> Self;
    /// One pass: list witnesses/*.cbor, skip CIDs already ingested (content row exists OR in `seen`), for each new file: decode → verify `DeathWitness::cid()` == filename stem (refuse and log on mismatch) → `ContentService::create` (row, reach private) → `BlobStore::store(bytes)` → assert digest == content.blob_hash digest. Returns the CIDs ingested this pass.
    pub async fn run_once(&mut self) -> Result<Vec<String>, StorageError>;
    pub fn spawn(self) -> tokio::task::JoinHandle<()>;   // loop { run_once; sleep(poll) }, errors logged and retried, never fatal
}
pub fn witness_content_input(w: &DeathWitness, cid: &str, bytes: &[u8]) -> CreateContentInput; // pure: id=cid, content_type "issue-report", title "death witness: <process> <exit class> (<died_at>)", reach "private", metadata JSON {kind:"death-witness", incident, process, pid, exit, died_at_epoch_ms, artifact_sha256}, blob_hash "sha256-<hex>", blob_cid "bafkrei…", dht_anchor_hash None
```
`config.rs`: `ark_spool_path: Option<PathBuf>` from `ELOHIM_ARK_SPOOL_PATH` (env wins; TOML optional), `ark_spool_poll_seconds: u64` default 5. `main.rs`: spawn only when set (log one line either way, the runtime_config style). ark-core is consumed as a path dep (`elohim-ark-core = { path = "../ark/core" }` — allowed Cargo.toml edit; it adds no heavy deps; storage's boundary rails must still pass).

- [ ] Failing tests (`#[cfg(test)]` in spool_ingest.rs, tempdir + the existing in-memory/temp DB test harness storage uses — grep `fn test_ctx` / `TestDb` in `src/services/*` for the precedent): `ingests_a_witness_row_before_blob_with_one_digest` (write a fixture `.cbor` from a `DeathWitness` built via ark_core; `run_once` → content row exists with `id == cid`, `reach == "private"`, `metadata.kind == "death-witness"`, `blob_hash == "sha256-<hex of the bytes>"`, `BlobStore` has it; and the digest in `blob_cid` equals the digest inside the witness CID), `rerun_is_idempotent` (second `run_once` returns `[]`, one row), `a_mislabelled_file_is_refused` (file named with another CID → skipped with an error log, no row, no blob), `row_is_written_before_blob` (inject a BlobStore that fails `store` → the row exists, the blob does not, and the next pass retries the blob without duplicating the row).
- [ ] Implement; `cargo test --lib spool_ingest; echo EXIT=$?` → 0; `cargo clippy --all-targets -- -D warnings` clean on the crate.
- [ ] Commit: `git add <the five files + Cargo.toml + Cargo.lock>` / `git commit -m "feat(storage): ark spool ingest — a death witness becomes a private content row then a pantry blob with one digest (ELOHIM_ARK_SPOOL_PATH)" -- <same>`.

### Task 2: pin the identity equality

**Executor:** Codex. **Reviewer:** Opus. **Files:** `src/blob_store.rs` (tests only).

- [ ] Test `witness_cid_and_blob_address_share_one_digest`: bytes → `Cid::new_v1(0x71, sha2)` string and `Cid::new_v1(0x55, sha2)` string → `parse_content_address` of both equals `parse_content_address("sha256-<hex>")`. Commit `test(storage): a witness CID and its blob address are one digest in two renderings`.

### Task 3: `SpoolCustodyAuthor` — the standing spool commitment expands to per-hash custody

**Executor:** Opus (rust-architect). **Reviewer:** Codex.

**Files:** create `src/services/spool_custody_author.rs`; modify `src/services/rea_commitment_service.rs` (pure helpers), `src/services/mod.rs`, `src/main.rs` (dispatch beside the custody sweep where `run_salvage_pass` is dispatched, ~`main.rs:2258`). Read first: `src/services/salvage_commitment_author.rs` (whole file — the shape to copy: `author_custody_blob`, `resolve_self_agent_cid`, the block_in_place bridge, `run_salvage_pass`), `src/reconcile/custody.rs` (`CommitmentAuthor` trait; how `custody-blob` rows with `provider == self` are pulled), `src/db/peer_blob_inventory.rs` (advertised hashes and which peer advertised them), `src/services/rea_commitment_service.rs:52-62,130-158` (`deterministic_custody_id`, `primary_classification`), `db/diesel_schema.rs` (`rea_commitments` columns; no `bounded_by`).

**Interfaces:**
```rust
pub const SPOOL_CUSTODY_ACTION: &str = "custody-spool";
pub fn spool_classification(ward_agent: &str) -> String;                   // "spool:witness:<ward agent uhCAk…>"
pub fn deterministic_spool_custody_id(provider: &str, receiver: &str, ward: &str) -> String; // "custody-spool-<sha256(provider|receiver|classification)[..16]>"
pub struct SpoolCustodyAuthor { hc: Arc<HcClient> }
impl SpoolCustodyAuthor {
    pub fn new(hc: Arc<HcClient>) -> Self;
    /// One tick on THIS peer: for each active `custody-spool` row with provider == self (resolved via resolve_self_agent_cid; None → skip tick), collect the ward = receiver; for each peer_blob_inventory row advertised BY the ward's peer whose local content row (if any) OR the inventory metadata says kind == death-witness, author `custody-blob(provider=self, receiver=ward, blob=<sha256-hex marker>)` via `content_store::create_rea_commitment` on the local conductor unless that row already exists. Returns (authored, skipped).
    pub async fn run_once(&self, conn: &mut SqliteConnection) -> Result<SpoolCustodyPass, StorageError>;
}
pub struct SpoolCustodyPass { pub authored: Vec<String>, pub already: usize, pub skipped_no_self: bool }
```
Bounds: honour `metadata_json.bounds.max_bytes` and `atoms_per_hour` from the spool row when present (skip and log `bounds-exceeded` — a witnessed refusal, never silent); `retention_days` is recorded, not enforced (S1 later).

- [ ] Failing tests with a mock `CommitmentAuthor`/`HcClient` seam (copy the salvage tests' harness at `salvage_commitment_author.rs:~700`): `expands_one_custody_blob_per_advertised_witness`, `is_idempotent_across_ticks`, `skips_when_self_agent_unresolved`, `refuses_beyond_atoms_per_hour_with_a_logged_reason`, `ignores_blobs_the_ward_did_not_advertise`.
- [ ] Implement; wire the dispatch on the custody sweep cadence; `cargo test --lib spool_custody_author`; clippy clean.
- [ ] Commit `feat(storage): custody-spool expands to per-hash custody-blob on the custodian's own conductor (SpoolCustodyAuthor)`.

### Task 4: fixture — the custodians commit, on their own conductors

**Executor:** Codex. **Reviewer:** Opus.

**Files:** create `genesis/seeder/src/seed-spool-custody.ts`; modify `app/elohim-app/scripts/hc-mesh-prologue.sh` (a leg after `seed-household-formation`, ~line 347). Read first: `genesis/seeder/src/seed-household-formation.ts:140-165` (`buildCeremonyCustodyInput` — copy the wire shape) and `:990-1020` (how each provider's OWN session `callZome`s `content_store::create_rea_commitment` on `lamadCell`), and how the prologue passes `CONDUCTOR_URLS`/agent keys.

**Interfaces:** `buildSpoolCustodyInput({ providerAgent, receiverAgent, collectiveCid, bounds: { maxBytes: 64<<20, atomsPerHour: 120, retentionDays: 90 } })` → `{ id: 'custody-spool-<16hex of sha256(provider|receiver|spool:witness:<receiver>)>', action: 'custody-spool', provider, receiver, resource_classified_as: ['spool:witness:<receiver>'], resource_quantity_value: maxBytes, resource_quantity_unit: 'B', in_scope_of: [collectiveCid], note: 'spool custody: <provider> holds <receiver>'s witnesses', metadata_json: JSON.stringify({ seedGeneration: 'spool-custody', kind: 'custody-spool', bounds }) }`. The seeder authors, for the household `{matthew, jessica, james}`, every ordered pair (six rows) so any peer may be the one that dies — each on the PROVIDER's own conductor.

- [ ] Shape test (`genesis/seeder` test runner; grep the precedent for `buildCeremonyCustodyInput` tests): the id is deterministic and equals the Rust `deterministic_spool_custody_id` for the same inputs (pin one vector in both languages).
- [ ] Prologue leg; the orchestrator runs `just mesh prologue` and asserts `GET :8090/api/v1/commitments?action=custody-spool` returns rows with `provider == matthew`'s agent for `receiver == jessica`'s agent.
- [ ] Commit `feat(seeder): spool custody — each household custodian authors a custody-spool toward every other member on its own conductor`.

### Task 5: mesh dials

**Executor:** Codex. **Reviewer:** Opus. **Files:** `app/elohim-app/scripts/hc-mesh.sh` only.

- [ ] In ark mode, each storage peer's launch environment gains `ELOHIM_ARK_SPOOL_PATH=$LOCAL_DEV_DIR/<peer>/ark`, `CUSTODY_SWEEP_SECONDS=${CUSTODY_SWEEP_SECONDS:-15}`, `INVENTORY_BROADCAST_SECONDS=${INVENTORY_BROADCAST_SECONDS:-15}`; `just mesh status` prints the spool path per storage peer; header doc updated. `bash -n` and shellcheck unchanged. Commit `feat(mesh): ark mode points each storage peer at its ark spool and dials custody/inventory to 15 s`.

### Task 6: station-2 steps

**Executor:** Opus. **Reviewer:** Codex. **Files:** `genesis/a2o/steps/mesh/death-witness.steps.ts`, `genesis/a2o/features/resilience/death-witness.feature` (remove `@wip` from station 2 only). Read first: `steps/mesh/household-chaos.steps.ts:690-714, 930-998` (custody probe precedent: commitments endpoint + on-disk `existsSync` via `peerStorageDir`), the station-1 steps.

- [ ] `Given Matthew and James have each counter-signed a commitment to custody Jessica's witnesses` — `GET {matthew}/api/v1/commitments?action=custody-spool` and `{james}` → a row whose `provider` is that peer's own agent and `receiver` is Jessica's agent; fail with "run `just mesh prologue`" when absent.
- [ ] `Then within 60 seconds Matthew and James each hold a copy of the witness with the same content hash` — poll ≤60 s: on each custodian, `GET /api/v1/commitments?action=custody-blob&limit=500` has a row whose `resourceClassifiedAs` digest equals the station-1 witness CID's digest, AND the bytes exist on that peer's disk (`peerStorageDir`), AND `GET {custodian}/blob/<witness cid>` returns the bytes whose sha256 equals the digest.
- [ ] `And Matthew and James each record on their own peer that they received that witness from Jessica` — `GET {custodian}/api/v1/economic-events` → `action == 'serve-blob'`, `provider` resolves to Jessica's agent (via the fixture's agent keys, never a transport id string-compare), `resourceInventoriedAs` digest matches.
- [ ] `pnpm exec tsc --noEmit -p .` green; the orchestrator runs `just test mesh '@concern:death-witness and @station-2'` on the live mesh.

### Task 7: habit delta, spec corrections, projection (orchestrator)

- [ ] Habit DELTA (receipt id); `habits-project.py`; spec §7/§12 corrections: custody rides the elohim DNA commitment with no whitelist; provider = author is the counter-signature; `attestation:custodian-commitment` reserved for S1's explicit second signature; missing nodes M7 (iroh fetch emits no receipt), M8 (`serve-blob.output_of` never names the commitment it discharges), M9 (no custody-scoped read gate on the libp2p/iroh blob planes — station 3b).

## Self-review
- Spec coverage: §6 amber-local → amber-offered (Tasks 1, 3, 5), custody as a standing commitment counter-signed (Tasks 3, 4), bounds (Task 3), row before blob (Task 1) ✓; §6's "redacted summary to custodians" is NOT in this slice — the custodian holds the full witness bytes at `private` reach, and the redaction is deferred with reach enforcement (station 3b) — declared.
- Type consistency: `spool_classification` (Task 3) == the seeder's `resource_classified_as` (Task 4) == the Given step's expectation (Task 6); the `sha256-<hex>` marker (Task 1 `blob_hash`) == what Task 3 authors == what Task 6's digest compare reads.
