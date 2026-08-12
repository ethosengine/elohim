# elohim-conductor — how a conductor change reaches the fleet

**This directory holds no source.** It holds one file — `build-manifest.json` — which
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

Below the conductor sits a networking stack worth naming once. Three layers, one of
which matters here mainly by its absence:

- **kitsune2** — the peer networking layer (gossip, DHT sharding). It reaches the
  build as an ordinary dependency, not as a fork we clone.
- **tx5** — the peer-to-peer *transport* underneath it (WebRTC, via a Go/pion
  backend). We fork this one too — `github.com/ethosengine/tx5` — and route it in
  through the conductor's `[patch.crates-io]`, which is why it is half of the pin
  tag: change tx5 and you change the binary.
- **iroh** — an alternate transport under evaluation, compiled *instead of* tx5 in
  an isolated variant build (see Variant builds).

The conductor is Rust with a CGo (Go-called-from-Rust) transport backend, which is
why building it costs ~20-30 GB and half an hour.

## The chain

```
elohim/holochain-conductor  +  elohim/tx5      ← submodule pointers, in THIS repo
        │  bump either one in a commit
        ▼
elohim-conductor                                ← this manifest; dispatches the
        │                                         Jenkins job `elohim-edgenode/main`
        │                                         (SCM: the che-devworkspaces repo)
        │  compiles both forks at those exact commits
        │  publishes  elohim-edgenode:conductor-<hc12>-<tx512>
        ▼
elohim-edge                                     ← dependsOn: elohim-conductor
        │  its Build Storage stage derives the SAME tag from the SAME pointers
        │  and passes it as --build-arg CONDUCTOR_SOURCE_IMAGE
        │  → the elohim-storage image embeds that conductor binary
        ▼
alpha fleet                                     ← each peer's `elohim-node` container
                                                  runs the elohim-storage image, which
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
| **`elohim-storage`** | the Rust service that holds each peer's content and serves its HTTP API. Its container image is what a peer actually runs, and it *embeds* the conductor binary and spawns it as a child process — which is why promoting a conductor means rebuilding this image rather than shipping a conductor container of its own |
| **the BUILD set** | the list of pipelines the orchestrator decided to dispatch for a given changeset. `genesis/orchestrator/preview.mjs` prints it per pipeline as `[BUILD]` or `[SKIP]` before you push |
| **`<hc12>` / `<tx512>`** | the first 12 characters of the `elohim/holochain-conductor` and `elohim/tx5` submodule SHAs, in that order — giving tags shaped like `conductor-6d08142662d9-923e17c36c04`. Step 3 of the procedure prints the pair for your commit |
| **the two forks** | `elohim/holochain-conductor/` is a submodule of `github.com/ethosengine/holochain`; `elohim/tx5/` is a submodule of `github.com/ethosengine/tx5`. Inside either path, `origin` is that fork — never upstream |
| **`elohim-node`** | a *container name*, not an image or a pipeline. It is the container on each peer that runs the `elohim-storage` image |
| **the alpha fleet** | the alpha peers — 7 at the time of writing (adam, matthew, jessica, james, gertrude, susan, eve), each running an `elohim-node` container. `genesis/orchestrator/data/deployments.json` is the authoritative roster |

**The committed submodule SHAs are the pin.** Both ends — the conductor build and
the edge build — derive the identical tag from the same two pointers, so nothing is
hand-edited to move a conductor onto the fleet. Specifically, none of these:

- a `sha256:` digest line in `elohim/elohim-storage/Dockerfile` (that ARG used to be
  hand-bumped after every conductor build — which is why a rebuilt conductor did not
  reach the fleet until a human remembered),
- a manual run of the `elohim-edgenode` job with hand-filled parameters,
- a manual retag in Harbor (the container image registry these images live in).

The tag names **both** forks because both determine the binary. A conductor-SHA-only
tag would collide: bump tx5 alone and the same tag would be republished over a
different binary. `elohim/kitsune2` is deliberately *not* in the tag and not watched —
the fork's sibling-clone kitsune2 path patches are commented out in its `Cargo.toml`,
so bumping that submodule cannot change what gets built.

## Moving a conductor onto the fleet

**Before you start.** Authoring the conductor change itself is out of scope here —
this is the promotion path, and it assumes your commit is already pushed to the
`ethosengine/holochain` fork. Promoting a **tx5** change is the same procedure with
`elohim/tx5` substituted for `elohim/holochain-conductor` throughout, including the
`build(conductor):` commit prefix — both forks feed the one conductor image. You need: push access
to a branch of this repo that CI watches (`dev` is the integration target), Jenkins
read access to watch hops 1-2, and Node + pnpm for step 3. You do **not** need Harbor
credentials, nor any local Rust/Go toolchain — the conductor is never built here.

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
#    match at every hop below. (Both pointers, because both are in the tag.)
git rev-parse HEAD:elohim/holochain-conductor HEAD:elohim/tx5 | cut -c1-12 | paste -sd-
#    → e.g. 6d08142662d9-923e17c36c04, so your tag is conductor-6d08142662d9-923e17c36c04

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
while a 40-minute build runs unobserved). Open the named job, then the branch, then
the newest build whose cause names your commit.

Match every hop against the tag you recorded in step 3.

**Mind the branch column.** Hop 1's `main` is *che-devworkspaces'* `main` — that job
lives in a different repository and always builds from its own default branch, no
matter which branch you pushed here. Hops 2 and 3 are this repo, so they follow your
branch.

| Hop | Where to look | Signal | If the signal is wrong |
|---|---|---|---|
| Conductor built | `elohim-edgenode` → `main` console | `source-derived pin tag: conductor-<hc12>-<tx512>`, and near the end `Pushed harbor.ethosengine.com/ethosengine/elohim-edgenode:{…,conductor-…}` | No pin-tag line at all means the job ran without `HC_REF` — it was started by hand rather than dispatched. Re-trigger from a commit |
| Edge consumed it | `elohim-edge` → your branch → Build Storage | `Conductor pin: …:conductor-<hc12>-<tx512>` followed by the two full SHAs | A pin you don't recognize means your pointer commit is not in this build — check you pushed it and that edge built *after* the conductor, not alongside it |
| Fleet running it | `elohim-edge` → your branch → Deploy Edge Node | first `STORAGE_TAG=1.0.0-dev-<sha>` (that `<sha>` is *this repo's* commit, not a fork pointer), then one `statefulset rolling update complete … pods at revision …` per peer | Fewer completions than peers is a partial rollout. It is not yours to finish — cluster reads and pod-level intervention are operator actions; hand it over with the build URL |

**That third row is the end of your path** — when every peer in
`genesis/orchestrator/data/deployments.json` (7 at the time of writing) has reported
its rollout complete for the `STORAGE_TAG` you just built, the promotion is done. (Confirming the tag on a running pod is a separate, operator
action: cluster reads are not performed from a dev environment, so the pipeline log is
your authority here, not `kubectl`.)

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
  of cargo plus a CGo toolchain.
- **A hard ceiling.** The monorepo's edge `Jenkinsfile` is at the JVM 64 KB CPS
  method-size limit — it has breached it before, and those builds died before any stage
  ran. It cannot absorb another build stage.
- **No checkout needed.** The conductor forks being `update = none` (above), the
  builder fetches each one from GitHub by SHA instead — so no pipeline anywhere has to
  check out a large source tree it would only read.

Unlike the conductor forks, `che-devworkspaces` is an ordinary submodule and *is*
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
| `…/elohim-edgenode:conductor-<hc12>-<tx512>` | one **image tag** in it — the pin |

⚠ **`elohim-edge` and `elohim-edgenode` are not the same thing.** They differ by four
characters, live in different repositories, and are different kinds of entity —
`elohim-edge` is a pipeline id in this repo, `elohim-edgenode` is a Jenkins job (and a
Harbor repository) belonging to che-devworkspaces. One consumes what the other
produces.

`ethosengine` is both a GitHub org (the forks) and a Harbor project (the images);
which one is meant follows from the surrounding URL.

### Variant builds

Two alternate conductor builds exist alongside the production one — same source,
different compiled configuration. Each publishes to its **own** image repository, so
it can never satisfy the production pin edge asks for. Reach for one only when
deliberately testing a transport or chasing an allocator bug, never as a way to ship:

- `elohim-edgenode-iroh` — the iroh-transport flip artifact. Its tag is
  `conductor-<hc12>-iroh`: the tx5 component is **omitted**, because iroh compiles
  instead of tx5 and a tx5 SHA would assert provenance the binary does not have. The
  differing shape is also what makes it unable to satisfy the production pin.
- `elohim-edgenode-prof` — a `jemalloc-prof` heap-profiling canary. Publishes no pin
  tag at all: a diagnostic build is not a deploy source.

Both are selected by a **commit message tag**, the same way any build is forced here:

```bash
git commit --allow-empty -m "build(conductor): iroh canary [conductor:iroh,canary]"
git push
```

`canary` adds the deployable storage image embedding that variant. Full grammar in
`genesis/orchestrator/README.md`.

## Triggers

| Trigger | Effect |
|---|---|
| A commit bumping `elohim/holochain-conductor` or `elohim/tx5` | Auto — orchestrator dispatches `elohim-conductor`, then `elohim-edge` |
| `[build:conductor]` in a commit message | Force-dispatch the build **regardless of whether a pointer moved** (this is how every `[build:*]` tag works). Needed for changes to the Dockerfile or the job itself, which live in the che-devworkspaces submodule and are not watched here — and to republish an identical pin after a tag loss, since the tag derives from the pointers rather than from build time |
| `[conductor:…]` in a commit message | Parameterize the build — variant, feature set, canary, fork-branch overrides. **It also dispatches**, so it does not need pairing with `[build:conductor]`. Full grammar in `genesis/orchestrator/README.md`; e.g. `[conductor:iroh,canary]` |

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
republish the conductor at the same pointers — an empty commit carrying
`[build:conductor]` — which reproduces the identical tag, because the tag is derived
from the pointers rather than from build time.

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
