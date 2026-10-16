---
id: commons-spaces-probe-a-report
status: complete
serves: nachalah-allotment
date: 2026-10-16
---

*Reading note (2026-10-16, after an independent review): where the receipt below says the production-cadence numbers are a "lower bound", read "a non-steady-state observation; steady-state cost unknown" — regrowth and restart work can move costs either way. "77 s" is when the write was first observed on the outsider (poll sample six), not a measured delivery latency.*

*Tracked copy of the probe receipt. Raw evidence (agent-info, DHT dumps, polls, resource profiles) stays in the gitignored receipt directory `genesis/a2o/reports/recovery/commons-spaces-2026-10-16/` on the device that ran it; every number below cites a file there by relative path.*

# Probe A: clone-space receipt (2026-10-16)

Plan: `/projects/.claude-config/plans/zippy-conjuring-gem.md` §1. All paths below are relative to this
directory (`R`) unless absolute. `D` = `/projects/elohim/.worktrees/commons-spaces/genesis/local-dev/commons-spaces-probe`.
Run window: 01:47–02:38 UTC. The household (admin 4444/4454/4464, storage, doorways, relay) was not
started, stopped or restarted; it stayed listening throughout.

## Versions and topology

- Conductor and CLI: `/projects/.claude-config/tools/hc-fork-96bcd2bee215-proof/bin/{holochain,hc}`.
  `holochain --build-info`: rev `96bcd2bee215f7718f6ecd125dc59c926294bb9f` (clean), 0.7.0, hdk 0.7.0 / hdi 0.8.0,
  rustc 1.96.1. `hc --version`: holochain_cli 0.7.0. Binary sha256 `b0ec7eb9c786…0ddc8` (rp-*/resource-profile.json `binaryFingerprints`).
- hApp: `bundle/workdir/elohim.happ` (lamad `clone_limit: 15`; `bundle/prepare.json`, seed `59734af3-3a52-492a-93f9-a08e18c27e2e`).
- Four empty conductors, no storage attached: P1 :4474, P2 :4484, P3 :4494 (members), P4 :4504 (seed-knowing outsider).
  Data: `D/p{1..4}`; config `D/p{i}/conductor-config.yaml`; launcher `D/launch.sh`; PIDs `D/pids/{dev-cadence,prod-cadence,}/p{i}.pid`.
- `hc sandbox generate` cannot generate without a hApp (it searches cwd: `D/logs/p1.generate.log`), so configs were
  written by hand in the exact shape of the household's `conductors/matthew/conductor-config.yaml`:
  `bootstrap_url: http://localhost:8888/bootstrap` (household doorway), `relay_url: http://localhost:3340/` (household
  iroh-relay 1.0.3), `irohTransport.relayAllowPlainText: true`, `target_arc_factor: 1`, in-proc lair, passphrase piped.
  Launch: `setsid nohup sh -c "echo test | exec holochain --piped --structured=Log --config-path …"`, `RUST_LOG=warn`.
- Gossip, dev cadence (= `hc-mesh.sh patch_mesh_gossip_config`): `k2Gossip.initiateIntervalMs: 1000`,
  `minInitiateIntervalMs: 1000`, `initialInitiateIntervalMs: 1000`, `roundTimeoutMs: 60000`, `maxConcurrentAcceptedRounds: 4`.
  Prod cadence (= `gossip-cadence.sh prod`): the three initiate keys deleted (upstream defaults), the other two kept.
  Backups: `D/p{i}/conductor-config.yaml.{dev,prod}-cadence`. Final state on disk: dev cadence.
- Connections were `direct: true` on every peer (rp-*/resource-profile.json `networkStats`); the relay is the homing URL.

## Commands (F = genesis/a2o/scripts/fixtures-clone.mjs, logs in `probe-logs/`)

1. `install --admin ws://localhost:{4474,4484,4494} --app probe --seed S --cells 5 --bundle bundle/workdir/elohim.happ --out sbx-<port>`
2. `measure … --cells 5` on each member → `sbx-<port>/cost/{agents-5,network-5}.json` (baseline, 0 clones)
3. `grow … --cells 6` on each member (creates clone `fixtures`, seed `S-fixtures-0`); `authorize` on P1 → `credentials-4474.json`
4. `measure … --cells 6`; `inventory` on each member → `inventory-before/<port>/` (see note)
5. `write --admin ws://localhost:4474 --clone fixtures` → `probe/write.json`; `poll --admin 4474,4484,4494 --timeout 600` → `probe/poll-600/poll.json`
6. `install … --cells 6` on P4 with the same seed; `poll --admin 4504` → `probe/poll-outsider/poll.json`;
   `measure` on P1 → `sbx-4474/cost-after-outsider/agents-6.json`, on P4 → `sbx-4504/cost/`
