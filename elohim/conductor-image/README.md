# elohim-conductor — how a conductor change reaches the fleet

**This directory holds no build source.** Its `build-manifest.json`
declares a pipeline whose *build* lives in another repository, so this repo's
orchestrator can route to it like any other pipeline. The manifest names the pipeline,
the Jenkins job and branch to dispatch, and the source paths whose change makes it
stale; every other pipeline in the repo is declared the same way.

This README is the map for that arrangement: what the pipeline does, how to move a
conductor onto the fleet, and why the build is somewhere else.

## What a conductor is, and why ours is custom

Every node in the fleet runs a **holochain conductor** — the long-lived process that
holds the node's DHT identity, gossips with peers, and executes zome calls. It is the
node. If it misbehaves, the node does.

We run a **fork** — `github.com/ethosengine/holochain` — rather than an upstream
release, because upstream at this version leaks native heap (glibc arenas chaining
64 MB secondary allocations; roughly 8 GB over ~5 hours, then OOM). The fork's
`jemalloc` feature swaps the global allocator, whose decay/munmap returns those
pages. Running a stock conductor is therefore not a neutral fallback — it
reintroduces the leak.

Below the conductor sits the networking stack:

- **kitsune2** — the peer networking layer (gossip, DHT sharding). It reaches the
  build as an ordinary dependency, not as a fork we clone.
- **iroh** — the only Holochain 0.7 transport; it has no conductor cargo feature.

> History: before Holochain 0.7, tx5 supplied WebRTC transport and contributed a second SHA to the pin tag.

The conductor is Rust, and building it costs ~20-30 GB and half an hour.

## The chain

```
elohim/holochain-conductor                     ← submodule pointer, in THIS repo
        │  bump it in a commit
        ▼
elohim-conductor                                ← this manifest; dispatches the
        │                                         Jenkins job `elohim-edgenode/main`
        │                                         (SCM: the che-devworkspaces repo)
        │  compiles the fork at that exact commit
        │  publishes  elohim-edgenode:conductor-<hc12>
        ▼
elohim-edge                                     ← dependsOn: elohim-conductor
        │  its Build Storage stage derives the SAME tag from the SAME pointer
        │  and passes it as --build-arg CONDUCTOR_SOURCE_IMAGE
        │  → the elohim-storage image embeds that conductor binary
        ▼
alpha fleet                                     ← each peer runs that one image in two
                                                  StatefulSets with independent cadences:
                                                  `<prefix>` serves storage in external-
                                                  conductor mode; `<prefix>-conductor`
                                                  spawns the embedded conductor
```

## Vocabulary

Everything above and below is built from these names:

| Name | What it is |
|---|---|
| **this repo** | `github.com/ethosengine/elohim` — the monorepo this directory lives in |
| **the orchestrator** | the single Jenkins pipeline that receives GitHub webhooks for this repo, reads which files a push changed, and dispatches every other pipeline in dependency order. Nothing else is webhook-triggered. `genesis/orchestrator/README.md` |
| **`elohim-conductor`** | a *pipeline id* — the string this directory's `build-manifest.json` declares. The orchestrator routes it to the Jenkins job `elohim-edgenode/main`. It is not a job name and not an image name |
| **`elohim-edge`** | a different *pipeline id*, in this repo: it builds the doorway, storage, and edge-node images and deploys them to the fleet. **Not** `elohim-edgenode` — see Names in play |
| **`elohim-storage`** | the Rust service and image artifact used by both peer workloads. The `<prefix>` StatefulSet runs it in external-conductor mode; the separate `<prefix>-conductor` StatefulSet runs the same image in embedded-conductor mode and is the workload that spawns Holochain. Promoting a conductor therefore rebuilds this shared image rather than shipping a conductor-only image |
| **the BUILD set** | the list of pipelines the orchestrator decided to dispatch for a given changeset. `genesis/orchestrator/preview.mjs` prints it per pipeline as `[BUILD]` or `[SKIP]` before you push |
| **`<hc12>`** | the first 12 characters of the `elohim/holochain-conductor` submodule SHA — giving tags shaped like `conductor-6d08142662d9`. Step 3 of the procedure prints it for your commit |
| **the conductor fork** | `elohim/holochain-conductor/` is a submodule of `github.com/ethosengine/holochain`. Inside that path, `origin` is the fork — never upstream |
| **`elohim-node`** | the storage *container name*, not an image or a pipeline. It runs in each peer's `<prefix>` StatefulSet and talks to the separate conductor workload |
| **the alpha fleet** | the registry currently lists 7 non-suspended alpha-capable peers (adam, matthew, jessica, james, gertrude, susan, eve), each represented by a storage `<prefix>` StatefulSet and a separate `<prefix>-conductor` StatefulSet. Environment assignment is dynamic: the run's `Resolved topology` log, not the registry alone, is authoritative for which prefixes belong to a target environment |

