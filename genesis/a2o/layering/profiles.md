# Runner profiles for the three acts

What to add to `cucumber.mjs`, what the mesh stage must export, and how the CI stages narrow per act.

**Status (2026-08-21): §1's `mesh` profile and the act gate it depends on have LANDED.** `cucumber.mjs`
carries a `mesh` profile with its own tag filter, `substrate-scope.ts` resolves `@act:<i|ii|iii|host>`
to that act's baseline caps, and `just test mesh [<path-or-tags>]` wires the env block below — see
`../LAYERS.md` §"How the gate resolves it" and §"Running the mesh lane". The Jenkinsfiles (§5) and
`cluster-state.yaml` remain untouched: everything below §2 still SPECIFIES rather than describes.

Vocabulary: `@act:i` / `@act:ii` / `@act:iii` / `@act:host` (see `../LAYERS.md`). The act tag carries the
act's baseline caps; `@requires:` appears only for a cap outside that baseline.

---

## 1. The `mesh` profile (add to `cucumber.mjs`)

```js
// Act I — the household. Runs against a mesh this run OWNS (`just mesh start`, or the CI
// mesh stage). Paths are the whole tree because the act is declared by TAG, not by directory:
// Act I scenarios live in features/dataplane, features/auth, features/lamad, features/qahal … alike.
mesh: {
  ...base,
  paths: ['features/**/*.feature'],
  worldParameters: { env: 'mesh', act: 'i' },
},
'mesh-browser': {
  ...base,
  paths: ['features/**/*.feature'],
  worldParameters: { env: 'mesh', act: 'i', deviceMode: 'playwright' },
},
```

Run them as:

```
pnpm exec cucumber-js --profile mesh         --tags '@e2e and @act:i and not @wip and not @browser-only'
pnpm exec cucumber-js --profile mesh-browser --tags '@e2e and @act:i and @browser-only and not @wip'
```

