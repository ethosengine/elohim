---
id: "backlog-app-port-4445-auth-timeout-fleet-wide"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Connection to Holochain app port 4445 timed out while awaiting auth — fleet-wide background condition, amplified on shem, unfiled until now"
slug: "app-port-4445-auth-timeout-fleet-wide"
written: "2026-07-29"
author: "resilience-cards-converge sprint (k8s-operator diagnostic)"
status: "open"
priority: "medium"
tags: [conductor, app-websocket, auth, alpha, observability]
cites: []
---

# App-WS auth timeouts are fleet-wide, not a shem fault

Found 2026-07-29 during the shem hairpin recovery, while chasing what
looked like a susan-specific blocker: `Connection to Holochain app port
4445 timed out while awaiting auth` fires on EVERY alpha conductor.
15-min-window counts post-restart: adam 104, susan 92, eve 84,
gertrude 62 — but also james 8, jessica 7, matthew 5. The main-side
comparison is what exonerated shem: this is a pre-existing background
condition, merely amplified on shem-placed pods, and it did NOT stop
susan from registering.

Untriaged questions: who is the client that connects and never completes
auth (doorway hosted-pool? storage signal_stream reconnects? health
probes?), why the shem amplification (~10x), and whether each timeout
burns a conductor worker slot while it waits.

Status: OPEN (investigate). Not blocking; filed so the next person who
sees it on one pod doesn't chase it as a node-specific fault (that cost
an hour on susan tonight).
