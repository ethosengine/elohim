---
id: "backlog-prerequisite-gate-fails-open-on-graph-init-failure"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Access-control hardening: the prerequisite gate fails OPEN if the graph subsystem fails to init (fail-mode should be a deliberate policy)"
slug: "prerequisite-gate-fails-open-on-graph-init-failure"
written: "2026-06-20"
author: "/code-review of the prerequisite-gate rebuild (commit e413523ff); the one real finding, surfaced + deferred (forward hardening, not a regression)"
status: "open"
priority: "medium"
tags: [access-control, prerequisite-gate, graph-engine, fail-open, epr, security-hardening]
cites:
  - elohim/elohim-storage/src/epr_service.rs
  - elohim/elohim-storage/src/main.rs
  - elohim/elohim-storage/src/p2p/mod.rs
  - elohim/elohim-storage/src/http.rs
---

# Prerequisite gate fails OPEN on graph-init failure

The rebuilt prerequisite-mastery access gate (commit `e413523ff`) enforces via `PREREQUISITE`
content-graph edges queried through a `GraphEngine`. The gate only fires when `graph_engine = Some`.
In `main.rs`, `GraphEngine::open` + `apply_core_schema` failures are **non-fatal** (`warn → None`).

**Consequence:** on a node whose content DB is healthy and serving bodies but whose `graph.db` fails
to open/apply-schema at boot (sled open under disk pressure / stale lock / corruption — realistic in
this 85%-watermark/OOM deployment), `graph_engine = None` → `check_prerequisite_mastery` is never
invoked on ANY transport (HTTP, libp2p, iroh) → content with prerequisites is served to agents who
have not mastered them. Surfaced only as a `warn` log.

**NOT a regression** (the pre-rebuild gate was dead — zero writers of prerequisite-mastery
attestations — so it always allowed; the new gate is strictly more enforcing). This is a **forward
fail-mode hardening** concern for a NEW access-control subsystem.

## The decision (operator policy: availability vs enforcement)
On graph-init failure in a `graph-native` (prod-default) build, the gate's fail-mode should be
DELIBERATE, not a silent default. Options:
- **Fail-open + loud** (cheapest): keep serving, but raise `warn → error` + a metric/alert so the
  disabled-enforcement state is visible, not silent. Preserves availability.
- **Fail-closed gate**: deny content that *might* have prereqs when the graph is absent — safer for
  access control but can over-block legitimate reads (can't tell which content has prereqs without
  the graph).
- **Refuse startup**: if access control depends on the graph subsystem, don't serve without it.
  Safest enforcement, but a graph-init failure becomes an availability outage (risky under disk
  pressure — the very condition that triggers it).

Recommended first step regardless of policy: make the disabled state OBSERVABLE (error-level log +
a gauge `elohim_prereq_gate_graph_absent`) so it's alertable, then let the operator pick the
fail-mode. The happy-path gate is correct + consistent across all three transports (verified by
the /code-review); this is robustness, not correctness.
