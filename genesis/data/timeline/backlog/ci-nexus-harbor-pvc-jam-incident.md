---
id: "backlog-ci-nexus-harbor-pvc-jam-incident"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Nexus/Harbor PVC-jam incident — npm-proxy 401 (elohim Install Deps) + missing .happ artifact (elohim-edge) are facets of one operator-fixed registry-substrate outage"
slug: "ci-nexus-harbor-pvc-jam-incident"
written: "2026-06-07"
author: "ci-failure-triage"
status: "backlog"
priority: "high"
ci_status: blocked
fingerprints: [e5c368717547, 06fe22f01a81]
jobs: [elohim, elohim-edge]
relatedNodeIds:
  - "memory:project_ci_storage_topology"
tags: [ci, infra, nexus, harbor, registry, pvc, install-dependencies, happ-artifact, operator-domain, incident-resolved, museum-trap-1, requires-env]
cites:
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1508/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1509/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1045/
  - https://jenkins.ethosengine.com/job/elohim-holochain/job/dev/1310/
  - https://jenkins.ethosengine.com/job/elohim-holochain/job/dev/1311/
  - genesis/data/timeline/backlog/harbor-registry-spof.md
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# Nexus/Harbor PVC-jam incident — two job-failures, one operator-fixed registry outage

## The failure

Two fingerprints, two jobs, **one** infrastructure condition (the same N:1
infra-incident shape as `ci-alpha-cluster-degraded-substrate.md`):

```
e5c368717547  elohim      — red build, stage:Install Dependencies   (1508–1509, FAILURE)
06fe22f01a81  elohim-edge — hApp artifact not found:
                            Run with FORCE_BUILD after DNA build succeeds  (1045, FAILURE)
```

Occurrence evidence (ledger): `e5c368717547` seen 2 (first 1508, last 1509);
`06fe22f01a81` seen 1 (build 1045). Both builds confirmed **FAILURE** via the
build API (not NOT_BUILT/ABORTED — museum trap #1 cleared for the *symptom*
builds themselves; the upstream that starved the edge build, however, WAS
ABORTED — see below).

### Facet 1 — elohim Install Dependencies, Nexus npm-proxy 401

`elohim/dev` #1509 (and #1508, same ~72–82s fast-fail signature) died in the
`Install Dependencies` stage on `pnpm install --frozen-lockfile`:

```
 ERR_PNPM_FETCH_401  GET https://nexus.ethosengine.com/repository/npm/@angular/animations/-/animations-19.2.19.tgz: Unauthorized - 401

An authorization header was used: Bearer NpmT[hidden]
//nexus.ethosengine.com/repository/npm/:_authToken=NpmT[hidden]
```

The auth token IS present and IS sent (Bearer header masked but logged) — the
Nexus npm proxy itself returned 401 Unauthorized on the package fetch. That is
a degraded-Nexus symptom (a jammed/unhealthy proxy on a wedged PVC returning
401 on upstream pulls), **not** a credential rotation or a missing token. The
stage fails → `Push to Harbor Registry` skipped due to earlier failure → build
FAILURE.

### Facet 2 — elohim-edge, hApp artifact not found

`elohim-edge/dev` #1045 died because it could not fetch the `.happ` from the
upstream DNA pipeline:

```
Skipping Edge Node image build - no hApp available
Run the pipeline with FORCE_BUILD after ensuring DNA artifacts exist
...
CANNOT BUILD HAPP INSTALLER: No hApp artifact available
Could not fetch hApp from elohim-holochain pipeline.
Ensure elohim-holochain pipeline has run successfully first.
```

The upstream `elohim-holochain/dev` build in that window (#1310) was **ABORTED**
(superseded), so it published no `.happ`. The edge build's "artifact not found"
is the downstream consequence of an ABORTED upstream during the incident — the
"Run with FORCE_BUILD" message is the pipeline's own guidance, not a code
defect in the edge Jenkinsfile.

## Verdict

**infra — Nexus/Harbor registry-substrate outage on jammed PVCs (operator-owned);
operator has FIXED it (Harbor+Nexus moved to stable EBS PVCs), rebuild-all in
progress.** Not a flake, not a code regression, no in-tree fix surface. Both
facets are downstream symptoms of the same registry/storage outage:

- Nexus npm proxy returning 401 on package fetch → elohim Install Deps FAILURE.
- The artifact-publish/storage path degraded → the DNA build that should have
  produced the `.happ` did not land it (the relevant run ABORTED), starving the
  edge build.

This is the **`harbor-registry-spof` standing risk made concrete a third time**
(registry SPOF: when the registry substrate is unhealthy, every pulling
pipeline wedges/fails with no self-heal) — see that backlog entry. The
ABORTED-upstream half is also a textbook **museum trap #1**: an ABORTED
upstream is not a regression; the edge "artifact not found" must be read as
"upstream didn't produce it," not "the edge code broke."

## Root cause

The registry substrate (Nexus npm proxy + Harbor OCI registry) was unhealthy on
jammed PVCs. Two consequences, two jobs:

1. **Nexus npm proxy 401** — the proxy could not authenticate/serve upstream
   npm pulls while degraded; `pnpm install` got 401 on the first package fetch
   and aborted the install.
2. **Missing `.happ`** — the DNA artifact never reached the edge build because
   the upstream DNA run (#1310) ABORTED during the incident, so there was no
   artifact to ORAS-fetch.

## Current decision

**BLOCKED on operator — registry/PVC topology is operator-owned (never
`kubectl`); the cleanup surface is operator-side and the operator has already
actioned it.** Unblock evidence is the **rebuild-all green streak**, not a tree
change:

- The operator reports Harbor+Nexus moved to stable EBS PVCs and a full
  rebuild-all in progress.
- Confirming evidence already visible: `elohim-holochain/dev` **#1311 = SUCCESS**
  (post-incident), so the `.happ` artifact is being published again — facet 2's
  upstream starvation is already cleared at the source.

No in-tree fix in this triage run (correct — registry substrate is
operator-owned, and the incident is resolved). Both ledger fingerprints set
`status: blocked` with this entry as the blocker; no `triaged_at_build` (nothing
landed in the tree). The harvester confirms disappearance on the rebuild-all
green streak (≥3 with no recurrence). Recurrence on a fresh build BEFORE the
rebuild completes is expected incident-tail, not a new regression; recurrence
AFTER a clean rebuild would mean the registry fix did not fully take and this
re-escalates to the operator.

## Durable hardening (already canonicalized — not re-derived here)

The self-heal gap this incident exercised is the open
`harbor-registry-spof` backlog item (HA registry / pull-through mirror / early
degradation signal so a pulling pipeline fails over instead of wedging). This
incident is its **third recurrence** (after 2026-04-28 ImagePullBackOff and
2026-05-30 storage EIO) and now spans Nexus (npm proxy) as well as Harbor (OCI)
— widening that entry's scope from "Harbor" to "the registry substrate
(Nexus + Harbor)." No second hardening doc is forked; that entry is the home.

## Fix trail

- No tree change (operator-domain registry-substrate incident, operator-fixed).
- Ledger: `e5c368717547` and `06fe22f01a81` set `status: blocked` (blocker:
  Nexus/Harbor PVC-jam incident, operator-fixed, rebuild-all in progress).
- Evidence the fix is taking: `elohim-holochain/dev` #1311 SUCCESS (artifact
  publish restored). Awaiting the elohim + elohim-edge rebuild-all greens for
  harvester disappearance-confirmation.