The existing `saga` profile keeps its directory scoping — it is a *measurement* profile with a
deliberately small blast radius, and the profile-discipline note in `cucumber.mjs` (cucumber MERGES a
profile's `paths` with CLI positionals) still applies. `mesh` is the act profile; `saga` is the measure.

## 2. The env block the mesh must export

`scripts/ci/run-mesh-quiesce-stage.sh` PHASE 2 currently exports five variables. Four more names are
load-bearing and missing; two of them are silent-wrong-target bugs, not merely missing config.

```sh
ELOHIM_CLUSTER_STATE_PATH_OVERRIDE="$REPO/genesis/manifests/cluster-state.act1-household.yaml" \
E2E_DOORWAY_ALPHA="$DOORWAY_A_URL" \
E2E_DOORWAY_B="$DOORWAY_B_URL" \
E2E_DOORWAY_BETA="$DOORWAY_B_URL" \
E2E_DOORWAY_APEX="$DOORWAY_B_URL" \
E2E_STORAGE_URL="$STORAGE_A_URL" \
E2E_STORAGE_MATTHEW="$STORAGE_A_URL" \
E2E_STORAGE_JESSICA="$STORAGE_B_URL" \
E2E_STORAGE_JAMES="$STORAGE_C_URL" \
E2E_DOORWAY_PRIMARY_STORAGE_URL="$STORAGE_A_URL" \
E2E_APEX_PRIMARY_STORAGE_URL="$STORAGE_B_URL" \
E2E_DOORWAY_POOL_STORAGE_URLS="$STORAGE_A_URL,$STORAGE_C_URL" \
E2E_DOORWAY_LOG_PATH="$MESH_DIR/doorway-alpha.log" \
E2E_HOUSEHOLD_FIXTURE_PATH="$MESH_DIR/household-mesh.fixture.json" \
PEER_STORAGE_URLS="$PEER_CSV" \
  pnpm exec cucumber-js --profile mesh --tags '…'
```

Why each of the new ones:

- **`E2E_DOORWAY_BETA`** — `features/resilience/doorway-footprint-convergence.feature` and
  `steps/federation-epr.steps.ts` resolve the second doorway under this name; `surfaces.ts` uses
  `E2E_DOORWAY_B`. Setting only one means the other resolves to the **live production doorway**.
  `run-dataplane-validation.sh` already sets both; the mesh stage sets only `E2E_DOORWAY_B`. Fix it in
  the stage, or unify the two names in the fixtures — but do not leave it split.
- **`E2E_STORAGE_<PEER>`** — `surfaces.ts::resolveStorageUrl()` reads `E2E_STORAGE_ALPHA ?? E2E_STORAGE_URL`
  for alpha-A and `E2E_STORAGE_<PEER>` for everything else. The stage exports `PEER_STORAGE_URLS`, which
  only `scripts/substrate-verify.ts` reads. Every scenario naming a non-alpha-A peer's `/metrics` or
  `/p2p/status` goes *pending* for this reason alone — an env-wiring gap that reads as a substrate gap.
  Keep `PEER_STORAGE_URLS` too; it is a different consumer.
- **`E2E_DOORWAY_APEX` / `E2E_APEX_PRIMARY_STORAGE_URL`** — the mesh's second doorway is apex-flavoured
  (`apex-elohim-host`, jessica-primary). `doorway-pool-degrade.feature` fails today with
  `Cannot resolve doorway URL from: E2E_DOORWAY_APEX`.
- **`E2E_DOORWAY_POOL_STORAGE_URLS`** — `requireFixturePoolStorageUrls()` fails closed without it;
  three `doorway-pool-degrade` scenarios red on the missing pool, not on behaviour.
- **`ELOHIM_CLUSTER_STATE_PATH_OVERRIDE`** — the act contract. Without it the run reads
  `cluster-state.yaml`, where the new caps are undeclared and therefore inert.

## 3. Mesh stage obligations (beyond env)

1. **Stage the lamad SPA bundle**, not only the landing blob. `app/lamad/dist/lamad/browser` already
   exists; `GET /lamad` currently returns `404 {"error":"App not found: lamad-spa"}`. This is the single
   change that flips `ssr-bundle` to `true` in the Act I lane contract and unholds **186 Act I scenarios**.
2. **Configure doorway admin bootstrap** (`API_KEY_ADMIN`) on both mesh doorways. Six scenarios red today
   with `403 Admin permission required` / `Admin bootstrap is not configured`.
3. **Set `ALLOW_SEED_NETWORK_STAKES`** on the mesh peers — the stage-manifest leg 403s without it, and act
   transitions (Simulacra → Bootstrap → Coordinated) are exactly what Act I exists to prove.
4. **Fail the stage on a non-zero seeder leg.** `seed-commitments` and `seed-delegates-compute` exit
   non-zero and the stage continues; seven scenario reds are downstream of that single swallowed error
   (see `code-reds.md` § B).

## 4. The household fixture manifest the mesh must emit

Write this to `$MESH_DIR/household-mesh.fixture.json` during bring-up (after PIDs exist) and point
`E2E_HOUSEHOLD_FIXTURE_PATH` at it. Shape and field meanings:
`src/framework/fixtures/household-mesh.ts`; the local-stack template is
`household-mesh.fixture.example.json`.

```json
{
  "$comment": "Emitted by run-mesh-quiesce-stage.sh / just mesh start. Act I owns its substrate.",
  "commonsEprId": "elohim-host-landing",
  "convergenceWindowMs": 60000,
  "connectedPeersFloor": 2,
  "processControl": true,
  "processControlReason": "Act I mesh — peers and doorways run as processes on this host",
  "doorways": {
    "alpha": {
      "url": "http://localhost:8888",
      "primaryStorageUrl": "http://localhost:8090",
      "poolStorageUrls": ["http://localhost:8090", "http://localhost:8092"],
      "logPath": "/tmp/elohim-local-mesh/doorway-alpha.log"
    },
    "beta":  { "url": "http://localhost:8889", "primaryStorageUrl": "http://localhost:8091",
               "logPath": "/tmp/elohim-local-mesh/doorway-apex.log" },
    "apex":  { "url": "http://localhost:8889", "primaryStorageUrl": "http://localhost:8091" },
    "gamma": { "absentReason": "Act I stages two doorways (A/alpha and B/beta=apex); a scenario naming a third must say which act it needs" }
  },
  "storagePeers": {
    "matthew": { "url": "http://localhost:8090", "pidFile": "/tmp/elohim-local-mesh/matthew.pid" },
    "jessica": { "url": "http://localhost:8091", "pidFile": "/tmp/elohim-local-mesh/jessica.pid" },
    "james":   { "url": "http://localhost:8092", "pidFile": "/tmp/elohim-local-mesh/james.pid" }
  }
}
```

`processControl: true` is the whole point. `requireFixturePeerPid` and `requireFixtureDoorwayLogPath` fail
closed without it, which is why the kill-a-peer and tail-the-log families can *never* be honestly green on
the fleet — and are ordinary Act I work here. `beta` and `apex` deliberately name the same doorway: the
mesh's second doorway is apex-flavoured, and features reach it under both ids.

**Declaring `gamma` absent is knowledge, not a gap.** `fixtureDoorwayUrl()` throws the topology fact
("this act stages two doorways") instead of an environment-variable name, which is what a reader needs.

## 5. CI narrowing per act

**Mesh stage, PHASE 2 (Act I).** Today: `--profile saga --tags 'not @wip and not @browser-only'` — 26
scenarios. Proposed: keep the saga measure, then add the act suite as a second, non-blocking leg while the
step debt drains.

```
# leg A (unchanged) — the measure
--profile saga --tags 'not @wip and not @browser-only'
# leg B (new) — the act
--profile mesh --tags '@e2e and @act:i and not @wip and not @browser-only'
```

`MESH_E2E_BLOCKING=1` stays off for leg B until the code-reds in `code-reds.md` are drained; the honest
framing already in the stage ("a red here is not automatically a code regression") applies to leg B too.

**Dataplane Validation (Act II).** Today: `--tags '@dataplane and not @wip and not @browser-only'` — a
directory-shaped filter that pulls in Act I scenarios which then measure the fleet for household facts.
Proposed:

```
ELOHIM_CLUSTER_STATE_PATH_OVERRIDE=genesis/manifests/cluster-state.act2-neighbourhood.yaml
--tags '@dataplane and @act:ii and not @wip and not @browser-only'
```

and, for the chapters that must keep measuring the fleet even though they are Act I by substrate (the
saga's recorded board), a second explicit leg:

```
--tags '@dataplane and @act:i and @concern and not @wip and not @browser-only'
```

Pair the dispatch with `[build:edge] [edge:validate-only]` so measurement does not restart the seven pods
it is measuring.

**Act III** has no stage of its own yet. Its 55 scenarios keep `@requires:shem` and stay held by
`cluster-state.yaml` until a shem lane exists. Do not fold them into the alpha lane to make them run.

**host lane.** 101 scenarios need no substrate at all. They belong in a cheap pre-push or genesis-pipeline
leg — `--tags '@act:host'`, no services, no fixtures — not in a stage that waits on a quiesce gate.
