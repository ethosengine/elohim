---
id: "backlog-ci-alpha-cluster-degraded-substrate"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Alpha cluster degraded (10/13 peers crashlooping) + shem down → cross-job UNSTABLE/red deploy/upload/E2E-health-gate (one infra condition, many fingerprints)"
slug: "ci-alpha-cluster-degraded-substrate"
written: "2026-06-06"
author: "ci-failure-triage"
status: "backlog"
priority: "high"
ci_status: blocked
fingerprints: [44518e179748, 597cbd37725a, 41b22c5d7ad1, 97f6d69af262, 6b6e5de4e4ef, 7bbfcf8928b9, 5af3f81c7dd4, 79748fd505af, ab55feadd29c, 8a0ee37aaa17, 39f396758ede, 9f60eb44561d, 43ba8b15ffeb, 63dd3437bede, 2e09854ec226, 1eda1f5d27e0, b4303a6d852e, 2d1b82ad175f, 1714722d9dab, 63ecdda7a81e, e9b60b28964c, 5d74b506f389, dc60d64b875f, ccdacb3bdf10, db7030259d93]
jobs: [elohim, elohim-edge, elohim-genesis]
relatedNodeIds: []
tags: [ci, infra, alpha-cluster-6peer, shem, substrate-degraded, reduced-scope, host-green-not-ci-green, museum-trap-1, requires-env, per-peer-rollout, cache-db-pool-saturation, lamad-deep-link, seed-substrate, call-zome-saturation, storm-heal-landed]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1100/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1111/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1113/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1250/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1253/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1262/
  - genesis/data/timeline/backlog/genesis-pipeline-substrate-gated-adam-arc-saturation.md
  - elohim/elohim-storage/src/db/peer_blob_inventory.rs
  - elohim/elohim-storage/src/p2p/mod.rs
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1504/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1518/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1522/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1042/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1051/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1053/
  - genesis/manifests/cluster-state.yaml
  - genesis/a2o/features/resilience/household-reciprocity.feature
  - genesis/a2o/features/lamad/deep-link-delivery.feature
  - genesis/a2o/steps/resilience.steps.ts
  - genesis/a2o/steps/lamad/deep-link-delivery.steps.ts
  - genesis/a2o/steps/common.steps.ts
  - genesis/a2o/src/framework/fixtures/substrate-scope.ts
  - Jenkinsfile
  - genesis/Jenkinsfile
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# Alpha-cluster-degraded substrate — one infra condition surfacing as UNSTABLE across three jobs

## The failure

Seven of the eleven open fingerprints are facets of **one** infrastructure
condition, not seven independent code bugs. The genesis E2E assertion cluster:

```
44518e179748  AssertionError [ERR_ASSERTION]: Expected values to be strictly equal:        (genesis 1097)
597cbd37725a  AssertionError: env var E2E_STORAGE_URL is not set (and "E2E_STORAGE_URL"…)   (genesis 1097–1099)
41b22c5d7ad1  AssertionError: No commitments listed — did the listing step run …?           (genesis 1097–1100)
97f6d69af262  AssertionError: "content-alpha" is stewarded by 0 households; expected ≥2.     (genesis 1098–1100)
6b6e5de4e4ef  AssertionError: Expected the cross-pillar resource viewer to render
              (data-testid="content-viewer"); URL is "https://alpha.elohim.host/"           (genesis 1100)
```

plus three deploy/upload/E2E facets on the sibling jobs:

```
7bbfcf8928b9  elohim — stage:Upload SPA Blob          (elohim 1500–1504, build result UNSTABLE)
5af3f81c7dd4  elohim-edge — deploy.alpha-doorway.elohim-doorway-alpha-b  (elohim-edge 1038–1042, UNSTABLE)
79748fd505af  elohim — stage:E2E Testing - Alpha Validation             (elohim 1518, red build)
ab55feadd29c  elohim — "+ kubectl rollout restart deployment/elohim-site-alpha …"  (elohim 1522, UNSTABLE)
8a0ee37aaa17  elohim — "+ kubectl rollout status deployment/elohim-site-alpha …"   (elohim 1522, UNSTABLE)
39f396758ede  elohim — "Waiting for … elohim-site-alpha rollout … 1 old replicas pending"  (elohim 1522, UNSTABLE)
```

