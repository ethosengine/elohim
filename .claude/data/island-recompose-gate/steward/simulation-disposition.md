# Simulation disposition — steward/node/simulation/ (session 6, fan-out D)

Subject: `steward/node/simulation/P2P-COMPUTE-FOOTPRINT.md` (+ currency check on `simulation/README.md`)
Prior judgment under test: "simulation/analysis, likely research-home or stay, not retire."

## VERDICT: STAY in simulation/ (dated-but-honest analysis, colocated with a live harness it operates)

Not research-home, not retire. Rationale and evidence below.

## 1. Harness structural check — the directory is a real, complete harness

`ls steward/node/simulation/` (verified 2026-06-11):
- `docker-compose.yml` (10,769 B) — services `family-a-node-1/2`, `family-b-node-1/2` match README topology exactly (compose :37-51 `family-a-node-1`, container_name + per-node toml mount `./configs/family-a-node-1.toml:/etc/elohim/elohim-node.toml:ro`).
- `configs/` — exactly the 4 tomls README:189-195 lists (`family-{a,b}-node-{1,2}.toml`).
- `simulate.sh` (8,802 B) — start/stop/logs/status/partition/heal/clean, matching README's scenario walkthroughs.
- `spawn-testnet.sh` + `gen-configs.sh` — the Phase-1 bare-process testnet scripts P2P-COMPUTE-FOOTPRINT.md:216 names as existing; both present and executable.
- Beyond what either doc documents: `personas.json`, `spawn-persona-testnet.sh`, `gen-persona-configs.sh`, `compute-budget.sh` (persona-testnet extension — undocumented surplus, not drift).

The footprint doc's usage block (:188-206, `./spawn-testnet.sh start 20 --families 4`, `partition`, `heal`, `--netem`) operates scripts that exist in the same directory. Relocating the analysis to a genesis research home would orphan the harness's only operating instructions for Phase-1 mode.

## 2. Load-bearing claim spot-checks (dated-but-honest, not falsified-and-misleading)

| Claim (P2P-COMPUTE-FOOTPRINT.md) | Live truth | Verdict |
|---|---|---|
| "Reed-Solomon blob sharding (4+3) in storage" (:215) | TRUE — `elohim/elohim-storage/src/sharding.rs:97-98` `rs_data_shards: 4, rs_parity_shards: 3`; `reed_solomon_erasure` import :13 | current |
| Seeder pipeline "27 humans in JSON → seeder → POST /api/db/*/bulk → 1 doorway → 1 elohim-storage (SQLite)" (:11) | Shape TRUE, path drifted: live endpoints are `/db/*/bulk` (no `/api` prefix) — seeder `genesis/seeder/src/seed-sqlite.ts:865` posts `${STORAGE_URL}/db/content/bulk`; storage routes the `/db/` prefix at `elohim/elohim-storage/src/http.rs:1113,3111` and handles `content/bulk` at :3138. Centralized bulk seeding is still the live mechanism. | dated path, true shape |
| "27 humans modeled in genesis/a2o" (:4) | Drifted both ways: human fixtures live at `genesis/data/lamad/content/humans/` (28 human JSONs + index.json, counted 2026-06-11); no humans dir under `genesis/a2o`. | dated count + location |
| "No human in genesis actually runs their own conductor or storage instance" (:16) | Partially overtaken: `genesis/orchestrator/data/deployments.json` declares 14 per-human deployments (e.g. `adam-firstman`, pattern `consolidated`, per-human manifest `genesis/orchestrator/manifests/humans/adam-firstman.yaml`); alpha is a live 6-peer mesh. The doc's Phase 2 ("conductors per human") partially HAPPENED — which validates rather than falsifies the analysis. | dated current-state; phased path partially realized |
| "SQLite schema: v3 (with v1→v2→v3 migrations)" (:55) | No live v3 surface: storage uses 60 dated diesel migrations (`elohim/elohim-storage/migrations/`, counted 2026-06-11) and `SUPPORTED_SCHEMA_VERSIONS: &[u32] = &[1]` (`elohim/elohim-views/src/shared.rs:73`). | dated (April-era versioning vocabulary) |
| Human `_schemaVersion: "1.0.0"` (:57) | TRUE — `genesis/data/lamad/content/humans/human-dan-developer.json` carries `"_schemaVersion": "1.0.0"`. | current |
| "simulation compose models infrastructure nodes (2 clusters × 2 nodes), not human agents" (:15) | TRUE — compose services verified above. | current |

Score: the doc is an April-2026 (file mtime Apr 15) compute-footprint *analysis* whose anchor numbers (per-human process stack, RS 4+3, resource tables) and harness instructions remain accurate or useful; its "Current State" framing has dated in count/location/path details, and reality has since moved *toward* its Phase 2 — the failure mode that would justify retire (falsified-and-misleading) is absent.

## 3. Inbound references (blast radius: 2, both process-layer)

Repo-wide grep for `P2P-COMPUTE-FOOTPRINT` (2026-06-11):
- `genesis/data/timeline/backlog/pillar-island-recompose-recipe.md:225` — recipe/process doc (parent updates at gate).
- `genesis/data/timeline/backlog/subject-routing-locus-census.md:70` (row 15) — routes `P2P-COMPUTE-FOOTPRINT` under the `steward/node` self-locus. **Relocation would invalidate the census row's routing plan; STAY keeps it coherent.**

No code, no CI, no skill, no spec cites it.

## 4. simulation/README.md currency check

Structurally current (topology, configs, scenarios, env vars all match the harness — see §1) with one dated edge:
- README :35,:58-63 advertise gRPC ports (`9091`, `9092`); compose maps `9091:9090 # gRPC API` (:59) and `9092:9090` (:174) — but `steward/node/src/api/grpc.rs` is a 3-line TODO stub ("// TODO: Implement device sync API"). The port maps point at nothing listening. Harmless-but-aspirational; one-line repair candidate IF the operator wants it (out of my lane — I edit nothing in-repo).
- OPEN QUESTION: whether the persona-testnet surplus scripts (`spawn-persona-testnet.sh`, `personas.json`, `compute-budget.sh`) deserve a README mention — undocumented machinery, currently zero-doc rather than zero-consumer.

## 5. What the gospel routing line should route FOR

One line in `steward/node/CLAUDE.md` (session A's draft), routing TWO concerns to `simulation/`:
1. **Multi-node testnet harness** — docker-compose 2-family × 2-node sim with WAN latency/partition drills (`simulate.sh`), plus bare-process Phase-1 testnet (`spawn-testnet.sh`, persona variant).
2. **P2P compute-footprint analysis** — per-human conductor/storage process economics and the seeding≠agency gap, **marked as dated 2026-04 analysis** (current-state section predates per-human deployments.json era).

Suggested phrasing shape: "Multi-node testnet harness (compose 2×2 + bare-process spawn-testnet) and dated-2026-04 per-human compute-footprint analysis → `simulation/`".
