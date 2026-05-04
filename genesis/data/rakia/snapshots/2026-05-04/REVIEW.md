# Alpha Cluster Compute Capacity Review

**Snapshot date:** 2026-05-04
**Captured by:** Matthew via cluster exploration session
**Companion data:** `compute-capacity-snapshot.json` (machine-readable)
**Refresh cadence:** ad-hoc until `snapshot-capacity.sh` is built — re-run `kubectl get nodes/pods/pvc` queries for current state

---

## Purpose

Ground CI/CD planning decisions in actual cluster topology and observed capacity, not guesses. Specifically supports:
- Deciding which humans (elohim deployments) to suspend when capacity is tight
- Sizing future steward commitments (Jenkins, Harbor, Nexus, Eclipse Che)
- Validating that declared deployment costs match observed reality
- Anchoring the future "story group" abstraction in real fate-sharing topology

---

## Cluster topology

7 nodes total, **6 Ready, 1 dead**.

| Node | Role | Type | CPU | RAM | Ephemeral | Status |
|---|---|---|---|---|---|---|
| `ethosengine` | control-plane | performance | 24c | 64 GiB | 909 GiB | Ready |
| `intel-nuc` | control-plane | operations | 8c | 15 GiB | 465 GiB | Ready (limits overcommitted) |
| `thinkc-p0h` | control-plane | edge | 4c | 7.5 GiB | 466 GiB | Ready |
| `thinkc-p0t` | control-plane | edge | 4c | 7.5 GiB | 457 GiB | Ready |
| `thinkc-p1s` | control-plane | edge | 4c | 7.5 GiB | 457 GiB | Ready |
| `hp-micro10` | worker | storage | 2c | 15 GiB | **3.66 TiB** | Ready (CPU limits 150%) |
| `shem` | worker | remote | 24c | 128 GiB | 294 GiB | **Dead — PSU failed; replacement in ~1 week. Cordoned 2026-05-04.** |

**Total Ready capacity:** 46 cores, 134 GiB RAM, 6.94 TiB ephemeral.
**Total Ready committed (requests):** 11.2 cores, 23 GiB RAM.
**Total Ready headroom:** **34.8 cores, 110 GiB RAM.**

### Headroom by node-type

| Type | Headroom CPU | Headroom RAM |
|---|---|---|
| performance | 19.97c | 53.5 GiB |
| operations | 4.95c | 8.1 GiB |
| edge (3 nodes) | 8.75c | 18.5 GiB |
| storage | 1.15c | 12.9 GiB |

---

## Live actuals (`kubectl top` — restored 2026-05-04)

| Node | CPU used | CPU% | Mem used | Mem% |
|---|---|---|---|---|
| ethosengine | 3.5c | 14% | 24.7 GiB | 38% |
| intel-nuc | 2.7c | 34% | 7.1 GiB | 45% |
| hp-micro10 | 0.3c | 15% | 3.9 GiB | 25% |
| **thinkc-p0h** | **3.8c** | **94%** | 4.0 GiB | 52% |
| thinkc-p0t | 3.2c | 79% | 4.9 GiB | 63% |
| thinkc-p1s | 3.1c | 76% | 4.0 GiB | 53% |

**Edge nodes are the bottleneck during Jenkins build storms** (multiple PR storybook builds run concurrently on edge agents). Each storybook builder consumes ~1 CPU + 200–1300 MiB. Three or four parallel PRs and an edge node tips into throttling.

### Top memory consumers

1. `mbd06b-gmail-com-che-3nkyak/workspace48851cf439434f53` — **10.8 GiB** (this workspace, no requests/limits set)
2. `observability/prometheus-kube-prom-stack-...prometheus-0` — 4.3 GiB
3. `jenkins/ee-jenkins-0` — 3.8 GiB (near 4 GiB limit; bump candidate)
4. `nexus/nexus-nexus3-0` — 2.0 GiB
5. `elohim-alpha/elohim-matthew-alpha-0` — 2.0 GiB (within 8 GiB TEMP BUMP limit)

