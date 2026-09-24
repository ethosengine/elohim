# Elohim Orchestrator

This guide is for contributors adding, previewing, or diagnosing a CI pipeline.
The orchestrator is the central controller for Elohim CI/CD: it alone receives
GitHub webhooks and dispatches the other Jenkins pipelines.

Start from the repository root with Node, pnpm, and `just` available. A local
preview needs a Git checkout and manifests; observing a pushed build also needs
read access to the Jenkins orchestrator job. The safe first check is read-only:

```bash
node genesis/orchestrator/preview.mjs origin/dev
```

A healthy preview exits zero, prints `ORCHESTRATOR DECISION (preview)`, and
marks each non-manual manifest pipeline `BUILD` or `SKIP` for committed plus
uncommitted paths. Use **Pipeline Configuration** to add a pipeline, **Commit-message
tags** to request a deliberate dispatch, and **Troubleshooting** when the preview
and Jenkins result disagree.

## Architecture

```
GitHub Webhook → Orchestrator → Analyze Changesets → Trigger Pipelines → Report
                                       ↓
                           Health checks & notifications
```

## How It Works

1. **Select** — changed paths and build-process hashes mark manifest steps stale;
   commit tags can force-include pipelines, and baseline debt can widen the
   changeset.
2. **Order** — `dependsOn` ranks only the selected pipelines. It never selects
   an absent prerequisite.
3. **Dispatch** — each dependency level runs in parallel. A failed level stops
   later levels. A selected long-running producer is awaited through a detached
   completion barrier when a selected consumer depends on it; otherwise it stays
   fire-and-forget.
   Levels run one after another, so a coupled change pays each level's full
   time in sequence. Every dispatchable pipeline owns its wall-clock budget in
   its own Jenkinsfile `options { timeout }`, and the orchestrator's budget is
   the **sum** along the longest dependency chain plus its own stages — never a
   tuned number. `pipeline-budget.test.mjs` enforces both, so the orchestrator
   is never the clock that stops a downstream. When a downstream budget grows,
   the fix is to shrink that downstream, not to raise the sum.
4. **Run Genesis** — on eligible dev branches, a selected pipeline with
   `triggersGenesis: true` adds Genesis unless `SKIP_GENESIS` is set. Genesis runs
   after every selected non-Genesis level returns a successful dispatch result.
   For a dependency-barrier producer that reflects completion; a standalone
   long-running producer retains its optimistic dispatch result.
5. **Record and reconcile** — predicted and actual graph artifacts retain the
   plan, downstream results, timings, and stage annotations.

For example, an app-only selection runs only `elohim`; its absent edge and
Sophia prerequisites are not added. A change that selects DNA, edge, and app is
ordered DNA → edge → app → Genesis. Step dependencies can make downstream steps
stale in Jenkins; the pipeline-level `cascades` field is retained as registry
metadata and does not itself expand the selected set.

## Pipeline Configuration

Each project declares steps and pipeline metadata in `build-manifest.json`.
`elohim/rakia/schemas/v1/build-manifest.schema.json` is authoritative: the
required top-level fields are `manifestVersion`, `pipeline`, `description`, and
`steps`; dispatchable pipelines also provide `jenkinsPath` (or the explicit
cross-repository job fields). Fields such as `dependsOn`, `manualOnly`,
`triggersGenesis`, `cascades`, `longRunning`, `gate`, and `deployment` are
optional and default in the registry when omitted.