**The committed conductor submodule SHA is the pin.** Both ends — the conductor
build and the edge build — derive the identical tag from the same pointer, so nothing is
hand-edited to move a conductor onto the fleet. Specifically, none of these:

- a `sha256:` digest line in `elohim/elohim-storage/Dockerfile` (that ARG used to be
  hand-bumped after every conductor build — which is why a rebuilt conductor did not
  reach the fleet until a human remembered),
- a manual run of the `elohim-edgenode` job with hand-filled parameters,
- a manual retag in Harbor (the container image registry these images live in).

The tag names the conductor fork alone. `elohim/kitsune2` is deliberately *not* in
the tag and not watched — the fork's sibling-clone kitsune2 path patches are commented
out in its `Cargo.toml`, so bumping that submodule cannot change what gets built.

## Moving a conductor onto the fleet

**Before you start.** Authoring the conductor change itself is out of scope here —
this is the promotion path, and it assumes your commit is already pushed to the
`ethosengine/holochain` fork. You need push access to a branch of this repo that CI
watches (`dev` is the integration target), Jenkins
read access to watch hops 1-2, and Node + pnpm for step 3. You do **not** need Harbor
credentials, nor any local Rust toolchain — the conductor is never built here.

From the repo root:

```bash
# 0. ONE-TIME, and only if the fork path you are promoting (here
#    elohim/holochain-conductor/) is an empty directory.
#    `update = none` means a normal clone (and `git submodule update --init`) skips
#    these forks entirely, so step 1's `git -C` would fail with "not a git repository".
#    --checkout overrides that setting for this one path:
git submodule update --init --checkout elohim/holochain-conductor

# 1. Point the submodule at the commit you want. `origin` inside this path is the
#    ethosengine fork, not upstream holochain. Nothing fetches it for you.
git -C elohim/holochain-conductor fetch origin <branch>
git -C elohim/holochain-conductor checkout FETCH_HEAD    # or an explicit <sha>
#    A detached HEAD here is expected and fine — only the commit id is recorded.

# 2. Record the pointer move. `git add` on the submodule path stages the gitlink
#    (the pointer entry itself) — that path IS the changed file the orchestrator sees.
git add elohim/holochain-conductor
git commit -m "build(conductor): <what changed>"

# 3. Print the pin tag this commit will produce, and KEEP IT — it is the string you
#    match at every hop below.
git rev-parse HEAD:elohim/holochain-conductor | cut -c1-12
#    → e.g. 6d08142662d9, so your tag is conductor-6d08142662d9

# 4. Confirm what CI will do BEFORE pushing. Needs the workspace deps installed
#    (`pnpm install` at the repo root, once). It diffs origin/dev..HEAD plus any
#    uncommitted changes, so fetch first and run it AFTER step 2. Expect a row reading
#    `[BUILD]  elohim-conductor  | BUILD | source: elohim/holochain-conductor`
#    and elohim-edge also in the BUILD set. If elohim-conductor says SKIP, the
#    pointer did not actually move — do not push.
git fetch origin
node genesis/orchestrator/preview.mjs

git push    # any branch the orchestrator indexes works; `dev` is the integration
            # target. Preview always diffs against origin/dev, so on a branch that
            # has drifted from it, treat preview as advisory and trust step 3's tag
```

Then watch the three hops on Jenkins at `https://jenkins.ethosengine.com` (read is
anonymous, so `…/job/elohim-edgenode/` should load without a login; a 403 means your
access needs raising with whoever administers Jenkins — do that *before* pushing, not
while a 40-minute build runs unobserved). Open the named job, then the branch, then follow the upstream orchestrator build URL.
Because hop 1 uses a different repository, correlate it by its `HC_REF` parameter and
`source-derived pin tag`, not by expecting its cause to name the monorepo commit.

Match every hop against the tag you recorded in step 3.

**Mind the branch column.** Hop 1's `main` is *che-devworkspaces'* `main` — that job
lives in a different repository and always builds from its own default branch, no
matter which branch you pushed here. Hops 2 and 3 are this repo, so they follow your
branch.

