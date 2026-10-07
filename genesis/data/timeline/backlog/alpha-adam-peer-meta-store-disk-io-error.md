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
