# Fixtures clone harness — 2026-09-06

**Result: selector and bundle preparation implemented; mesh isolation and gossip
cost legs NOT RUN. This experiment has not proven fixture isolation.**

Branch: `sprint/fixtures-clone-harness`, based on `dev`. All authored files and
bundle copies are in `/projects/elohim/.claude/worktrees/fixtures-clone`.
No push, fleet change, or existing mesh-process mutation was performed.
Neither `reports-committed/` nor `genesis/docs/superpowers/specs/reports/` existed,
so this is the requested committed fallback report. Its ignored copy is
`genesis/a2o/reports/fixtures-clone-2026-09-06/REPORT.md`.

## Implemented scope

- Production seeder: `--cell lamad.fixtures` or `SEED_CELL_TARGET=lamad.fixtures`.
  `lamad.0` selects a conductor clone ID; `lamad` selects the provisioned role.
  CLI overrides environment. Explicit selectors require exactly one enabled
  match; malformed, absent, disabled, wrong-role, and ambiguous targets fail.
  Resolution happens before blob/projection writes and again before DNA writes.
  Targeted calls authorize signing credentials on the selected cell only, and
  targeted connection/selection failures propagate to a nonzero exit.
- The two identity seeders use the same selector, retaining `imagodei` by
  default. `lamad.fixtures` does not acquire imagodei zomes: identity fixtures
  would need an appropriate imagodei clone. No cross-role fallback is provided.
- Storage: existing `HcClientConfig.role` and discovery role arguments accept
  `role.clone`; both raw MessagePack discovery and the typed client reject
  missing/disabled/ambiguous clones. The storage executable reads
  `SEED_CELL_TARGET` into **its import API's** role configuration. It does not
  retarget the whole storage process or its reconciliation/signal handlers.
  Restart a test storage with that environment to target its import API.
- `scripts/fixtures-clone.mjs` provides `prepare`, `install`, `grow`, `authorize`,
  `inventory`, and `measure`. It never launches or stops conductors. `prepare`
  unpacks a deployed bundle into a new worktree-local directory and changes
  only the copied lamad role's `dna.clone_limit` to 15 (20 total cells).
  `install` requires an empty test conductor; by default it installs five
  provisioned roles and creates one clone named `fixtures`. A UUID recorded
  by `prepare` supplies fresh network seeds; share that UUID among the three
  peers so corresponding clones can gossip with each other.
- Holochain 0.7 landmines: `clone_only` panics in app-info assembly; `deferred`
  is ignored on installation. Keep `strategy: create` and raise `clone_limit`.

## Commands actually run and verification

From the worktree root:

```sh
env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-onboarding-target cargo run \
  --manifest-path elohim/eprfs/Cargo.toml -q -p elohim-epr-cli -- setup
/tmp/eprfs-onboarding-target/debug/epr doctor
# epr explain was run for the seeder selector, storage discovery, and harness.
berth status
just mesh status
pnpm install --offline --frozen-lockfile
pnpm --dir genesis/seeder exec vitest run src/cell-target.test.ts
pnpm --dir genesis/seeder exec tsc --noEmit
node --check genesis/a2o/scripts/fixtures-clone.mjs
node genesis/a2o/scripts/fixtures-clone.mjs prepare \
  --out genesis/a2o/reports/fixtures-clone-2026-09-06/bundle \
  --hc /projects/.claude-config/tools/hc-0.7/hc \
  --bundle /projects/elohim/elohim/holochain/local-dev/deployed-bundles/elohim.happ
CARGO_TARGET_DIR=/projects/.cargo-pool/fixtures-clone cargo test \
  --manifest-path elohim/elohim-storage/Cargo.toml --lib cell_discovery::tests
echo EXIT=$?
CARGO_TARGET_DIR=/projects/.cargo-pool/fixtures-clone cargo fmt \
  --manifest-path elohim/elohim-storage/Cargo.toml --check
echo EXIT=$?
just --justfile elohim/elohim-storage/justfile migration-hygiene
git diff --check
```

