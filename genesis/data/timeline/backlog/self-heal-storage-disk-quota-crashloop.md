---
id: "backlog-self-heal-storage-disk-quota-crashloop"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Shem-placed storage peers fill their 20Gi storage-data volume with sled garbage, then crashloop on the first write instead of degrading (adam's took elohim.host down)"
slug: "self-heal-storage-disk-quota-crashloop"
written: "2026-10-28"
author: "runtime-triage"
status: "backlog"
priority: "high"
self_heal_status: open
severity: high
fingerprints: [ccce437a2ecf]
nodes: [alpha-b, elohim-adam-alpha, elohim-susan-alpha, elohim-eve-alpha, elohim-gertrude-alpha, shem]
relatedNodeIds: []
tags: [self-heal, circuit, storage, disk, sled, rollout-evidence, operator]
cites:
  - https://elohim.host/admin/self-healing
  - https://elohim.host/status.json
  - https://elohim.host/health
  - https://doorway-alpha.elohim.host/admin/self-healing
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1741/consoleText
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1756/consoleText
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1757/consoleText
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1758/consoleText
  - elohim/elohim-storage/src/sync/mod.rs
  - elohim/elohim-storage/src/sync/doc_store.rs
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
  - scripts/ci/grow-conductor-pvc.sh
  - genesis/orchestrator/data/deployments.json
  - genesis/orchestrator/manifests/humans/adam-firstman.yaml
  - genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml
  - genesis/data/timeline/backlog/self-heal-adam-storage-breaker-compute-worker-readiness-coupling.md
  - .claude/data/runtime-findings.jsonl
---

# Storage crashloops when its volume is out of quota

## What is exhausted

**2026-10-27, susan.** `elohim-susan-alpha-0`'s `elohim-node` container (elohim-storage, node
`shem`) hit its disk quota while running. Loki, 2026-10-27T17:38:24Z, `sled::pagecache::logger`
(sled 0.34.7): `hit error while writing iobuf with lsn 187416777: Io(Os { code: 122, kind:
QuotaExceeded })`. Every restart since exits 1 on its first write:
`Error: Refusal { kind: "io", detail: "Disk quota exceeded (os error 122)" }`
(restarts about 5 min apart, restart count at least 27 for pod UID aa9ab99d by 2026-10-28T00:32Z).
elohim-edge dev #1741 counted it as a rollout failure (2/7, with adam).

**2026-10-28/29, adam: elohim.host is down.** Ledger line (poll 270):

```
{"fp": "ccce437a2ecf", "class": "self-heal-exhaustion", "node": "alpha-b",
 "provenance": "circuit:http://elohim-adam-alpha.elohim-alpha.svc.cluster.local:8090",
 "line": "upstream ... circuit Open >= 3 consecutive polls", "ts": "2026-10-29T16:08:06+00:00"}
```

Re-fetched 2026-10-29T16:49Z:

```
GET https://elohim.host/                      503 "This page is converging on its current release."
GET https://elohim.host/admin/self-healing    upstreams: elohim-adam-alpha:8090 circuit open,
                                              errorStreak 52, skipped true; elohim-matthew-alpha:8090 closed
GET https://elohim.host/status.json           storage.reachable false, "Connection failed: error sending
                                              request for url (http://elohim-adam-alpha...:8090/health)"
GET https://doorway-alpha.elohim.host/...     200; its storage (matthew) reachable, circuit closed
```

The fingerprint is the same one the compute-worker readiness concern carried on 2026-10-22, because
the poller hashes node + provenance. The cause this time is different: the container that is not
Ready is `elohim-node` itself, not the compute worker (see below).

Timeline (UTC), from Prometheus `kubelet_volume_stats_*`, Loki and the edge logs:

