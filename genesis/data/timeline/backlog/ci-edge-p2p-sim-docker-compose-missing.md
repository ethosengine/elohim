---
id: "backlog-ci-edge-p2p-sim-docker-compose-missing"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-edge 'P2P Simulation Test' stage fails exit-127 (docker-compose missing) — the CI runtime is daemonless buildkit/nerdctl, not Docker"
slug: "ci-edge-p2p-sim-docker-compose-missing"
written: "2026-06-13"
author: "ci-failure-triage"
status: "backlog"
priority: "low"
ci_status: blocked
fingerprints: [e40490941083, 6df5f3ebbb1a]
jobs: [elohim-edge]
relatedNodeIds: []
tags: [ci, elohim-edge, p2p-simulation, docker-compose, nerdctl, buildkit, daemonless-runtime, host-green-not-ci-green, advisory-stage, misleading-fingerprint-label, fp-remint-across-shifts]
cites:
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1162/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1161/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1069/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1068/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1067/
  - steward/node/simulation/simulate.sh
  - steward/node/simulation/docker-compose.yml
  - elohim/holochain/Jenkinsfile
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# elohim-edge "P2P Simulation Test" — exit-127 docker-compose-missing, against a daemonless containerd runtime

## The failure

```
[Pipeline] { (P2P Simulation Test)
+ ./simulate.sh test
[INFO] Running P2P simulation test...
[INFO] Starting elohim-node cluster simulation...
./simulate.sh: line 44: docker-compose: command not found
ERROR: script returned exit code 127
```

`simulate.sh:14` is `set -e`; line 44 is `docker-compose up -d` (the default branch
of `cmd_start`, reached via `cmd_test` → `cmd_start "$@"`). With `docker-compose`
absent the command exits 127, `set -e` aborts the script, and the whole
`./simulate.sh test` returns 127.

The stage (`elohim/holochain/Jenkinsfile:1375-1396`) wraps the call in
`catchError(buildResult: 'SUCCESS', stageResult: 'UNSTABLE')` and is gated by
`when { changeset "elohim/elohim-storage/src/p2p/**" OR "steward/node/simulation/**"
OR FORCE_BUILD }`. So the failure surfaces as **stage-UNSTABLE / build-UNSTABLE**,
non-blocking — the build proceeds to Build Edge Node Image, push, and Deploy.

Occurrence evidence: present in builds **#1067, #1068, #1069** (all UNSTABLE),
verified by direct log search of each — a persistent, deterministic condition, not
an intermittent flake. (The ledger shows `seen: 1, first_build: 1069, last_build:
1069` for fp `e40490941083` because the harvester fingerprinted it under the
*deploy*-path line label below, which collides/coarsens differently across builds;
the recurrence is real and visible in the raw logs.)

**Second-shift recurrence — 2026-07-07, builds #1161 and #1162 (new fp
`6df5f3ebbb1a`).** Verified by direct log search: #1162 line 16637 and #1161 line
16645 BOTH carry `./simulate.sh: line 44: docker-compose: command not found` under
the `(P2P Simulation Test)` stage — byte-identical to the #1067–#1069 signature, ~1
month and a distinct shift later. Two facts settle the correct reading and refute the
dispatch's "fresh env exposure — CI image dropped docker-compose at #1162" hypothesis:

1. **#1161 had the identical failure.** The stage ran (`+ ./simulate.sh test`, line
   16642) and died the same way (line 16645). So there is no #1161→#1162 regression;
   "#1161 was green" is only the harvester declining to mint `6df5f3ebbb1a` at #1161
   — the exact fp-instability this entry already documented (§"The misleading
   fingerprint label"). This is museum trap #1 (a fp-flip read as a regression);
   resisted.
2. **The stage ran via `FORCE_BUILD`, not a p2p/sim changeset.** #1162's triggering
   commit `84aea07a7` touches `elohim/holochain/Jenkinsfile`, root `Jenkinsfile`,
   `scripts/ci/stage-spa-blob.sh`, and three `elohim/elohim-storage/src/*` files —
   **none** under the stage's changeset gate (`elohim/elohim-storage/src/p2p/**` or
   `steward/node/simulation/**`). The gate opened via `FORCE_BUILD`. So the #1161↔#1162
   variation is *which builds open the advisory gate*, not a runtime that lost a
   binary. "docker-compose no longer present" is refuted by prior evidence: it was
   never present in this daemonless runtime — absent as far back as #1067, before an
   image change could have removed it.