Setup, doctor, offline install, five selector tests, seeder typecheck, JavaScript
syntax check, and test-bundle unpack/patch/repack passed. The source deployed
bundle was read only. A semantic manifest comparison passed: the sole change
was `lamad.dna.clone_limit: 0 → 15` (`bundle-diff.txt`). Rust discovery tests: **4 passed** using a small `rustc --test` binary that
includes the actual `cell_discovery.rs`, `error.rs`, and `getrandom_custom.rs`,
linked against this run's exact Cargo-built dependencies. Both compilation and
test execution exited 0; this validates both raw and typed selectors. The raw
wrapper, dependency arguments, and `discovery-tests.log` are in the ignored
report directory. The full `cargo test --lib cell_discovery::tests` invocation
was suspended by the host resource guard for minutes (Cargo and rustc in `T`
state); only its own processes were terminated, yielding explicit `EXIT=143`.
It is **not** recorded as a passing Cargo run. Formatting, migration hygiene,
and `git diff --check` also passed.
The custom getrandom `RUSTFLAGS` was retained for storage builds.
The gate recipe was attempted with a 120-second budget and 10-second kill grace:
`timeout --kill-after=10s 120s env CARGO_TARGET_DIR=/projects/.cargo-pool/fixtures-clone just --justfile elohim/elohim-storage/justfile gate`.
This executes the manifest-owned storage recipe while retaining the task's
explicit target slot; root `just gate elohim-storage` would select a different,
shared pool slot. Gate result: **EXIT=124**, timed out during Clippy dependency checking
(`automerge` was the last reported dependency); Clippy and the full test suite
remain **INCOMPLETE**. The gate's migration-hygiene and fmt steps passed.
`epr check` exited 0 with advisory findings; habit projection freshness passed.

Mesh preflight: berth reported two live Claude sessions and a free mesh lease.
However, ports 4445 and 4485 were listening, storage was UP on 8090, while this
worktree reported matthew/jessica/james conductors **not-running**. The other two
storages and relay were down. Reported default next-launch conductor and hc were
**0.6.0**. These are not three owned, gossiping 0.7 peers. `just mesh start` and
`just mesh prologue` were therefore NOT RUN; starting on those fixed ports
would interfere with existing processes. Actual mesh implementation is
`app/elohim-app/scripts/hc-mesh.sh`, not the path in the task description.

## Raw cell costs

Prior idle measurement, copied from the supplied `cellcost-2026-09-06/measurements.jsonl`:

| Cells | RSS KiB | Disk bytes | FDs | Threads |
|---:|---:|---:|---:|---:|
| 5 | 1233644 | 195493888 | 118 | 56 |
| 10 | 1244496 | 196866048 | 208 | 96 |
| 20 | 1270004 | 199593984 | 390 | 177 |
| 40 | 1318292 | 205066240 | 754 | 339 |

New gossip rows at 5/10/20: **NOT RUN** in every column.
Idle-versus-gossip delta: **not measurable**, not zero. The harness preserves
RSS from `/proc/PID/status`, allocated disk from `du -s -B1`, `/proc/PID/fd`
count, thread count, and a 60-second settling interval. It reuses an existing
app interface to avoid adding a persistent listener per measurement. Network
stats and app-info accompany each row; network stats alone are not proof of
successful record propagation. Verify remote record identities as well.

Prior provenance: stock hc/conductor 0.7.0, revision
`84cdce7d4df17b95189324d5cecc3f1bfd5db30f`, x86_64, 24 logical CPUs. Prior method
ran under `unshare -Urn`, with unreachable external bootstrap/relay endpoints.
Its results describe idle isolated cells, not gossip costs.

## Identity inventory result

```text
base lamad source-chain action identities: NOT RUN
base lamad full DHT operation inventories: NOT RUN
base lamad zome-export content/action/entry identities: NOT RUN
storage content rows (all columns, including ids and declared heads): NOT RUN
sync-plane document ids and heads (all pages in content namespace): NOT RUN
resolved seeder cell/app targets before versus after: NOT RUN
clone contains the seeded record identities: NOT RUN
identity diff: NOT PRODUCED — no empty-diff or isolation claim
```

The inventory command retains full DHT dumps and follows their opaque cursor,
plus sorted full operation identities, source-chain action hashes, zome-export
content/action/entry identities, every content row, and all `/sync/v1/elohim/docs`
pages. It fails on nonadvancing cursors or incomplete pages. The content export
is source-chain-local, so it is supplemented by DHT dumps; do not mistake it
for all remotely integrated content. Query credentials must be minted **before**
the baseline: authorization itself writes a CapGrant to the source chain.
An inventory is a sequence of reads, not an atomic cross-plane snapshot; wait
for convergence and obtain repeated identical baselines before seeding.

## Storage-instance finding and deferred seams

