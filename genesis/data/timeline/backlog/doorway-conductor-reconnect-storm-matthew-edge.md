---
id: "backlog-doorway-conductor-reconnect-storm-matthew-edge"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Matthew-edge doorway still crashloops AFTER the getaddrinfo cure — driven by the conductor closing app-ws sessions ~40/min (reconnect storm) → warm_stream re-replay → runtime wedge → watchdog SIGKILL. Doorway-internal root is FIXED (adam edge 0 restarts proves it); residual is upstream conductor instability (operator-owned) + a design-level doorway candidate (suppress warm_stream re-replay on reconnect)."
slug: "doorway-conductor-reconnect-storm-matthew-edge"
written: "2026-06-15"
author: "agentic-developer (shift doorway-crashloop-stabilize-then-seeder-shakeout)"
status: "backlog"
priority: "high"
ci_status: blocked
jobs: [elohim-edge]
relatedNodeIds: []
nodes: [matthew, doorway-alpha]
tags: [doorway, alpha, conductor, reconnect-storm, app-ws-session-close, warm-stream, liveness-watchdog, restart-on-hang, upstream-operator-owned, needs-brainstorm, sqlite-read-pool-saturation, matthew-edge, ab-control-adam-healthy]
cites:
  - genesis/orchestrator/manifests/doorway/alpha.yaml
  - doorway/doorway-service/src/projection/subscriber.rs
  - doorway/doorway-service/src/projection/warm_stream.rs
  - genesis/a2o/features/doorway/peer-conductor-connection-resilience.feature
---

# Matthew-edge doorway crashloop residual — conductor reconnect storm, not the (fixed) getaddrinfo root

## What's fixed (do not re-open)

The 2026-06-15 doorway-crashloop root cure shipped and is **confirmed live** on image
`elohim-doorway:1.0.0-dev-4fd6c1c7` (commits be313cfc4 band-aids, f236416db off-pool-DNS
root cure, 19fb41974 watchdog; a2o 4fd6c1c75):

- **A/B control proves it:** `elohim-doorway-alpha-b` (adam / elohim.host apex, stable
  conductor) = **0 restarts, Ready** on the new image. The blocking-`getaddrinfo`
  worker-park is gone (conductor connects resolve async/off-pool, succeed/fail fast).
- The dedicated heartbeat-gated liveness watchdog (`DOORWAY_HEALTH_PORT=8079`) binds and
  works (restart-on-hang preserved; no longer the silent-150s-freeze).

## The residual (this item)

The **matthew-side** doorway (`elohim-doorway-alpha-665bc6954d-*`, backs
`alpha.elohim.host`) still crashloops on the new image: **7 restarts in ~30 min**, Ready
oscillating 1↔0, `alpha.elohim.host` + `doorway-alpha/health` = 503.

**Pinned mechanism:** the matthew **conductor closes the doorway's app-ws session
~40×/min** (`Conductor closed connection: None` — 476 occurrences / 12 min). Each
(re)connect re-triggers `warm_stream`'s whole-corpus replay (`subscriber.rs`,
single-flight-guarded but re-fires after each completion), so under a 40/min storm the
projection replay runs continuously → the main runtime periodically wedges >15s → the
watchdog correctly SIGKILLs → restart → the new pod hits the same storm. The watchdog is
working *as designed*; against a **persistent upstream** cause, fast-restart just churns.

**Why matthew and not adam:** matthew's conductor is resource-saturated — SQLite read
connection pool 500% oversubscribed (`Database read connection is saturated. Util 500.00%`,
~1.58M/13h) + cpu-limited (1 core) per the 2026-06-15 RCA workflow — a stressed conductor
that drops sessions under load. adam's conductor is stable → no storm → 0 restarts.

## Two work items (operator decision required)

1. **Upstream / operator-owned — stop the conductor dropping sessions (root).** The
   matthew (`elohim-matthew-alpha-0`) conductor's session-closing is driven by resource
   saturation. Levers (operator/cluster, NOT a repo doorway change): raise the conductor
   cpu limit (currently 1000m while pulling ~3 cores), size the holochain SQLite read
   pool, and/or lower the arc factor (per `project_per_node_memory_is_conductor_authority_arc`).
   Until the conductor stops closing sessions, the matthew doorway will keep wedging.

2. **Doorway design candidate — `/brainstorm` (NOT a blind shift edit).** Make the doorway
   resilient to a session-close storm: suppress `warm_stream` re-replay when the corpus was
   replayed within the last N seconds (a re-trigger cooldown), so a 40/min reconnect storm
   can't drive a continuous corpus-replay firehose. Design questions: correctness of
   suppressing re-replay, cooldown sizing vs projection freshness, interaction with the
   single-flight guard. This is above an agentic-shift's grind-ceiling → route to
   `/brainstorm` before implementing.

## Operational note

`elohim.host` (apex / adam edge) is HEALTHY. Only `alpha.elohim.host` (matthew edge)
crashloops. If alpha availability is urgent before the conductor fix lands, the operator
could route alpha traffic to the healthy edge as a stopgap (operator-owned).

## Provenance

Surfaced by the overnight shift `2026-06-15T03-46-doorway-crashloop-stabilize-then-seeder-shakeout`
(Phase 1 gate: deploy did not stabilize the matthew edge → Phase 2 seeder shakeout NOT
started, held per the hard gate). Evidence: Prometheus restart/ready trends + Loki
session-close counts, 2026-06-15 ~04:35Z.