7. `grow`/`measure` at 10 and 20 cells on members; `resource-profile.ts --mode metrics --network-stats --seconds 300`
   at 6, 10, 20 (`rp-dev-{6,10,20}/`) via a shim mesh dir `D/mesh/` (`conductors/p{i}/conductor-config.yaml` symlinks + `household-fixture.json` with the four admin ports)
8. Switch the four configs to prod cadence, restart only the four sandboxes by verified PID, `measure` at 20 (`sbx-<port>/cost-prod/`), `rp-prod-20/`
9. Control: back to dev cadence, restart, `measure` (`cost-dev-restart/`, P1 again at +9 min `cost-dev-restart-late/`), `rp-dev-20-restart/`
10. `inventory` again → `inventory-after/<port>/`; teardown by PID (02:38).

`--cells` counts all cells, including the 5 provisioned roles: 5 = 0 clones, 6 = 1, 10 = 5, 20 = 15 (the clone limit).

## Results

| Question | Result | Receipt |
|---|---|---|
| Clone space membership (3 members) | exactly 3 agents (P1 `usSP972Y`, P2 `uS/ZlkI6`, P3 `2QQYO735`), full arcs `[0,4294967295]`, on all three peers | `sbx-*/cost/agents-6.json` |
| Entry propagation, members | P1/P2/P3 all held the write at the first sample, 01:55:48.55, 5.7 s after the write returned (01:55:42.86). Not slow, not never. | `probe/write.json`, `probe/poll-600/poll.json` |
| Outsider who knows the seed joins? | **Yes.** P4 got the same clone DNA (`uhC0ky17uRLQIW…`); P1 then listed 4 agents in the clone and in the base space | `sbx-4474/cost-after-outsider/agents-6.json`, `probe/poll-outsider/poll.json` |
| Outsider receives the entry? | **Yes**, 01:58:15, 77 s after install returned (01:56:58) | `probe/poll-outsider/poll.json` |
| Base-cell isolation | The probe entry never appeared in the base cell (0 matches). The added base ops are P4's genesis (Dna, AVP, Create agent) and the `InitZomesComplete` actions of P2/P3. | `inventory-{before,after}/*/base-ops.json` |
| At 20 cells, every clone lists its 3 members | yes (15 of 15 clones list 3; the `fixtures` clone and base list 4 because of P4) | `sbx-*/cost/agents-20.json` |

Per-clone marginal cost (members, average of P1–P3; continuous process, dev cadence):

| Interval | RSS / clone | Disk / clone | fds / clone | Threads / clone | CPU / clone | Net / clone |
|---|---|---|---|---|---|---|
| 5→6 (P1) and 6→10 (P2, P3) | one-time step of +150–210 MiB RSS and +140 MiB disk per conductor | | | | | |
| 6→10 (+4 clones) | see step above | 2.3 MiB (P1) | 25.2 | 11.7 | 3.1 % of a core (21.9→34.5 %) | +411 B/s |
| 10→20 (+10 clones) | 6.2 MiB | 3.2 MiB | 25.8 | 11.9 | 3.0 % (34.5→64.7 %) | +383 B/s |

Cost is linear across 6→10→20. Sources: `sbx-*/cost/measurements.jsonl`; `resource-profile-summary.txt` (from `rp-*/resource-profile.json`).

At 20 cells (15 clones), measured about 2 minutes after a restart:

| Window | CPU (avg P1–P3) | RSS | fds | Write I/O | Net | Storage arcs |
|---|---|---|---|---|---|---|
| dev cadence, continuous process | 64.7 % | 838 MiB | 581 | 512 KiB/s | 8.3 kB/s | 47 of 50 agent-arcs full per peer |
| prod cadence, after restart | 2.97 % | 321 MiB | 232 | 23.6 KiB/s | 430 B/s | 0–2 of 50 full, rest **null** |
| dev cadence, after restart (control) | 23.4 % | 340 MiB | 433 | 176 KiB/s | 2.3 kB/s | 0–2 of 50 full; at +9 min 39 of 50 (P1) |

How to read the 20-cell windows:

- **Prod-cadence number is a lower bound.** After a restart every storage arc reads `null`, so neither post-restart window is at steady state.
- **Cadence effect at the same phase:** dev cadence costs about 8× the CPU of prod cadence (23.4 % vs 2.97 %).
- **Restart effect:** the RSS drop from about 830 to 330 MiB comes from the restart itself, not from the cadence.