---

## Steward commitments (declared)

| Steward | Workload | Node | Req CPU | Req Mem | Lim CPU | Lim Mem | PVC |
|---|---|---|---|---|---|---|---|
| Jenkins | `ee-jenkins` STS | ethosengine | 50m | 256 Mi | 2c | 4 GiB | 78 GiB |
| Harbor | 8 workloads | mixed | **200m** ⚠ | **512 Mi** ⚠ | **1c** ⚠ | **1 GiB** ⚠ | 108 GiB |
| Nexus | `nexus-nexus3` STS | hp-micro10 | 500m | 2 GiB | 2c | 4 GiB | 100 GiB |
| Eclipse Che | 4 deploys | edge | 650m | 992 Mi | 500m* | 9.5 GiB | 0 |

⚠ **Harbor: 8 of 9 containers have NO requests AND NO limits set.** Only trivy is capped. Core, database, portal, redis, registry, registryctl, jobservice, exporter are uncapped — under build-storm load they can starve neighbors on ethosengine / thinkc-p1s / hp-micro10.

\* Most che containers have memory limits but no CPU limit.

**Steward total (declared, excluding uncapped Harbor):** 1.4c req / 5.5c lim, 3.8 GiB req / 18.7 GiB lim, 286 GiB PVC.

---

## Elohim humans (per-deployment cost)

### Alpha environment

| Human | Pod | Node | Req CPU | Req Mem | Lim CPU | Lim Mem | PVC | Status |
|---|---|---|---|---|---|---|---|---|
| matthew (manager) | `elohim-matthew-alpha-0` | ethosengine | 1c | 2 GiB | 3c | 8 GiB | 40 GiB | Running (TEMP BUMP active) |
| jessica (spouse) | `elohim-jessica-alpha-0` | intel-nuc | 250m | 512 Mi | 1c | 1.5 GiB | 40 GiB | Running |
| timothy (tutor) | `elohim-timothy-alpha-0` | thinkc-p0h | 250m | 1 GiB | 1c | 3 GiB | 40 GiB | Running |
| **adam (firstman)** | `elohim-adam-alpha-0` | (was shem) | 500m | 2 GiB | 2c | 6 GiB | 40 GiB lost | **Pending — node-type=remote unavailable** |
| **frank (farmer)** | `elohim-frank-alpha-0` | (was shem) | 250m | 512 Mi | 1c | 1.5 GiB | 40 GiB lost | **Pending — node-type=remote unavailable** |
| **pete (pastor)** | `elohim-pete-alpha-0` | (was shem) | 250m | 512 Mi | 1c | 1.5 GiB | 40 GiB lost | **Pending — node-type=remote unavailable** |

Shared infra (alpha): doorway (110m/160 Mi req), site (100m/128 Mi), mongodb (100m/256 Mi), nats (50m/64 Mi). All on intel-nuc.

### Prod / Staging

| Human | Env | Node | Req CPU | Req Mem | Lim CPU | Lim Mem | PVC |
|---|---|---|---|---|---|---|---|
| frank (farmer) | prod | intel-nuc | 330m | 616 Mi | 1.4c | 1.95 GiB | 15 GiB |
| pete (pastor) | staging | intel-nuc | 330m | 616 Mi | 1.4c | 1.95 GiB | 15 GiB |
| timothy (tutor) | staging | intel-nuc | 330m | 616 Mi | 1.4c | 1.95 GiB | 15 GiB |

**Elohim total (declared, including stranded):** 4.1c req / 15.8c lim, 9.1 GiB req / 36 GiB lim, 320 GiB PVC.

---

## Open issues (CI/CD-relevant)