### Abbreviated manifest shape

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-holochain",
  "jenkinsPath": "Jenkinsfile",
  "manualOnly": false,
  "triggersGenesis": true,
  "cascades": true,
  "dependsOn": [],
  "steps": {
    "build": {
      "inputs": { "sources": ["project/src/**"] },
      "outputs": { "artifacts": ["project-image"], "verify": null },
      "depends": [],
      "executor": { "stage": "Build", "function": null }
    }
  }
}
```

### How to Add a New Pipeline

1. Create `<project>/build-manifest.json` from the schema and an existing peer
   manifest. Declare source inputs, step dependencies, outputs, and the pipeline
   metadata that actually applies.
2. Add the Jenkinsfile named by `jenkinsPath`; it must accept only
   `UpstreamCause` or `UserIdCause`. Keep the Groovy mirror in
   `build-graph.groovy` aligned when changing graph algorithms; ordinary manifest
   additions need no hand-maintained registry entry.
3. From the repository root, regenerate and validate:

   ```bash
   node genesis/orchestrator/scripts/generate-pipeline-list.mjs
   just _gate-rakia-validate
   just _gate-pipeline-list-fresh
   just gate orchestrator
   node genesis/orchestrator/preview.mjs origin/dev
   ```

   The schema validator must accept every manifest, the generated list must have
   no diff after regeneration, the orchestrator gate must pass, and the preview
   must show the new pipeline only for its declared inputs (or `MANUAL`).
4. Commit the manifest, Jenkinsfile, generated `pipeline-list.json`, and any
   necessary graph/test updates together. After pushing, confirm the orchestrator
   build for that commit dispatched the same pipelines the preview predicted.

The orchestrator automatically discovers all manifests at startup.

## Dependency Graph

This table covers every dispatchable pipeline in the generated
`pipeline-list.json`; manifests without a Jenkins target remain local gate
metadata.

| Pipeline | Builds | `dependsOn` | Dispatch behavior |
|---|---|---|---|
| `elohim-holochain` | DNA / hApp bundles | — | long-running; triggers Genesis |
| `elohim-conductor` | custom conductor image | — | cross-repository |
| `elohim-edge` | doorway, storage, conductor, edge image | DNA, conductor | triggers Genesis |
| `elohim` | Angular app | edge, Sophia | triggers Genesis |
| `elohim-sophia` | Sophia bundle | — | selected by Sophia inputs |
| `elohim-storybook` | component Storybook | — | independent |
| `elohim-epr` | EPR tooling | — | independent |
| `elohim-eprfs` | EPR filesystem tooling | — | independent |
| `elohim-orchestrator` | this controller's checks | — | self-dispatch is suppressed |
| `elohim-steward` | desktop/device build | — | manual-only |
| `elohim-genesis` | seed and acceptance validation | edge, app | dispatched after selected levels |

Pipelines are dispatched in dependency **levels** — topological ranks, run in
order, with everything in a level running in parallel and a failed level
aborting the next.

Dependencies order pipelines only when both are selected; they do not select a
prerequisite by themselves. An app-only change therefore remains app-only,
while a coupled DNA + edge + app change deploys the repaired backend before the
app publishes against it.

`elohim-conductor` is the one pipeline whose Jenkins job tracks a **different
repository**: the `elohim-edgenode` job on che-devworkspaces (itself a submodule
of this repo, at `che-devworkspaces/`). Its manifest therefore pins `jenkinsJob`
and `jenkinsBranch`, and its dispatch carries a dedicated parameter set rather
than the standard downstream vocabulary (`FORCE_BUILD`, `DEPLOY_ONLY`,
`CHANGED_PATHS`) — that job declares none of those. `elohim-edge` depends on it
because the storage image embeds the conductor binary. Full chain:
`elohim/conductor-image/README.md`.

## Health Endpoints

The orchestrator monitors these endpoints after deployments:

| Endpoint | URL |
|----------|-----|
| doorway-dev | https://doorway-alpha.elohim.host/health |
| doorway-prod | https://doorway.elohim.host/health |
| alpha | https://alpha.elohim.host |
| staging | https://staging.elohim.host |
| prod | https://elohim.host |

## Key Behaviors

### Skipped Pipelines
Individual pipelines check if they were triggered by the orchestrator. If triggered directly by webhook (not orchestrator), they show `NOT_BUILT` instead of running.

> **Measure semantics — `NOT_BUILT`/`ABORTED`/superseded are NOT success and NOT failure.** A `NOT_BUILT` child means "didn't run", not "passed". Do not let a downstream readiness/measure step read `NOT_BUILT`/`ABORTED`/`UNSTABLE` as green or as 0-failures — that is a *lossy* measure that has repeatedly masked real regressions. When `abortPrevious` preempts an in-flight child (a new push superseding an older one), the superseded build lands `ABORTED`; reading that as a pass is the single most common false "it's fixed" signal. Tighten any pass-test to require `lastBuild.commit == HEAD` AND a non-`NOT_BUILT`/non-`ABORTED` result before trusting it.

### Baseline state
Each pipeline carries a per-pipeline baseline (the last build the orchestrator considers known-good). **Watch-out — baseline-rollback over-build:** a `FAILURE`/`ABORTED` result can invalidate the per-pipeline baseline and roll back to the *global* baseline, which then fans out into a full cascade rebuild; `lastSuccessful()` can pin an ancient green build that no longer reflects HEAD. The baseline should advance only on a *confirmed-downstream-success*, never on a dispatch that merely started. (Backlog: convert the baseline into an explicit state machine + a `build-manifest ⊆ orchestrator changePatterns` drift test — see the recurring-anti-patterns museum record below.)

**Watch-out — a pipeline is dispatched only when its OWN baseline is stale, never the frozen global one.** Per-pipeline baselines (`pipeline-baselines.json`) can advance past a frozen `__global__` diff base independently of each other, so the global changeset alone over-dispatches; and any `longRunning` producer that a surviving pipeline still depends on is always kept, regardless of that producer's own dispatch status. The already-built filter enforces this with a pure git+manifest read that groups pipelines by their own baseline, then re-runs the same Groovy manifest walk the plan stage uses per group, skipping a pipeline only when that walk excludes it and no survivor still needs it. Full three-phase mechanism, the floating-tag argument against trusting controller build history, and the measured incidents that motivated it (#1888, #1886, #1844): `genesis/docs/superpowers/specs/2026-09-22-orchestrator-already-built-filter-design.md`.

### Genesis Triggering

Genesis is selected by its own changed inputs or force tag, or automatically on
an eligible dev branch when another selected pipeline declares
`triggersGenesis: true`. It runs outside the dependency-level loop, after all
selected non-Genesis levels report success. For dispatch control,
`SUCCESS` and `UNSTABLE` are successful; `FAILURE`, `ERROR`, `ABORTED`, and a
missing/`NOT_BUILT` result prevent progression. Genesis detects its target
environment from the branch.

### Manual-Only Pipelines
`elohim-steward` is marked `manualOnly: true` - the orchestrator never triggers it automatically.

### Commit-message tags
Read from the tip commit's message on the push being analyzed.

Force-dispatch a pipeline regardless of changeset:
`[build:edge|dna|app|genesis|sophia|steward|conductor|eprfs|epr|all]`
(comma-separated, e.g. `[build:edge,app]`).

Three semantics that have each cost a build cycle (2026-08-15, #1684-#1686):

- **`[build:*]` force-INCLUDES, never subtracts.** Changeset analysis still
  adds every pipeline with stale per-pipeline baselines or matching changed
  files. Only an empty commit with no baseline debt dispatches exactly the
  tag set.
- **`[edge:validate-only]` isolates.** It suppresses the whole dependency
  graph INCLUDING force-included `[build:*]` pipelines on the same commit —
  by design (a recording run must not perturb the substrate). Use a separate
  push for anything else.
- **Tag regexes scan the full commit body** (`git log -1 --format=%B`), so
  QUOTING a tag in prose arms it — a commit body must never mention a
  bracketed tag it does not intend (tag-quote-poisoning; #1686 re-armed
  validate-only from an explanatory sentence).

Modes:

| Tag | Effect |
|---|---|
| `[skip ci]` (or `[ci skip]`, `[no ci]`) | dispatch nothing at all |
| `[deploy-only]` | skip build stages, redeploy existing Harbor tags. Webhook-only — a timer/replay on the same HEAD must not silently redeploy |
| `[reseed]` | genesis wipes `content.db` and reseeds from scratch. Webhook-only, same reason |
| `[edge:validate-only]` | run only edge's Dataplane Validation against the live fleet — measure without rolling the pods |

**`[conductor:…]`** parameterizes the custom-conductor image build from the
commit, so a variant no longer needs a hand-filled Jenkins parameter form. It
implies `[build:conductor]`. Items are comma-separated and the tag repeats:

| Item | Effect |
|---|---|
| `iroh` | transport-iroh variant → isolated `elohim-edgenode-iroh` images |
| `prof` | jemalloc-prof heap-profiling canary → isolated `-prof` images |
| `canary` | also build the deployable storage image embedding this conductor |
| `no-push` / `dry` | build only, no Harbor push |
| `hc=<branch>` | holochain fork branch — consulted **only** if the `elohim/holochain-conductor` submodule pointer is unreadable, since an exact SHA outranks a branch tip |
| `tx5=<branch>` | tx5 fork branch — same conditional as `hc=` |
| `features=a+b+c` | raw `HC_FEATURES` (`+` separates — `,` is the item separator) |

```
git commit --allow-empty -m "build(conductor): iroh canary [conductor:iroh,canary]"
```

Two things are deliberately not expressible in a tag. The **conductor commit**
always comes from this repo's submodule pointers, which is what makes the built
image correspond to committed source. The **default feature set** stays in the
job — a second copy here would drift, and the feature set without `jemalloc` is
the one carrying the conductor heap leak, so it must not be reachable by
mistyping a tag.

Grammar lives in `commit-tag-parser.mjs` (single home; the Jenkinsfile shells out
to it rather than keeping a third copy).

### Triggers — webhook double-fire
**Watch-out — one dev push can produce two orchestrator builds.** When a Jenkinsfile declares an explicit `triggers { githubPush() }` AND the job is a Multibranch item (which fires its own implicit branch-indexing trigger), a single push fires *both*; the first build is immediately superseded (lands `ABORTED` — see measure semantics above) and looks like a phantom failure. The fix is to drop the explicit `triggers { githubPush() }` and rely on the Multibranch implicit trigger alone. A sibling variant is timer/cron collision with the webhook window (a scheduled build colliding with a late-EDT/PDT push) — reschedule the cron off that window. (Backlog: `orchestrator-trigger-dedup`.)

## Recurring CI/orchestrator anti-patterns

The frequency-ranked, curated catalog of recurring CI/orchestrator/build failure modes (measure-semantics, baseline-rollback, Dockerfile/build-manifest completeness, `HUSKY=0`-is-non-functional, sccache-poisons-rustc-output, `#[ignore]`-is-a-CI-no-op, cucumber parse-abort, CPS method-size, webhook double-fire) lives in the history museum:
**`genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`**.
Read it before debugging a "regression" or proposing a measure/baseline change — most of these have cost real shift time more than once and each carries the specific symptom + the fix.