| when | evidence |
|------|----------|
| 10-27 16:50 to 10-28 ~14:50 | `storage-data-elohim-adam-alpha-0` at 0.44 to 0.70 of 21,487,026,176 bytes (20 GiB) |
| 10-28 15:00 to 18:50 | 0.826, 0.92 (16:50), 0.9816 (17:40), 0.9996 (18:30), then 1.0 from 18:50 to now |
| 10-28 18:41:51 | first quota error: `hit error while writing iobuf with lsn 288603789: Io(Os { code: 122, kind: QuotaExceeded, message: "Disk quota exceeded" })` |
| 10-28 21:16:32 | last log from the still-running pod: serving inventory, but `Failed to upsert custodian_metrics: disk I/O error` |
| 10-28 21:16:55 | first startup refusal (storage commit b94f5ad): `Error: Refusal { kind: "io", detail: "Disk quota exceeded (os error 122)" }`; crashloop every ~5 min after |
| 10-29 09:04 | `elohim-node` restarts_total already 111; compute-worker restarts 0 |
| 10-29 09:04 to ~11:50 | edge #1756 (e67c6cef8, `[edge:validate-only]`): deploy SKIPPED; validation `projected head still divergent ... on elohim.host elohim-host-landing: served=<absent>` and `Served shell does not boot through https://elohim.host` |
| 10-29 15:07 to ~16:20 | edge #1757 (bf810489c): full alpha deploy, UNSTABLE. `storage-pvc:` lines for adam, gertrude, susan and eve each end in `STORAGE-PVC-GROW-REFUSED` (the class does not allow expansion). Adam's rollout evidence: `elohim-adam-alpha-0=Running/NotReady[node=shem;containers=compute-worker=true/;elohim-node=false/CrashLoopBackOff;]`, same shape for gertrude, eve, susan. Both doorways rolled (elohim.host restarted 16:18:33) |
| 10-29 16:20 to ~16:44 | edge #1758: deploy SKIPPED, three Dataplane Validation scenarios fail on elohim.host 503 |
| 10-29 16:40, 16:45 | new pod (uid d83df639), restart 12: same Refusal at start; volume still 1.0 |

So elohim.host has been without its storage since 2026-10-28 between 18:41Z (first failed write)
and 21:16Z (process refuses to start). Last provably fine: before 18:41Z on 10-28 (volume 0.9996
at 18:30Z). Builds #1756 to #1758 observed the outage; none caused it.

## Root-cause inventory

1. `elohim/elohim-storage/src/sync/mod.rs:120-139` (`SyncManager::apply_changes` on dev):
   loads the doc, applies the peer's change blobs, then calls `self.doc_store.save(...)` (line
   135) unconditionally, even when every change was already held. Loki shows an off-fleet iroh
   peer (`e84ec663…`) that cannot name our heads re-sending the full `node:manifesto` history
   (34 MB) every minute, so every pod rewrote the 17.5 MB doc once a minute
   (`elohim/elohim-storage/.epr-meta/idle-is-free.habit.md`, DELTA 2026-10-29: `sync.sled` grew
   about 1.1 GB/h on every alpha storage pod while the corpus stayed flat; matthew 14.9 to
   29.6 GB between 10-28 06:40Z and 10-29 00:40Z).
2. `elohim/elohim-storage/src/sync/doc_store.rs:71-75`: the sync sled (0.34) stores the doc as a
   large value in its own blob file and never removes the superseded file on overwrite. Nothing
   reclaims it at runtime, so every redundant rewrite is permanent garbage.
3. `genesis/orchestrator/manifests/humans/adam-firstman.yaml:407-411` and
   `_edgenode-consolidated.template.yaml:613-616`: `storage-data` is a fixed 20Gi
   volumeClaimTemplate on `shem-zfs` for the remote (shem) humans. On ZFS the quota is
   enforced, so the sled hits `os error 122` at 20 GiB. Matthew, on `openebs-hostpath` with no
   quota, grows past 29 GB and stays up, which is why only shem pods crashloop.
4. `scripts/ci/grow-conductor-pvc.sh` + `deployments.json storageDataSize: "80Gi"` (1e1d74265,
   on origin/dev): the declared reconciler ran in #1757 and printed `STORAGE-PVC-GROW-REFUSED`
   for all four claims, saying the class does not allow expansion. That was wrong: `shem-zfs`
   does expand (the operator resized all four claims to 80Gi on 2026-10-29). The deployer cannot
   read cluster-scoped storageclasses, and the script turned an unanswered read into `false`.
   Fixed on the big train: only a class that answers `false` is refused; an unread one goes on to
   the patch and the cluster decides. Kubelet capacity still read 21,487,026,176 bytes after the
   roll.
