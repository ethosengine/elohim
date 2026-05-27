# Sprint Result: substrate-rea Task 10 — alpha deploy verification

**Date:** 2026-05-27
**Plan:** [genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md](../plans/2026-05-26-substrate-rea-replication-fix.md) Task 10 (lines 1127–1198)
**Trigger commit:** `c014e896f` (orchestrator dev #1069)
**Verdict:** **HALTED — three independent regressions surfaced. No fixes attempted.**

> **Update 2026-05-27 post-operator-correction.** My initial diagnosis attributed the halt to cluster-state alone and exonerated the substrate-rea code. The operator's cluster-side `kubectl describe` / `kubectl logs` pass corrected this in three places — preserved below in the "Corrected diagnosis" section. The CI-visible facts (pipeline outcomes + probe responses) stayed the same; what changed is the root-cause attribution.

---

## Outcome at a glance

The substrate-rea fix (Tasks 1–9) **is on alpha** — the storage image at tag `1.0.0-dev-c014e896` was pushed to Harbor and applied to all 14 peer manifests; doorway-alpha rolled out cleanly. **But the deploy did not stabilize**, and once the operator looked cluster-side, three independent regressions surfaced (see "Corrected diagnosis" below). Every Step 3 probe failed:

| Probe | Expected | Actual |
| --- | --- | --- |
| `GET /api/v1/commitments?action=project-epr` | ≥6 rows, all with `dhtAnchorHash` | `[]` (empty array, 200 OK) |
| `HEAD /apps/elohim-host-landing/index.html` | `x-content-address: sha256-<real-hex>` | `HTTP/2 404` (bundle no longer in cache) |
| `HEAD /lamad` | `HTTP/2 200`, `text/html` | `HTTP/2 404`, `application/json` (doorway not-found hint) |
| `GET /` | 200 → SPA chunks resolve | `HTTP/2 302` → `/apps/elohim-host-landing/index.html` (which 404s) |

The CI-visible evidence (pipeline logs + HTTPS probes) made this look like a cluster-side peer-health issue. The operator's cluster-side pass corrected the diagnosis — substrate-rea code IS implicated in at least one of the three regressions.

---

## Corrected diagnosis (post-operator cluster pass)

**1. The 9 "timed-out" peers are scheduled and Running — they're failing readiness, not rollout.**
All 14 StatefulSets have a Pod scheduled with the container running. 5 are Ready=1/1 (`adam`, `matthew`, `jessica`, `james`, `terrance`); 9 are stuck 0/1 (the same 9 the pipeline log named). What `kubectl rollout status --timeout=600s` was actually waiting on was the readiness probe, not pod scheduling. The real failure mode is `install_app` over the embedded conductor's admin websocket timing out — daniel's previous-instance log ends with `Error: install_app failed: Websocket error: Timeout` after Conductor ready + 10-attempt handshake. Pete's readiness probe times out 52× in 16 min while the embedded conductor is still installing the hApp.

The forcing function is **CPU contention on the `shem` node**: shem is at 48% CPU (11.6 cores) carrying 13 of 14 elohim alpha pods + a doorway replica; each stuck conductor burns 700–1000m just spinning up. The 5 healthy peers either landed on `ethosengine` (matthew, jessica, james) or got onto shem early enough to complete `install_app` before contention spiked (adam, terrance).

**2. Genesis is polling a workload that doesn't exist — name mismatch, not peer down.**
There is no `elohim-timothy-tutor-alpha` Pod / StatefulSet / Deployment / Job anywhere in any namespace. `kubectl get all -A | grep timothy` only returns Services (`elohim-timothy-alpha` / `-headless`) and a 0/0 `elohim-timothy-staging` StatefulSet. The "0 completed, 0 pending for the full poll window" symptom is exactly what you'd see polling a name that has no backing pod. **Either the seeder manifest didn't render at all this run, or the polled name (`...-tutor-...`) doesn't match the deployed name (`elohim-timothy-alpha`).** This was attributed to "cluster restart cascade" in the initial report — wrong; it's a seeder-manifest / persona-name mismatch.

**3. substrate-rea code IS implicated — inventory verifier rejects every hash.**
Every healthy peer's log is currently spamming:

```
WARN elohim_storage::inventory: Inventory snapshot failed structural verify — dropped
  from=12D3KooW... error=InvalidHashFormat("sha256-1f3ed518a975f0eb55ae72c7cca8ef396c8f73c61ecf730ad54920ea0a24a955")
```

Root cause: `is_blob_hash_shaped()` at `elohim/elohim-storage/src/p2p/inventory_gossip.rs:132–134` requires exactly 64 lowercase hex chars:

```rust
fn is_blob_hash_shaped(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}
```

But the canonical wire format per `elohim/elohim-storage/CLAUDE.md` is **prefixed**: `sha256-<64-hex>` (71 chars total). `BlobStore::list_hashes()` returns prefixed hashes; the broadcaster stuffs those directly into `BlobInventorySnapshot.hashes`; the verifier's length check (`s.len() == 64`) rejects every one. The two `is_blob_hash_shaped()` call sites (lines 91 and 123 in `verify_structural`) thus drop **every** inventory snapshot and delta with `InvalidHashFormat`. Even if Genesis ran, no inventory data would reach `/api/v1/commitments` and the project-epr propagation the substrate-rea fix is supposed to enable would still appear broken.

`inventory_gossip.rs` itself landed 2026-05-02 as commit `9169ab99d` (T13). The mismatch likely went latent until substrate-rea exercised inventory gossip end-to-end on a real multi-peer cluster — this is the kind of bug that only fires when the producer + verifier sit on different nodes carrying real data. The fix is small (relax the verifier to accept `sha256-<64-hex>` OR strip the prefix at the broadcaster); it sits cleanly **outside Tasks 1–9** but **inside the substrate-rea bring-up surface area**.

**4. The 404 probes are a separate pre-existing miss.**
Ingress `elohim-site-alpha-ingress` (`alpha.elohim.host`) exists, but `elohim-site-alpha-service` has no backing pods anywhere — no `elohim-site-alpha-*` Deployment in `kubectl get all`. So the 404s on `/apps/elohim-host-landing/index.html`, `/lamad`, and `/` will **not** clear by re-running Genesis — that frontend was never deployed in this run. The initial report attributed these to "bundle cache lost across doorway pod restart"; wrong — the bundle was never deployed.

---

## Pipeline outcomes (orchestrator dev #1069)

Triggered by `c014e896f`. Final orchestrator result: **UNSTABLE**.

| Pipeline | Build | Result | Notes |
| --- | --- | --- | --- |
| elohim-orchestrator/dev | #1069 | UNSTABLE | Aggregated from Edge UNSTABLE + Genesis FAILED |
| elohim-holochain/dev (DNA — Lamad) | #1292 | SUCCESS | clean |
| elohim-edge/dev | #1012 | UNSTABLE | Storage built + pushed; doorway-alpha rolled out clean; 9/14 storage peer rollouts timed out at 600s |
| elohim-genesis/dev | #1045 | FAILED | Seed Database failed on `timothy-tutor` peer offline; cascade through 14 downstream seed stages |
| elohim (App, root Jenkinsfile) | — | NOT DISPATCHED | substrate-rea changeset matched only `elohim-storage/*` + `holochain/*` paths; App pipeline did not trigger for #1069 |
| DNA — Mishpat | — | NOT DISPATCHED | (no dispatch visible; not in the changed paths for this run) |
| elohim-steward | — | manual-only | skipped per brief |

### Edge #1012 — what actually happened

The new storage image **was** built and pushed (`Build Storage: SUCCESS`, `Push to Harbor: SUCCESS`, image tag `1.0.0-dev-c014e896`). The "Deploy Edge Node - Alpha" stage applied the manifest substitution `STORAGE_TAG_PLACEHOLDER → 1.0.0-dev-c014e896` and issued `kubectl rollout restart statefulset/<peer>-alpha -n elohim-alpha` for every peer. The doorway deployment rolled out clean and the health probe at `https://doorway-alpha.elohim.host/health` returned green.

Then the stage waited on `kubectl rollout status --timeout=600s` for each peer. **5 reached Ready:** `adam`, `matthew`, `jessica`, `james`, `terrance`. **9 timed out:**

```
error: timed out waiting for the condition
ERROR: deploy elohim-pete-alpha: rollout failed
[... same pattern for: frank, gertrude, susan, caleb, daniel, emma, eve, nancy]
Edge deploy partial failure: 9/14 peers did not reach Ready
Stage marked UNSTABLE (test-result shape); downstream pipelines proceed.
```

Build description: `hApp:NO | Push:Skip | Deploy:?` — the `?` reflects the partial failure. **No seeder ran in Edge** — pipeline log states `Note: Seeding handled by genesis/Jenkinsfile`.

### Genesis #1045 — what actually happened

`Verify Target Health` was green (`https://alpha.elohim.host` reachable). Content seeding **succeeded**: `Total inserted: 3,433 / Total errors: 0` for matthew-manager; jessica-spouse replicated 400 items, 0 pending. Then the replication poller hit `timothy-tutor`:

```
human-timothy-tutor: 0 completed, 0 pending
[... repeats ~52 times across the 600s poll window ...]
❌ human-timothy-tutor replication timed out
ERROR: script returned exit code 1
```

The pod never reported a non-zero queue. The remaining 14 stages cascaded FAILED. **No mention of `rea_commitments`, `project-epr`, `ContentStore`, `dht_anchor`, or `AppWebsocket` anywhere in the failure path** — the substrate-rea logic was never reached.

Last successful Genesis run (#1042) showed `human-timothy-tutor caught up: 17 replicated this run, 0 pending`. The peer regressed between #1042 and #1045; the most likely cause is the cascade of pod restarts in Edge #1012 (kubectl rollout restart fires on every peer regardless of prior state).

---

## Alpha cluster state (post-pipeline, observed via HTTPS only)

```
GET /admin/routes
  stewardUrl: http://elohim-terrance-alpha.elohim-alpha.svc.cluster.local:8090
  totalRoutes: 1395  (was 3069 before this run)

GET /health/startup
  identity.ready: true   (did:web:alpha.elohim.host)
  storage.ready: true
  projection: { ready: true, content: 540, humans: 0, relationships: 0 }
  rootProjection: null
  warmup.lastError: "Failed to connect to elohim-nancy-alpha.elohim-alpha.svc.cluster.local:8090/api/v1/cache/stream"
```

`stewardUrl` shifted from `elohim-nancy-alpha` (pre-deploy state observed in the FeaturePromise) to `elohim-terrance-alpha` — terrance was one of the 5 successful rollouts, so doorway has paired with a healthy substrate-rea peer. But `rootProjection=null` and content count dropped from "landing-page cached with placeholder hash" to "540 rows, no rootProjection" — consistent with the new pod boot reading from terrance which never received the seeded landing-bundle blob.

---

## @wip removal status

The plan called for dropping `@wip` from the two `@substrate-rea-replication-fix` scenarios in `genesis/a2o/features/delivery/spa-bundle-delivery.feature` (lines 56–89). Per the brief's stop conditions ("Step 3 probe returns 404… halt + report"), the `@wip` removal **was reverted before commit** — the seatbelt must not land in a state where it actively fails CI on every alpha run.

### What did land in the working tree (uncommitted)

Step-definition scaffolding for the two regression scenarios is complete and validated, but **not committed**:

- `genesis/a2o/steps/delivery/substrate-rea-replication.steps.ts` (new) — 6 step defs for scenario 1 (substrate-verifiable via HTTP probes), 4 step defs for scenario 2's pod-restart steps (return `'pending'` since kubectl is operator-only)
- `genesis/a2o/steps/delivery.steps.ts` (modified) — exported `fetchApp` and `responseStore` so the new file can share response capture with the existing `Then('the response status is {int}')` step
- Both files pass `pnpm run typecheck`, `pnpm run lint` (0 errors), `pnpm run format:check`, and `npx cucumber-js --profile delivery --tags '@substrate-rea-replication-fix' --dry-run` (20 steps, all defined, 0 ambiguous)

The operator can land this scaffold separately as `test(a2o): scaffold step defs for substrate-rea regression scenarios` once the cluster heals, and a follow-up commit (`test(a2o): regression seatbelt — substrate-rea replication on alpha`) drops `@wip` from the two scenarios after probes pass.

---

## /deliver verdict

**Not minted.** The FeaturePromise at `.claude/deliver/feature-promise-epr-app-delivery.json` requires Step 3 + Step 5 to pass before `/deliver epr-app-delivery` can issue a verdict. Step 3 failed on every probe; Step 5 was deferred.

---

## What was left for the operator

Three independent regressions must clear before Task 10 verification can complete. They're independent (any one can be addressed without the others); but **all three must be green** for the substrate-rea fix to actually be visible on alpha.

### Recommended operator sequence (per the operator's cluster-side pass)

1. **Verify the seeder manifest** — either the rendered StatefulSet name is `tutor` and was never applied, or Genesis is polling the wrong name. Easiest check: `helm get manifest <release> -n elohim-alpha | grep -i timothy`. Compare polled name (`elohim-timothy-tutor-alpha`) against deployed name (`elohim-timothy-alpha`); reconcile in whichever side is wrong (seeder polling logic or persona-manifest rendering).

2. **Fix the `InvalidHashFormat` verifier regression** at `elohim/elohim-storage/src/p2p/inventory_gossip.rs:132–134`. The fix is small — `is_blob_hash_shaped` needs to accept the canonical `sha256-<64-hex>` wire format. Two viable shapes:
   - **Relax the verifier** (preferred per canonical wire format): strip the `sha256-` prefix before the length check, e.g. `s.strip_prefix("sha256-").is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))`. Update the tests at lines 140–225 to use the prefixed form.
   - **Normalize at the broadcaster** (less preferred — diverges from `elohim-storage/CLAUDE.md`'s "Wire-level identifiers (`/blob/{hash}`, `BlobStore`, `sha256-{hex}`) keep their existing names"): strip the prefix in `inventory_broadcaster::snapshot_from_inventory()` before constructing `BlobInventorySnapshot.hashes`.
   The first option keeps the wire format canonical and minimises blast radius. The peers will resume accepting each other's snapshots immediately on the next image roll; no DHT state surgery needed.

3. **Reschedule storage peers off `shem`** — anti-affinity by household or a topology spread on `kubernetes.io/hostname` before the next `kubectl rollout restart`. Otherwise the install_app cascade will time out for the same conductor-spin-up reason. Shem is currently carrying 13 of 14 alpha pods + a doorway replica at 48% CPU; spreading across `ethosengine` + other available nodes should let the embedded conductor handshakes complete inside the 600s window.

4. **Address the missing `elohim-site-alpha` Deployment** — the ingress exists but the service has no backing pods, so `/apps/elohim-host-landing/index.html`, `/lamad`, and `/` will 404 even when the three above are green. This was not part of the substrate-rea sprint; it's a separate deploy gap (the landing/SPA frontend was never deployed in this run). Best path forward depends on whether the intent is to deploy via App pipeline (root Jenkinsfile) or surface a follow-up plan.

5. **Re-trigger Genesis** (after #1, #2, #3 land) — re-run `elohim-genesis/dev` from Jenkins UI, or push to dev. The seeder is what exercises the substrate-rea write path end-to-end; until it runs against a cluster with the verifier fixed, the fix is unverifiable.

6. **Re-probe alpha** — repeat the four Step 3 probes. If `dhtAnchorHash` is non-null on every row and `/lamad` returns 200 (assuming #4 is also addressed), the substrate-rea fix is confirmed.

7. **Step 4 pod-restart robustness check** (deferred from this shift) — `kubectl delete pod -n elohim-alpha -l app=doorway-alpha` and immediately re-probe `/lamad`. Expected: 200, regardless of which storage peer the new pod paired with. This is the regression seatbelt's central claim.

8. **Land the seatbelt** — drop `@wip` from the two scenarios in `spa-bundle-delivery.feature` and commit alongside the (currently uncommitted) step-def scaffold. The seatbelt would have caught regression #2 (and would correctly fail today because of regression #2's downstream effect on commitment visibility); seatbelt was held back because it would have shipped red.

### Why I did not commit anything

Two reasons:

1. **Stop-condition compliance** — the brief's stop conditions say "Step 3 probe returns 404, placeholder hash, or missing dht_anchor → halt + report." All four probes failed in this category. The seatbelt cannot land while it would actively red-light CI on every alpha run.
2. **Working-tree noise** — the session opened with a "clean" git status but nine unrelated files (7 modified, 2 untracked) appeared in the working tree during the session, all in `app/` paths I did not touch: `app/elohim-app/src/app/app.config.ts`, `app/elohim-app/src/app/shefa/services/signal-emit.service.ts`, `app/elohim-library/projects/elohim-rea-runtime/src/public-api.ts`, `app/lamad/src/app/models/index.ts`, `app/lamad/src/app/services/index.ts`, `app/lamad/src/app/services/signal-harness.service.ts` + spec, plus two untracked files at `app/elohim-library/projects/elohim-rea-runtime/src/lib/signal-emit*.ts`. These look like an in-progress signal-emit refactor (consistent with the branch's "Slice 2.3 milestone" feel) — but I did not author them and have left them untouched so the operator can decide whether they're intentional in-progress work or stale workspace state.

The branch I was placed on is `sprint/cross-pillar-cleanup` (4 commits ahead of `dev`), not `dev` itself, so pushing to dev from this branch would have required a separate decision the brief didn't authorize.

---

## Lessons / memory candidates

- **CI-visible evidence under-determines cluster diagnosis.** Three separate ground-truth corrections came from the operator's `kubectl describe` / `kubectl logs` pass: (a) "9 peer rollouts timed out" was actually "9 readiness probes fail because embedded conductor `install_app` is timing out", (b) "timothy-tutor offline" was actually "no workload named timothy-tutor exists; seeder polls the wrong name", (c) "substrate-rea code not implicated" was actually "every peer is spamming `InvalidHashFormat` from a verifier in substrate-rea's adjacent code". Pipeline logs gave me Edge UNSTABLE + Genesis FAILED + empty probes; from there, three different conclusions were possible and I picked the most charitable to the new code. Worth a feedback memory: **when CI surfaces UNSTABLE + downstream failure + empty probes against a fresh deploy, the failure shape on the cluster is the ground truth — ci-investigator output is necessary but not sufficient. Ask the operator for `kubectl get pods -A | grep <namespace>` + `kubectl logs <stuck-pod>` before attributing causality.**
- **`is_blob_hash_shaped` rejects the canonical wire format** at `elohim/elohim-storage/src/p2p/inventory_gossip.rs:132–134`. The verifier was added 2026-05-02 (T13, commit `9169ab99d`) before the prefixed `sha256-<hex>` wire format was canonized in `elohim-storage/CLAUDE.md`. A test that explicitly used `"a".repeat(64)` instead of a real-shape `"sha256-…"` masked this until multi-peer alpha exercised the gossip path end-to-end. Worth a feedback memory: **structural-verify tests should use the canonical wire-format string shape, not constructor synthetic shapes**, or they will pass while the production verifier rejects every real message.
- **Multi-peer CPU contention scales install_app latency past the rollout window.** Shem at 48% CPU with 13 of 14 alpha pods spinning embedded Holochain conductors blew the 600s rollout-status window for 9 of the 14. A spread/anti-affinity policy on `kubernetes.io/hostname` would have kept this from compounding. Worth a project memory under the placement-policy bucket.
- **The seeder is the only thing that exercises the substrate-rea write path on alpha.** Edge ships the code; Genesis exercises it. If Genesis can't poll the right workload, the fix is unverifiable regardless of whether the code is correct. Worth a coupling memory: **substrate-rea verification depends on (a) Genesis seeder-poll names matching deployed StatefulSet names, (b) inventory verifier accepting canonical wire format, (c) sufficient CPU headroom for `install_app` to complete within rollout window, (d) the landing-SPA frontend actually deployed**. All four are required; none alone is sufficient.

---

*Sprint result written 2026-05-27 by Opus 4.7 (1M context) under Matthew's direction during the Task 10 verification shift.*
