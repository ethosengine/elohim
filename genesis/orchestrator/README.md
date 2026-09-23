# Elohim orchestrator

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
uncommitted paths. Use **Pipeline configuration** to add a pipeline, **Commit-message
tags** to request a deliberate dispatch, and **Troubleshooting** when the preview
and Jenkins result disagree.

File names given without a directory are in `genesis/orchestrator/`, and "the
orchestrator Jenkinsfile" is `genesis/orchestrator/Jenkinsfile`.

## Terms

- **Conductor**: the local runtime a participant runs on their own device.
- **DNA**: an application's rule-set, whose content hash defines a distinct network.
- **Edge**: the node services `elohim-edge` deploys: a doorway (the web projection
  of protocol content), `elohim-storage`, and a conductor.
- **Alpha**: the development environment, `alpha.elohim.host`; the fleet is the
  alpha peer pods that `elohim-edge` rolls.
- **Genesis**: `elohim-genesis`, which seeds content and runs acceptance checks.

## How it works

1. **Select.** Changed paths and build-process hashes mark manifest steps stale;
   commit tags can force-include pipelines, and baseline debt can widen the
   changeset.
2. **Order.** `dependsOn` ranks only the selected pipelines. It never selects
   an absent prerequisite.
3. **Dispatch.** Each dependency level runs in parallel. A failed level stops
   later levels. A selected long-running producer is awaited through a detached
   completion barrier when a selected consumer depends on it; otherwise it stays
   fire-and-forget.
4. **Run Genesis.** On eligible dev branches, a selected pipeline with
   `triggersGenesis: true` adds Genesis unless `SKIP_GENESIS` is set. Genesis runs
   after every selected non-Genesis level returns a successful dispatch result.
   For a dependency-barrier producer that reflects completion; a standalone
   long-running producer retains its optimistic dispatch result.
5. **Record and reconcile.** Predicted and actual graph artifacts retain the
   plan, downstream results, timings, and stage annotations.

For example, an app-only selection runs only `elohim`; its absent edge and
Sophia prerequisites are not added. A change that selects DNA, edge, and app is
ordered DNA → edge → app → Genesis. Step dependencies can make downstream steps
stale in Jenkins; the pipeline-level `cascades` field is retained as registry
metadata and does not itself expand the selected set.

## Pipeline configuration

Each project declares steps and pipeline metadata in `build-manifest.json`.
`elohim/rakia/schemas/v1/build-manifest.schema.json` is authoritative: the
required top-level fields are `manifestVersion`, `pipeline`, `description`, and
`steps`, and every step requires `description`, `inputs`, `outputs`, `depends`,
and `executor`. Dispatchable pipelines also provide `jenkinsPath` (or the
explicit cross-repository job fields). Fields such as `dependsOn`, `manualOnly`,
`triggersGenesis`, `cascades`, `longRunning`, `gate`, and `deployment` are
optional and default in the registry when omitted. `manualOnly: true` keeps a
pipeline out of path-based selection: the orchestrator dispatches it only when a
`[build:*]` tag names it, and otherwise someone starts it in Jenkins.

### Abbreviated manifest shape