Sources: `rp-prod-20`, `rp-dev-20-restart`, `sbx-*/cost-prod`, `sbx-*/cost-dev-restart`, `sbx-4474/cost-dev-restart-late`.

## Notes, deviations, limits

- **Harness:** no code change to `fixtures-clone.mjs`. The task asked for `--cells 5` at install. Because 5 means zero clones, install ran at 5 and was then grown to 6 before `authorize` and `write`.
- **Inventory without storage:** `inventory` needs `--db` and `--storage`, so it was run with dummy values. It exits 1 at the projection step, after it has written `base-*` and `lamad.*` dumps.
  - P2 and P3 also fail at `export_all_content`, because P1's credentials do not authorise their cells. Only their `base-{dht,ops,actions}.json` exist.
  - That failed call ran `init` on P2 and P3. This is the source of their `InitZomesComplete` base actions.
- **Bootstrap was the household doorway's `/bootstrap`, unchanged:** the probe spaces were written there. Their network seeds are unique, so the household's own spaces were not touched.
- **`blockedIncoming`:** members reported `blockedIncoming` 41–129 during the dev windows; P4 reported 0. This is unexplained and was not investigated.
- **Not measured:**
  - Steady-state prod-cadence cost at 20 cells. Arcs had not regrown in the window, and there was no time left before the 03:00 lease.
  - perf CPU attribution.
  - The `:946x` Prometheus scrape. `HOLOCHAIN_PROMETHEUS_LISTEN` was not set.
  - Whether the one-time ~140 MiB disk / ~170 MiB RSS step comes from the wasm cache or from the first zome call. It happened on each conductor the first time a zome was called or a clone was created after install.
- 2026-09-07's "no clone-to-clone propagation in 8 min" did **not** reproduce. Here propagation took under 6 s on a clean 0.7 fork with a real relay.

## Missing nodes (story-graph)

- `chain: commons-space membership / between "agent learns the seed" → "agent holds the space's data" / missing node: admission check (membrane proof validated at genesis; probe: same-seed install on a non-member, then agentInfo + poll) / current state: absent — P4 admitted to base and clone spaces and received the entry in 77 s`
- `chain: per-clone cost measurement / between "conductor (re)started" → "steady-state cost window" / missing node: arcs regrown (every agent's storage_arc non-null/full in agents-N.json before the window opens; probe: measure agents-N until full) / current state: arcs null 2 min after restart at both cadences; 39/50 full after 9 min at dev cadence; prod-cadence regrowth not observed`

---

*Tracked copy of the probe B receipt, appended 2026-10-16. Raw evidence stays in the gitignored receipt directory `genesis/a2o/reports/recovery/commons-spaces-probe-b-2026-10-16/` on the device that ran it; paths below are relative to that directory.*

# Probe B: fractional-arc (sharding) receipt (2026-10-16)

Plan: `/projects/.claude-config/plans/zippy-conjuring-gem.md` §2. Paths are relative to this directory
unless absolute. Shift journal: `/projects/elohim/.claude/shifts/2026-10-16T03-40-commons-spaces-land-and-probe-b.journal.md`
(iterations 8–11). The household's conductors, storage peers and doorways were not started, stopped,
restarted or reconfigured; james, matthew and jessica stayed on the pinned fork binary throughout.

### Binaries

| Name | Path | sha256 | What it is |
|---|---|---|---|
| pinned (household, control) | `/projects/.claude-config/tools/hc-fork-96bcd2bee215-proof/bin/holochain` | `b0ec7eb9c786…0ddc8` | fork `96bcd2bee` |
| shard probe | `/projects/.claude-config/tools/hc-fork-96bcd2bee215-shard/bin/holochain` | `519a1053ce26…504da` | fork `96bcd2bee` + probe commit `37276cae4` (branch `probe/shard-arc-96bcd2bee`, local worktree `/projects/elohim/.worktrees/hc-shard-probe`, never pushed): `unstable-sharding` → `kitsune2_gossip/sharding`; `NetworkConfig.target_arc_span` gives every local agent a fixed, sector-aligned partial target arc from its own sector; agents without a span keep full/empty behaviour. `BUILD-RECEIPT.txt` beside it. 4 unit tests pass. |
| launcher | `/projects/.claude-config/tools/hc-fork-96bcd2bee215-shard-span50/bin/holochain` | (script) | writes `network.target_arc_span: 2147483647` (half the u32 ring) into the `--config-path` file, then execs the shard probe |

No submodule pin moved; nothing was pushed to the fork remote.

### P0 — current binary (receipts `p0-*`)