**One storage instance per mesh peer, shared across the app's cells; not one
instance per cell, and not context-qualified shared projection storage.**
`hc-mesh.sh` starts a single storage with `STORAGE_DIR=$MESH_DIR/$name` per peer.
`src/db/diesel_schema.rs` defines `content (id)` with `h_app_id`, anchor hash,
and declared-head fields, but no cell ID/DNA hash namespace. App ID is not
clone context. `sync/doc_store.rs` stores app namespace plus document ID;
`sync/projector.rs` uses the fixed `elohim` namespace and `node:{id}` documents.
The new import selector does not change those keys. A clone-aware conductor
write is therefore insufficient evidence of projection or sync isolation.
This is a static finding, not a measured collision, and is deliberately unfixed.

Mintable seams retained within this bounded report:

- Chain: fixtures clone harness / between clone-targeted authoring → isolated
  fixture projection / missing node: projection and sync identity include cell
  context, measured by full base/clone row and document inventories / current
  state: unproven live; absent context in the checked schema.
- Chain: coordinator upgrade / between app-info cell discovery → coordinator
  hot-swap / missing node: every enabled clone receives the coordinator update,
  measured by clone DNA coordinator definitions before/after / current state:
  excluded by the provisioned-only match in
  `src/happ_manager.rs:1310-1314` (`sync_coordinators_for_app_info`). No fix here.

## Exact continuation commands (NOT RUN)

Run from this worktree in an owned, free-port environment. For the stock-conductor
experiment, enter `unshare -Urn bash` in the foreground and run `ip link set lo up`
so the entire three-peer mesh, relay, and probes share an isolated namespace.
That also prevents the copied deployed base DNA seeds from contacting alpha.
Do not background the mesh start command or run it in a background tool task.
Provide an installed 0.7-compatible relay binary and storage binary first.

```sh
cd /projects/elohim/.claude/worktrees/fixtures-clone
export CARGO_TARGET_DIR=/projects/.cargo-pool/fixtures-clone
export HOLOCHAIN_BIN=/projects/.claude-config/tools/hc-0.7
export MESH_CONDUCTOR_LAUNCH=direct
export MESH_DIR="$PWD/genesis/a2o/reports/fixtures-clone-2026-09-06/mesh"
export MESH_HAPP_PATH="$PWD/genesis/a2o/reports/fixtures-clone-2026-09-06/bundle/workdir/elohim.happ"
export STORAGE_BIN="$CARGO_TARGET_DIR/debug/elohim-storage"
# Set MESH_RELAY_BIN to the installed iroh-relay 1.0.3 server executable.
berth status
just mesh status
just mesh start
just mesh prologue
export EXP_SEED=$(node -p "require('./genesis/a2o/reports/fixtures-clone-2026-09-06/bundle/prepare.json').seed")
```

Measure 5 cells before creating any clone. Confirm the PID belongs to this
worktree's matthew conductor, then set `EXP_PID` to that PID (not the storage,
ark parent, sandbox wrapper, or another session's conductor).

```sh
node genesis/a2o/scripts/fixtures-clone.mjs measure \
  --admin ws://localhost:4444 --app elohim --cells 5 --pid "$EXP_PID" \
  --data "$PWD/elohim/holochain/local-dev/matthew" \
  --out genesis/a2o/reports/fixtures-clone-2026-09-06/gossip
for port in 4444 4454 4464; do
  node genesis/a2o/scripts/fixtures-clone.mjs grow \
    --admin "ws://localhost:$port" --app elohim --cells 6 --seed "$EXP_SEED" \
    --out "genesis/a2o/reports/fixtures-clone-2026-09-06/peer-$port"
done
node genesis/a2o/scripts/fixtures-clone.mjs authorize \
  --admin ws://localhost:4444 --app elohim \
  --credentials genesis/a2o/reports/fixtures-clone-2026-09-06/credentials.json \
  --out genesis/a2o/reports/fixtures-clone-2026-09-06/auth
```

After convergence, capture the baseline, seed an explicitly selected fixture
content directory (`EXP_FIXTURES`) and paths directory (`EXP_PATHS`), and capture
the after state. Use dedicated fixture IDs and retain the seed input files and
stdout (which includes the resolved target and DNA/agent identities).