### Blockers
1. **adam / frank / pete (alpha) are Pending** because their StatefulSet `spec.affinity.nodeAffinity` requires `node-type=remote`, and shem (the only `remote`-labeled node) is dead. Three options:
   - Relabel another node `node-type=remote` (quick fix, but co-locates them with whatever already lives there).
   - Update the StatefulSet templates in this repo to allow `edge` or `performance` and re-deploy via Jenkins.
   - Wait ~1 week for shem replacement PSU.

### Risks
2. **Single point of failure on intel-nuc** for elohim. mongodb-alpha + nats-alpha + 4 elohim humans all live there. limits overcommitted at 135% CPU / 112% memory. If intel-nuc goes the way of shem, all elohim envs lose data plane simultaneously.
3. **Harbor uncapped resources** (see above) — under heavy image push/pull during a release storm, Harbor can starve Jenkins on ethosengine.
4. **Edge nodes throttle under build storms.** Plan: cap concurrent Jenkins agents per edge node, or move builders to ethosengine where there's slack.
5. **No backup story for openebs-hostpath PVCs.** Shem's death = irrecoverable loss of 6 elohim PVCs. Same fate awaits any other node holding hostpath data without external backup.

### Latent bugs
6. `openebs` jiva replica `pvc-0b391c00-…-jiva-rep-1` on thinkc-p0t in CrashLoopBackOff (~2900 restarts). Patch `MAX_CHAIN_LENGTH` on the jiva-ctrl Deployment.
7. 4 jiva volumes lost their shem-resident replica — degraded but serving. openebs will keep retrying on shem until the node is removed.
8. `kube-prom-stack-prometheus-node-exporter-sk5mh` on thinkc-p0t has 7215 lifetime restarts (currently stable).
9. metrics-server was disabled cluster-wide for an unknown duration — restored 2026-05-04 via `microk8s enable metrics-server`. HPA / VPA / actuals-based decisions were blind during that window.

---

## Recommended next steps

1. **Decide adam/frank/pete affinity policy.** Either relabel a node or change StatefulSet specs in this repo.
2. **Set requests/limits on every Harbor container.** Even conservative caps (200m / 256 Mi req) will prevent starvation cascades.
3. **Add backup for openebs-hostpath PVCs** — at minimum elohim-alpha holochain data and mongodb data. Consider migrating from `openebs-hostpath` to `openebs-jiva` (multi-replica) for anything you can't lose.
4. **Build `snapshot-capacity.sh`** so this ledger refreshes on demand without manual kubectl spelunking. Live in `genesis/orchestrator/scripts/`.
5. **Wire up a validator** (CI step or `genesis` hook) that checks: `sum(declared_steward_commitments) <= observed_actual_usage * safety_factor` and flags drift.
6. **Bump ee-jenkins memory limit** from 4 GiB → 6 GiB (it sits at 3.8 GiB during normal load with build agents queued).
7. **Plan a `microk8s` upgrade window** — 1.29 reached upstream EOL Feb 2026. microk8s LTS gives extra runway but don't drift further.

---

## Story-group preview (for future capacity reasoning)

Once we formalize "story groups" as deployment groups that share fate when a node goes down, the natural groupings on this cluster are:

- **`group:ethosengine-perf`** — matthew-alpha, ee-jenkins-0, harbor (most), workspace48851cf, sonarqube. Loss of ethosengine breaks all build/CI + matthew + sonarqube simultaneously.
- **`group:intel-nuc-ops`** — jessica-alpha, frank-prod, pete-staging, timothy-staging, mongodb-alpha, nats-alpha, doorway-alpha, site-alpha. Loss of intel-nuc breaks all elohim envs.
- **`group:edge-builders`** — timothy-alpha, eclipse-che gateway/operator, jenkins build agents. Edge throttling delays merges.
- **`group:hp-micro10-storage`** — nexus, harbor-registry. Loss = lost artifacts.
- **`group:shem-remote`** — adam, frank-alpha, pete-alpha. Already realized: node death = pod stranding + data loss.

The shem outage is a real-world test case for the abstraction: had we modeled it, we would have predicted exactly which 3 humans got stranded and which PVCs were unrecoverable.
