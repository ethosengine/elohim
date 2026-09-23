---
id: "backlog-runtime-config-fleet-mount-subpath-never-propagates"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Fleet runtime-config mount is subPath + readOnly — ConfigMap edits never reach a running pod, and the config-epr vehicle cannot write it"
slug: "runtime-config-fleet-mount-subpath-never-propagates"
written: "2026-09-23"
author: "ci-wallclock design pass (runtime-artifacts spec §12.7)"
status: "backlog"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
tags: [rung-4, runtime-config, config-epr, fleet, k8s-scaffold, upgrade-propagation]
---

`genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml` (~l.533-542) mounts
`<prefix>-runtime-config` at `/etc/elohim/runtime-config.toml` with `subPath: runtime-config.toml`
and `readOnly: true`. The comment above it says kubelet propagates ConfigMap edits to this mount
and that a flag flip is "a ConfigMap edit + apply, never a roll". That is false for a `subPath`
mount: kubelet never updates a `subPath` mount after the container starts. As a result:

1. **A fleet config flip only lands on a pod restart.** The recorded fleet flips (edge #1437
   rendering james `=canary`) arrived together with edge rolls, so "rung 4 without a roll" is
   proven on the mesh (a plain file) but not on the fleet.
2. **Writes from inside the pod are refused.** `readOnly: true` blocks `ConfigEprVehicle` (the
   `config-epr` release class, `release_adoption/apply.rs`) and `POST /admin/runtime-config/follow`
   (`http.rs`) from writing the watched file. Config-class election on the fleet cannot apply.

The same read-only `subPath` shape was already recorded for conductor config
(`genesis/docs/superpowers/specs/2026-06-15-node-resource-tunables…md` ~l.203).

- chain / between "config release elected on channel" → "fleet peer runs the new setting" /
  missing node "the watched file is writable by the node and updated in place": assertion — a
  `config-epr` apply on an alpha pod reloads without a restart; probe — `GET /admin/runtime-config`
  before/after on one pod with the conductor PID unchanged. State: **unverified on the fleet;
  finding from code reading, 2026-09-23.**

Fix shape (bounded):
1. Mount the ConfigMap as a directory (no `subPath`) for the propagation path, and point
   `ELOHIM_RUNTIME_CONFIG_PATH` at a writable node-owned file seeded from it at boot (an
   `emptyDir` or the PVC). The node's own file is then the truth that elected releases write;
   the ConfigMap is only the boot seed.
2. Correct the template comment.
3. The k8s ConfigMap is scaffold for the config class. The elected `config-epr` channel is the
   delivery path; do not grow the ConfigMap into one.
