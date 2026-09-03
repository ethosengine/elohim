---
title: "Holochain 0.7.0 Upgrade Guide — the last deliberate big-bang (Wave 3 of the convergence campaign, executed)"
id: holochain-0-7-upgrade-guide
status: Active
class: substrate
domain: substrate (conductor fork · DNA zomes · client families · transport topology · fleet re-genesis)
sprint: 2026-09-02 (lanes A–E, I parallel; F after local-mesh green; G/H after fleet green)
serves:
  - runtime-upgrade-propagation
  - dataplane-convergence
cites:
  - "holochain-iroh-convergence-upgrade-campaign | Holochain–iroh Convergence Upgrade Campaign | sha256:381e7dad57e8cd23 | path: genesis/docs/superpowers/plans/2026-08-04-holochain-iroh-convergence-upgrade-campaign.md"
  - "wave2-relay-sovereignty-design | Wave 2 relay custody | sha256:f2f9459b0530aefa | path: genesis/docs/content/elohim-protocol/architecture/2026-08-05-wave2-relay-sovereignty-design.md"
  - genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md
  - genesis/data/timeline/backlog/iroh-cross-relay-preflight-fails-closed.md
  - genesis/data/timeline/backlog/governance-native-dna-upgrade-path.md
memory_anchors:
  - project_holochain_0_7_0_assessment
  - project_alpha_dna_migration_2026_09_02
  - project_upgrade_authority_constitutional_elohim
  - feedback_upgrade_propagation_north_star_wall_clock
  - feedback_local_mesh_first_cadence
  - feedback_push_branch_discipline
  - project_cargo_pvc_disk_discipline
---

# Holochain 0.7.0 Upgrade Guide

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (one fresh
> subagent per lane, review between lanes) or superpowers:executing-plans. Steps use checkbox syntax.
> Each lane is a disjoint write-set; a lane's agent reads ONLY its lane plus §Global Constraints and
> §Version Table. Every lane commits path-limited and never pushes (integrator pushes one batch).

**Goal:** Move the whole substrate from Holochain 0.6.3 (our fork) to Holochain 0.7.0 in one atomic
family move, re-genesis the alpha fleet once, and retire the tx5 era — while carrying forward the
fork patches that still matter and the per-doorway relay topology that the multi-doorway steward
model requires.

**Architecture:** Five independent code lanes (fork rebase · DNA+sdk+sweettest · storage · doorway ·
JS clients) plus one toolchain/CI/config lane run in parallel on disjoint write-sets, all pinned to
the same 0.7.0 finals. Integration is proven on the local household mesh with stock 0.7.0 binaries
first (the fleet CONFIRMS, never discovers). The fleet cutover is a rehearsed re-genesis ceremony
using the guards the 2026-09-02 incident built (packed-hash guard, migration intent gate, supervisor
death witness). Cleanup lanes run only after the fleet is green on 0.7.