```sh
snapshot() {
  node genesis/a2o/scripts/fixtures-clone.mjs inventory \
    --admin ws://localhost:4444 --app elohim --storage http://localhost:8090 \
    --db "$MESH_DIR/matthew/content.db" \
    --credentials genesis/a2o/reports/fixtures-clone-2026-09-06/credentials.json \
    --out "genesis/a2o/reports/fixtures-clone-2026-09-06/$1"
}
snapshot before
HOLOCHAIN_ADMIN_URL=ws://localhost:4444 HOLOCHAIN_APP_ID=elohim \
  CONTENT_DIR="$EXP_FIXTURES" PATHS_DIR="$EXP_PATHS" \
  pnpm --dir genesis/seeder exec tsx src/seed-production.ts --cell lamad.fixtures \
  --skip-blobs > genesis/a2o/reports/fixtures-clone-2026-09-06/seeder.log 2>&1
echo EXIT=$?
snapshot after
for inventory in base-actions base-ops base-content projection sync targets; do
  diff -u "genesis/a2o/reports/fixtures-clone-2026-09-06/before/$inventory.json" \
    "genesis/a2o/reports/fixtures-clone-2026-09-06/after/$inventory.json"
done
```

Require **every** base diff to exit 0, then compare fixture input IDs to
`after/lamad.0-content.json` and retain each clone action hash. Repeat inventories
on jessica/james (their own pre-baseline credentials and databases) and prove the
same fixture records reached the corresponding clone DHTs. For the import-API
leg, restart only the owned test storage with `SEED_CELL_TARGET=lamad.fixtures`
and submit the same fixture batch through its existing `/import` workflow;
repeat inventories. Do not infer that leg passed from direct seeder calls.

For each of 10 and 20 cells, grow **all three peers** with the same seed, perform
real writes and cross-peer identity reads, then run `measure` on matthew:

```sh
for cells in 10 20; do
  for port in 4444 4454 4464; do
    node genesis/a2o/scripts/fixtures-clone.mjs grow \
      --admin "ws://localhost:$port" --app elohim --cells "$cells" --seed "$EXP_SEED" \
      --out "genesis/a2o/reports/fixtures-clone-2026-09-06/peer-$port"
  done
  # Establish and record actual gossip/remote identities before calling measure.
  node genesis/a2o/scripts/fixtures-clone.mjs measure \
    --admin ws://localhost:4444 --app elohim --cells "$cells" --pid "$EXP_PID" \
    --data "$PWD/elohim/holochain/local-dev/matthew" \
    --out genesis/a2o/reports/fixtures-clone-2026-09-06/gossip
done
```

## What this does NOT prove

It does not remove old fixtures from the commons, change fleet churn, establish
production clone support, confer architectural authority, or prove projection
isolation. It does not repair clone coordinator hot-swaps. It does not establish
a gossip cost or capacity recommendation. Bundle preparation is tested; live
clone creation, inventory capture, and measurement commands remain unverified
against the blocked three-peer mesh.

## 2026-09-07 — both mesh legs RUN on the local household mesh

Run 10:00–10:23Z from the main tree (`dev` @ 5b377eb8c, which carries the merged
`sprint/fixtures-clone-harness` work), against the three-peer household mesh
(matthew/jessica/james) on pinned-fork holochain 0.7.0, iroh-relay 1.0.3, storage in
`dual` transport mode. Full evidence — before/before2/after inventories, DHT dumps, seeder
log, network stats, measurement rows — in the gitignored receipt dir
`genesis/a2o/reports/fixtures-clone/mesh-20260907T1000Z/` (`RECEIPT.md` is the index).
The test bundle was prepared from the **workdir** happ the mesh actually installs, not the
2026-09-06 deployed bundle, for DNA parity with today's storage binary; the sole manifest
change was `lamad.dna.clone_limit: 0 → 15`, and the committed `workdir/happ.yaml` was not
touched. The mesh was restarted with `MESH_HAPP_PATH` pointed at it.

**Storage-instance model: one storage per mesh peer, shared across that peer's cells** — the
`hc-mesh.sh` default, no separate instance and no context-qualified projection storage. The
storage `content.db` and Automerge sync docs survived the restart (117 rows / 105 docs) while
the conductor sandboxes were regenerated, so the base lamad cell started this run with an empty
DHT and a non-empty projection.

