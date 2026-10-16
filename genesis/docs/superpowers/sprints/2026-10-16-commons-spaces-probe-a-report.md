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