The three **`*-site-alpha`** fingerprints (elohim #1522) are the same root
condition surfacing on the App job's **Deploy-to-Alpha** stage — but they are a
**harvester false-positive on echo/progress lines from a stage that SUCCEEDED**.
In #1522's log the site-alpha rollout actually *finished*:
`deployment "elohim-site-alpha" successfully rolled out` (line 6578); the
captured lines are the `sh`-trace command echoes (`+ kubectl rollout restart` /
`+ kubectl rollout status`) and the transient mid-rollout progress line
(`1 old replicas are pending termination` — the NORMAL rolling-update message),
not a failure. The build's actual UNSTABLE was set in the **Upload SPA Blob**
catchError (`Setting overall build result to UNSTABLE`, line 6093, exit code 1)
— i.e. facet #4 (`7bbfcf8928b9`) again, the PUT/PATCH against degraded alpha
backends. So all three are the degraded-alpha condition, attached here, with the
fingerprinting weakness noted below (Fix trail, 2026-06-10).

The newest facet — **`79748fd505af` (elohim #1518)** — is the E2E
**post-deploy health gate** failing, the cleanest single signature of the whole
condition. Everything UPSTREAM of E2E on that build was green: **Unit Test
PASSED (4597 tests)**, SonarQube / Upload SPA Blob / Build Image / Push /
Deploy-to-Alpha all completed. The ONLY failing stage is `E2E Testing - Alpha
Validation`, and within it the `runE2ETests('alpha', …)` "verify environment is
up" gate (`Jenkinsfile:1456` → helper at `Jenkinsfile:140–146`):

```
timeout 60s bash -c 'until curl -s -o /dev/null -w "%{http_code}" https://alpha.elohim.host \
    | grep -q "200|302|301"; do sleep 5; done'
→ exit 124   (alpha.elohim.host never returned 200/30x within the 60s window, post-deploy)
```

Exit 124 = the `timeout` killing the `until`-loop — i.e. the alpha edge never
came up within 60s of the deploy completing. No Cypress test even ran. This is
the **deploy-succeeds-but-app-never-serves** shape of the same degraded
substrate: the App pipeline reports green through Deploy (the rollout *applied*),
then the availability gate catches that the pods never reached Ready. Distinct
in mechanism from #1500–1504's `Upload SPA Blob` (a PUT/PATCH against degraded
backends) but the **same root condition** (degraded alpha). The sibling
**elohim-edge #1051** UNSTABLE is the matching evidence one layer down — a
doorway-alpha StatefulSet *rollout timeout* — the deploy-side mirror of the same
unavailability. And **orchestrator #1197 FAILURE is pure cascade** of #1518:
its own post stages were skipped because the App child failed; it is NOT the
`a90e18c0cf94`/`ddd8ed2cbdc7` shapes (those are unrelated concerns).

Occurrence evidence: genesis builds **1091–1100 are ALL UNSTABLE** (no green
genesis run in the window; 1090 ABORTED, 1101 FAILURE = the separate TS2739
concern, see `ci-genesis-projectionspec-ts2739.md`). elohim 1495–1504 are ALL
UNSTABLE; elohim 1518 carries the condition forward into the E2E gate; elohim-edge
1035–1042 are ALL UNSTABLE and 1051 UNSTABLE again on the doorway-alpha rollout
timeout. The whole edge/genesis surface has been UNSTABLE-not-green for the
entire recent window — the signature of an environment-down condition, not a
code regression.

## Verdict

**infra — degraded alpha substrate (operator-owned), running TOLERANT-but-LOUD
(intentional UNSTABLE).** Not a flake, not a code regression to fix in the tree.
This is the declared steady state in `genesis/manifests/cluster-state.yaml`:

- `shem: available: false` — "offline / inaccessible (operator-declared
  2026-06-01)."