Two distinct shifts now (June 13 #1067–#1069; July 7 #1161–#1162). Museum-row
promotion threshold is ≥3 distinct shifts — still **held at 2**, so this stays a
backlog entry, not a museum trap. Re-mint on the third shift graduates it.

## Verdict

**real — CI image/runtime tooling gap, deterministic (NOT a flake).** This corrects
the dispatch note's loose "flake" wording. The mechanism is fixed: the build pod has
**no Docker daemon** — it runs **buildkit + nerdctl over containerd**. Build #1069's
pod spec shows a `buildkitd` sidecar (`moby/buildkit:v0.12.5`,
`unix:///run/buildkit/buildkitd.sock`) and every image build/tag/push in the run uses
`nerdctl -n k8s.io …` (e.g. `nerdctl -n k8s.io push
harbor.ethosengine.com/ethosengine/elohim-edgenode:…`). There is no `docker` daemon
and no `/var/run/docker.sock`. `docker-compose` (the deprecated v1 hyphenated binary)
is therefore not installed — and neither would `docker compose` (v2 plugin) help,
because v2 still needs a Docker daemon this pod does not have. The containerd-native
equivalent is `nerdctl compose`.

This sits in the **host-green ≠ CI-green** cluster (museum cluster #3/#5/#6) — the
script works on a developer laptop with Docker Desktop but fails in the daemonless CI
runtime — but it is **not** any existing numbered museum trap (not Dockerfile
build-context completeness #3, not sccache #5, not `#[ignore]` #6). It is a distinct
"missing CI tool / docker-v1-vs-v2-vs-nerdctl runtime drift" shape. Recorded here for
the next planner; promote to a museum row only if it recurs across ≥3 distinct shifts.

It is also **independent of `ci-alpha-cluster-degraded-substrate`**: that concern is
deploy/E2E/health-gate failures *against degraded alpha pods* (exit-124 timeouts,
PUT/PATCH against down backends). This is an **exit-127 before any cluster is
contacted** — the script dies on the missing binary at `docker-compose up -d`, never
reaching a peer. The `ci-doorway-dockerfile-fixture-context` entry already flagged
this in passing (its line 49-51: "a *separate* concern not captured as its own
fingerprint and not addressed here") — this file is that separate concern,
canonicalized.

## The misleading fingerprint label (harvester weakness, recorded)

The ledger `line` for `e40490941083` is
`elohim-edge.deploy.alpha-doorway.elohim-doorway-alpha` — a **deploy** stage-path,
not the sim-test cause line. Verified this is cosmetic, not a second concern, by
elimination on #1069:

- The **only** `exit code` marker in the entire #1069 log is `ERROR: script returned
  exit code 127` (the sim test). No exit-124, no exit-101, no deploy-stage failure.
- No `Setting overall build result to UNSTABLE` originates from a deploy stage; the
  deploy ran 14 per-human branches with no captured failure.
- The fp-named `elohim-doorway-alpha` deploy path appears in #1069 **only** as routine
  template rendering (`DOORWAY_TAG_PLACEHOLDER` sed substitutions), a `==== Doorway
  preview: elohim-doorway-alpha ====` echo, and an `[ingress-check] Manifest ingresses:
  elohim-doorway-alpha` line — **no `Deploy … Doorway` failure block, no error**. The
  genuine doorway-deploy-failure sibling `5af3f81c7dd4` (`…-doorway-alpha-b`, owned by
  `ci-alpha-cluster-degraded-substrate`) does **not** appear in #1069 at all.
- Live `doorway-alpha.elohim.host/health` returns HTTP 200 (~54ms) — the alpha
  doorway deploy is serving (consistent with the deploy having succeeded).

So `e40490941083` mechanically **is** the P2P-sim exit-127 failure despite its
deploy-path label. **Caveat on fp tracking (do not over-trust):** the harvester's
fingerprinting of this failure is **coarse and unstable across builds** — the
`docker-compose` exit-127 was present at #1067 and #1068 too, yet this fp shows
`seen:1, first_build:1069`, i.e. the *same* failure did NOT mint this fp at 1067/68.
So the fp is demonstrably not a stable function of this failure across builds; do not
assume a future recurrence re-derives `e40490941083` specifically. This does not
affect the blocked verdict — for a blocked entry, disappearance-confirmation is not
load-bearing and recurrence-reopen is the harvester's job, not a stability claim this
entry needs to make. Classifier lesson (harvester-side, not sentinel): a stage-path
label drawn from the wrong stage mis-names the concern — prefer the failing **cause
line** (`docker-compose: command not found` / `exit code 127`) over an adjacent
stage path. Same family as the alpha-cluster entry's "fingerprint the failing-stage
cause line, not the catch-all banner" note; not opened as a separate concern.

## Root cause

`steward/node/simulation/simulate.sh` and `docker-compose.yml` assume a Docker +
docker-compose-v1 environment (a developer laptop). The edge CI build pod is a
**containerd / buildkit / nerdctl** runtime with no Docker daemon, so `docker-compose`
(and `docker compose`) cannot run. The P2P simulation 2×2-family compose harness has
**no containerd-native path** wired for CI. Git history (`d3255c9d9`, `e41aef671`,
the steward/ restructure) shows the `docker-compose` usage predates the current
runtime; there is no evidence the stage ever passed in this CI runtime — it has been
silently UNSTABLE-advisory.

## Current decision

**BLOCKED — needs an operator/infra decision; no bounded sentinel tree-fix exists.**
Three candidate unblock paths, none a clean in-tree sentinel edit:

1. **Provision a compose-capable runtime for this one stage** (operator/CI-image) —
   add `nerdctl` *with the compose subcommand* (or a docker-compose shim) to the
   `builder` image, or run the sim stage in a pod with a real Docker daemon. Operator
   surface (CI pod template / image), not a tree edit.
2. **Migrate `simulate.sh` to `nerdctl compose`** (in-tree, but verification-gated) —
   `docker-compose up -d` → `nerdctl compose up -d`, plus the `--profile latency`
   path, `docker ps`/`docker exec`/`docker network {disconnect,connect}` calls
   (`cmd_status`/`cmd_partition`/`cmd_heal`), and the implicit compose network name
   `simulation_wan-bridge`. nerdctl-in-`k8s.io`-namespace networking semantics differ
   from Docker's, so this cannot be verified without a CI run — and the sentinel
   **cannot trigger builds** (anonymous Jenkins MCP). A blind multi-call rewrite of a
   network-partition harness is exactly the unbounded, unverifiable change the
   discipline says to document rather than guess at. This is an operator-scoped
   `/shift` (Sonnet glue + a CI run to verify), not a sentinel fix.
3. **Drop / properly-skip the stage** if the 2×2 compose sim is not the intended CI
   signal here (the per-peer P2P assertions already run live against alpha) — a
   product/CI-owner judgment about whether this advisory stage earns its keep, not a
   sentinel call.

This entry stays `ci_status: blocked` (priority **low** — it is a non-blocking
advisory stage; deploys and the real pipeline are unaffected). It disappears on a
green streak once an operator picks a path and it lands — the changeset gate
(`steward/node/simulation/**`) means the next integrator push carrying any
`simulate.sh` change re-runs the stage automatically, so the harvester confirms by
disappearance.

## Fix trail

- **No tree change in this triage run** (correctly): the runtime is daemonless
  buildkit/nerdctl, so the obvious `docker-compose`→`docker compose` edit would NOT
  fix it, and the `nerdctl compose` migration is verification-gated work the sentinel
  cannot run. Documenting the blocker beats landing an unverifiable rewrite.
- Ledger: `e40490941083` set `status: blocked` (blocker: daemonless CI runtime lacks
  any compose tooling; operator/infra-owned image or a verification-gated nerdctl
  migration). **No `triaged_at_build` stamp** (nothing landed). Recurrence is expected
  every run the sim changeset gate opens, until a path lands — that is the honest
  advisory-UNSTABLE signal, not a re-fire bug.
- **2026-07-07 re-encounter:** new fp `6df5f3ebbb1a` (elohim-edge #1162, sim-stage
  cause line) folded into this concern and set `status: blocked` with `backlog`
  pointer — same daemonless-runtime blocker, no tree change, no `triaged_at_build`
  (nothing landed). Confirmed the same signature in #1161 and #1162 and that the stage
  ran via `FORCE_BUILD` (not a p2p/sim changeset), refuting the "image dropped
  docker-compose" hypothesis. The unblock paths are unchanged and still operator- or
  verification-gated; nothing became bounded, so the entry holds at `blocked`/low.
- Sentinel ceiling note (anonymous MCP): confirmation requires an operator-chosen path
  + a build, which the sentinel cannot trigger.