**Tech Stack:** Rust (cargo, pool slots), Holochain 0.7.0 / hdk 0.7.0 / hdi 0.8.0 / kitsune2 0.5.0 /
iroh 1.0.3 (conductor side, the tag's own lock — a bare `cargo generate-lockfile` drifts to kitsune2 0.5.1 / iroh 1.1, never do that) / lair 0.7.1, `@holochain/client` 0.21, holonix `main-0.7`, Jenkins
(che-devworkspaces `elohim-edgenode` job + monorepo orchestrator), k8s manifests under
`genesis/orchestrator/manifests/`.

**Spec:** the 2026-08-04 convergence campaign plan (Wave 3 "0.7 re-genesis") plus the 2026-09-02
assessment in memory `project_holochain_0_7_0_assessment`. This guide replaces Wave 3's task list.

---

## Why now (decision record, 2026-09-02)

- 0.7.0 is real (released 2026-07-30, tag `holochain-0.7.0` = `84cdce7d4`, already fetched in the
  `elohim/holochain-conductor` submodule). holonix `main-0.7` exists. Prebuilt 0.7.0 linux binaries exist.
- Alpha has never been cheaper to re-genesis: chains were re-keyed 2026-09-02, the DHT is empty, the
  re-seed has not run. A 0.7 jump after the re-seed pays re-key + re-seed twice. **The re-seed waits
  for this guide.**
- Rung 5 (coordinator releases by election) is proven on the mesh and coordinator-only by design; a
  0.7 jump moves the conductor binary, HDK and integrity hashes — a forced re-genesis rung 5 cannot
  perform. This is **the last deliberate big-bang**; its debut on the fleet happens on 0.7, not 0.6.
- Conductor rebase sized by dry-run cherry-pick onto the tag (2026-09-02): 3 patches clean, 1 moot,
  1 small sqlx re-port, 1 real re-port (~1 day), 1 transport patch re-derived (~116 lines). Days, not weeks.

## Relay topology decision (operator ruling 2026-09-02 — D2 STANDS)

Per-doorway relays stay. Doorways are **different operators**; peer-stewards register with several
doorways so they keep web2 conveniences to their dataplane context without fear of any one portal
being captured or closed (resiliency epic). Therefore heterogeneous home relays inside one network
are a **protocol requirement**, and the cross-relay preflight fallback is a permanent fork patch
(Lane A, Task A6) with an upstream issue behind it (Lane H). kitsune2 0.5.0 codified the opposite
("exact relay URL match, no fallback"; one relay per space) because upstream assumes one operator per
network. The p2p dataplane remains the authority for which doorways a human is registered with; the
conductor's single `relay_url` is that human's *home* relay (their primary doorway's), not a limit
on whom they can reach.

## Global Constraints

Every task's requirements implicitly include this section.

- **Atomic family move.** DNA zomes, the six `elohim/sdk/domains/*/types` crates, `crates/doorway-client`,
  sweettest, elohim-storage, doorway-service and steward/node move to the 0.7.0 family in ONE
  integration batch. A partial move re-creates the `holo_hash` split that blocked Wave 1 (two
  `ActionHash` types in one zome build). Lanes land on branch `upgrade/holochain-0.7`; dev receives
  it as one fast-forward after the local-mesh gate (§Lane F, Task F1).
- **Push discipline.** Lanes commit path-limited to their write-set; **never push**. The integrator
  pushes once per batch (`feedback_push_branch_discipline`). The fork branch in `elohim/holochain-conductor`
  is pushed by the integrator to `ethosengine/holochain` (`origin`), never to `upstream`.
- **Cargo discipline.** Native builds set `CARGO_TARGET_DIR` to the pool slot printed at session
  start (`cargo-pool key`); DNA/WASM workspaces use plain cargo (no target redirect). `RUSTFLAGS=""`
  for doorway/steward/sweettest; `RUSTFLAGS='--cfg getrandom_backend="custom"'` for elohim-storage
  and WASM. `cargo nextest` is NOT installed — plain `cargo test`. **A dependency bump is verified by
  `cargo test`, never `cargo check`.** Print `echo EXIT=$?` on its own line after every cargo run.
- **One heavy cargo at a time.** The RAM guard sheds any cargo tree at 80% committed memory (six
  `cargo test --features p2p-iroh` runs were killed on 2026-09-02 alone). Every cargo build/test in
  this guide is wrapped: `flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock <cargo cmd>`.
  Lanes may run in parallel; their compiles serialize on that lock.
- **DNA hash law.** Any commit that moves a packed DNA hash carries `[dna:migrate]` in its message
  AND updates `elohim/holochain/dna/dna-hashes.baseline` in the same commit (`scripts/ci/dna-hash-guard.sh`).
  0.7 moves every hash. The baseline update lands in Lane F, Task F1, from CI-packed hashes.
- **Genesis pair atomicity.** Any fleet action that changes DNA hash or conductor line hits BOTH
  bootstrap peers (adam + matthew) in the same roll, or the namespace partitions.
- **Cluster ops are operator-owned.** No `kubectl` from the dev environment. Fleet steps in Lane F
  are written as an operator runbook; the repo is the cleanup surface.
- **Managed surfaces.** `CLAUDE.md`, `AGENTS.md`, skills under `.claude/skills` / `.codex/skills` /
  `.agents/skills` are PROJECTIONS of `.epr-meta/elohim/packages/**`. Edit the package, then project
  (`elohim-package-authoring` skill). Never edit `genesis/manifests/habits.yaml` (run
  `.claude/scripts/habits-project.py`). Cites in docs are tool-generated (`semantic-links` skill).
- **Story-first.** Lane B adds/updates the a2o scenario tags it touches; Lane F records the receipt
  in the habit atoms named in `serves:` (one-line delta each) and re-projects.
- **No new instruments.** No new ledger, registry or ranking script. Evidence goes into the habit
  atoms, the campaign plan's Wave 3 delta, and this guide's checkboxes.

## Version Table (verbatim from the `holochain-0.7.0` tag and the upstream compatibility table)

| Component | 0.6 line (today) | 0.7 target |
|---|---|---|
| holochain (conductor) | fork `elohim-0.6.3` on `fixt-0.6.3` + 9 patches | fork `elohim-0.7` on `holochain-0.7.0` (`84cdce7d4`) + 6 patches |
| `hdk` / `hdi` | `=0.6.0` / `=0.7.0` | `=0.7.0` / `=0.8.0` |
| `holo_hash` | `=0.6.0` (sdk types), `=0.7.0-dev.9` (doorway) | `=0.7.0` |
| `holochain_types` / `holochain_zome_types` / `holochain_integrity_types` | 0.6.x / 0.7.0-dev.x | `=0.7.0` |
| `holochain_conductor_api` / `holochain_websocket` / `holochain_keystore` | 0.6.x / 0.7.0-dev.23 | `=0.7.0` |
| `holochain_client` (Rust) | git fork pin (storage), `=0.9.0-dev.24` (doorway) | `=0.9.0` |
| `holochain_serialized_bytes` | `=0.0.56` (vendored path patch in 3 crates) | `=0.0.57` (crates.io; vendor dir retired in Lane G) |
| `kitsune2_*` (inside conductor) | 0.4.1 | 0.5.0 (iroh 1.0.3 per the tag lock) |
| `lair_keystore` (inside conductor) | 0.6.3 | 0.7.1 (no prebuilt binary published; in-proc keystore needs none) |
| conductor cargo features | `sqlite-encrypted,wasmer_sys,transport-tx5-backend-go-pion,jemalloc` | `encryption,wasmer-sys-cranelift,jemalloc` with `--no-default-features` (drops `schema`; iroh is the only transport, no feature) |
| `@holochain/client` | 0.20.x (`^0.19.2` in `elohim/holochain/rna/typescript`) | `^0.21.0`, single copy via root `pnpm.overrides` |
| holonix flake ref (DNA pipeline) | `main-0.6` | `main-0.7` |
| dev-container binaries (`HOLOCHAIN_VERSION`) | 0.6.0 | 0.7.0 (assets `holochain-x86_64-unknown-linux-gnu`, `hc-…`, `hcterm-…` confirmed 200 at `releases/download/holochain-0.7.0/`) |
| `tauri-plugin-holochain` (steward desktop) | `main-0.6` | **no `main-0.7` branch yet — HELD** (Lane J) |

Conductor config shape at 0.7.0 (`crates/holochain_conductor_api/src/config/conductor.rs`):
`ConductorConfig { tracing_override, wasm_backend, data_root_path, keystore, admin_interfaces, network,
db_sync_level, db_max_readers, incoming_request_concurrency_limit, restore_chain_quorum, tuning_params,
tracing_scope }` · `NetworkConfig { base64_auth_material_bootstrap, base64_auth_material_relay,
bootstrap_url, relay_url, request_timeout_s, target_arc_factor, report, advanced, disable_bootstrap,
disable_publish, disable_gossip }`. **`signal_url` and `webrtc_config` no longer exist; unknown keys
hard-fail startup.** Our templates carry no `db_sync_strategy`/`chc_url`, so only the two removals apply.

## 0.7 code-migration patterns (used by Lanes A, B, C, D)

| 0.6 | 0.7 |
|---|---|
| `match action { Action::Create(c) => c.author … }` | `match &action.data { ActionData::Create(c) => action.author() … }` — common fields (`author`, `timestamp`, `action_seq`, `prev_action`) live in `action.header` / getter methods |
| `Create`, `Update`, `Delete`, `CreateLink`, `DeleteLink` structs | `CreateData`, `UpdateData`, `DeleteData`, `CreateLinkData`, `DeleteLinkData` |
| `EntryCreationAction` | `TypedAction<EntryCreationData>` |
| `FlatOp::StoreEntry` / `StoreRecord` / `RegisterUpdate` / `RegisterDelete` / `RegisterAgentActivity` | `FlatOp::CreateEntry` / `CreateRecord` / `Update` / `Delete` / `AgentActivity` |
| `FlatOp::RegisterCreateLink{..}` / `RegisterDeleteLink{..}` | `FlatOp::Link(OpLink::CreateLink { link_type, action })` / `FlatOp::Link(OpLink::DeleteLink { original_action, link_type, action })` — STRUCT variants (measured hdi 0.8.0, Lane B); read `action.data.base_address` / `.tag` directly; `OpRecord::CreateLink` no longer carries `tag` |
| `OpType::…` (older alias) | same renames as `FlatOp` |
| `signal_action`: `match action.hashed.content { Action::CreateLink(cl) … }` | `match &action.hashed.content.data { ActionData::CreateLink(cl) … }` |
| `Record::new(signed_action, Option<Entry>)` | `Record::new(signed_action, RecordEntry)` |
| `get_agent_activity(agent, filter, activity)` | adds 4th `GetOptions`; returns `AgentActivityStatus` (we have zero call sites) |
| JS `record.action.author` / `.timestamp` | `record.action.header.author` / `.header.timestamp` (we import no action types; sweep anyway) |
| JS `dumpNetworkStats()` result | unified `ApiTransportStats` under `transport_stats`; `is_webrtc` → `is_direct` |
| Rust client signing traits | `SignedActionHashedExt` etc. move to `holochain_keystore` |

Verification sweeps (from upstream's `upgrade-holochain-0.7` skill; every lane runs the ones for its tree):

```bash
grep -rnE '\.(action|content)\.(author|timestamp|action_seq|prev_action)\b' <tree> --include='*.ts' --include='*.rs'
grep -rnE 'signal_url|signaling|webrtc' <tree> --exclude-dir=node_modules --exclude-dir=target
grep -A1 '^name = "hd[ik]"$' Cargo.lock          # exactly ONE hdi and ONE hdk version per lockfile
```

---

## Lane map and dispatch shape

| Lane | Write-set (disjoint) | Agent | Depends on |
|---|---|---|---|
| A — conductor fork rebase | `elohim/holochain-conductor/**` (new branch `elohim-0.7`) | rust-architect (Opus) | — |
| B — DNA zomes + sdk types + sweettest | `elohim/holochain/dna/**`, `elohim/sdk/domains/*/types/Cargo.toml`, `crates/doorway-client/Cargo.toml`, `elohim/holochain/tests/sweettest/**`, `genesis/a2o/features/**` (tags only) | rust-architect (Opus) | — |
| C — elohim-storage client family | `elohim/elohim-storage/**`, `steward/node/Cargo.toml` (hsb pin only) | rust-architect (Opus) | — |
| D — doorway-service | `doorway/doorway-service/**` | rust-architect (Sonnet acceptable) | — |
| E — toolchain, CI, images, config templates | `che-devworkspaces/**` (submodule), `elohim/conductor-image/**`, `scripts/ci/**`, `genesis/orchestrator/{Jenkinsfile,commit-tag-parser.mjs}`, `elohim/holochain/Jenkinsfile`, `elohim/holochain/edgenode/**`, `genesis/orchestrator/manifests/**`, `app/elohim-app/scripts/hc-{mesh,start}.sh`, `elohim/holochain/dna/elohim/flake.nix`, `steward/device/src-tauri/src/{doorway,lib}.rs` | general-purpose (Sonnet) | — |
| I — JS clients | every `package.json` naming `@holochain/client`, root `package.json` overrides, `app/**/*.ts`, `elohim/sdk/**/*.ts`, `genesis/{a2o,seeder}/**/*.ts`, `elohim/holochain/rna/typescript/**` | general-purpose (Sonnet) | — |
| F — local-mesh gate + fleet cutover ceremony | integration branch, gitlink bump, baseline, habit atoms, operator runbook | integrator (this session) + operator | A–E, I |
| G — tx5-era retirement | `elohim/tx5` + `elohim/kitsune2` submodules, `doorway/doorway-service/src/signal/**` + its routes, coturn manifests, `vendor/holochain_serialized_bytes-0.0.56`, docs/skills packages | general-purpose (Sonnet) | F green on the fleet |
| H — upstream contribution | none in-repo (kitsune2 issue + PR text under `genesis/docs/content/elohim-protocol/history/`) | general-purpose (Sonnet) | A6 |
| J — steward desktop | held until `tauri-plugin-holochain` publishes a 0.7 line | — | external |

Cargo-heavy lanes (A, B, C, D) serialize their compiles on the flock; everything else in them is parallel.

**Lane status (2026-09-03 02:xxZ):** A DONE (`elohim-0.7` @ `25dd2d0be144`, pushed to ethosengine/holochain) ·
B DONE pending the lamad + rea_commitment_replication isolation proof (commit `5367b20bd`) · C DONE
(`b679abbd6`, storage 3206/3206, steward/node rebuilt) · D DONE (`9fc89d5d0`, doorway 1150/1150) ·
E DONE (`949ad16a6` + che `5d7e4829aaad`, pushed) · I DONE (`e422d3f84`, app 4661/4661). All folded onto
`upgrade/holochain-0.7` (scratch worktree `wt-integ`) with the two gitlink bumps. Next: F2 local-mesh
gate (waiting on the household mesh owner's "mesh free"), then F1's `[dna:migrate]` tip + one push.

**Prerequisite hunk (learned 2026-09-02, Lane D's first run):** the six `elohim/sdk/domains/*/types`
crates are path-dependencies of doorway (`imagodei-types`, `infrastructure-types`), storage
(`shefa-types`, `lamad-types`) and the DNA zomes; their `holo_hash =0.6.0` pin drags serde `=1.0.219`
into every graph and makes hsb 0.0.57 unresolvable. That one hunk (`=0.6.0` → `=0.7.0`, six lines)
lives on branch `hc07/sdk-pins` (commit `97696e76c`). Lanes C and D cherry-pick it before their
pin step; Lane B carries the identical hunk; integration merges byte-for-byte.

---

## Lane A — Conductor fork rebase onto `holochain-0.7.0`

**Files:** all inside the submodule `elohim/holochain-conductor` (remote `origin` = `ethosengine/holochain`,
`upstream` = `holochain/holochain`; both 0.7.0 tags already fetched). New branch: `elohim-0.7`.
A dry-run worktree with the clean picks already applied exists at
`/tmp/claude-0/-projects-elohim/0b015666-05d9-4a1d-bf90-6e1e121f70b3/scratchpad/hc07` (branch
`elohim-0.7-dryrun`); use it or recreate from the steps below.

**Fork commit disposition (measured by cherry-pick dry-run, 2026-09-02):**

| Commit | Subject | Disposition |
|---|---|---|
| `fc6724ca5` | `[patch]` tx5 0.8.1 → ethosengine/tx5 zombie fix | **DROP** — tx5 does not exist at 0.7.0 |
| `bd0a250a0` | jemalloc-prof feature | **PICK** (clean) |
| `f9a796b06` | jemalloc production feature | **PICK** (clean) |
| `da823fc6a` | store_slice_hash change-check | **RE-PORT** to sqlx (Task A4) — upstream still inserts unconditionally |
| `23ab107ed` | lockfile repair | **DROP** — regenerate the lock |
| `6d0814266` | Stage-0 iroh relay proof test | **PICK** (clean) — if it no longer compiles against kitsune2 0.5.0 API, drop it and say so |
| `e4a1c9bb2` | cross-relay preflight fallback (vendored kitsune2_transport_iroh 0.4.1) | **RE-DERIVE** against 0.5.0 (Task A6) |
| `b9c7458ae` | sqlite saturation log | **DROP** — `holochain_sqlite` crate deleted at 0.7.0; all DBs are sqlx pools (WAL) in `crates/holochain_data` |
| `c9a6c4439` | sys-validation per-dependency backoff | **RE-PORT** (Task A5) — upstream still refetches every missing dep each pass |

**Interfaces produced:** branch `elohim-0.7` (pushed by integrator) whose HEAD sha12 becomes the
conductor image tag `conductor-<hc12>` (Lane E drops the tx5 half) and the `rev` Lane C may pin if it
keeps a git source.

- [ ] **A1: Branch.** `cd elohim/holochain-conductor && git checkout -b elohim-0.7 holochain-0.7.0`.
- [ ] **A2: Pick the clean three.** `git cherry-pick -x bd0a250a0 f9a796b06 6d0814266`. Confirm the
  jemalloc features compose with the new default set: `grep -nA3 '^jemalloc' crates/holochain/Cargo.toml`
  must show `jemalloc = ["dep:tikv-jemallocator"]` and the `tikv-jemallocator` dep block; the
  `--features` example comment in that block is updated to `encryption,wasmer-sys-cranelift,jemalloc`.
- [ ] **A3: Regenerate the lock.** `flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock cargo generate-lockfile; echo EXIT=$?`
  (CARGO_TARGET_DIR unset is fine for lockfile generation; no build yet).
- [ ] **A4: Re-port the slice-hash change-check in sqlx.** File `crates/holochain_data/src/dht/inner/slice_hash.rs`.
  Replace the unconditional insert with a change-checked one and keep the `ON CONFLICT REPLACE`
  semantics for real changes:

```rust
pub(crate) async fn insert_slice_hash<'e, E>(
    executor: E,
    arc_start: u32,
    arc_end: u32,
    slice_index: u64,
    hash: &[u8],
) -> sqlx::Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    // ELOHIM PATCH (ported from 0.6.3 da823fc6a): kitsune2's historical catch-up
    // re-stores byte-identical slice hashes every cycle. Skip the write when the
    // stored hash already matches; a real change still replaces (PK ON CONFLICT REPLACE).
    sqlx::query(
        "INSERT INTO SliceHash (arc_start, arc_end, slice_index, hash)
         SELECT ?1, ?2, ?3, ?4
         WHERE NOT EXISTS (
           SELECT 1 FROM SliceHash
           WHERE arc_start = ?1 AND arc_end = ?2 AND slice_index = ?3 AND hash = ?4
         )",
    )
    .bind(arc_start as i64)
    .bind(arc_end as i64)
    .bind(slice_index as i64)
    .bind(hash)
    .execute(executor)
    .await?;
    Ok(())
}
```

  Add a test next to the existing slice-hash tests in that crate (find them with
  `grep -rn 'insert_slice_hash' crates/holochain_data/src --include=*.rs | grep -i test`): store
  `h1`, store `h1` again, assert exactly one row; store `h2`, assert one row with `h2`. Run:
  `flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock cargo test -p holochain_data --features test-utils slice_hash 2>&1 | tail -15; echo EXIT=$?` → EXIT=0.
  Commit: `git commit -m "fix(data): change-check slice-hash writes (port of da823fc6a to sqlx)"`.
- [ ] **A5: Re-port the sys-validation per-dependency backoff.** `git cherry-pick -x c9a6c4439` conflicts in
  `crates/holochain/src/core/workflow/sys_validation_workflow.rs` (1 hunk) and its two test files
  (3 hunks) plus `crates/holochain/CHANGELOG.md`. Resolve by re-applying the patch's design to the
  0.7.0 workflow, which still has `fetch_missing_dependencies(&workspace, network, deps)` at ~line
  187 and a single `workspace.sys_validation_retry_delay`: keep two independent per-dependency
  exponential schedules (local re-check capped at 60 s, network fetch capped at 1 h, both due
  immediately at first), the unfetchable threshold after 12 failed fetches (reporting state only —
  never dropped, never validated), the retry-interval log line carrying the unfetchable count, and
  both metrics. Read the original patch for the exact structures: `git show c9a6c4439 -- crates/holochain/src/core/workflow/sys_validation_workflow.rs`.
  Take the upstream CHANGELOG side and append our entry under "Unreleased". Run:
  `flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock cargo test -p holochain --lib --features build_wasms sys_validation 2>&1 | tail -30; echo EXIT=$?` → EXIT=0
  (this compiles the conductor crate; expect 20+ minutes and ~15 GB in the pool slot — set
  `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/crates/dev`).
- [ ] **A6: Re-derive the cross-relay preflight fallback against kitsune2_transport_iroh 0.5.0.**
  Pristine crates are unpacked at `…/scratchpad/k2/kitsune2_transport_iroh-{0.4.1,0.5.0}`; our true
  delta vs 0.4.1 is 116 lines in `src/lib.rs` + `src/tests/url.rs` (`…/scratchpad/k2/patch-delta.diff`).
  Replace `patches/kitsune2_transport_iroh/` with a pristine copy of 0.5.0
  (`curl -sfL https://static.crates.io/crates/kitsune2_transport_iroh/kitsune2_transport_iroh-0.5.0.crate | tar xz`),
  then apply the fallback in `own_url_for_preflight` (0.5.0 signature:
  `fn own_url_for_preflight(peer_url: &Url, space_relays: &HashMap<SpaceId, (RelayUrl, Option<Url>)>, global_url: &Option<Url>) -> Option<Url>`).
  The per-space and global exact-match arms stay; the final arm changes from refuse to fall back:

```rust
        // ── ELOHIM PATCH (2026-08-09, re-derived for 0.5.0 on 2026-09-02) ──
        // Upstream fails closed when the peer homes to a relay we do not
        // ("Peer is on unknown relay, failing preflight"). Our fleet has
        // heterogeneous home relays BY DESIGN: doorways are different operators
        // and peer-stewards register with several. The preflight advertises OUR
        // address so the remote can reach us through OUR home relay; whether
        // the remote's relay is one of ours is irrelevant to our addressability.
        // `None` remains only for the genuinely address-less case.
        if let Some(global) = global_url {
            info!(%peer_url, %peer_relay, own_url = %global,
                "Peer homes to a relay we do not; advertising our own home-relay URL for preflight");
            return Some(global.clone());
        }
        warn!(%peer_url, %peer_relay, "No home-relay URL of our own is known yet, failing preflight");
        None
```

  Port the two url tests from `patch-delta.diff` (`src/tests/url.rs`) so a foreign-relay peer yields
  `Some(global)` and an address-less node yields `None`. Update root `Cargo.toml` `[patch.crates-io]`
  entry comment to say 0.5.0 and keep `kitsune2_transport_iroh = { path = "patches/kitsune2_transport_iroh" }`.
  Run (inside `patches/kitsune2_transport_iroh/` — the crate is workspace-`exclude`d, so cargo refuses `-p` from the root): `flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock cargo test --lib 2>&1 | tail -15; echo EXIT=$?` → EXIT=0.
  Commit: `git commit -m "fix(transport-iroh): cross-relay preflight fallback re-derived on kitsune2 0.5.0 (multi-doorway home relays)"`.
- [ ] **A7: Production feature build check (no image).** In the pool slot:
  `RUSTFLAGS="" flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock cargo check -p holochain --bin holochain --no-default-features --features encryption,wasmer-sys-cranelift,jemalloc 2>&1 | tail -5; echo EXIT=$?` → EXIT=0.
  (`check` is acceptable HERE because nothing but feature composition is being verified; the tests
  in A4–A6 are the dependency-level proof. The image build in Lane E is the binary proof.)
- [ ] **A8: Hand-off.** `git log --oneline holochain-0.7.0..elohim-0.7` must list exactly: jemalloc-prof,
  jemalloc, Stage-0 test (or a note that it was dropped), slice-hash port, sys-validation port,
  transport patch. Report the HEAD sha12. Do NOT bump the monorepo gitlink (Lane F does it after
  the branch is pushed).

## Lane B — DNA zomes, shared sdk type crates, sweettest

**Files (measured sites, 2026-09-02):**

| File | 0.7-breaking hits |
|---|---|
| `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` | 20 |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` | 12 |
| `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` | 8 |
| `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs` | 4 |
| `elohim/holochain/dna/{node-registry/zomes/node_registry_integrity,mishpat/zomes/mishpat_integrity,mishpat/zomes/mishpat,infrastructure/zomes/infrastructure_integrity,imagodei/zomes/imagodei_integrity}/src/lib.rs` | 3 each |
| `elohim/holochain/dna/elohim/zomes/content_store/src/{governance_action,feedback_signal}.rs` | 2 each |
| `elohim/holochain/dna/infrastructure/zomes/infrastructure/tests/peer_status.rs`, `imagodei/zomes/imagodei/src/{qahal_coordinator,hosted_binding}.rs` | 1 each |

Pins to change: `elohim/holochain/dna/{elohim,imagodei,infrastructure,mishpat}/Cargo.toml` lines 9-11
and `node-registry/zomes/node_registry_{coordinator,integrity}/Cargo.toml` (`hdi = "=0.7.0"` →
`"=0.8.0"`, `hdk = "=0.6.0"` → `"=0.7.0"`, `holochain_serialized_bytes = "=0.0.56"` → `"=0.0.57"`);
`elohim/sdk/domains/{lamad,shefa,infrastructure,imagodei,qahal,avodah}/types/Cargo.toml:8`
(`holo_hash = { version = "=0.6.0", … }` → `"=0.7.0"`); `crates/doorway-client/Cargo.toml:20`
(`hdi = { version = "0.6", … }` → `"0.8"`); `elohim/holochain/tests/sweettest/Cargo.toml:187-199`.
`lamad-v1/` is a v1 archive: leave it untouched and note it in the commit. `elohim/holochain/rna/rust/Cargo.toml`
(`hc-rna`, a non-optional path dep of the elohim DNA, `hdk = "0.6"` → `"0.7"`) belongs to this lane too —
without it the elohim lockfile carries two HDK lines. DNA wasm builds need the justfile's
`-C link-arg=--import-undefined` locally (the hc-rna `__hc__*` link atom; CI must never carry it).

- [ ] **B1: Pins.** Apply the table above with `sed -i` per file; then in each DNA workspace run
  `cargo update -p hdk -p hdi -p holochain_serialized_bytes 2>&1 | tail -3` (plain cargo, in-tree target).
  `grep -A1 '^name = "hd[ik]"$' */Cargo.lock` must show exactly one `hdi 0.8.0` and one `hdk 0.7.0` per lock.
- [ ] **B2: Integrity zomes first.** For each `*_integrity/src/lib.rs`: rewrite `validate` per the
  pattern table — `FlatOp::StoreEntry{..}`→`FlatOp::CreateEntry{..}`, `RegisterUpdate`→`Update`,
  `RegisterDelete`→`Delete`, `RegisterCreateLink{base_address,target_address,tag,..}`→
  `FlatOp::Link(OpLink::Create(action))` reading `action.data.base_address` / `.target_address` / `.tag`,
  `RegisterDeleteLink`→`FlatOp::Link(OpLink::Delete(..))`, `RegisterAgentActivity`→`AgentActivity`;
  any helper taking `EntryCreationAction` takes `TypedAction<EntryCreationData>`. Compile each with
  `flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock cargo build --release --target wasm32-unknown-unknown -p <integrity-crate> 2>&1 | grep -E '^(error|warning: unused)' | head -20; echo EXIT=$?`
  (RUSTFLAGS custom getrandom) until EXIT=0 and zero errors.
- [ ] **B3: Coordinator zomes.** `Action::X(..)` matches become `match &action.data { ActionData::X(..) }`
  with common fields via `action.author()` / `.timestamp()` / `.action_seq()` / `.prev_action()`;
  `signal_action`/`post_commit` match on `signed_action.hashed.content.data`; `Record::new(sa, opt)`
  becomes `Record::new(sa, RecordEntry::…)`. The carried-head verification in
  `content_store/src/lib.rs` (`verify_signature(author, signature, record.action())` at ~5171 and
  ~5900) is unchanged in shape: both signer and verifier re-encode the 0.7 `Action` identically.
  Compile each coordinator crate as in B2.
- [ ] **B4: Sweettest family.** In `elohim/holochain/tests/sweettest/Cargo.toml` set
  `holochain = { version = "=0.7.0", default-features = false, features = ["sweettest", "test_utils", "encryption", "schema", "wasmer-sys-cranelift"] }`,
  `holochain_types = "=0.7.0"`, `hdk = "=0.7.0"`, `hdi = "=0.8.0"`, `holo_hash = "=0.7.0"`,
  `holochain_serialized_bytes = "=0.0.57"` (drop `datachannel-vendored`: tx5 is gone). Fix
  compile-driven test-code changes (Action shape). Run in the sweettest pool slot:
  `RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__holochain__tests__sweettest/dev flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock cargo test -- --include-ignored 2>&1 | tail -30; echo EXIT=$?` → EXIT=0
  (**`--include-ignored` is load-bearing**: 146 of the suite's tests are `#[ignore]` and CI runs them with
  `--run-ignored all`; a plain `cargo test` runs 14 and proves nothing about the cross-agent surface).
  0.7 removed `SweetConductor::from_config` and `get_rendezvous()`; `SweetConductorConfig::standard()`
  now demands a rendezvous — use `SweetConductor::standard()` for non-isolated conductors and
  `from_config_rendezvous(config, SweetLocalRendezvous::new().await)` per isolated conductor;
  `await_consistency(N, cells)` → `await_consistency_s(N, cells)` to keep explicit timeouts.
  **Trap measured 2026-09-03:** one `SweetLocalRendezvous` PER conductor makes an "isolated pair" home
  to two different relays, and stock kitsune2 0.5.0 then refuses the cross-relay path — both peers end
  with 0 in limbo and "Consistency not reached" (`rea_commitment_replication.rs:234`, `lamad.rs:1024`).
  Share ONE rendezvous per pair; `disable_bootstrap` alone supplies the discovery partition (the 0.6
  invariant was discovery isolation, never transport isolation). Same failure class as Lane A6's
  fleet partition, reproduced inside the harness. Each rendezvous also costs real RAM: 15 parallel
  multi-conductor tests were shed at 80% with the mesh up — size CI shards accordingly.
  Also run `RUSTFLAGS="" cargo build --features hdk` in `crates/doorway-client` (standalone crate, no
  workspace; the hdi 0.8 pin there has no in-tree consumer that enables the feature).
  **Local scope of this gate (measured 2026-09-03):** the full include-ignored matrix is 39 binaries at
  ~490 s each and the RAM guard sheds it at ~13 GB — CI's sharded nextest (`--run-ignored all`) owns
  the full matrix; locally run `--test lamad --test rea_commitment_replication` (the two binaries that
  exercise `two_agent_conductors_isolated`, i.e. the rendezvous-constructor change) plus whatever your
  edit touched. Known pre-existing red, NOT 0.7: `attention_tending.rs` `create_and_list_succeeds` and
  `refresh_ttl_appends_timestamp` — `list_my_tending` builds `ChainQueryFilter::new()` without
  `.include_entries(true)` (default false on 0.6.0/0.6.3/0.7.0), so the list is always empty; the four
  tests are `#[ignore]` and quarantined by `build-nextest-filter.sh`, so CI never sees them. One-line
  coordinator fix (hash-neutral), filed separately — not part of the migration commit.
  Sweettest needs packed DNAs: pack with the 0.7.0 `hc` from Lane E's `HOLOCHAIN_BIN` directory
  (`hc dna pack` / `hc app pack` per DNA `justfile`); if Lane E has not landed yet, download the
  two binaries into `…/scratchpad/hc-0.7/` from `https://github.com/holochain/holochain/releases/download/holochain-0.7.0/{holochain,hc}-x86_64-unknown-linux-gnu`.
- [ ] **B5: Sweeps + story tag.** Run the three verification sweeps over `elohim/holochain/dna` and
  `elohim/sdk/domains`. Add `@holochain:0.7` beside the existing tags on the a2o scenarios sweettest's
  `@concern:` tags name (find them: `grep -rln '@concern:' genesis/a2o/features | xargs grep -l 'notary\|dna'`);
  no step changes.
- [ ] **B6: Commit** path-limited: `git add elohim/holochain/dna elohim/sdk/domains crates/doorway-client elohim/holochain/tests/sweettest genesis/a2o/features && git commit -m "feat(dna): hdk 0.7 / hdi 0.8 family move — Action header/data, FlatOp renames, sweettest on holochain 0.7.0"`.
  Do NOT touch `dna-hashes.baseline` and do NOT tag `[dna:migrate]` here — Lane F does both from CI-packed hashes.

## Lane C — elohim-storage client family

**Files:** `elohim/elohim-storage/Cargo.toml:155-156` (git-pinned `holochain_client`/`holochain_types`),
`:472` (vendored hsb path patch), `src/happ_manager.rs` (22 API hits: `InstallAppPayload`, `AppStatus`,
`UpdateCoordinatorsPayload`, `uninstall_app`, `install_app`, `update_coordinators`),
`src/services/holochain_humans_replayer.rs:112-124` (`rmp_serde::from_slice::<Vec<Record>>` — Action
shape), `src/services/conductor_writes.rs`, `src/http.rs:7037-7121` (`dump_network_stats` →
`transportStats` JSON), `src/hc_client.rs:543-649`, `src/services/release_adoption/verify.rs`,
`src/api/source_chain.rs`, `src/main.rs`, `src/conductor/process_manager.rs`, `src/services/arc_actuator.rs`,
`src/p2p/mod.rs` (signal mentions), `tests/happ_manifest_relay_url_compat.rs`; `steward/node/Cargo.toml:126` (hsb path patch).

**Not in this lane:** the storage-side iroh `=0.92` → 1.0.x lift (campaign Task A1). The serde
interlock dissolves with hsb 0.0.57, which makes the lift *possible*, not *required*; it is a
follow-on slice after the fleet is green. Storage's `p2p_iroh` and the conductor's kitsune2 iroh are
different processes and crate graphs — no conflict.

- [ ] **C1: Pins.** Replace the two git pins with crates.io finals:
  `holochain_client = { version = "=0.9.0", default-features = false, features = ["lair_signing"] }`,
  `holochain_types = { version = "=0.7.0", default-features = false }`; remove the
  `[patch.crates-io] holochain_serialized_bytes = { path = "../../vendor/holochain_serialized_bytes-0.0.56" }`
  line (and the same line in `steward/node/Cargo.toml`); if a direct `holochain_serialized_bytes` dep
  exists set it to `"=0.0.57"`. Rewrite the long pin-rationale comment (lines ~100-140) to one
  paragraph: the git pin existed to dodge a `holo_hash` 0.6.0/0.6.3 split that the 0.7 family move
  removes. `cargo update -p holochain_client -p holochain_types -p holo_hash 2>&1 | tail -5`.
- [ ] **C2: Compile-driven migration.** `RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock cargo build 2>&1 | grep -E '^error' -A6 | head -80; echo EXIT=$?`.
  Expected classes: `InstallAppPayload` field changes; `AppStatus` variants; `dump_network_stats`
  now returns the unified transport-stats type (adjust the JSON projection at `http.rs:7090-7121`
  so `transportStats` keeps its current top-level keys — add a schema-contract assertion if one
  exists for that route); `Record`/`Action` field access in `source_chain.rs`, `conductor_writes.rs`,
  `release_adoption/verify.rs` per the pattern table. Mechanical adaptation only — a semantic
  choice (a removed field with two replacement paths) is STOPPED and surfaced, not chosen silently.
- [ ] **C3: Replayer fixture.** Add a unit test in `holochain_humans_replayer.rs` that decodes a
  `Vec<Record>` produced by `holochain_types` 0.7.0 fixtures (`holochain_types::fixt` or a
  hand-built `Record::new(SignedActionHashed…, RecordEntry::NA)`) through the existing code path, so
  the cross-boundary msgpack shape is pinned by a test rather than by luck.
- [ ] **C4: Conductor-config generation.** `grep -rn 'signal_url\|webrtc_config\|iceServers' src tests` —
  every hit that WRITES a conductor config or hApp manifest (process_manager, arc_actuator,
  `happ_manifest_relay_url_compat.rs`) drops the two keys; `relay_url` and `bootstrap_url` stay.
  Read-side mentions in `p2p/mod.rs` become comments-only or are removed.
- [ ] **C5: Test.** `RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=… flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock cargo test 2>&1 | tail -30; echo EXIT=$?` → EXIT=0
  (default features; the `p2p-iroh` feature build is exercised by `cargo build --features p2p-iroh`
  only — its tests are known-red per `project_iroh_dataplane_actual_state` and out of scope).
  Then `just gate elohim-storage` (schema contract + codegen freshness).
- [ ] **C6: Sweeps + commit.** Run the three sweeps over `elohim/elohim-storage`. Commit path-limited:
  `git commit -m "feat(storage): holochain_client 0.9.0 / holochain_types 0.7.0 family move; drop vendored hsb 0.0.56; conductor config without tx5 keys" -- elohim/elohim-storage steward/node/Cargo.toml`.

## Lane D — doorway-service

**Files:** `doorway/doorway-service/Cargo.toml:57-75` (pins), `:251` (vendored hsb path patch),
`src/conductor/typed_admin.rs:17-19,248` (`CapAccess` → `CapAccessType`, breadcrumb already present),
`src/routes/health.rs` (`dump_network_stats` shape), `src/auth/permissions.rs:42,88` (method name
unchanged), `src/services/recording.rs`, `src/bootstrap/{k2,k2_mongo,store,types,mod}.rs`.
**Not in this lane:** removing the signal server (`src/signal/**`, 1154 lines) and its routes in
`auth_routes.rs`/`health.rs`/`federation.rs`/`config.rs` — that is Lane G after the fleet is green,
because staging/prod keep tx5 as the rollback line until then.

- [ ] **D1: Pins.** `holochain_client = { version = "=0.9.0", default-features = false }`,
  `holo_hash = { version = "=0.7.0", features = ["encoding"] }`, `holochain_zome_types`/`holochain_types`/
  `holochain_websocket`/`holochain_conductor_api` = `"=0.7.0"`, `holochain_serialized_bytes = "=0.0.57"`;
  delete the `[patch.crates-io]` hsb path line at `:251`. Rewrite the "WHY dev.23 IS THE FLOOR"
  comment block to one line: finals, matching the 0.7.0 conductor. `cargo update` the six crates.
- [ ] **D2: Compile-driven.** `RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock cargo build --release 2>&1 | grep -E '^error' -A6 | head -60; echo EXIT=$?`.
  Apply `CapAccess::Assigned { secret, assignees }` → `CapAccessType::Assigned { secret, assignees }`
  at `typed_admin.rs:248` and the import at `:19`; adapt the `dump_network_stats` consumer in
  `health.rs` to the unified `transport_stats` field; any `AppStatus` match per the union.
- [ ] **D3: Bootstrap wire check.** Diff the HTTP surface of `kitsune2_bootstrap_srv` 0.4.1 vs 0.5.0
  (`curl -sfL https://static.crates.io/crates/kitsune2_bootstrap_srv/kitsune2_bootstrap_srv-0.5.0.crate | tar xz -C …/scratchpad/k2/` and the same for 0.4.1; `diff -ru` the `src/` trees and read every
  route/handler change). Write the verdict as a comment block at the top of `src/bootstrap/k2.rs`:
  CONFIRMED-COMPATIBLE with the changed routes listed, or the exact handler that must change (and
  change it). The relay is NOT served by doorway (separate `iroh-relay` deployments), so the
  integrated-relay feature of the upstream bootstrap server is out of scope.
- [ ] **D4: Test + gate.** `RUSTFLAGS="" CARGO_TARGET_DIR=… flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock cargo test --lib --bins 2>&1 | tail -20; echo EXIT=$?` → EXIT=0;
  `cargo clippy -- -D warnings 2>&1 | tail -5; echo EXIT=$?` → 0; `cargo fmt --check; echo EXIT=$?` → 0;
  `just gate doorway-service`.
- [ ] **D5: Sweeps + commit.** Three sweeps over `doorway/doorway-service`. Commit:
  `git commit -m "feat(doorway): holochain client family 0.9.0/0.7.0 finals; CapAccessType; bootstrap wire verified against kitsune2 0.5.0" -- doorway/doorway-service`.

## Lane E — Toolchain, CI, images, config templates

**Files:** che-devworkspaces submodule: `containers/elohim-edgenode/Dockerfile:19-77` (HC_FEATURES
default, `--no-default-features`, Go toolchain at `:57-58`, `TX5_FORK/TX5_BRANCH/TX5_REF` args),
`containers/elohim-edgenode/conductor-config.yaml`, `jenkins/Jenkinsfile-elohim-edgenode:126-174`
(params + the `irohTransport`/`profiling` isolation logic + tag derivation),
`containers/{rust-dev,udi-plus-mem-rust-nix}/Dockerfile:146-165` (`HOLOCHAIN_VERSION`, `LAIR_VERSION`),
`containers/elohim-storage-zombiefix/Dockerfile`. Monorepo: `elohim/conductor-image/{README.md,build-manifest.json}`,
`scripts/ci/build-storage-image.sh:19,89`, `scripts/ci/validate-conductor-config.sh`,
`scripts/ci/conductor-workload-pin.sh`, `genesis/orchestrator/Jenkinsfile:651-655`,
`genesis/orchestrator/commit-tag-parser.mjs:35-38`, `elohim/holochain/Jenkinsfile` (`:554` coturn deploy,
`:905-954` `resolveRelayUrl` + `resolveStorageImage` iroh split, `:1056-1059` SIGNAL_URL sed,
`:2300,:2516-2535` iroh-lane image pushes), `elohim/holochain/edgenode/{conductor-config.yaml,Dockerfile,Dockerfile.zombie-fix,build-zombie-fix.sh,README.md}`,
`genesis/orchestrator/manifests/edgenode/{alpha,staging,prod}.yaml`,
`genesis/orchestrator/manifests/humans/{_edgenode-conductor.template.yaml,adam-firstman-conductor.yaml,_edgenode-consolidated.template.yaml}`,
`genesis/orchestrator/manifests/doorway/{alpha,alpha-b,prod,staging}.yaml` (signal env only — relays stay),
`app/elohim-app/scripts/hc-{mesh,start}.sh`, `elohim/holochain/dna/elohim/flake.nix:5`,
`steward/device/src-tauri/src/{doorway,lib}.rs` (signal URL plumbing → relay URL), `elohim/elohim-storage/Dockerfile`, `elohim/elohim-storage/build-storage-canary.sh`.

- [ ] **E1: Conductor image build (che-devworkspaces).** In `containers/elohim-edgenode/Dockerfile`:
  `ARG HC_FEATURES=encryption,wasmer-sys-cranelift,jemalloc`; keep `--no-default-features` (drops
  `schema`); delete the Go install (`:57-58`) and every `TX5_*` ARG and clone step; rewrite the
  comment block at `:46-48` (no tx5 rename note; jemalloc rationale stays). In
  `jenkins/Jenkinsfile-elohim-edgenode`: `HC_FEATURES` default = the same string; delete `TX5_REF`
  handling; the source-derived tag becomes `conductor-<hc12>` (12 chars of `HC_REF` only); delete
  the `irohTransport` isolation branch (iroh is the only transport — every build pushes the fleet
  tag; keep the `jemalloc-prof` isolation branch as is). Commit in the submodule on a branch
  `holochain-0.7`; the integrator pushes it and bumps the gitlink.
- [ ] **E2: Monorepo pin derivation.** `scripts/ci/build-storage-image.sh`: derive `CONDUCTOR_PIN`
  from the conductor gitlink only (`git rev-parse HEAD:elohim/holochain-conductor | cut -c1-12`);
  `genesis/orchestrator/Jenkinsfile:651-652`: drop `TX5_REF`; `elohim/conductor-image/build-manifest.json`:
  remove `elohim/tx5` from watched paths; `genesis/orchestrator/commit-tag-parser.mjs:35-38`: the
  `iroh` variant maps to the default feature string (keep the key for old commit messages, document
  it as a no-op); `elohim/conductor-image/README.md`: tag shape `conductor-<hc12>`, the tx5 paragraph
  becomes a one-line history note, the "Variant builds" iroh entry is removed.
- [ ] **E3: Edge Jenkinsfile.** Collapse `resolveStorageImage` (`:926-954`) so every env resolves the
  single `elohim-storage:${STORAGE_TAG}` image (delete the alpha-only `-iroh` repoint and the
  `push-storage-iroh.sh` lane at `:2516-2535`; keep the relay build/push at `:2300`); keep
  `resolveRelayUrl` (D2 stands) but key it off `primaryDoorway.bootstrapUrl` instead of the signal
  URL (add a `relayUrl` field next to `bootstrapUrl` in the primaryDoorway maps at `:535-566` and
  read it directly — no mapping switch); delete the `SIGNAL_URL_PLACEHOLDER` sed at `:1057`; delete
  the coturn deploy at `:554` ONLY behind a `RETIRE_TX5=true` env default false — Lane G flips it
  after the fleet is green.
- [ ] **E4: Conductor config templates.** In each of `genesis/orchestrator/manifests/edgenode/{alpha,staging,prod}.yaml`,
  `humans/_edgenode-conductor.template.yaml`, `humans/adam-firstman-conductor.yaml`,
  `elohim/holochain/edgenode/conductor-config.yaml`, `che-devworkspaces/containers/elohim-edgenode/conductor-config.yaml`:
  delete the `signal_url:` line and the whole `webrtc_config:` block (with its comments); keep
  `bootstrap_url`, `relay_url`, `advanced.k2Gossip`, `data_root_path`, `keystore`. The result must
  parse as `NetworkConfig { bootstrap_url, relay_url, advanced }` at 0.7.0 — validate by running the
  0.7.0 binary once: `HOLOCHAIN_BIN/holochain --config-path <rendered-file> --help` is not a parse;
  instead use `hc sandbox` is gone at 0.7 for webrtc only, so parse with a 10-line Rust test in the
  fork? NO — keep it simple: `python3 -c "import yaml,sys; c=yaml.safe_load(open(sys.argv[1])); n=c['network']; assert set(n) <= {'bootstrap_url','relay_url','advanced','target_arc_factor','request_timeout_s'}, set(n)" <file>` for each file, and let Lane F's local mesh boot be the runtime proof.
- [ ] **E5: Validator inversion.** `scripts/ci/validate-conductor-config.sh`: FAIL if `signal_url` or
  `webrtc_config` is present ("tx5 keys hard-fail a 0.7 conductor"); FAIL if `relay_url` is absent;
  keep the existing D1 check that `relay_url` does not match `*.iroh.network`. Update the header comment.
- [ ] **E6: Local mesh + dev container.** `app/elohim-app/scripts/hc-mesh.sh` and `hc-start.sh`: stop
  emitting `signal_url`/`webrtc_config` in generated conductor configs; the `HOLOCHAIN_BIN`
  directory contract (both `holochain` and `hc`) stays; the "0.6.0 never writes relay_url" notes at
  `:1103,:1148` become 0.7 notes. `containers/{rust-dev,udi-plus-mem-rust-nix}/Dockerfile`:
  `ARG HOLOCHAIN_VERSION=0.7.0`; lair: 0.7.1 publishes no binary asset — replace the lair download
  with `cargo install lair_keystore --version 0.7.1 --locked` ONLY if the container needs a
  standalone lair (grep the devfile and skills for `lair-keystore` usage); otherwise delete the
  download and the `LAIR_VERSION` arg and say so in the commit.
- [ ] **E7: DNA pipeline toolchain.** `elohim/holochain/dna/elohim/flake.nix:5`: `ref=main-0.6` →
  `ref=main-0.7`. `nix` is not in this container, so the lock cannot be regenerated here: add a
  first step to the DNA Jenkinsfile's nix stage `nix flake update holonix --flake elohim/holochain/dna/elohim`
  guarded by `if ! nix flake metadata … | grep -q main-0.7` and commit the regenerated `flake.lock`
  back through the operator, OR run `nix flake update` on the operator's machine and commit the lock
  with this lane. State which path was taken.
- [ ] **E8: steward device.** `steward/device/src-tauri/src/{doorway,lib}.rs`: any conductor-config
  emission drops `signal_url`/`webrtc_config`; a "signal URL" resolved from the doorway becomes the
  doorway's relay URL (same per-doorway pairing). Compile: `RUSTFLAGS="" cargo check` in the
  steward slot (the Tauri plugin itself is held — Lane J — so this is a config-shape change only).
- [ ] **E9: Commit** path-limited to the files listed; message
  `chore(ci,config): holochain 0.7 toolchain — conductor features encryption/wasmer-sys-cranelift/jemalloc, no Go, tag conductor-<hc12>, tx5 keys removed from every conductor config, holonix main-0.7`.

## Lane I — JavaScript clients

**Files:** `app/elohim-app/package.json:63`, `app/elohim-library/package.json:30`,
`app/elohim-library/projects/elohim-service/package.json:22`, `elohim/holochain/edgenode/scripts/package.json:10`,
`elohim/holochain/rna/typescript/package.json:25`, `elohim/sdk/package.json:25,36`, `genesis/a2o/package.json:61`,
`genesis/seeder/package.json:64`, root `package.json` (`pnpm.overrides`). Imports in the tree are limited
to `AdminWebsocket`, `AppWebsocket`, `encodeHashToBase64`, `CellId`, `ActionHash`, `AgentPubKey`, `EntryHash`, `AppInfo`.

- [ ] **I1:** Set every pin to `"^0.21.0"`; add `"@holochain/client": "^0.21.0"` under root `pnpm.overrides`
  (single copy — dual copies break `instanceof`). `pnpm install`; `pnpm ls @holochain/client -r | grep -c 0.21` equals the number of consumers.
- [ ] **I2:** Sweep: `grep -rnE '\.(action|content)\.(author|timestamp|action_seq|prev_action)\b|is_webrtc|signalingServerUrl|dumpNetworkStats' app elohim/sdk genesis/a2o genesis/seeder elohim/holochain --include='*.ts' --exclude-dir=node_modules`;
  fix each hit per the pattern table (header nesting, `is_direct`, `relayServerUrl`).
- [ ] **I3:** `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -15; echo EXIT=$?` → 0;
  `just gate app`; `cd genesis/a2o && pnpm exec tsc --noEmit; echo EXIT=$?` → 0.
- [ ] **I4:** Commit: `git commit -m "chore(js): @holochain/client 0.21 across the workspace, single copy via pnpm override"` path-limited to the files above and `pnpm-lock.yaml`.

## Lane F — Local-mesh gate, integration, fleet cutover ceremony

**Owner:** integrator (this session) for the repo steps; the operator for every fleet step.

- [ ] **F1: Integration branch.** Fast-forward lanes A–E and I onto `upgrade/holochain-0.7`. Bump the
  gitlinks (`elohim/holochain-conductor` → `elohim-0.7` HEAD, `che-devworkspaces` → its
  `holochain-0.7` HEAD) in one commit. Pack every DNA with the 0.7.0 `hc`, read the five hashes
  (`hc dna hash` per packed DNA — roles: `elohim/workdir/happ.yaml` names), write them to
  `elohim/holochain/dna/dna-hashes.baseline`, and commit with `[dna:migrate]` in the message.
- [ ] **F2: Local mesh on stock 0.7.0 binaries.** `HOLOCHAIN_BIN=…/scratchpad/hc-0.7 MESH_CONDUCTOR_LAUNCH=ark just mesh start`
  (stock binaries are enough: the fork patches are fleet-scale behaviours; the cross-relay patch is
  not exercisable on a single-relay mesh). Then `just mesh prologue`, `just seed validate`,
  `just test mesh` (Act I). Then the rung-5 chain on 0.7: the runnable check in
  `elohim/elohim-storage/.epr-meta/runtime-upgrade-propagation.habit.md` (8/8 cucumber) — this is
  the proof that `update_coordinators` and the release controller survive the line change.
  Record receipts under `genesis/a2o/reports/`.
- [ ] **F3: Conductor image.** Integrator pushes `ethosengine/holochain` `elohim-0.7` and the
  che-devworkspaces branch; pushes `upgrade/holochain-0.7` with `[build:conductor]`. Wait for
  `elohim-edgenode` → `conductor-<hc12>` in Harbor (console line `source-derived pin tag`).
  Expect a faster build (no CGo).
- [ ] **F4: Land on dev.** Fast-forward `upgrade/holochain-0.7` → `dev`, push once. Dispatch order
  is conductor → dna → edge (the orchestrator does it; do not hand-trigger). The edge build's
  DNA Hash Guard prints `DNA-HASH <role> <hash>` matching F1's baseline.
- [ ] **F5: Fleet ceremony (operator runbook, sequenced — from `project_alpha_dna_migration_2026_09_02`).**
  **Operator authorization 2026-09-03: wipe the WHOLE fleet clean for this upgrade** — every alpha
  conductor's `databases/` + keystore, and each peer's storage state (diesel DB, blob stores, iroh/libp2p
  keys) so the 0.7 fleet is born from one clean genesis and one re-seed; nothing 0.6-era is carried.
  The same authorization covers the household mesh data root for the F2 gate. Sequencing below still
  holds (wipe AFTER the 0.7 images have rolled, bootstrap pair together).
  (1) Confirm the edge roll carrying the 0.7 hApp and the storage image embedding `conductor-<hc12>`
  has reached every alpha StatefulSet (7 peers + conductor workloads). (2) ONLY THEN clear each
  conductor's full `databases/` tree — 0.7 cannot read 0.6 databases and the lair/agent re-key is
  expected; clearing earlier makes old storage install the 0.6 hashes and re-key twice. adam and
  matthew (bootstrap pair) are cleared in the same window. (3) `DNA_MIGRATION_INTENT=<five baseline hashes>`
  on every node for the first boot. (4) Watch the supervisor: a dead child is reported with exit
  status + stderr tail; `CellWithoutGenesis` on boot means step 2 ran before step 1. (5) Verify with
  the substrate trust-contract probes: every peer `caughtUp`, `GET /db/p2p/conductor-diagnostics`
  shows connections across BOTH relays (the D2 measure: a peer homed on `relay.alpha` connected to a
  peer homed on `relay.elohim.host`), `✓ canonical head propagated` on the deploy. (6) Re-seed once:
  `just seed apply mesh content` per the seed-workflow for the household this host owns; content
  seeds via the operator's pipeline for the fleet.
- [ ] **F5a: Manifest strictness (learned in Lane C).** 0.7.0's `AppManifestV0` restores
  `deny_unknown_fields`, and `NetworkConfig` also dropped `enable_mdns`. A conductor-fork manifest
  field the storage client does not know takes the admin seam down fleet-wide (the 2026-08 dev.23
  incident class). Rule from here: any conductor-side manifest addition and the storage
  `holochain_types` pin land in the SAME batch, and `tests/happ_manifest_relay_url_compat.rs`
  (now asserting strictness) is the tripwire.
- [ ] **F6: Evidence.** Also re-check `elohim/holochain/.epr-meta/notary-authority.habit.md`: its DELTA
  2026-08-09 records `rea_commitment_replication::project_epr_commitment_replicates_to_peer_b` as
  known-RED on 0.6 (attributed to cold-cell wasm warm-up); it PASSES on 0.7 with the shared-rendezvous
  harness (2026-09-03, 186.9 s) — the attribution was probably wrong, and the atom's line is stale.
  One-line DELTA in `elohim/elohim-storage/.epr-meta/runtime-upgrade-propagation.habit.md`
  and in the dataplane-convergence habit atom (fleet on 0.7 on fresh chains, receipt id);
  `.claude/scripts/habits-project.py`; Wave 3 delta in the campaign plan (`OUTCOME 2026-09-xx: DONE via
  2026-09-02-holochain-0-7-upgrade-guide`); memory `project_holochain_0_7_0_assessment` gets a
  "LANDED" line. Cycle-time row for "conductor line change" in the upgrade-propagation arc table.

## Lane G — tx5-era retirement (after F5 is green for 48 h)

- [ ] **G1:** Remove submodules `elohim/tx5` and `elohim/kitsune2` (stale at v0.3.2, patches commented
  out since before 0.6.3): `git submodule deinit -f`, `git rm`, delete `.gitmodules` entries and
  `.git/modules/<name>`; grep `elohim/tx5|elohim/kitsune2` across `scripts/`, `genesis/orchestrator/`,
  `elohim/conductor-image/`, `.github/`, `CLAUDE.md` (managed — via package) and fix each.
- [ ] **G2:** Doorway signal server: delete `doorway/doorway-service/src/signal/**` (bus, bus_mongo,
  cmd, mod, store), its module declaration, routes in `auth_routes.rs`, `health.rs`, `federation.rs`,
  `config.rs` fields, the `signal` permission entries; `cargo test --lib --bins`, clippy, fmt, `just gate doorway-service`.
- [ ] **G3:** Flip `RETIRE_TX5=true` default in `elohim/holochain/Jenkinsfile` (E3) and delete the
  coturn manifests `genesis/orchestrator/manifests/infra/alpha-coturn-{operations,shem}.yaml`; the
  `signal.*.elohim.host` env in the four doorway manifests; operator retires the DNS records and
  the coturn hosts (repo is the cleanup surface; the operator reconciles).
- [ ] **G4:** Delete `vendor/holochain_serialized_bytes-0.0.56/` (no path patch references it after C1/D1);
  delete `elohim/holochain/edgenode/{Dockerfile.zombie-fix,build-zombie-fix.sh}` and
  `che-devworkspaces/containers/elohim-storage-zombiefix/` (tx5 zombie era).
- [ ] **G5:** Docs and skills: `hc-dev-orchestrator` package under `.epr-meta/elohim/packages/skills/`
  (signal_url mentions) → project; `elohim-root-gospel` package (CLAUDE.md mentions of tx5 / `<tx512>`)
  → project; `elohim/holochain/docs/DEPLOYMENT-RUNTIMES.md`, `elohim/holochain/edgenode/README.md`,
  `steward/device/CLAUDE.md` (package), `app/elohim-app/CLAUDE.md` (package). History docs and
  timeline backlog are left as history.
- [ ] **G6:** Commit per sub-step, path-limited; `just gate` for each touched tree.

## Lane H — Upstream contribution (kitsune2)

- [ ] **H1:** Open an issue on `holochain/kitsune2`: "Allow heterogeneous home relays within one
  space — `own_url_for_preflight` fails closed for peers on a foreign relay". Include: the
  multi-operator model (independent doorway operators each hosting a relay; peers registered with
  several), the 2026-08-09 partition evidence, and the one-arm patch from A6. Reference PR #479's
  "no fallback" rule as the design point being questioned.
- [ ] **H2:** PR from a branch of the pristine 0.5.x crate with the A6 change and tests, behind a
  config flag if maintainers prefer (`allow_foreign_relay_peers: bool`, default false upstream, true
  in our fork config). Record issue/PR links in
  `genesis/data/timeline/backlog/iroh-cross-relay-preflight-fails-closed.md`.

## Lane J — steward desktop (HELD)

`tauri-plugin-holochain` has only `main` (2026-05-15) and `main-0.6` (2026-07-16). Retire this hold
when a `main-0.7` branch or a 0.7-compatible tag appears; then: bump
`steward/device/src-tauri/Cargo.toml:20`, re-run the Tauri build, and carry the `wasm_backend: wasmi`
option through for the iOS/App Store path (0.7's declared purpose for wasmi). The upstream Dev Pulse
notes a lair keystore loading issue on iOS (holochain PR #5976) — check its state before any mobile work.

---

## Self-review (2026-09-02)

- **Spec coverage:** campaign Wave 3 items — family move (B, C, D, I), conductor rebase carrying fork
  patches (A), transport sovereignty (D2 stands; A6, H), re-genesis as authorized reset (F5), iroh
  storage lift explicitly deferred (Lane C note). Upstream upgrade-guide items — Cargo pins (B1, C1,
  D1), Action/FlatOp/validation rewrites (B2, B3), coordinator `signal_action` (B3), conductor config
  (E4, E5), JS client (I), toolchain (E6, E7), clean state (F5), sweeps (every lane), lockfile
  single-hdi check (B1).
- **Placeholders:** none; every step names its files, values, commands and exit expectation. E4's
  parse check is deliberately a schema-key assertion plus the F2 runtime boot, not a fake parser.
- **Type consistency:** `conductor-<hc12>` tag (A8, E1, E2, F3); feature string
  `encryption,wasmer-sys-cranelift,jemalloc` (A7, E1, Version Table); `0.0.57` hsb everywhere; the
  A6 function signature matches the 0.5.0 source read on 2026-09-02.