- `alpha-cluster-6peer: available: degraded` — "10/13 peers CrashLooping
  (158–168 restarts, failedSince build #1024). Whether this is a CODE
  regression or env-down is UNRESOLVED — needs operator/investigation before it
  earns a regression cascade."

The genesis `Probe Substrate` stage confirms it every run:
`🛰️ SUBSTRATE PROBE — remote pool (shem): UNAVAILABLE` →
`⚠️ REDUCED SCOPE — operating on available compute only` →
`This run is intentionally UNSTABLE (loud signal), not FAILED — re-run when
shem returns to restabilize to full topology.`

This is **museum trap #1's mirror image**: there the danger is reading
NOT_BUILT/UNSTABLE as a regression; here UNSTABLE is the *correct, intentional*
signal of a degraded substrate, and the trap would be chasing it as a code bug.
It is also the canonical **host-green ≠ CI-green / availability-≠-regression**
boundary (`genesis/docs/PLACEMENT.md` § "Availability ≠ regression").

## Root cause

Each fingerprint traces to the same degraded backends:

1. **`No commitments listed` / `stewarded by 0 households`** — the
   household-reciprocity and observable-distribution scenarios assert on the
   resilience snapshot and active-commitment state. Both light up ONLY through
   the substrate placement chain (real `shard_locations.peer_id =
   humans.agent_pub_key` + peer heartbeats + `state='active'` provide
   commitments) — see memory `project_resilience_snapshot_humans_junction` and
   `project_local_stack_dht_anchor_gap`. With 10/13 alpha peers crashlooping,
   the placement substrate cannot produce ≥2 stewarding households, and the
   seeded custody commitments cannot reach `active` across the down pods. The
   resilience snapshot reads 0 — honestly.

2. **`cross-pillar resource viewer … URL is "https://alpha.elohim.host/"`** — a
   `@browser-only` scenario hitting the live (degraded) alpha deployment; the
   shell viewer doesn't render because the backend it loads from is degraded.

3. **`E2E_STORAGE_URL is not set`** — the storage env var is only exported in
   the BROWSER stage (`genesis/Jenkinsfile:2100`), not the API stage; an API
   scenario reaching a storage-direct step under reduced scope trips the guard
   (`genesis/a2o/steps/resilience.steps.ts:52`, `delivery-admin.steps.ts:251`).
   A scenario-routing/tagging artifact of the reduced-scope run, not a missing
   secret.

4. **`Upload SPA Blob` (elohim)** — uploads the SPA blob (PUT /blob + PATCH
   /db/content) to `https://alpha.elohim.host` **and** `https://elohim.host`
   (`Jenkinsfile:1036`), both fronting the degraded alpha storage backends. A
   PUT/PATCH against a crashlooping backend fails the stage → UNSTABLE.

5. **`deploy.alpha-doorway.elohim-doorway-alpha-b` (elohim-edge)** — the edge
   deploy/health-verify of the alpha doorway-B pod, on the same degraded
   cluster. (Distinct from the doorway *image quality-gate* fixture concern at
   edge #1043, which is the separate `ci-doorway-dockerfile-fixture-context`
   entry — that one is host-green-≠-CI-green build-context, this one is
   live-deploy-against-degraded-pods.) Re-surfaced at **edge #1051** as a
   doorway-alpha StatefulSet rollout timeout — the deploy-layer mirror of the
   App-job E2E gate below.

6. **`E2E Testing - Alpha Validation` (elohim #1518)** — the App pipeline's
   post-deploy availability gate. After Deploy-to-Alpha *applies* the rollout,
   `runE2ETests('alpha', 'https://alpha.elohim.host', …)` runs a 60s health
   probe (`Jenkinsfile:140–146`) before any Cypress test. With the alpha edge
   degraded, the deployed pods never reach Ready, the probe loops on a
   non-200/30x (or connection-refused) response, and `timeout` fires → **exit
   124**, failing the build red. The deploy "succeeded" (k8s accepted the
   manifest) but the app never *served* — the classic
   availability-≠-deploy-success boundary. The fix is the substrate, not the
   gate: a 60s window is reasonable for a healthy alpha; the gate is doing its
   job by going red instead of running Cypress against a dead backend (which
   would mask the real signal — cf. museum trap "host-green ≠ CI-green").

## The tagging seam (the one bounded, in-tree improvement)

`common.steps.ts:241` HOLDS (skips, not fails) any scenario tagged
`@requires:<cap>` when that cap is unavailable/degraded in `cluster-state.yaml`
(`substrate-scope.ts` reads `degraded` as conservatively unavailable). The
observable-distribution feature already tags its remote scenarios
`@requires:shem` (lines 85, 104) and is correctly held. But three failing
scenarios lack a substrate `@requires:` tag and therefore RUN against the
degraded cluster and FAIL — **exactly the seam the gate's own docstring names**
("a scenario that needs the remote canvas but doesn't happen to name a
remote-only persona would otherwise run against down pods and fail, masking the
real signal", `common.steps.ts:235`):

- `genesis/a2o/features/resilience/household-reciprocity.feature` — untagged;
  asserts `active "custody-blob"` pairs that need a stable cluster to activate.
- `observable-distribution.feature` `distributed to at least N households` /
  `stewarded by` scenarios — need real placement.
- the `cross-pillar resource viewer renders` deep-link scenarios — `@requires:
  doorway` only (a fixture precondition, not a substrate cap).

**Tagging these `@requires:alpha-cluster-6peer` is the bounded, correct,
in-tree fix** — it would convert these from masking UNSTABLE-noise into clean
HELD skips that auto-return when the cluster stabilizes (same cybernetic
reconciler the rest of the suite uses). It is NOT done in this triage run on
purpose: which scenarios are legitimately "needs the 6-peer cluster" vs "a
genuine seeding gap that should be fixed so the household (matthew/jessica/james
— available) CAN satisfy it" is a per-scenario story judgment (the household IS
a 3-node cluster, so some of these may be household-testable once seeding
activates commitments — cf. the a2o CLAUDE.md "`shem` ≠ multi-node" note). That
is `@e2e`-authoring (Opus story work) + a possible seeder activation fix, i.e.
an operator-scoped `/shift` Objective, not a sentinel fix. Sketch below.

## Current decision

**BLOCKED on operator — substrate is operator-owned; the live cluster is not a
sentinel surface (never `kubectl`).** Two unblock paths, both operator-initiated:

1. **Substrate returns** — operator brings alpha-cluster-6peer back to a stable
   ≥6-peer topology (and/or shem returns). Then the placement substrate
   produces real households/commitments and these assertions pass; the
   harvester confirms by green streak. The repo move is to flip
   `cluster-state.yaml` `alpha-cluster-6peer: available: true` (evidence-backed,
   mirroring a probe) when it stabilizes — then `scope-reconcile.py` cascades.

2. **Tag + seed-activation `/shift`** (in-tree, doesn't need the cluster) — add
   `@requires:alpha-cluster-6peer` to the genuinely-cluster-dependent scenarios
   so they HELD-skip cleanly, AND fix the seeder so the household-testable
   custody/provide commitments actually reach `active` for the 3-node household
   (memory `project_resilience_snapshot_humans_junction` item 2: POST inserts
   `proposed`, activation needs the PATCH path). This is a bounded ~5–10 file
   `/shift` Objective (a2o tags + seeder activation), Opus-authored scenarios +
   Sonnet glue. Naming it here for the operator; not opening it as sentinel work.

This entry stays `ci_status: blocked` (live-trajectory documented) until either
path lands. The fingerprints will disappear on a green streak once the substrate
stabilizes OR the tags hold the scenarios out of the degraded run.

## Fix trail

- No tree change in this triage run (correctly — substrate is operator-owned and
  the tagging/seeding fix is an operator `/shift`, not a sentinel edit).
- Ledger: all 8 fingerprints set `status: blocked` (blocker: degraded
  alpha-cluster-6peer + down shem; operator-owned). No `triaged_at_build` stamp
  (nothing landed). Recurrence is expected every run until the substrate flips —
  that's the intended LOUD signal, not a re-fire bug.
- **2026-06-09 extension** — added `79748fd505af` (elohim #1518, E2E health-gate
  exit-124 on `alpha.elohim.host` post-deploy). Same root condition, new facet:
  the App-job *post-deploy availability gate* (deploy applied, app never served).
  Sibling evidence edge #1051 (doorway-alpha rollout timeout) cited; orchestrator
  #1197 confirmed pure cascade (skipped post stages), not an independent concern.
  No `@requires:` tag exists for an App-pipeline E2E *health gate* (it's a
  Jenkinsfile shell probe, not an a2o scenario) — so the substrate-return path
  (unblock #1) is the only mover for this facet; it cannot be HELD-skipped by the
  a2o tagging fix (unblock #2). It disappears on a green streak the moment alpha
  serves 200/30x within 60s post-deploy.
- **Ceiling note for the operator** (sentinel cannot trigger builds; anonymous
  MCP): confirmation requires either a substrate flip + re-run, or the
  tag+seed `/shift` above. Until then, these UNSTABLE results are the
  cluster-state telling the truth.
- **2026-06-10 extension** — added `ab55feadd29c` / `8a0ee37aaa17` /
  `39f396758ede` (elohim #1522, the `elohim-site-alpha` kubectl rollout watch).
  #1522 is the RECOVERY build after the CPS `MethodTooLargeException` breach was
  cut (fix `ec581d5ea` "CPS breach cut 2 — the killer was the Upload-SPA-Blob
  script block"; the earlier `b3755bf9` heredoc-extraction was cut 1 and #1521
  still threw MethodTooLarge at it). With the Jenkinsfile parsing again, #1522
  ran end-to-end and finished **UNSTABLE on Upload SPA Blob** (line 6093) —
  same degraded-alpha condition as facet #4. All three new fingerprints set
  `status: blocked` (no `triaged_at_build` — nothing landed; operator-owned
  substrate). They disappear on a green streak the moment alpha serves cleanly.
- **Fingerprinting weakness (recorded here per dispatcher, no new doc):** the
  harvester captured **`sh`-trace command-echo lines** (prefixed `+ `, e.g.
  `+ kubectl rollout restart …`, `+ kubectl rollout status …`) and a
  **transient mid-rollout progress line** (`1 old replicas are pending
  termination` — the normal rolling-update wait message) as distinct "finding
  lines," even though that stage SUCCEEDED (`successfully rolled out`, line
  6578). The stage's real failure signal was elsewhere (Upload SPA Blob exit 1).
  Two cheap classifier improvements would suppress this whole false-positive
  family: (1) drop lines matching `^\+ ` (shell xtrace echoes are commands, not
  outcomes); (2) treat a `kubectl rollout status` "Waiting for …" progress line
  as a finding only if NOT followed by `successfully rolled out` within the same
  stage. Both are harvester-side (classifier) changes, not sentinel work —
  noted here so the lesson isn't lost; not opened as a separate concern.
- **2026-06-10 extension** — added `9f60eb44561d` (genesis #1111, the terminal
  `❌ GENESIS PIPELINE FAILED` banner). #1111's actual failing stage is **`Verify
  Target Health`**: `timeout 120s bash -c 'until curl -sf -o /dev/null
  https://alpha.elohim.host; do … sleep 5; done'` looped "Waiting for target
  site…" ×22 and the `timeout` killed it → **exit code 124** (`ERROR: script
  returned exit code 124`, build log line 1680). Every downstream stage (Seed
  Database … E2E Verification) then reported "skipped due to earlier failure(s)",
  ending in the banner. Same exit-124-on-`alpha.elohim.host` shape as facet #6
  (elohim #1518) — the **genesis-side pre-seed availability gate** mirror of the
  App-job's post-deploy gate. The substrate-return path (unblock #1) is the only
  mover; like the App E2E gate it's a Jenkinsfile shell probe, not an a2o
  scenario, so the tagging fix (unblock #2) cannot HELD-skip it. Set
  `status: blocked` (no `triaged_at_build` — operator-owned substrate); it
  disappears on a green streak the moment alpha serves within the 120s window.
- **Re-home, not a recurrence (fingerprint-coarseness, recorded per dispatcher).**
  `9f60eb44561d` is the **generic terminal banner**, the coarsest signature in
  the genesis ledger — it matches *any* genesis pipeline FAILURE. It was
  originally pinned to `ci-genesis-projectionspec-ts2739` at #1101 (the lone
  genesis FAILURE in that window) and stamped `triaged_at_build: 1101`. The
  harvester reopened it at #1111 (seen→2) reading the banner's reappearance as a
  recurrence of the TS2739 fix — but the TS2739 fix (`39c3c8b6b`) is genuinely IN
  #1111: **`Validate Constants` passed** ("✅ Constants validation passed"; the
  `routeClaims`/`redirectTemplates` fields now appear as *passing* unit-test
  assertions, build log lines 1265/1279–1284), and the TS2739-specific
  fingerprint `0a93d2d79477` did NOT recur (stays `triaged` at #1101). So #1111's
  red is a *different root cause wearing the same coarse banner* — re-homed here
  (the degraded-alpha concern that owns the `Verify Target Health` exit-124),
  with the stale `triaged_at_build: 1101` stamp dropped (it's an open infra
  blocker, not a fix awaiting disappearance). The TS2739 entry keeps only its
  specific fingerprint `0a93d2d79477`. Classifier lesson (harvester-side, not
  sentinel): a terminal "PIPELINE FAILED" *banner* line is a poor fingerprint —
  it carries no stage/cause identity, so it re-pins to whatever concern last held
  it and masks the true (changed) root cause. Prefer fingerprinting the
  **failing-stage cause line** (here: the `Verify Target Health` exit-124 / the
  TS2739 `error TS2739` line) over the catch-all banner. Noted with the two
  classifier improvements above; not opened as a separate concern.
- **2026-06-10 extension — the per-peer EDGE rollout facet (8 fingerprints,
  elohim-edge #1053).** The edge pipeline deploys each alpha conductor as its
  own parallel `deploy-elohim-<peer>-alpha` branch; the harvester fingerprinted
  each branch's failure line as a distinct TEST_FAILURE, yielding eight
  fingerprints for ONE condition:
  ```
  43ba8b15ffeb  elohim-edge.deploy.alpha.elohim-pete-alpha       (edge 1053)
  63dd3437bede  elohim-edge.deploy.alpha.elohim-terrance-alpha   (edge 1053)
  2e09854ec226  elohim-edge.deploy.alpha.elohim-frank-alpha      (edge 1053)
  1eda1f5d27e0  elohim-edge.deploy.alpha.elohim-gertrude-alpha   (edge 1053)
  b4303a6d852e  elohim-edge.deploy.alpha.elohim-susan-alpha      (edge 1053)
  2d1b82ad175f  elohim-edge.deploy.alpha.elohim-caleb-alpha      (edge 1053)
  1714722d9dab  elohim-edge.deploy.alpha.elohim-daniel-alpha     (edge 1053)
  63ecdda7a81e  elohim-edge.deploy.alpha.elohim-emma-alpha       (edge 1053)
  ```
  These eight peers are precisely the **household peers that are offline while
  the bootstrap pair (adam+matthew) serves** — the declared
  `alpha-cluster-6peer: degraded` topology (`project_alpha_topology_bootstrap_pair`:
  the household/non-bootstrap conductors are the ones crashlooping; #1053's build
  result is UNSTABLE, `hApp:NO | Push:Skip`). The per-peer deploy/health-verify
  of each crashlooping household conductor cannot complete → the branch fails →
  UNSTABLE. This is the **deploy-layer, per-conductor** mirror of facets #5/#6
  (the single-`doorway-B` deploy and the App/genesis availability gates): same
  degraded substrate, now fanned out one fingerprint per household peer because
  the edge pipeline parallelizes the rollout. **The doorway/bootstrap node DID
  take this build's image** — the landing heal it carried is verified working
  (dispatcher-confirmed; the bootstrap pair serves, only the household peers are
  down) — which is exactly why the build is the LOUD-but-tolerant UNSTABLE, not a
  total red: the serving path landed, the degraded household-peer rollout is the
  honest red. All eight set `status: blocked` (no `triaged_at_build` — nothing
  landed; operator-owned substrate). They disappear on a green streak the moment
  the household peers rejoin a stable ≥6-peer cluster. Classifier note
  (harvester-side, not sentinel): a parallel-rollout stage that fans one
  condition across N peers will mint N fingerprints — these collapse to one
  concern; the per-peer multiplicity is fingerprint granularity, not N bugs.
- **2026-06-10 extension — the genesis browser-E2E facet (2 fingerprints,
  elohim-genesis #1113).** Two genesis E2E assertion failures, both rooted in the
  same degraded alpha backend not serving its assets:
  ```
  e9b60b28964c  AssertionError: Failed network requests: …alpha.elohim.host/wasm/
                elohim-cache-core/…; …/version.json; doorway-alpha…/epr/…/nav-context;
                …/db/content/manifesto; …/health; …fonts/logo…  (genesis 1113)
  5d74b506f389  AssertionError: feedback dialog backdrop element not found   (genesis 1113)
  ```
  - `e9b60b28964c` (`discovery-assessment.steps.ts:306`, "no failed network
    requests should be captured") — the captured failures are **dominated by
    `alpha.elohim.host` / `doorway-alpha.elohim.host` resources** (cache-core
    wasm, `version.json`, the EPR `nav-context`, `/db/content/manifesto`,
    `/health`, fonts, logo) — the live degraded backend not serving. (A few
    third-party externalities — `youtube.com/embed`, `buymeacoffee`, `shields.io`
    badge — also appear in the same list; they are unrelated external flakiness
    riding the same coarse assertion, NOT the root cause. If the alpha backend
    served, the assertion would still need those third-party hosts to be
    reachable — a secondary brittleness worth a `@browser-only` allowlist someday,
    but not this concern's mover.)
  - `5d74b506f389` (`feedback-gate.steps.ts:171`) — a `@browser-only` UI scenario
    whose feedback dialog never renders because the alpha backend it loads from is
    degraded — the same shape as facet #2 (`cross-pillar resource viewer …
    doesn't render`). Genesis #1113 also carries the `Verify Target Health`
    exit-124 (the `9f60eb44561d` gate facet) — confirming the alpha backend is
    down for this whole build, which is *why* these browser scenarios fail.
  Both set `status: blocked` (no `triaged_at_build` — operator-owned substrate).
  Like the other browser-E2E facets, the durable bounded fix is the
  `@requires:alpha-cluster-6peer` tagging `/shift` (so they HELD-skip cleanly
  instead of running against down pods) — named in "The tagging seam" above, an
  operator-scoped story Objective, not a sentinel edit. They disappear on a green
  streak the moment alpha serves its assets.

- 2026-06-10 ~11:50Z (operator root-cause): the per-peer edge-deploy failures
  (the 10 fps folded 2026-06-10, elohim-edge#1053 pete/terrance/frank/gertrude/
  susan/caleb/daniel/emma…) are **OOM SIGKILLs on the human StatefulSets** —
  CONSTRAINT_MEMCG, anon-rss ~1488MiB against the 512Mi/1536Mi archetype limits —
  not offline peers. Operator raised the $recycledLaptopFloor archetype to
  768Mi/3Gi across deployments.json (9 humans), per-human YAMLs, and all three
  edgenode environment manifests (alpha/staging/prod), with restoration path
  documented (profile RSS on stable shem, step down with evidence). Closure:
  these fingerprints should disappear on the first edge wave after the manifest
  bump lands; if a peer still fails post-bump, that residue is a NEW concern,
  not this one.

- **2026-07-05 extension — a NEW degradation mechanism on the same
  operator-owned substrate (2 fingerprints, elohim-genesis #1250–#1253),
  triaged from dispatcher-supplied telemetry and independently verified
  against the build logs.**

  ```
  dc60d64b875f  AssertionError [ERR_ASSERTION]: Expected the lamad path overview to render
                (data-testid="path-overview"); URL is "https://alpha.elohim.host/lamad/path/
                foundations-christian-technology"                     (genesis 1250–1253)
  ccdacb3bdf10  AssertionError [ERR_ASSERTION]: Expected the lamad step navigator to render
                (data-testid="path-navigator"); URL is "https://alpha.elohim.host/lamad/path/
                foundations-christian-technology/step/2"               (genesis 1250–1253)
  ```

  Dispatcher-supplied telemetry claimed the shem-side conductor
  `elohim-adam-alpha-0` (shem bootstrap anchor + doorway-B's primary storage
  backend) had its Holochain Cache-DB read connection pool saturated
  (sustained ~1612%, spiking to 6437%), causing app-port websocket auth
  timeouts, `CellDisabled` cells, and storage bridge-reconnect loops. **This
  triage independently confirmed the shape (not the exact percentages —
  those are Loki/Grafana-side, out of Jenkins-log reach) directly in genesis
  #1253's build log**: `/auth/me` on adam times out ("pod may be
  unreachable; skipping"); `call_zome` requests against adam's identities
  time out at 60000ms; `propagation.custody-convergence` reports a
  custody-blob commitment "missing on: adam after 300s"; `adam: heal GET
  /blob/… → 000000` (unreachable) with "0 serve-blob event(s)";
  `projection.adam.projector — status/projector unavailable (000)`; and
  `federation.doorways — GET /api/v1/federation/doorways → 000`. Note the
  build's own `Probe Substrate` stage reported `shem: AVAILABLE` / "Full
  topology available" — that probe checks the **shem** resource only
  (`cluster-state.yaml`), not **alpha-cluster-6peer**, which is the resource
  actually degraded here; the two are independent probes/resources by
  design, so a shem-available run can still run straight into a degraded
  alpha-cluster-6peer backend, which is exactly what happened.

  **Verdict: infra (same root class as this whole entry) — NOT a code
  regression in the lamad renderers.** The lamad path-overview /
  step-navigator components legitimately can't render because the epr-spa
  can't sync content from `adam-alpha`'s degraded storage/conductor.

  **Root cause is the ALREADY-DOCUMENTED "tagging seam" gap above, not a new
  pattern**: `genesis/a2o/features/lamad/deep-link-delivery.feature` (steps
  are `genesis/a2o/steps/lamad/deep-link-delivery.steps.ts`) declares only
  `@requires:doorway` at the feature level (line 1) — and per
  `genesis/a2o/CLAUDE.md`, `@requires:doorway` is explicitly a **fixture
  precondition, not a substrate-scope tag** ("Caps NOT in cluster-state
  (`@requires:doorway`, `@requires:seeded-content`) are fixture
  preconditions … ignored by the scope reconciler"). So these deep-link
  scenarios run unconditionally against whatever alpha backend is live,
  exactly the gap named in "The tagging seam" section above (which already
  cites "the cross-pillar resource viewer renders deep-link scenarios —
  `@requires: doorway` only" as an example). No new root-cause class to
  declare; this is a fresh occurrence of the same known gap, with a
  different specific degradation mechanism (Cache-DB pool saturation on one
  conductor) than June's OOM CrashLoop — worth keeping for future
  correlation, not a reason to fork a new entry.

  **Current decision**: `status: blocked` for both fingerprints (no
  `triaged_at_build` — nothing landed; substrate is operator-owned and the
  bounded fix — per-scenario `@requires:alpha-cluster-6peer` tagging of
  `deep-link-delivery.feature`'s scenarios — was already named and
  deliberately deferred as an operator/story-authoring `/shift` Objective in
  "The tagging seam" section above, since `deep-link-delivery.feature` has
  many `@browser-only` scenarios that normally pass fine against alpha and
  blanket-tagging the whole feature would over-hold them). They disappear on
  a green streak the moment `adam-alpha`'s Cache-DB pool recovers, or when
  the tagging `/shift` lands.

  **Cross-reference**: the sibling fingerprint dispatched alongside these two
  (`5dba80d982e1`, "No Playwright device found") is a **separate, unrelated
  concern** — a genuine a2o step-definition bug (inconsistent hard-fail vs.
  graceful-pending guard), bounded-fixed in this same triage pass. See
  `genesis/data/timeline/backlog/a2o-playwright-device-hardfail-topology.md`.
  It does not belong in this entry's fingerprint list.

- **2026-07-06 extension — the Seed Substrate WRITE-path facet (1 fingerprint,
  elohim-genesis #1262) AND the first LANDED code mover for the adam-conductor
  mechanism — three fingerprints re-dispositioned blocked → triaged.**

  ```
  db7030259d93  red build, stage:Seed Substrate   (genesis 1262, build result UNSTABLE)
  ```

  Verified directly in #1262's build log. The `Seed Substrate` stage (log line
  2240) is the **write-path** mirror of the 2026-07-05 lamad-render read-path
  facets: the HTTP direct-ingest phase SUCCEEDED (`✅ human-adam-firstman: 4164
  content`, matthew seeded, `imported=4 failed=0` per peer) and the upstream
  `Verify Target Health` gate PASSED (the SPA host served — so this is NOT the
  exit-124 availability-gate facet #6/#9f60eb44561d), but **every
  conductor-`call_zome` sub-step timed out at 60000ms**, exactly the adam
  app-port saturation signature:

  ```
  Seed Accounts:            2 failed — Matthew, Jessica "Request timed out in 60000 ms: call_zome"
  Seed Conductor Identities: partial — 1 exists, 2 failed  → [Pipeline] unstable (line 2758)
  Seed Agent Peer Bindings:  TOTAL failure — 0 of 6 humans → [Pipeline] unstable (line 2864)
  Household formation:       exited 1                       → [Pipeline] unstable (line 2914)
  content-provide-rows / downstream: ERROR: script returned exit code 1 (lines 3058/3115/3198…)
  ```

  The storage HTTP surface (`/auth/me`, direct blob ingest) answers fine on adam
  while the conductor **app-port websocket** (`call_zome` — needed to create
  accounts, conductor identities, peer bindings, households) is starved. That is
  the exact shape the storm doc predicts: adam pegged on ~150 non-idempotent
  `Inventory snapshot applied`/sec → `/health` starved → app-port `call_zome`
  auth times out → `CellDisabled`. Each sub-step is `catchError`-wrapped and
  degrades the build to UNSTABLE (loud-but-tolerant), never FAILURE — the
  intended signal, not a code regression.

  **Verdict: infra (same root class) — NOT a code regression in the seeder or
  the pipeline.** No novel root cause; this is the write-path facet of the
  documented adam inventory-snapshot storm. Root-cause doc:
  `genesis/data/timeline/backlog/genesis-pipeline-substrate-gated-adam-arc-saturation.md`
  (adam ~150 snapshot-applies/sec, ~1000× amplification, non-idempotent
  `apply_snapshot`).

  **What changed since the 2026-07-05 (blocked, no-mover) entry: the storm-heal
  code mover has now LANDED.** `b1ef627ed` "fix(storage): receive-side snapshot
  idempotency" (HEAD on `feat/frontend-eyes-sprint`) makes `apply_snapshot`
  dedup on a SHA-256 content fingerprint persisted on
  `peer_inventory_cursor.last_content_hash` — a byte-identical re-flooded
  snapshot becomes a no-op that skips `score_and_enqueue_snapshot` (the CPU
  sink). Locally verified (`cargo test --lib peer_blob_inventory` → 21 passed);
  touches `elohim/elohim-storage/src/db/peer_blob_inventory.rs` +
  `.../src/p2p/mod.rs`. Per dispatcher, it **deployed via the edge pipeline this
  session** (dev f79ab5a40); awaiting drain-verify (Loki apply-rate collapse
  toward the ~0.1/sec design cadence + adam CPU off the peg).

  **Disposition — three adam-conductor-mechanism fingerprints set `triaged`**
  (this is the *only* mechanism in this umbrella with a landed tree mover; the
  fix collapses the CPU storm that starves BOTH the conductor `call_zome` write
  path AND the epr-spa blob-sync read path):
  - `db7030259d93` (Seed Substrate, write path) — `triaged_at_build: 1262`.
  - `dc60d64b875f` (lamad path-overview render, read path) — `triaged_at_build: 1256`.
  - `ccdacb3bdf10` (lamad step-navigator render, read path) — `triaged_at_build: 1253`.

  The sweep confirms by disappearance (genesis green streak ≥3 with no
  recurrence) once the deploy drains adam's storm. `last_build >
  triaged_at_build` on any of the three later means the fix did not take → the
  harvester reopens it (expected if the next genesis run predates full drain;
  the ≥3 streak guards against a single premature re-capture decomposing early).
  No `decompose_on_confirm` on any of them — this backlog file is shared by 24+
  fingerprints across still-blocked mechanisms, so on disappearance the harvester
  deletes only the individual ledger line and reports this shared entry for
  graduate-then-decompose review (it must NOT auto-delete the whole umbrella).

  **The rest of the umbrella stays `blocked` (unchanged) — different movers:**
  the household-peer OOM edge-deploy facets (`43ba8b15ffeb` et al.) are healed by
  the 2026-06-10 memory-archetype bump (operator-landed, awaiting its own
  disappearance), the browser-E2E facets by the `@requires:alpha-cluster-6peer`
  tagging `/shift`, and the SPA-503 availability-gate facets (`79748fd505af`,
  `9f60eb44561d`) by substrate return. Concern-level `ci_status` stays `blocked`
  because those operator-gated mechanisms dominate; the storm-heal facets are the
  in-flight exception documented here. No museum edit — this remains the
  already-cited availability-≠-regression / museum-trap-1 mirror, not a novel
  trap.
