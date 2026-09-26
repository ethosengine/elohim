---
name: feedback_measure_class_runs_off_dev_berth
title: Measure-class runs go to a peer, never the dev berth
description: "Operator 2026-09-25: long measured runs (A/B windows, soaks, proofs on grown stores) never run on the workspace a human develops in — delegate to a measurement peer; dev berth runs bounded verify lanes only."
metadata:
  node_type: memory
  title: Measure-class runs go to a peer, never the dev berth
  type: feedback
  originSessionId: 9adf9f01-3f4d-4b27-8758-d78fda7cada3
  modified: 2026-09-25T13:09:00.927Z
---

Operator, 2026-09-25, after the K0 conductor A/B and the household proof held this workspace's mesh
lease ~16 h across two container restarts: "this LONG measured run… is exactly the kind of thing that
holds up development" — delegate it to another peer.

**Why:** a measurement that blocks the dev workspace is measurement-by-blocking, the same anti-pattern
the native-delivery plan priced in the App pipeline (7200 s readiness wait). Attention does not compose:
a human's development berth waiting on a measure-class run is the operator-surfaced pain the algedonic
persona exists to prevent.

**How to apply:** classify before dispatching. `measure`/`profile`-class work (window/A-B measurements,
soaks, proofs on grown stores, anything > 30 min of mesh or cargo lease) goes to a measurement peer — a
second stewarded device or devworkspace on the T3 hybrid rung — or is QUEUED until one exists; it never
holds the dev berth. The dev workspace runs `verify`-class lanes bounded to their declared budget,
refusing fast past it. If a subagent starts looping restarts/sleeps to "wait for convergence", stop it
and record the window instead. Ruling lives in the plan's wave-2 rulings log
([[project_native_delivery_sprint_2026_09_24]]); seam: [[project_lvi_devspace_peer_runtime]],
[[feedback_local_mesh_first_cadence]], [[feedback_limit_raises_are_design_signals_not_capacity]].
