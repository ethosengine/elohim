---
id: "backlog-alpha-doorways-record-no-hosting-promise"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Alpha doorways record no hosting promise for anyone they host — no POOL_COMPUTE_* on the doorways and no compute local API on the pool peers' storage — so the CI runner cannot be sponsored on the fleet (SERVICE_AGENT_NO_SPONSOR)"
slug: "alpha-doorways-record-no-hosting-promise"
written: "2026-10-20"
author: "overnight shift 2026-10-20 (ci-runner-story-green)"
status: "open"
priority: "high"
jobs: [elohim-edge, elohim-genesis]
cluster: "security-ci-substrate-authorization-grant-coherence"
tags: [delegated-compute, hosted-cell, ci-runner, fleet-config]
---

**Evidence (2026-10-20).** doorway-alpha runs `1b2deee` (service-agent registration deployed
02:25Z). The next genesis build's runner preflight printed `REFUSED standing: runner-ensure: cure
— set POOL_COMPUTE_URL/POOL_COMPUTE_TOKEN/POOL_COMPUTE_PERFORMER on the doorway`. No fleet manifest
sets any of the three on a doorway (`genesis/orchestrator/manifests/doorway/alpha.yaml`,
`alpha-b.yaml`), and no storage container sets `ELOHIM_COMPUTE_LOCAL_API` /
`ELOHIM_COMPUTE_LOCAL_TOKEN` (`genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml`).
`pool_compute_config` (`doorway/doorway-service/src/routes/hosted_cell.rs`) returns `None` and the
hosting-promise leg is skipped silently — for hosted HUMANS too: alpha has never notarized a
`hosted-cell` promise, so `humansServed` reads 0 and nobody's hosting is accountable on the
substrate. The household mesh wires all of it (`app/elohim-app/scripts/hc-mesh.sh`, search
`POOL_COMPUTE`), which is why the runner works there and not on the fleet.

**The change (operator-gated: a new Secret, a live key read, a fleet roll).**
1. Storage container env for the pool peers (matthew behind doorway-alpha, adam behind
   elohim.host): `ELOHIM_COMPUTE_LOCAL_API=1`, `ELOHIM_COMPUTE_LOCAL_TOKEN` from a new Secret
   (`secretKeyRef`, as the doorway reads `API_KEY_ADMIN`).
2. Doorway env, per doorway: `POOL_COMPUTE_URL` = its `STORAGE_URL` (alpha.yaml:290 /
   alpha-b.yaml:298), `POOL_COMPUTE_TOKEN` = the same Secret, `POOL_COMPUTE_PERFORMER` = that
   peer's own cell agent key. Check `DOORWAY_HOSTING_CONDUCTORS` (`computeHostingConductorUrls`,
   `elohim/holochain/Jenkinsfile:2079`) names that same peer's conductor before choosing it.
3. The performer key: nothing on the fleet derives it (the household reads it with `list-apps`,
   `hc-mesh.sh` `peer_agent_key`). Read it once from the live conductor (stable on the persistent
   PVC) into a ConfigMap/placeholder, or — better, and repo-side — let the doorway learn it from
   its pool peer at boot so no deploy-time key exists to drift (the storage would need to expose
   its own cell key on the local compute API; none does today).

**Then:** genesis's runner preflight sponsors `agent:ci-runner-jenkins-matthew` on matthew's
pool peer by itself (`genesis/scripts/ci/runner-ensure.sh`); a provider with the compute API on
(jessica's fleet peer, or adam) grants it capacity with `measure.sh grant --recipient`, and its
action hash goes in `providers.json` as `runnerGrant`.

**Done when:** a genesis build's runner preflight prints `ok standing` with the hosting promise's
days remaining, and `GET /api/v1/compute/grants?scope=hosted-cell` on matthew's alpha storage
lists the runner's promise.
