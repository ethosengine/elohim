---
id: "backlog-alpha-adam-peer-meta-store-disk-io-error"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "adam's conductor volume was full: every peer-meta write failed with SQLite 778 (SQLITE_IOERR_WRITE) for three hours after the 17:24Z roll — four million gossip-initiate lines, and eve is at 90 percent"
slug: "alpha-adam-peer-meta-store-disk-io-error"
written: "2026-10-06"
author: "claude-fable-5-1 (conductor lock-deferral landing; Loki read by ci-investigator)"
status: "open"
priority: "high"
severity: high
nodes: [alpha]
tags: [conductor, kitsune2, peer-meta-store, sqlite, disk-io, adam, elohim-alpha, ops, observability, loki, log-volume, dataplane-convergence, lane-operator, phase-steady]
cites:
  - genesis/data/timeline/backlog/conductor-residual-cpu-full-chain-read-and-perpetual-republish.md
  - genesis/data/timeline/backlog/ops-adam-pod-log-volume-saturates-loki.md
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
---

# adam's peer-meta store refused every write with a disk I/O error

## Evidence (Loki, pod `elohim-adam-alpha-conductor-0`, namespace `elohim-alpha`, 2026-10-06 17:20Z to 19:00Z)

- 3,980,646 lines of one template, every one carrying SQLite code 778:

  ```
  2026-10-06T18:59:00.999656Z  WARN kitsune2_gossip::initiate: /usr/local/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/kitsune2_gossip-0.5.0/src/initiate.rs:129: Failed to initiate gossip: Other { ctx: "Failed to put peer meta", src: Some(Sqlx(Database(SqliteError { code: 778, message: "disk I/O error" }))) }
  ```

