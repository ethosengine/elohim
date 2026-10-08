---
id: "backlog-alpha-gertrude-conductor-dead-child-ready-pod"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "gertrude's conductor child died at 10:41Z and the pod stayed Ready for hours: both doorways kept routing hosted registrations to the dead slot and answered 503"
slug: "alpha-gertrude-conductor-dead-child-ready-pod"
written: "2026-10-08"
author: "claude-fable-5-1 (session elohim-cb; Loki and Prometheus reads)"
status: "open"
priority: "high"
severity: high
nodes: [alpha]
tags: [conductor, process-manager, readiness, liveness, health-probe, gertrude, doorway, hosted-pool, registration, elohim-alpha, ops, lane-operator, phase-steady]
cites:
  - genesis/data/timeline/backlog/alpha-adam-peer-meta-store-disk-io-error.md
  - doorway/doorway-service/.epr-meta/hosted-human-lifecycle.habit.md
---

# A conductor pod whose child has exited must not answer its probe green

## What happened (alpha, 2026-10-08)

- `elohim-gertrude-alpha-conductor-0` (node shem, container restart count 5) lost its `holochain` child
  around 10:39Z–10:41Z, during edge #1578's roll. The child's last words: `Failed to bind IPv4 listener:
  AddrInUse` (three starts in a row at 10:39:28Z, 10:40:19Z, 10:40:28Z — a respawn landed while the
  previous child still held 4444/4445) and `Pruning peer meta store failed: (code: 14) unable to open
  database file`. After 10:41Z the `elohim-conductor` container logs only the supervisor's own
  memory-attribution ticks (`elohim-storage` pid 1, 155 MB anon); the in-pod `ws-proxy` reports
  `TCP:127.0.0.1:4445: Connection refused` on every connect (still at 14:27Z). Her dataset is NOT full
  (3.3 GiB free), unlike adam's and eve's.
- `kube_pod_container_status_ready` read 1 for the `elohim-conductor` container the whole time: the
  readiness AND liveness probes are `GET /health` on the supervisor's port 8090, and that handler
  deliberately stays 200 whatever the conductor bridge says (it is the STORAGE pod's probe, where a
  dead bridge must not kill a pod that still serves projections). In a conductor pod the child IS the
  pod, so the exemption was wrong there.
- Consequence: the doorway hosted pool kept conductor-4 (gertrude) in rotation. Both doorways logged
  `Failed to connect to conductor: WebSocket protocol error: Handshake not finished` every 30 s (doorway
  A on intel-nuc, doorway B on shem, 14:26Z), and every hosted registration routed to that slot answered
  `503 PROVISIONING_FAILED … Admin WebSocket connect to ws://elohim-gertrude-alpha-conductor-0…:8444
  failed … Handshake not finished`. Genesis #1634 read it as four failed scenarios (portal login,
  "Validator" on doorway alpha, discovery assessment, completion feedback) plus the gertrude seeder legs
  (`Seed Conductor Identities partial: 1 failed`, `Seed Agent Peer Bindings partial: 1 failed`).

## Cure (code, 2026-10-08)

`elohim-storage` `GET /health`: when this process supervises an embedded conductor and `try_wait`
reports the child exited, answer **503 + Retry-After** with `status: "conductor-exited"` and
`conductorChild: "exited"`; a running child, a restart in progress (manager mutex held by the arc
actuator — `try_lock`, never awaited) and an unsupervised conductor stay 200. A storage pod is unchanged.
Decision is the pure `health_status_for(embedded, child)` in `http.rs`, pinned by a table test. With the
existing probes (readiness 6 × 10 s, liveness 10 × 30 s) a dead child now drops out of the Service in about
a minute and the container is restarted within five.

## Operator move (now)

Restart `elohim-gertrude-alpha-conductor-0` (`kubectl delete pod … -n elohim-alpha`) — the fix above only
reaches the fleet with the next edge roll. Until then every hosted registration the pool routes to
conductor-4 fails.

## Still open after the cure

- The respawn-on-top-of-a-live-child (`AddrInUse` three times) is the process manager's own race: a
  restart that does not wait for the old child's sockets to close. Not cured here; the probe now at least
  makes the outcome visible and self-healing.
- The doorway pool keeps routing registrations to a slot whose admin socket refuses handshakes;
  provisioning should fall over to the next conductor on an admin-connect failure instead of answering
  503. Home: `doorway/doorway-service` hosted pool (hosted-human-lifecycle).