## Before Editing Orchestrator Dispatch Logic

Read the substrate pieces before touching Execute Builds, dispatch ordering, or trigger logic:

- `graph-walker.mjs` (`walkGraph`) performs local changed-path detection for
  pre-push gates and `preview.mjs`; it reports affected pipelines but deliberately
  does not propagate staleness.
- `build-graph.groovy` (`walkBuildGraph`) is Jenkins's server-side manifest walk;
  it propagates stale manifest steps through step dependencies and returns the
  selected pipeline/step map.
- `preview.mjs` combines the local walker with registry metadata and commit tags
  to print the pre-push prediction.
- Jenkinsfile `groupByDependencyLevel` owns pipeline-level ordering of the final
  selected set; `triggerPipeline` owns dispatch and result classification.
- `timer-dispatch.mjs` owns the `groups`/`decide` phases of the already-built
  filter — git and manifest reads only, no glob matcher of its own.
- `commit-tag-parser.mjs` is the single home for commit-message tag grammar
  (`[build:…]`, `[conductor:…]`, skip/deploy-only/reseed/validate-only).
- `reconcile-build-graph.mjs` compares `predicted-build-graph.json` against
  `actual-build-graph.json` and marks the orchestrator UNSTABLE on drift.

Key invariants that naive edits break: `levelFailed` must stop later levels;
waited baselines advance only after confirmed success; standalone long-running
pipelines retain their deliberate optimistic baseline; and Genesis stays outside
the levels loop. Algorithm changes must keep the Groovy walker, local preview,
and their tests aligned. Manifest metadata changes need matching ordering tests,
not a second hard-coded registry.