For the fleet hop, derive the set from this run's literal `Resolved topology
(<strategy>):` lines and `=== Deploying N humans to <env> ===` line. Those lines give
N and the exact `<namespace>/<prefix>` values for the target environment. Expect 2 × N
workload checks. Do not infer environment membership from `deployments.json`; if
topology resolution errors before those lines, no fleet rollout has begun.

| Hop | Where to look | Signal | If the signal is wrong |
|---|---|---|---|
| Conductor built | `elohim-edgenode` → `main` console | `HC_REF` equals the full conductor SHA, `source-derived pin tag: conductor-<hc12>`, and near the end `Pushed harbor.ethosengine.com/ethosengine/elohim-edgenode:{…,conductor-…}` | A missing pin-tag line means `HC_REF` is absent **or** this is the intentionally unpinned profiling build. Inspect the parameters and upstream cause; never promote that run as the fleet pin |
| Edge consumed it | `elohim-edge` → your branch → Build Storage | `Conductor pin: …:conductor-<hc12>` followed by the full conductor SHA | A pin you don't recognize means your pointer commit is not in this build — check you pushed it and that edge built *after* the conductor, not alongside it |
| Fleet running it | `elohim-edge` → your branch → Deploy Edge Node | capture the literal emitted `STORAGE_TAG=<value>` from Component Tags/build.env and separately verify the edge build's SCM SHA is the monorepo commit you intended; tag shape varies by branch and is bare on `main`. Then require 2 × N distinctly named rollout checks: N commands matching `kubectl rollout status statefulset/<prefix> …` and N matching `kubectl rollout status statefulset/<prefix>-conductor …`, each followed by `statefulset rolling update complete … pods at revision …`. For every conductor prefix, require `conductor image <prefix>-conductor: conductor fork pin moved (… -> conductor-<hc12>)` (or `FIRST ROLLOUT — taking …` on initial installation), the matching annotate command carrying `elohim.host/conductor-pin=conductor-<hc12>`, and `conductor <prefix>-conductor CHANGED` | Fewer than 2 × N distinctly named command/completion pairs, a missing positive pin-move/first-rollout or `CHANGED` signal, `pin unavailable … HOLDING`, `holding …`, `UNCHANGED`, or `conductor phase: HALTED` means a partial rollout. It is not yours to finish — cluster reads and pod-level intervention are operator actions; hand it over with the build URL |

**That third row is the end of your path** — when all 2 × N distinctly named
workload checks complete and every one of the N conductors has positive pin-move (or
first-rollout), annotation, and `CHANGED` evidence—with none of the rejection
signals—the promotion is done. (Confirming the tag on a running pod is a separate
operator action: cluster reads are not performed from a dev environment, so the
pipeline log is your authority here, not `kubectl`.)

The conductor hop typically runs 30-45 minutes; its Jenkins timeout is 90. It runs
only when a pointer actually moved — ordinary pushes skip it entirely, and edge then
resolves the existing pin unchanged.

**If the conductor build fails**, nothing has reached the fleet: edge waits on it, and
a failed pipeline aborts everything that depends on it. Its console (hop 1) is where the failure output lives. Fix forward and push again, or
revert the pointer commit — there is no half-applied state to clean up.

**There is no fallback conductor.** If the pinned tag is absent from Harbor, the edge
build fails with a remediation message rather than baking a different conductor — a
storage image whose conductor does not match its committed source is the exact
failure this pin exists to prevent.

## Why the build stays in che-devworkspaces

`che-devworkspaces` is a separate repository — also a submodule of this one, at
`che-devworkspaces/` — holding CI and devspace container images. The conductor image
is built there by `jenkins/Jenkinsfile-elohim-edgenode` from
`containers/elohim-edgenode/`. Three reasons it has not been pulled into this repo
(verified 2026-08-11):

- **Resource shape.** Its Jenkins pod is tuned for this build alone: `node-type: edge`,
  a dedicated 60 GB buildkit scratch volume, a 90-minute timeout. The build is ~20-30 GB
  of cargo build data.
- **A hard ceiling.** The monorepo's edge `Jenkinsfile` is at the JVM 64 KB CPS
  method-size limit — it has breached it before, and those builds died before any stage
  ran. It cannot absorb another build stage.
- **No checkout needed.** The conductor fork is `update = none` (above), so the
  builder fetches it from GitHub by SHA instead — no pipeline has to
  check out a large source tree it would only read.

Unlike the conductor fork, `che-devworkspaces` is an ordinary submodule and *is*
checked out here, so the paths above resolve on disk.

## Names in play

This is a disambiguation of near-identical strings, not a second glossary. Five
similar-looking names, five different kinds of thing:

| Name | Kind |
|---|---|
| `elohim-conductor` | the **pipeline id** this directory's `build-manifest.json` declares |
| `elohim-edgenode/main` | the **Jenkins job** that id dispatches to (multibranch, SCM: che-devworkspaces) |
| `elohim-edge` | a **different pipeline**, in this repo, that consumes the result and deploys it |
| `harbor.ethosengine.com/ethosengine/elohim-edgenode` | the **Harbor image repository** that job pushes to |
| `…/elohim-edgenode:conductor-<hc12>` | one **image tag** in it — the pin |

⚠ **`elohim-edge` and `elohim-edgenode` are not the same thing.** They differ by four
characters, live in different repositories, and are different kinds of entity —
`elohim-edge` is a pipeline id in this repo, `elohim-edgenode` is a Jenkins job (and a
Harbor repository) belonging to che-devworkspaces. One consumes what the other
produces.

`ethosengine` is both a GitHub org (the forks) and a Harbor project (the images);
which one is meant follows from the surrounding URL.

### Variant builds

One alternate conductor build exists alongside the production one. It publishes to
its **own** image repository, so it can never satisfy the production pin edge asks for:

- `elohim-edgenode-prof` — a `jemalloc-prof` heap-profiling canary. Publishes no pin
  tag at all: a diagnostic build is not a deploy source.

It is selected by a **commit message tag**, the same way any build is forced here:

```bash
git commit --allow-empty -m "build(conductor): allocator profile [conductor:prof,canary]"
git push
```

`canary` adds the deployable storage image embedding that variant. Full grammar in
`genesis/orchestrator/README.md`.

## Triggers

| Trigger | Effect |
|---|---|
| A commit bumping `elohim/holochain-conductor` | Auto — orchestrator dispatches `elohim-conductor`, then `elohim-edge` |
| `[build:conductor]` in a commit message | Force-dispatch the build **regardless of whether the pointer moved** (this is how every `[build:*]` tag works). Needed for changes to the Dockerfile or the job itself, which live in the che-devworkspaces submodule and are not watched here — and to republish an identical pin after a tag loss, since the tag derives from the pointer rather than from build time |
| `[conductor:…]` in a commit message | Parameterize the build — feature set, profiling, canary, or conductor-fork branch override. **It also dispatches**, so it does not need pairing with `[build:conductor]`. Full grammar in `genesis/orchestrator/README.md`; e.g. `[conductor:prof,canary]` |

## Operator prerequisite — unverified

Harbor retention must not garbage-collect `conductor-*` tags on the
`ethosengine/elohim-edgenode` repository. The edge build resolves that tag **by name**;
if it is GC'd, edge fails until someone republishes the conductor.

The rule needed, in concrete terms: on the `ethosengine/elohim-edgenode` repository,
**retain tags matching `conductor-*` indefinitely**, exempt from the org-wide
keep-N-latest-artifacts policy. Nothing in this repo configures it — Harbor retention
is operator territory, so raise it with whoever administers Harbor. *Delete this
section once the rule is confirmed in place.*

**If it bites**, the symptom is an `elohim-edge` build failing in Build Storage on a
missing `conductor-*` tag, with a remediation line in the log. Recovery is to
republish the conductor at the same pointer — an empty commit carrying
`[build:conductor]` — which reproduces the identical tag, because the tag is derived
from the pointer rather than from build time.

The predecessor of this failure is on record: a hand-pinned conductor *digest* was
garbage-collected between two same-day builds and 404'd the entire edge build with no
diagnosis. A named tag plus a retention rule is the durable form of that fix.

## See also

- `genesis/orchestrator/README.md` — the orchestrator, the full `[conductor:…]` tag
  grammar, and how dependency levels are dispatched
- `che-devworkspaces/jenkins/Jenkinsfile-elohim-edgenode` — the build itself; its
  parameters are the contract this manifest drives
- `scripts/ci/build-storage-image.sh` — the consuming end, where the pin is derived
  a second time and passed as `--build-arg`. It must stay in lockstep with the job's
  tag derivation
- `elohim/elohim-storage/Dockerfile` — where the conductor binary is copied into the
  image the fleet runs