**Isolation leg — identity diff, split verdict.** Noise floor first: two baselines 20 s apart
were byte-identical on every base inventory (only the just-created clone's own `lamad.0-ops`
drifted), so the base cell was quiescent and any later delta is attributable. The clone
(`lamad.0`, name `fixtures`, seed `<uuid>-fixtures-0`) was created on all three peers in
128–191 ms. The **production seeder leg did not write**: its cell selector resolved correctly
to the clone cell, but both `create_content` calls returned `WasmError Deserialize` because
`seed-production.ts` sends nine fields and never sends `reach`, which
`lamad_types::CreateContentInput` requires with no serde default — a pre-existing seeder↔DNA
vocabulary drift, unrelated to cloning, and the same failure the base-corpus prologue seed
hits. The probe records were therefore written directly against the clone cell with a correct
payload. Result: **the DHT planes are isolated by identity — `base-actions`, `base-ops`,
`base-content` and `targets` diffs are all EMPTY**, and both probe action hashes appear only in
the clone's action inventory. **The projection and sync planes are NOT isolated**: two rows
appeared in the peer's shared `content` table (`h_app_id: "lamad"`, `dht_anchor_hash` = the
clone's action hash, no cell/DNA qualifier) and two docs
(`node:fxclone-alpha-20260907`, `node:fxclone-beta-20260907`) in the `elohim` sync namespace.
That **upgrades this report's earlier static finding to a measured collision**. One further
observation, recorded but not diagnosed: `export_all_content` on the clone still returned `[]`
at the after-capture while its ops were mid-integration.

**Cost leg — per-cell cost with gossip, measured.** Gossip was live throughout (iroh backend,
direct connections to both peers, send_message_count 1438 → 2591 → 3541 across the three
points). Rows: 5 cells → RSS 892680 KiB / 704393216 B / 239 FDs / 108 threads; 10 cells →
887188 / 715862016 / 369 / 170; 20 cells → 917488 / 737624064 / 557 / 254. Marginal per clone
over 5→20 (n=15): **RSS +1.62 MiB, disk +2.11 MiB, FDs +21.2, threads +9.7**, with a linear
disk slope across both intervals. Against the prior idle isolated run (0.26 MiB disk, 18.1
FDs, 8.1 threads per clone) the **per-clone disk cost under gossip is ~8× the idle figure**,
FDs +17% and threads +20%; RSS is not comparable across the two runs (different conductor
build and happ), only the within-run slopes are. The idle-versus-gossip delta is therefore
large and one-directional: an isolated measurement understates per-cell disk by nearly an order
of magnitude, because peer-store/gossip state dominates it.

**Harness fix.** `fixtures-clone.mjs`'s existing-clone guard matched `appInfo` clone order
positionally against `fixtures-${index}`; `appInfo` does not guarantee clone ordering, so a
second `grow` pass against the same peer threw
`Existing clone does not belong to this experiment seed/sequence` and blocked the 20-cell row
on the first attempt. The guard now derives the index from the clone name and is
order-independent, with the network_seed check unchanged in strictness.

**Still NOT RUN:** the seeder-mediated write into the clone (blocked by the `reach` drift); the
storage `/import` API leg under `SEED_CELL_TARGET=lamad.fixtures`; cross-peer confirmation that
the probes reached jessica's and james's corresponding clone DHTs (inventories were captured on
matthew only). The `sync_coordinators` clone-enumeration gap remains a filed seam, untouched.
This experiment still confers no architectural authority and makes no habit claim.

### 2026-09-07 addendum — the cross-peer leg, and a revised isolation verdict

The cross-peer confirmation listed above as not-run was then run on jessica (admin 4454,
storage 8091; `jessica-after/` in the same receipt dir). It changes the verdict, so it is
recorded here rather than left as a gap.

Eight minutes after the clone-targeted write, **jessica's DHT — her base cell and her own
corresponding `lamad.0` clone alike — contained neither of matthew's probe action hashes**;
clone-to-clone DHT gossip had not propagated in the window. But **jessica's storage projection
did contain `fxclone-beta-20260907`, carrying a different `dht_anchor_hash` than matthew's
clone action** — and that hash is present in jessica's **base** source-chain inventory and
absent from her clone's. Jessica's storage re-authored the clone-originated content into her
**base lamad cell**, minting a fresh DHT action there; james then carries the same row with
jessica's anchor and a NULL anchor state. Both peers' sync inventories carry both probe docs,
but only `beta` completed the projection/re-author path inside the window, so the propagation
is partial and ordering-dependent rather than all-or-nothing.

The revised verdict: **clone isolation holds only for the authoring peer's own DHT.** Content
written into a clone escapes through that peer's single shared storage projection, crosses the
storage sync plane, and is re-authored into a *receiving* peer's base cell — the precise
commons pollution the clone was meant to prevent. The earlier static finding is therefore not
just a local namespace collision; it is cross-peer write amplification into base DHTs. Fixture
isolation via conductor clones is **not achieved on the current substrate** without a
cell-qualified projection and sync identity. That missing node — projection/sync identity
includes cell context — remains the blocking seam, and it is now measured rather than inferred.
