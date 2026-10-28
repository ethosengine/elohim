---
id: "backlog-self-heal-storage-disk-quota-crashloop"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A storage peer whose volume hits its disk quota crashloops on its first write instead of degrading"
slug: "self-heal-storage-disk-quota-crashloop"
written: "2026-10-28"
author: "claude (overnight integration shift, session 3f9afd7a, steward human:matthew)"
status: "open"
priority: "high"
ci_status: none
jobs: [elohim-edge]
tags: [self-heal, storage, disk, sled, rollout-evidence, operator]
---

# Storage crashloops when its volume is out of quota

## What happened

`elohim-susan-alpha-0`'s `elohim-node` container (elohim-storage, node `shem`) hit its disk quota
while running. Loki, 2026-10-27T17:38:24Z, `sled::pagecache::logger` (sled 0.34.7):
`hit error while writing iobuf with lsn 187416777: Io(Os { code: 122, kind: QuotaExceeded })`.
Every restart since exits 1 on its first write:
`Error: Refusal { kind: "io", detail: "Disk quota exceeded (os error 122)" }`
(restarts about 5 min apart, restart count at least 27 for pod UID aa9ab99d by 2026-10-28T00:32Z).
elohim-edge dev #1741 counted it as a rollout failure (2/7, with adam). No storage source change
caused it: `git log 9d1f0e8cd..be54c97d5 -- elohim/elohim-storage/src` is empty.

## Three parts

1. **Operator, now:** free or enlarge susan's volume (PVC or shem node allocation). Repo cannot
   reconcile a filled PVC; that is cluster state.
2. **Storage self-heal:** a full volume should hold the peer in a read-serving, write-refusing state
   with an exported `disk-quota` exhaustion signal, rather than exiting at start. Today the process
   dies before /health answers, so the runtime-harvest poller never sees an exhaustion report.
3. **Rollout evidence:** `jenkins-deployer` lacks `pods/log` (and `nodes` list), so
   `rollout-evidence/*/--elohim-node--previous.log` and `node-pressure.txt` carry only a Forbidden
   line. The evidence collector should either be granted those reads in the repo's RBAC manifest or
   name Loki as the log source in its summary.

## Done when

Susan's storage is Ready again after the operator's volume action. A storage peer started on a full
volume stays up, serves reads, refuses writes with a named reason and reports the exhaustion. The
next edge rollout's evidence for a crashlooping container includes its last log lines.