This is the manifest for the Elohim Protocol Record (EPR) codec,
`elohim/epr/build-manifest.json`, with its second step, its `gate` block, and its
empty `deployment` block left out:

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-epr",
  "jenkinsPath": "elohim/epr/Jenkinsfile",
  "manualOnly": false,
  "triggersGenesis": false,
  "cascades": false,
  "dependsOn": [],
  "description": "Elohim EPR codec — canonical CBOR + CIDv1 + Ed25519 for the graph substrate (Rust crate + TS SDK)",
  "steps": {
    "rust-build-test": {
      "description": "fmt + clippy + nextest for the elohim-epr and elohim-epr-rea Rust crates",
      "inputs": {
        "sources": ["elohim/epr/**", "elohim/epr-rea/**", "elohim/Cargo.toml", "elohim/Cargo.lock"],
        "buildProcess": ["elohim/epr/Jenkinsfile"]
      },
      "outputs": { "artifacts": ["elohim-epr-rlib"], "verify": null },
      "depends": [],
      "executor": { "stage": "Rust — tests", "function": null }
    }
  }
}
```

A step names the files that make it stale (`inputs.sources` globs, and
`inputs.buildProcess` files, where `File@function` hashes one Jenkinsfile
function), what it produces (`outputs`), the steps it needs first (`depends`,
written `pipeline:step` across manifests), and the Jenkins stage and helper
that run it (`executor`, with `null` for an inline stage).

### How to add a new pipeline

1. Have the Jenkins job created. Creating the Jenkins job is a controller task;
   ask the maintainer. It is a Multibranch Pipeline named exactly as the
   manifest's `pipeline` value, with this GitHub repository as its branch source
   and `jenkinsPath` as its script path. The orchestrator dispatches to
   `<pipeline>/<branch>`, so a job under any other name is never found (see
   [A new pipeline is skipped as not provisioned](#a-new-pipeline-is-skipped-as-not-provisioned)).
2. Create `<project>/build-manifest.json` from the schema and an existing peer
   manifest. Declare source inputs, step dependencies, outputs, and the pipeline
   metadata that actually applies.
3. Add the Jenkinsfile named by `jenkinsPath`; it must accept only
   `UpstreamCause` or `UserIdCause`. Keep the Groovy mirror in
   `build-graph.groovy` aligned when changing graph algorithms; ordinary manifest
   additions need no hand-maintained registry entry.
4. From the repository root, regenerate and validate:

   ```bash
   node genesis/orchestrator/scripts/generate-pipeline-list.mjs
   just _gate-rakia-validate
   just _gate-pipeline-list-fresh
   just gate orchestrator
   node genesis/orchestrator/preview.mjs origin/dev
   ```

   The schema validator must accept every manifest, the generated list must have
   no diff after regeneration, the orchestrator gate must pass, and the preview
   must show the new pipeline only for its declared inputs (a manual-only
   pipeline does not appear in the preview at all).
5. Commit the manifest, Jenkinsfile, generated `pipeline-list.json`, and any
   necessary graph/test updates together. After pushing, confirm the orchestrator
   build for that commit dispatched the same pipelines the preview predicted.

The orchestrator automatically discovers all manifests at startup. In Jenkins,
`build-graph.groovy` finds each `build-manifest.json` and builds the pipeline
registry from them; locally, `pipeline-registry.mjs` loads the same metadata for
`preview.mjs` and the gates. `scripts/generate-pipeline-list.mjs` writes
`pipeline-list.json` from that registry, and Jenkins does not read it. It is
committed for shell tooling that cannot import the registry module:
`scripts/count-pipeline-failures.sh` reads it to decide which Jenkins jobs to
count. That is why steps 4 and 5 regenerate and commit it, and why
`just _gate-pipeline-list-fresh` fails when the committed copy differs.

## Dependency graph

This table covers every pipeline in the generated `pipeline-list.json`, which
lists each manifest that names a `jenkinsPath`; manifests without a Jenkins
target remain local gate metadata.

| Pipeline | Builds | `dependsOn` | Dispatch behavior |
|---|---|---|---|
| `elohim-holochain` | DNA / hApp bundles | — | long-running; triggers Genesis |
| `elohim-conductor` | custom conductor image | — | cross-repository |
| `elohim-edge` | doorway, storage, conductor, edge image | DNA, conductor | triggers Genesis |
| `elohim` | Angular app | edge, Sophia | triggers Genesis |
| `elohim-sophia` | Sophia bundle | — | registered by the orchestrator Jenkinsfile (see below) |
| `elohim-storybook` | component Storybook | — | long-running |
| `elohim-epr` | EPR codec (Rust crate and TypeScript SDK) | — | independent |
| `elohim-eprfs` | EPR filesystem library crates | — | independent |
| `elohim-orchestrator` | this controller's checks | — | the orchestrator's own job, never dispatched; its checks run in the local gate (`just gate orchestrator`) |
| `elohim-steward` | desktop/device build | DNA | manual-only; long-running |
| `elohim-genesis` | seed and acceptance validation | edge, app | runs after the selected levels (see [Genesis](#genesis)) |

The Sophia manifest lives inside the `sophia` submodule, which the orchestrator's
checkout does not initialize, so the orchestrator Jenkinsfile registers
`elohim-sophia` itself. In Jenkins it builds only for `[build:sophia]` or a
manual start, never from Sophia's paths; the preview, with the submodule
initialized, can select it by path. Regenerating `pipeline-list.json` needs
`git submodule update --init sophia` first, or the list drops `elohim-sophia`.

`elohim-conductor` is the one pipeline whose Jenkins job tracks a different
repository: the `elohim-edgenode` job on che-devworkspaces (itself a submodule
of this repo, at `che-devworkspaces/`). Its manifest therefore pins `jenkinsJob`
and `jenkinsBranch`, and its dispatch carries a dedicated parameter set rather
than the standard downstream vocabulary (`FORCE_BUILD`, `DEPLOY_ONLY`,
`CHANGED_PATHS`), because that job declares none of those. `elohim-edge` depends
on it because the storage image embeds the conductor binary. Full chain:
`elohim/conductor-image/README.md`.

## Commit-message tags

Read from the full message of the tip commit on the push being analyzed. Which
tags a run honors depends on how the run started; see
[Which runs honor which tags](#which-runs-honor-which-tags).

Force-dispatch a pipeline regardless of changeset:
`[build:edge|dna|app|genesis|sophia|steward|conductor|eprfs|epr|all]`
(comma-separated, e.g. `[build:edge,app]`). `dna` names `elohim-holochain`,
`app` names `elohim`, and the other names except `all` add the `elohim-` prefix.
`[build:steward]` ends `NOT_BUILT`, because that job runs only for a person.
The orchestrator Jenkinsfile expands `all` in its Checkout stage, before the
pipeline registry is loaded, so in Jenkins `[build:all]` adds nothing; name the
pipelines instead. No name selects `elohim-storybook`, and an unknown name is
ignored with a warning in the orchestrator log.

Three rules about commit tags are easy to get wrong:

- **`[build:*]` force-includes, never subtracts.** Changeset analysis still
  diffs the commit against the global baseline and adds every pipeline with
  matching changed files; per-pipeline baselines add nothing. Only an empty
  commit with no baseline debt dispatches exactly the tag set.
- **`[edge:validate-only]` isolates.** It suppresses the whole dependency
  graph, including force-included `[build:*]` pipelines on the same commit,
  by design: a recording run must not perturb the substrate. Use a separate
  push for anything else.
- **Tag regexes scan the full commit body** (`git log -1 --format=%B`), so
  quoting a tag in prose arms it. A commit body must never mention a bracketed
  tag it does not intend.

Modes:

| Tag | Effect |
|---|---|
| `[skip ci]` (or `[ci skip]`, `[no ci]`) | dispatch nothing at all |
| `[deploy-only]` | dispatch only `elohim-edge` (plus Genesis unless `SKIP_GENESIS` is set), which skips its build stages and redeploys image tags already in Harbor, the container registry. Webhook runs only |
| `[reseed]` | Genesis wipes `content.db` and reseeds from scratch. Webhook runs only |
| `[edge:validate-only]` | dispatch only `elohim-edge`, which skips build and deploy and runs its Dataplane Validation (the `@dataplane` acceptance scenarios) against alpha's live fleet, so measuring does not restart the pods |

When tags combine, `[skip ci]` wins over everything, `[edge:validate-only]`
wins over `[deploy-only]` and `[build:*]`, and `[deploy-only]` replaces the
selected set with edge and Genesis. Before pushing `[edge:validate-only]`, run
the preflight checks under "Measuring without deploying" in the root
[CLAUDE.md](../../CLAUDE.md).

`[conductor:…]` parameterizes the custom-conductor image build, so a variant can
be built from a commit message. The tag also dispatches `elohim-conductor`, so
it needs no `[build:conductor]` beside it. Items are comma-separated and the tag
can repeat:

| Item | Effect |
|---|---|
| `prof` | heap-profiling build with the `jemalloc-prof` allocator, published to the separate `elohim-edgenode-prof` image repository and never as the conductor tag edge deploys |
| `canary` | also build the deployable storage image embedding this conductor |
| `no-push` / `dry` | build only, no Harbor push |
| `hc=<branch>` | holochain fork branch, consulted only if the `elohim/holochain-conductor` submodule pointer is unreadable, since an exact SHA outranks a branch tip |
| `features=a+b+c` | raw `HC_FEATURES`, replacing the job's whole default feature set (`+` separates features because `,` separates items); overrides `prof` |
| `iroh` | accepted for old commit messages; selects the default feature set, so it has no effect |
| `tx5=<branch>` | accepted for old commit messages; parsed but not forwarded, so it has no effect |

```
git commit --allow-empty -m "build(conductor): allocator profile [conductor:prof,canary]"
```

Two things are deliberately not expressible in a tag. The **conductor commit**
always comes from this repo's submodule pointers, which is what makes the built
image correspond to committed source. The **default feature set** stays in the
job: a second copy here would drift, and the feature set without `jemalloc` is
the one carrying the conductor heap leak, so it must not be reachable by
mistyping a tag.

List `jemalloc` yourself in a `features=` item. Nothing checks the list, and a
list without `jemalloc-prof` is published as a production conductor under the
tags edge deploys.

The `[conductor:…]` grammar has one home, `commit-tag-parser.mjs`, which the
orchestrator Jenkinsfile calls. The `[build:*]` names Jenkins acts on live in
the `buildTagAliases` map in its Checkout stage, so a new pipeline has no name
until someone adds one there. The parser's own copy, `BUILD_TAG_ALIASES`, lacks
`eprfs` and `epr` and changes nothing in Jenkins.

### Which runs honor which tags

The opening banner of every orchestrator run prints the run kind on its
`Trigger:` line, followed by the Jenkins build cause:

- A webhook run (`WEBHOOK`) was started by a GitHub push; its cause reads
  `Push event to branch <name>`.
- A timer run (`TIMER`) was started by the daily cron (see
  [Triggers](#triggers)); its cause reads `Started by timer`.
- A manual run (`MANUAL`) is any other start: a person pressing Build, a
  replay, or a branch-indexing scan.

| Tag | Webhook run | Timer run | Manual run |
|---|---|---|---|
| `[skip ci]` | honored | honored | honored |
| `[build:*]` | honored | honored | honored in `auto` mode |
| `[conductor:…]` | honored | honored | honored in `auto` mode |
| `[edge:validate-only]` | honored | honored | honored |
| `[deploy-only]` | honored | ignored | ignored |
| `[reseed]` | honored | ignored | ignored |

A person who starts the job with a `MODE` other than `auto` (see
[Starting a run by hand](#starting-a-run-by-hand)) gets that mode's fixed
pipeline list, and `[build:*]` and `[conductor:…]` do not add to it.

`[deploy-only]` and `[reseed]` are limited to webhook runs so that a timer run
or a replay of the same commit cannot redeploy or wipe content again unasked.
The force tags have no such limit (see [Triggers](#triggers)).

## Run behavior

### Skipped builds and result semantics

Each downstream Jenkinsfile in this repository sets
`overrideIndexTriggers(false)` and checks what started it; the cross-repository
conductor job does neither. Most run for the orchestrator (`UpstreamCause`) or a
person (`UserIdCause`). The app (`elohim`), `elohim-epr`, and `elohim-eprfs`
also accept `BranchIndexingCause`, and `elohim-steward` runs only for a person.
A build started any other way skips itself and ends `NOT_BUILT`, as does an
orchestrator run that a newer run supersedes.

`pipeline-results.mjs` holds the shared classification: `SUCCESS` and
`UNSTABLE` count as success, `FAILURE` is a failure, `ABORTED` is waste, and
`NOT_BUILT` is skipped. A `NOT_BUILT` build did not run and an `ABORTED` build
never reached a verdict, so neither is a pass. A downstream build ends
`ABORTED` when a person stops it or when the orchestrator run waiting on it is
aborted, which is what superseding that run does; a long-running pipeline keeps
running even then. A downstream readiness or measure step must not read
`NOT_BUILT`, `ABORTED`, or `UNSTABLE` as green or as zero failures. Before
trusting a pass, require `lastBuild.commit == HEAD` and a result other than
`NOT_BUILT` or `ABORTED`.

### Baseline state

A baseline is the last commit the orchestrator treats as built. Each run
archives `pipeline-baselines.json` as a build artifact. It holds a global
baseline (the `__global__` key) and one baseline per pipeline, and the next run
on the same branch loads it from the last completed orchestrator build.
Selection reads only the global baseline: each run diffs the commit being built
against it. When the orchestrator run ends `SUCCESS` or `UNSTABLE`, the global
baseline moves to the commit it built, so the next push does not re-select what
that run covered, including a pipeline it skipped. When the run ends `FAILURE`,
`ABORTED`, or `NOT_BUILT`, the global baseline goes back to the one the run
loaded, so the next run diffs the same range again, plus any new commits, and
re-selects what failed. A per-pipeline baseline moves to the commit only when
the orchestrator waited for that pipeline and it returned `SUCCESS` or
`UNSTABLE`. It holds on every other result: `FAILURE`, `ABORTED`, `NOT_BUILT`,
`ERROR` (a dispatch call that threw), or a job that is not provisioned. A
long-running pipeline that no selected pipeline depends on is dispatched
without waiting, so its baseline advances at dispatch. If it later fails, the
failure shows only in that pipeline's own Jenkins view, and the next push does
not re-dispatch it unless its files change again or a `[build:*]` tag names it.
Per-pipeline baselines never add a pipeline to the selection; only timer runs
read them, to drop pipelines already built at the commit (see
[Triggers](#triggers)).

For example, the orchestrator waits for a DNA build, which succeeds, then
dispatches edge, which fails. The DNA baseline moves to this commit and edge's
stays where it was. The run ends `FAILURE`, so the global baseline goes back,
and the next push re-dispatches both DNA and edge.

### Genesis

Its own changed inputs or `[build:genesis]` select Genesis, as does step 4 of
[How it works](#how-it-works) on the eligible dev branches: `dev`, `dev-*`,
`feat-*`, and `claude/*`. It runs outside the dependency-level loop, after all
selected non-Genesis levels report success, so its `dependsOn` does not order
it. `SUCCESS` and `UNSTABLE` let it proceed; `FAILURE`, `ERROR`, `ABORTED`, and
a missing or `NOT_BUILT` result stop it. A failed Genesis run marks the
orchestrator `UNSTABLE`. Genesis detects its target environment from the
branch (see [The wrong environment was targeted](#the-wrong-environment-was-targeted)).

A long-running pipeline that no selected pipeline depends on, such as a DNA
build forced alone with `[build:dna]`, is not awaited, so Genesis tests
whatever is already deployed and a green Genesis says nothing about that build.

#### Per-peer seeding

Genesis probes each peer's conductor before seeding, seeds the peers that are
ready, and reports the rest as unready instead of aborting the stage.
Partial-cluster is the steady state in P2P architecture, so readiness is judged
per peer and no single pod's health gates the whole seed. When some conductors
are unready, `runProbedSeeder` in `genesis/Jenkinsfile` marks the run `UNSTABLE`
and seeds the ready ones; when none is ready, it marks the run `UNSTABLE` ("All N
conductors unreachable — skipping seed") and skips the seed. The Verify Seeding
stage marks the run `UNSTABLE` for peers whose storage is unreachable and fails
it when a reachable peer is empty or no reachable peer has content.
`actual-build-graph.json` carries Genesis's per-peer readiness and seed details
under its result's `stageAnnotations`.

### Triggers

A push reaches the orchestrator through its Multibranch job, which starts the
branch job with the cause `Push event to branch <name>`: a webhook run. That job
indexes `dev` and pull-request branches; other branch names never reach it. The
only trigger the orchestrator Jenkinsfile declares is a daily cron at 09:00 UTC,
which exists for the iroh parity soak (a nightly stage that runs
`elohim-storage`'s `iroh_*` tests with the iroh features), so one push starts one
run. A timer run plans like any other run and honors the tip commit's tags as
[Which runs honor which tags](#which-runs-honor-which-tags) shows. It then drops
every pipeline whose own baseline already equals the commit being built, unless
a `[build:*]` or `[conductor:…]` tag forces it. The anti-patterns museum linked
under [Troubleshooting](#troubleshooting) records the retired webhook
double-fire trap.

### Starting a run by hand

Someone with a Jenkins login can start a branch job of `elohim-orchestrator`
from its Build with Parameters page. Webhook and timer runs use the defaults.

| Parameter | Default | Effect |
|---|---|---|
| `MODE` | `auto` | `auto` selects from the changeset. `status` dispatches nothing and reports deployment status. `rebuild-all` dispatches DNA, edge, and app; `rebuild-edge` dispatches edge; `rebuild-app` dispatches the app; each adds Genesis unless `SKIP_GENESIS` is set. `genesis-only` dispatches Genesis alone |
| `SKIP_GENESIS` | off | in `auto` mode, skip the automatic Genesis include for a selected `triggersGenesis: true` pipeline; a Genesis selected by its own inputs or by `[build:genesis]` still runs. The manual modes and `[deploy-only]` check it for their fixed lists |
| `DEPLOY_ONLY` | off | the same as `[deploy-only]` |
| `RESET_STORAGE` | off | the same as `[reseed]` |
| `VERIFY_DEPLOYMENT` | on | run the Verify Deployment stage |
| `FORCE_COMMIT` | empty | diff from this commit instead of the global baseline |

### Health checks

The orchestrator keeps no list of health endpoints: each manifest's
`deployment.targets.<env>.healthCheck` names one per environment. The
Pre-flight Health Check stage probes the `alpha` endpoints and warns; the
Post-flight Health Check stage probes them after dispatch and marks the run
`UNSTABLE` if any is unhealthy. A dispatch to a pipeline that declares a health
check carries `FORCE_DEPLOY=true` (false on an `[edge:validate-only]` run). The
Verify Deployment stage compares deployed versions with the commit using the
hand-kept `VERSION_ENDPOINTS` map in the orchestrator Jenkinsfile, and a
mismatch only prints a warning.

## Before editing orchestrator dispatch logic

Read these modules before changing the Execute Builds stage, dispatch ordering,
or trigger logic:

- `graph-walker.mjs` (`walkGraph`) performs local changed-path detection for
  pre-push gates and `preview.mjs`; it reports affected pipelines but deliberately
  does not propagate staleness. `graph-walker.test.mjs` covers it.
- `build-graph.groovy` (`walkBuildGraph`) is Jenkins's server-side manifest walk;
  it propagates stale manifest steps through step dependencies and returns the
  selected pipeline/step map. No test exercises it directly.
- `preview.mjs` combines the local walker with registry metadata to print the
  pre-push prediction. It does not read a commit message, so commit tags take
  effect only in Jenkins.
- Jenkinsfile `groupByDependencyLevel` owns pipeline-level ordering of the final
  selected set; `triggerPipeline` owns dispatch and result classification.

Key invariants that naive edits break: `levelFailed` must stop later levels;
waited baselines advance only after confirmed success; standalone long-running
pipelines retain their deliberate optimistic baseline; and Genesis stays outside
the levels loop. Algorithm changes must keep the Groovy walker, local preview,
and their tests aligned. Manifest metadata changes need matching ordering tests,
not a second hard-coded registry.

## Build-graph evidence

The pipeline writes and archives `predicted-build-graph.json` before dispatch,
then `actual-build-graph.json` after dispatch. The actual artifact records
downstream results, durations, dependency levels, and hydrated stage
annotations. `reconcile-build-graph.mjs` compares them; a disconnect marks the
orchestrator UNSTABLE and supplies investigation pointers. `pipeline-registry.mjs`
loads pipeline metadata directly from manifests, so do not build a second
registry or result classifier.

## Troubleshooting

Before calling a red build a regression, or before changing a measure or the
baseline, read the
[recurring CI and orchestrator anti-patterns museum](../docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md).
Each entry gives the symptom and the fix. [`CI_RELIABILITY.md`](CI_RELIABILITY.md)
in this directory is a dated incident note on build-state persistence and the
Stage View layout.

### The preview and Jenkins disagree

The two start from different inputs. The preview does not read commit
messages, so tags change only the Jenkins run. It diffs against the base ref
you pass plus uncommitted files; Jenkins diffs against its global baseline,
which spans several pushes after a failed run. Jenkins also marks a step stale
when a build-process file's hash changed and propagates staleness through step
dependencies, while the local walker matches build-process files only when they
appear in the diff and does not propagate. Jenkins adds Genesis on eligible
branches, never dispatches the orchestrator itself, cannot see the Sophia
manifest, and on a timer run drops pipelines already built at the commit. To
compare Jenkins's own plan with what it dispatched, read the
`build-graph-reconciliation.json` that `reconcile-build-graph.mjs` archives on
the orchestrator build.

### A pipeline shows `NOT_BUILT`

Nothing ran, so it has not passed. Something other than the orchestrator or a
person started the build and it skipped itself, or, for the orchestrator's own
job, a newer run superseded it. Check the orchestrator build for the same
commit to see whether the pipeline was selected and what it returned.

### The orchestrator run is `UNSTABLE`

The log names the cause: a dispatched pipeline returned `UNSTABLE`, a job is
not provisioned, Genesis failed, reconciliation found a disconnect, the
Post-flight Health Check found an endpoint unhealthy, or a later stage such as
Verify Deployment or the iroh parity soak hit an error. An `UNSTABLE` run still
moves the global baseline, so the next push does not re-select the same work.
To run a pipeline again at the same commit, push an empty commit with its
`[build:*]` tag.

### Genesis is not running

Check that Genesis was selected and that every earlier selected level returned
`SUCCESS` or `UNSTABLE`. `[edge:validate-only]` suppresses Genesis, and
`SKIP_GENESIS` suppresses the automatic include. On a timer run, the
already-built filter can leave no pipeline that adds Genesis. See
[Genesis](#genesis).

### A new pipeline is skipped as not provisioned

When a manifest declares a pipeline whose Jenkins job does not exist yet, the
orchestrator logs it as not provisioned, marks itself `UNSTABLE`, and continues.
That run moved the global baseline past the commit, so creating the job (step 1
of [How to add a new pipeline](#how-to-add-a-new-pipeline)) does not build the
pipeline: push an empty commit with its `[build:*]` tag, once `buildTagAliases`
has a name for it, or have someone with a Jenkins login start its branch job.

### The wrong environment was targeted

Check the branch. The orchestrator passes branch info to pipelines: it runs
each downstream job on the pushed branch (the cross-repository conductor job
runs on its pinned branch), and each pipeline chooses its environment from the
branch name.

- dev/feat-*/claude/* → alpha, staging* → staging, main → prod (paused)

Genesis, for example, sends `main` to `elohim.host`, `staging` and `staging-*`
to `staging.elohim.host`, and every other branch to `alpha.elohim.host`. Only
`dev` and pull-request branches reach the orchestrator (see
[Triggers](#triggers)); a pull-request run passes the pull request's source
branch downstream. The `elohim-edge` alpha deploy stage runs on every
orchestrator dispatch except an `[edge:validate-only]` run, because the
orchestrator sends `FORCE_BUILD`, so a push that selects edge rolls alpha's
peer pods. The comment above `VERSION_ENDPOINTS` in the orchestrator
Jenkinsfile records production as paused.