5. Storage start path: a full volume makes the process exit on its first write instead of serving
   reads. The pod goes NotReady, the ClusterIP Service `elohim-adam-alpha` drops its only
   endpoint, and the elohim.host doorway's breaker (`doorway/doorway-service/src/routes/upstream_health.rs`)
   correctly holds it Open. The breaker is reporting truthfully; the app shell has no pool
   fallback, so `GET /` answers 503.

## Fix path

1. **Operator, now (unblocks adam and elohim.host):** give `storage-data-elohim-adam-alpha-0`
   room on shem: resize the claim to 80Gi AND raise the dataset's hand-set refquota to 80G (the
   resize alone left 0 free, because the 2026-10-01 refquota of 20G stayed), then restart the pod.
   The same move applies to gertrude, eve and susan. **Done by the operator 2026-10-29 ~17:00Z:**
   all four Running/Ready, about 60G free each, elohim.host 200 with storage reachable. The
   StatefulSet's volumeClaimTemplate stays 20Gi by design (immutable); the live claims carry the
   size.
2. **Stop the growth (repo, in flight):** `fix/storage-sled-bounded` (`c956510b0` and siblings,
   merged into `land/big-train`, not on dev): `apply_changes` skips the save when the heads equal
   the stored heads and no orphan is queued, counted by `elohim_sync_doc_writes_total{outcome}`.
   `fix/storage-sled-reclaim` (`2035c83f5`) adds `needs_pull` so a node that already holds every
   remote head does not ask. Without these, an 80 GiB volume refills in roughly two days at
   1.1 GB/h.
3. **Return the garbage (repo, uncommitted in `.worktrees/storage-sled-reclaim`):**
   `sync/sled_reclaim.rs` exports and re-imports the sync sled at boot when it is >2 GiB and >4x
   its logical bytes. It refuses when free space is under about twice the logical bytes plus a
   fixed headroom, so on a volume at 100% it cannot run until step 1 frees space. It is not a
   substitute for the operator action.
4. **Degrade instead of dying (repo, unstarted):** on a full volume, storage should keep serving
   reads, refuse writes with a named reason and report a `disk-quota` exhaustion on
   `/admin/self-healing`. Today it exits before `/health` answers, so the only signal is the
   doorway's breaker.
5. **Rollout evidence (repo, unstarted):** `jenkins-deployer` lacks `pods/log` and `nodes` list,
   so `rollout-evidence/*/--elohim-node--previous.log` carries only a Forbidden line. Grant those
   reads in the RBAC manifest or name Loki as the log source in the summary.

## Current decision

**2026-10-29: step 1 done by the operator (~17:00Z); open on steps 2 and 3.** No repo change
could bring adam back: the volume was full, the grow step falsely refused (an unread class read as
`false`, fixed on the big train), and the reclaim needs free space to run. 80G buys about two days
at 1.1 GB/h. Steps 2 and 3 are the repo half and must land before (or with)
the volume action, or the volume refills. Do not roll the fleet to "fix" it: #1757 already
recreated adam's pod and it crashed the same way.

A recurrence of `ccce437a2ecf` (elohim.host's breaker to adam Open) while `elohim-node` is the
NotReady container and the volume reads 1.0 is this concern. If the NotReady container is
`compute-worker`, it is
`self-heal-adam-storage-breaker-compute-worker-readiness-coupling.md` instead.

## Verification

Not yet verified. Done when: after the operator's volume action, `kubelet_volume_stats_used_bytes`
for `storage-data-elohim-adam-alpha-0` is under 0.8 of capacity; `GET https://elohim.host/` is
200; `https://elohim.host/admin/self-healing` shows the adam upstream `closed`, errorStreak 0;
`status.json` shows `storage.reachable: true`. After the write cure deploys,
`elohim_node_store_bytes{store="sync_sled"}` stays flat at rest for 24 h. A storage peer started on
a full volume stays up, serves reads, refuses writes with a named reason and reports the
exhaustion. The next edge rollout's evidence for a crashlooping container includes its last log
lines.