Note: `graph-walker.mjs` is per-pipeline manifest-step gating (change detection + lint/test); `groupByDependencyLevel` is orchestrator-level dispatch ordering. Different layers, different concerns.

## Build-Graph Evidence

The current pipeline writes and archives `predicted-build-graph.json` before
dispatch, then `actual-build-graph.json` after dispatch. The actual artifact
records downstream results, durations, dependency levels, and hydrated stage
annotations. `reconcile-build-graph.mjs` compares them; a disconnect marks the
orchestrator UNSTABLE and supplies investigation pointers. `pipeline-registry.mjs`
loads pipeline metadata directly from manifests, so do not build a second
registry or result classifier.

## Seed Stage — Per-Peer, Not All-or-Nothing

When one peer's conductor admin WebSocket is down, the seeder continues against ready peers and reports the unready one as partial — it does not abort the whole stage. Partial-cluster is the steady state in P2P architecture; an all-or-nothing seeder pretends the substrate is monolithic and masks per-peer health.

Rules:
- Readiness probes belong at the per-peer level; gate nothing globally on one pod's health.
- Record a per-peer readiness snapshot at start; seed ready peers; surface unready peers in the report.
- `actual-build-graph.json` carries Genesis's per-peer readiness and seed details
  under its result's `stageAnnotations`; downstream advisories decide whether a
  partial seed warrants UNSTABLE or is informational.
- E2E tests targeting a specific peer should skip-with-reason if that peer was unready, not fail-cascade.

## Troubleshooting

**Q: Pipeline shows NOT_BUILT?**
- Expected! The orchestrator didn't trigger it because no relevant files changed.

**Q: Genesis not running?**
- Check that Genesis was selected and that every earlier selected level returned
  `SUCCESS` or `UNSTABLE`. `ABORTED`, `NOT_BUILT`, `ERROR`, and `FAILURE` do not
  permit progression.

**Q: Wrong environment targeted?**
- Check the branch. Orchestrator passes branch info to pipelines.
- dev/feat-*/claude/* → alpha, staging* → staging, main → prod
