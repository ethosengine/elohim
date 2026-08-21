---
id: "backlog-mesh-prologue-cast-and-env-gaps"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The mesh Prologue casts the household wrong (adam on james's conductor) and lacks seven env legs CI sets — one root behind four Act I reds"
slug: "mesh-prologue-cast-and-env-gaps"
written: "2026-08-21"
author: "claude (first full a2o inventory + saga run on the local mesh)"
status: "refined"
priority: "high"
jobs: [elohim-genesis, elohim-edge]
nodes: []
relatedNodeIds:
  - "memory:project_tests_layered_as_acts_of_one_story"
  - "memory:project_local_pair_failover_validation_rail"
  - "memory:feedback_mesh_is_the_proving_ground"
tags: [local-mesh, prologue, act-i, seeder, conductor-identities, household-formation, resiliency-saga, doorway-failover]
cites:
  - genesis/seeder/src/seed-conductor-identities.ts
  - app/elohim-app/scripts/hc-mesh.sh
  - scripts/ci/run-mesh-quiesce-stage.sh
  - genesis/a2o/layering/profiles.md
  - genesis/a2o/layering/code-reds.md
---

# The mesh Prologue: cast and env gaps (2026-08-21)

## The cast error (root of four reds)

`seed-conductor-identities.ts` is name-affine by HOSTNAME (`elohim-<name>-*`). On a loopback mesh
(`ws://localhost:4445/4455/4465`) no URL carries a name, so it fell back to first-reachable-wins and
cast **Adam onto james's conductor (4465)**, then refused Jessica/James/Eve/Pete/Terrance
(`conductor at ws://localhost:4465 embodies 'human-adam-firstman'`). Each storage peer already knows who it
is (`GET :809x/auth/me` → human-matthew-manager / jessica-spouse / james-son). Downstream, with one root:
household participants 0 of 0 · `collective_cid` unstamped · no MEMBER_OF / stewarding edges ·
`hasHouseholds: 0` / `onlinePeers known: 0` · saga ch10 `stewardingCollectives 0` (and ch09's landing
`replicates-commons` never minted). Fix in flight: named `CONDUCTOR_URLS` entries
(`matthew=ws://…:4445,…`) — the mesh script should print them in its probe env. A conductor that already
embodies the wrong human cannot be re-keyed: this mesh must be regenerated to re-cast.

## Env legs the Prologue must set (each cost a red or an hour today)

| leg | where | why |
|---|---|---|
| named `CONDUCTOR_URLS` | seed chain / `just mesh` probe env | the cast above |
| `API_KEY_ADMIN` on both doorways | mesh doorway launch | `/admin/*` 403/503 → 6 env-reds |
| `ALLOW_SEED_NETWORK_STAKES=1` on storage peers | mesh storage launch | stage manifest leg 403 → act transitions impossible |
| `ALLOW_SEED_DELEGATES_COMPUTE=1` on storage peers | mesh storage launch | `seed-delegates-compute` honest-fails |
| stage landing AND lamad-spa browser bundles + landing SERVER bundle, per host (upload+PATCH via A, `DECLARE_ONLY` propagate to B) | Prologue step after projections | `/lamad` 404; `serverBlobHash` null → landing custody pair unresolvable; gossip alone left a head un-adopted >8 min behind a 3,400-head seed |
| `CONTENT_BLOB_HASH`/`CONTENT_BLOB_SIZE_BYTES` = the server bundle | before `seed-commitments` | CI exports these from its upload stage; the chain never did |
| household fixture manifest (`E2E_HOUSEHOLD_FIXTURE_PATH`, `processControl: true`, `E2E_STORAGE_<PEER>`, `E2E_DOORWAY_POOL_STORAGE_URLS`, `E2E_DOORWAY_B` + `_BETA`) | a2o run env | 7 env-reds (peer-loss-failover, doorway-pool-degrade) |
| `seed-operator-bindings` → `seed-projections` explicitly | seed chain | `--ids` seeding skips both; router empty → `/` sheds |

Scoping trap for every re-measure: `cucumber-js -p local <files>` runs the whole suite (profile paths merge);
use `--config <empty .mjs, path relative to the REPO ROOT>` or `-p local --name '^…$'`.

## What the corrected Prologue should yield

Saga in order on the mesh today: 17 passed / 2 failed (ch09, ch10) / 2 undefined (ch11 `@wip`) / 2 pending /
2 held — with ch07 custody-witnessed GREEN (the landing's own custody pair made `stocked ≥ 1` real; that red
has been on the alpha shelf as "dead custody-announce plane"). With the cast fixed, ch10 and the
household-formation reds should re-measure; ch09 still needs the doorway's pull-plane `replicates-commons`
for the landing, which needs known peers — re-measure after the re-cast before calling it a defect.

## Addendum 2026-08-21 (evening) — regeneration under load, and the re-key that needs no regeneration

- **`hc sandbox generate` runs the happ install inside the conductor's 60 s admin-websocket timeout.** A COLD
  wasm compile of the five DNAs took 65 s on this box at load ~79 (mesh + two cargo gates) → `Websocket
  error: Timeout`, twice; `generate` refuses an existing directory, so every retry pays the full cold cost.
  `just mesh start` regeneration is therefore load-sensitive: regenerate on a quiet box, or warm the
  `wasm-cache` first. (Found by rehearsing the chaos re-key in a throwaway root — chaos agent, 20f5a6dcd.)
- **A re-key never needed a new sandbox, only a new key.** `hc-mesh-chaos-rekey.sh --method reinstall`
  deletes only `databases/` + `ks/`, keeps `wasm-cache` and the conductor config, boots, then
  `hc sandbox call install-app` (mints a fresh agent key) + `enable-app`: ~12 s, identical DNA hashes (the
  re-keyed peer rejoins the SAME DHT instead of partitioning), and the closer analogue of the 2026-07-24
  alpha event. `--method regenerate` keeps the full wipe for a quiet box (3 attempts, per-attempt logs).
- Cast order fixed in the Prologue (35a2a58b6): identities BEFORE hosted registrations; proven on the next
  regeneration (a conductor that already embodies a UUID cannot be un-embodied).