- **Refusal.** On a throwaway sandbox (not a mesh peer), the pinned binary with
  `target_arc_factor: 0.5` exits 42: `network.target_arc_factor: invalid type: floating point 0.5,
  expected u32 at line 19 column 22` (`p0-refusal.log`, `conductor-config.yaml`).
- **Baseline arcs.** Every local agent on matthew (7 per DNA), jessica and james holds storage arc
  1.000 on every DNA (`p0-baseline-arcs.json`).
- **Baseline sizes.** RssAnon matthew 7.4 GiB · jessica 1.8 GiB · james 1.2 GiB. Data dir matthew
  1.82 GB · jessica 2.21 GB · james 1.86 GB.
- **Integrated ops** (admin `dumpFullState`, matthew unless noted): lamad 174,182 (james 173,476) ·
  infrastructure 197,467 · mishpat 72,945 · node_registry 25,008 · imagodei 4,982
  (`p0-baseline-ops-*.json`).

### P1 — why a fresh joiner, not james

Neither kitsune2 0.5 nor holochain at `96bcd2bee` deletes ops that fall outside an agent's arc, so
narrowing james's arc would have left his 173,476 ops in place and answered his out-of-arc reads
from his own store. P1 therefore ran on a **fresh joiner `shard50`** (admin 4514, data
`/projects/elohim/genesis/local-dev/commons-spaces-probe-b/shard50`), installed from the on-disk
`elohim/holochain/dna/elohim/workdir/elohim.happ` with no seed override, joined to the household
through doorway bootstrap `:8888` and relay `:3340`. Gossip config = james's (prod initiate
defaults, roundTimeoutMs 60000, maxConcurrentAcceptedRounds 4).

**DNA parity.** infrastructure, mishpat, node_registry and imagodei match the household's hashes.
**lamad does not** (joiner `uhC0kT_R8…`, household `uhC0kUQPSe…`): the on-disk bundle is newer than
the one the household installed, so lamad is excluded and the joiner's lamad cell sat alone.

### P1 results (receipts `p1/`)

| Criterion | Result | Evidence |
|---|---|---|
| Arc ≈ 50 % | **PASS.** Target arc exactly half `[2021654528, 4169138175]` from install. Storage arc Empty ≈ 10 min, then exactly the target on mishpat, node_registry and imagodei by 09:34–09:39Z; infrastructure (197k ops) partial at 10:08Z and exactly the target by 10:18Z (≈ 56 min). Never jumped to full. | `p1/gossip.log`, `p1/gossip2.log`, conductor log `PROBE: partial target arc` |
| Ops 50 % ±10 % of matthew | **Split by space.** infrastructure **PASS-shaped**: 84,344 integrated vs 198,717 (42.4 %), plus 27,993 in validation limbo (56.5 % with them; limbo not classified by basis). node_registry **FAIL**: 24,185 vs 25,015 (96.7 %). mishpat **FAIL**: 72,136 vs 72,956 (98.9 %). | `p1/snap-final-2.json` |
| …per op, by basis | For basis-readable kinds (AgentActivity → author; CreateLink/DeleteLink → base_address; location = last 4 bytes LE): Counts only, not op identities, integrated ops only. **infrastructure: 36,156 in-arc against matthew's 36,237 (99.8 %) and 6 out-of-arc against his 64,727** — consistent with a correct shard. **node_registry holds all 8,072 in-arc and 3,388 of 3,445 out-of-arc (98 %)** — a leak. mishpat mid-sync: 15,651 of 15,653 in-arc, 4,022 of 20,435 out-of-arc; its total later reached 98.9 %. | `p1/basis-*.json` |
| …the yardstick itself | Data is not uniform on the ring: 70 % of matthew's located node_registry ops and 43 % of mishpat's lie in this half. "50 % of matthew" is the wrong test; "holds the in-arc set and not the out-of-arc set" is the right one. | `p1/basis-*-matthew.json` |
| Out-of-arc reads correct | **PASS.** 120 records from james's and jessica's mishpat chains read through the joiner (`get_commitment_record`, a plain network get): 64 out-of-arc + 56 in-arc, **120 found, 0 not found, 0 errors**; median 59.5 ms out-of-arc vs 27 ms in-arc. Caveat: whether each read was served locally or by another peer was not recorded; with the joiner holding most ops, local service is likely. | `p1/reads.json` |
| Gossip completes | **PASS** on the four matching spaces (lamad excluded: no household peer). mishpat/node_registry/imagodei: rounds complete with all 3 peers (5–9 per peer by 09:51Z). infrastructure: 6–8 completed with each household peer by 10:18Z (4–7 timeouts each along the way; 2 with the control joiner). 17 "Initiated round timed out" in the first minutes. | `p1/gossip*.log` |
| `hc-mesh.sh blocks` empty | **PASS** on matthew, jessica, james (no block rows). Joiner log: no warrant/block lines. | `p1/blocks.txt` |
| Savings | **Integrated-op counts only.** infrastructure −57.6 % integrated, −43.5 % counting limbo. node_registry −3.3 %, mishpat −1.1 % (not a sharding saving). No disk or bandwidth comparison. Joiner at 10:30Z: RssAnon 0.76 GiB, data 0.51 GB across all five cells (household peers 1.2–7.4 GiB and 1.8–2.2 GB, but those carry hosted agents, storage and history — not a matched comparison; no CPU or throttle reading taken). | `p1/snap-final-2.json` |