- Rate 1,000 to 1,450 lines a second; 97 percent of the pod's 4,028,567 lines in the window. The line names no database file.
- Onset: 17 lines in the five minutes to 17:50Z, then 307,749 to 17:55Z and 442,046 to 18:00Z; a ten-minute gap 18:05Z to 18:15Z; then 300,000 to 430,000 per five minutes through 19:00Z. The roll that restarted the pod was at 17:24Z (dev `c02870e17`, edge #1558).
- Stop: between 19:00Z and 21:48Z. In the fifteen minutes to 22:03Z the pod logged 1,808 lines in total and none with code 778. Whether the pod restarted in between is not read (Loki answered 502 from 22:00Z; the earlier log-volume entry says adam's volume alone can do that).
- The same code reached other writers on adam in the window: 127 `could not record publish time` lines carry 778 instead of a lock code.
- Code 778 is `SQLITE_IOERR_WRITE` (primary 10, extended 3 << 8): a `write()` call on the database or its WAL failed. It is not lock contention and no busy timeout applies to it. The common causes are a full volume and an erroring block device.

## What it does

kitsune2 cannot record a peer's metadata, so every gossip initiation with that peer fails at once and is retried at once: the loop runs as fast as it can fail, which is where the four million lines come from. While it lasts adam does not gossip, which is the same window in which adam's publish loop flooded the lock (conductor-residual-cpu entry, MEASURED 2026-10-06) and genesis #1626 could not read adam's custody row.

## The volume (Prometheus, `kubelet_volume_stats_*`, namespace `elohim-alpha`)

- `holochain-data-elohim-adam-alpha-0` (node `shem`), used over capacity: 1.00 at 17:00Z, 18:00Z, 19:00Z and 20:00Z; 0.965 at 21:00Z; 0.947 at 22:00Z; 91.5 percent at 22:05Z. Used bytes 6.9 GB at 17:00Z, 7.13 GB at 19:00Z, 6.73 GB at 22:00Z; capacity read at 22:05Z 7,412,908,032 bytes. The volume was full for the whole window in which 778 was logged, and the lines stopped once it was not.
- The same pod's `storage-data` PVC sat at 8 percent. Matthew's conductor PVC (node `ethosengine`) sat at 16 percent of 3.94 TB: adam's conductor volume is about 530 times smaller.
- At 22:05Z the five fullest PVCs in the namespace: adam conductor 91.5 percent, **eve conductor 90.05 percent**, gertrude conductor 67 percent, susan conductor 41 percent, `elohim-doorway-alpha-node-key` 32 percent. Adam, eve, gertrude and susan are the four conductors on `shem`; eve is next.
- The container did not restart: `kube_pod_container_status_restarts_total` 0 from 17:00Z to 22:00Z, `container_start_time_seconds` 17:21:22Z (the roll). What freed the space between 20:00Z and 21:00Z is not read (a WAL truncation, a prune, or an operator resize; the ratio and byte series do not reconcile against one capacity, which reads as a resize).
- Not read: `kubelet_volume_stats_inodes_free`, capacity history, eve's history, shem's filesystem behind the hostpath PVs at 18:30Z (193.7 GB free on `/` at 22:05Z, mountpoint unconfirmed as the PV's).

## Where the size is declared

**DELTA 2026-10-07 (declared, since dev `2f4679293`):** adam and eve carry `conductorStorage: 20Gi` in `genesis/orchestrator/data/deployments.json`, and `deployHumanConductor` runs `scripts/ci/grow-conductor-pvc.sh` before each roll: it reads the live claim, raises the request when the storage class allows expansion, and otherwise prints `CONDUCTOR-PVC-GROW-REFUSED` with the operator's move (warn-only; a roll is never failed by it). The next edge roll is the probe for whether the class expands; gertrude and susan are not declared. The paragraph below describes the state before that commit.

Before 2026-10-07: nowhere in the repo. `holochain-data-<prefix>-0` was minted by the storage StatefulSet's retired `holochain-data` volumeClaimTemplate and is now mounted by `_edgenode-conductor.template.yaml` (and `adam-firstman-conductor.yaml`) by explicit `claimName`; the PVC is Retained on an openebs-hostpath PV with node affinity. Its 7.4 GB is whatever the PV carried when it was minted; the live `storage-data` template says 20Gi. So this is operator-owned volume state, not a manifest the next pipeline reconciles.

## Fix shape

Operator (the live volume): grow `holochain-data-elohim-adam-alpha-0` and `holochain-data-elohim-eve-alpha-0` (and read gertrude's and susan's) well past their chains' size, or move the four shem conductors' hostpath PVs onto the filesystem that has the 194 GB; a chain that cannot write is not a node. Adam at 91.5 percent and eve at 90 percent will be full again after the next roll's integration.

Repo (so it cannot happen silently again): the edge pipeline's substrate probe reads PVC usage before it rolls (`kubelet_volume_stats_used_bytes / capacity_bytes` per conductor PVC in the namespace) and prints one `PVC <name> <pct>%` line per conductor, refusing the roll above 90 percent the way the quiesce gate refuses on matthew's `caughtUp`; and the dataplane-convergence habit's post-roll measure carries the same read, so a full volume reads as a full volume and not as a lock storm (conductor-residual-cpu entry) or a Loki outage (`ops-adam-pod-log-volume-saturates-loki`).

Conductor (kitsune2, as a dependency): a peer-meta write that fails with an I/O error is retried at the gossip loop's pace, which is how 7.4 GB of full volume became four million log lines. One line per peer per minute and a backoff belong in `kitsune2_gossip` `initiate.rs` (0.5.0, line 129); the fork carries kitsune2 as a crate, not as source.

Probe: zero `code: 778` lines on any conductor pod in the hour after a roll; every conductor PVC under 80 percent at the roll's substrate probe; adam's pod log rate within 2× its siblings'.

## DELTA 2026-10-06 (after the conductor roll: eve is full and flooding; adam is at 96 percent)

Prometheus at 23:46Z: `holochain-data-elohim-eve-alpha-0` 100 percent (100 at 23:00Z, 98.1 at 23:30Z, 100 now); `holochain-data-elohim-adam-alpha-0` 96.2 percent (100 at 23:00Z, 94.6 at 23:30Z). Eve's conductor, restarted 23:01:52Z on the new pin, logged 51,163 `code: 778` lines in its first 44 minutes, from 23:31Z at 15,000 to 21,000 per five minutes; the first of them is the peer-meta store's expiry sweep failing (`holochain_p2p::spawn::actor`, `actor.rs:782`, `error returned from database: (code: 778) disk I/O error`), then the gossip-initiate form. Adam logged none. Eve is the same incident on the next node, and adam will be back in it on the next roll's integration. The operator read of the four shem conductor volumes is the open action; nothing here is answered by the conductor change that just landed.

## DELTA 2026-10-07 (the claim says 20Gi, the filesystem is still 7.1 GiB, and adam's conductor crash-loops on the roll)

Edge #1568 (e68c909f5, 08:35Z) ran the declared-size step for the first time: `conductor-pvc:
holochain-data-elohim-adam-alpha-0: live class=shem-zfs requested=20Gi capacity=20Gi declared=20Gi`
→ `ok — the live request already meets the declared size`; eve the same. But kubelet reports the
mounted filesystem at `kubelet_volume_stats_capacity_bytes` = 7,635,075,072 (7.1 GiB), used
0.89–1.00 since 2026-10-06T12:00Z: the claim object carries 20Gi and the volume underneath it does
not. That is the shem-zfs provisioner's resize not reaching the dataset's quota (or a volume moved
under the claim name at the old size) — operator-owned; nothing the repo declares can grow a
filesystem the storage class does not.

Consequence on the roll: adam's conductor pod was restarted by the pin annotation
(`conductor-c916eddbed02`); the new instance (uid 1540233e) fails within 2 s on every start —
`Failed to spawn Lair keystore in process err={"error":"Other"}` (builder.rs:167), holochain exit
101 — 9 restarts by 09:00Z (`last_terminated_reason=Error`, not OOM); one attempt at 09:03:52Z got
the keystore up and the conductor ready, so the loop is intermittent, consistent with ENOSPC on the
keystore directory. The old instance had been logging SQLite 778 `disk I/O error` on peer-meta
writes and WAL maintenance since at least 07:30Z (restarts 0 — it limped; the restart is what
exposed the full disk to lair). The rollout timed out, `remaining conductors HELD`, roll gate
step 6 HALTED, matthew's conductor kept its running process. Storage rolled on all seven peers.

Operator move (unchanged from above, now urgent on adam): give `holochain-data-elohim-adam-alpha-0`
(and eve's) a filesystem that is actually 20Gi — resize the ZFS dataset's quota behind the PV, or
move the data to a new volume under the claim name as doorway-B's key volume was moved on
2026-09-30 — then re-run the edge deploy (or set `CONDUCTOR_ROLL_CONTINUE_ON_FAILURE=1` once for
matthew). Probe: `kubelet_volume_stats_capacity_bytes{persistentvolumeclaim="holochain-data-elohim-adam-alpha-0"}`
≈ 21.5e9 and adam's conductor restarts flat for an hour.

## DELTA 2026-10-08 (the quota DID reach 20Gi; snapshots consumed it again — the 10-07 reading "resize not reaching the dataset" was wrong)

Prometheus `kubelet_volume_stats_capacity_bytes` (kubelet reports a ZFS dataset's capacity as used + available, and
under `quota` the snapshots decide what is available) for the four shem conductor datasets, 2026-10-01 → 10-08:

| dataset | 10-01 09:00Z | 10-01 21:00Z | 10-03 | 10-05 | 10-07 | 10-08 03:00Z | live used now |
|---|---|---|---|---|---|---|---|
| adam | 6.04 GiB | **16.5** | 12.6 | 10.4 | 7.4 | **6.32** | 6.32 |
| eve | 5.53 | **16.3** | 13.9 | 12.0 | 8.5 | **6.38** | 6.38 |
| gertrude | 6.86 | **17.2** | 15.7 | 13.6 | 9.4 | 7.29 | 6.12 |
| susan | 10.8 | **17.0** | 15.7 | 15.0 | 14.0 | 12.6 | 5.66 |

The step to ~16.5 GiB at 10-01 21:00Z is the operator's quota raise and snapshot-retention cut landing; the decline
since is ~1.4 GiB/day on every dataset in lockstep while live `used` stays flat at ~6 GiB. So the 20Gi quota is real
and the space is going to snapshots again (`quota` counts them; sanoid hourly 36 → 6 did not hold the churn — the
daily/weekly tiers and syncoid replication history keep the blocks). Adam and eve reached `available = 0` on ~10-06
and have sat there since (adam at 1.00 in 36 of the last 37 hourly samples). The Che workspace dataset on shem
(`storage-workspace0d7b60ba2d3247a9`, 220Gi) is on the same slope: capacity 220 → 50.6 GiB over the week, 82 % used.
`tank/k8s` itself has 3,169 GiB free — the pool is not full, each dataset's quota is.

What it does now (03:00Z): adam's storage logged 839 conductor-call timeouts in 6 h (every other peer 0); adam's
conductor writes ~4 blocks/s against jessica/james/matthew's 1,100–3,000/s; eve's conductor restarted 5× in 24 h
(`last_terminated_reason=Error`). elohim.host reads adam (alpha-b `STORAGE_URL`), so every elohim.host reading of
dataplane convergence — `deadRemainingStuck`, the FCT rows stuck `private` (lamad habit DELTA 2026-10-08) — is
downstream of this disk until it is cured; read doorway-alpha (matthew, ethosengine) for the substrate's own state.

Operator move (shem; the datasets behind `holochain-data-elohim-{adam,eve,gertrude,susan}-alpha-0` and the Che
workspace claim): `zfs set refquota=20G quota=none <dataset>` so snapshots stop counting against the conductor's
writable space (snapshot space then bounds against the pool's 3 TiB), or take `tank/k8s` out of sanoid autosnap and
let syncoid keep its own. Then recycle adam's and eve's conductor pods so lair and SQLite see the space.
Probe: `kubelet_volume_stats_capacity_bytes` for adam/eve ≈ 21.5e9 and flat over a day; zero `code: 778` lines.

Repo (landed with this DELTA): `scripts/ci/grow-conductor-pvc.sh` no longer says `ok` on a claim whose request
meets the declared size while the filesystem under it is full — it reads `df` through the pod that mounts the claim
and prints `CONDUCTOR-VOLUME-FULL` naming the snapshot-quota mechanism and the move above (warn-only; the roll goes
on). Regression: `scripts/ci/grow-conductor-pvc.test.sh`.

SEEN 2026-10-08 10:40Z (edge #1578, the first roll with the reading in place): eve's conductor container was
CrashLoopBackOff on the full volume, so the step could not exec `df` into it and printed only "answered nothing";
the rollout then timed out at 600 s and adam's and matthew's conductors were HELD behind it. The step now names
the exec error and says a container that is not running cannot be asked — read the claim's kubelet capacity series
instead. The reading this incident needs will come from adam's or eve's conductor once it runs again, which is after
the `refquota` move above. Until then the "0 free" fact lives only in Prometheus and this file.