Side observations: one `PeerBehaviorError { ctx: "Unsolicited RingSector…" }` (kitsune2_gossip
gossip.rs:363); ~1,200 `SQLITE_BUSY`(5)/`BUSY_SNAPSHOT`(517) errors in `integrate_dht_ops_consumer`
during initial sync, probably compounded by the observers' full dumps every 2 minutes.

### Mechanism (Codex read-only reading; key claim verified)

- **Gossip is not the leak.** Both sides exchange the union of their local *target* arcs and
  intersect them (kitsune2_gossip `respond.rs:274`, `respond/initiate.rs:64`, `respond/accept.rs:274`);
  `sharding` only changes storage-arc growth (`storage_arc.rs:75,99`), so full-arc peers need not
  enable it. An Empty storage arc still downloads target-arc data — why ops arrived before it grew.
- **Publish + ingest are** (and they fit why infrastructure's integrated ops look correctly sharded while node_registry and mishpat do not: infrastructure's historic ops reached the joiner by gossip, which honours the arc; node_registry's and mishpat's also arrive by publication). holochain_p2p `spawn/actor.rs:1981` publishes each op to every
  responsive remote agent near its basis (`usize::MAX`), with no check that the basis lies inside
  the receiver's arc (verified; last touched by fork `e8a396d15` "restore validation receipts
  behavior for published ops"). kitsune2_core requests and delivers every advertised id
  (`core_publish.rs:394`, `core_fetch.rs:169,497`). `incoming_dht_ops_workflow.rs:118,162` puts
  received ops into limbo with no arc gate.
- Why those two spaces are still being published to a new peer is **PLAUSIBLE, not confirmed**: holochain republishes an op until it holds `DEFAULT_RECEIPT_BUNDLE_SIZE` = 5 validation receipts (`publish_dht_ops_workflow.rs:29`, checked at `conductor/cell.rs:695`); ops that never reach 5 in a 3-conductor household would be republished to every newcomer, regardless of its arc. Not measured: receipt counts per op per space.
- Validation dependencies are cached into the DHT store with `locally_validated=false` and are
  excluded from the dump, so they do not explain the count.

**Fix shape (not done; an operator-gated fork change):** filter publish destinations by basis
containment in the receiver's arc; gate decoded incoming ops against the local target-arc union
(not the current storage arc, or bootstrap starves); optionally filter ids before fetch in
kitsune2; handle warrants and self-authored exceptions; clean up excess rows already held.

### Control (full-arc joiner on the pinned binary)

A fresh **full-arc** joiner on the pinned proof binary (admin 4524, data
`/projects/elohim/genesis/local-dev/commons-spaces-probe-b/control-full`, same bundle and gossip config),
installed 09:55Z. At 10:19Z (24 min): imagodei and node_registry storage arc full; **infrastructure and
mishpat still Empty**, with infrastructure rounds 0–1 completed and 1–2 timed out per peer
(`p1/control-final-gossip.txt`, `p1/control.log`, `p1/ctl-final.json`). The half-arc joiner reached
its infrastructure target at ≈ 56 min. So a full-arc fresh joiner can also stay Empty a long time on a large space at these gossip
settings; one control does not establish the cause. The half arc grew no slower here.
Control run ended at 24 min, before its infrastructure arc grew; its final op counts were not taken.

### Teardown

10:30Z: both joiners stopped by their recorded PIDs (sh wrapper + holochain child each); ports 4514
and 4524 confirmed closed; observers stopped. Mesh lease released. `just mesh status` afterwards:
matthew/jessica/james storage UP, three doorways, portal, relay and mongod UP, conductors RUNNING
on the pinned fork. Their data directories are kept under
`/projects/elohim/genesis/local-dev/commons-spaces-probe-b/` (≈ 0.9 GB) as evidence; they hold
fresh agent keys used nowhere else and can be removed.
