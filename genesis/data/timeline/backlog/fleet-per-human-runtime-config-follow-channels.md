---
id: "backlog-fleet-per-human-runtime-config-follow-channels"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Fleet cannot express a canary: the per-human runtime-config ConfigMap is rendered from one empty template with no per-human override — following a release channel (observe/canary/apply) needs a per-human knob"
slug: "fleet-per-human-runtime-config-follow-channels"
written: "2026-09-02"
author: "shift-2026-09-02T02-20-land-rung5-batch"
status: "in-tree"
priority: "high"
jobs: [elohim-orchestrator, elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "backlog-task-runtime-upgrade-a2o-receipt"
tags: [upgrade-propagation, rung4, rung5, manifests, canary, delegable]
---

## Measured (2026-09-02T02:45Z)

`genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml` renders every
human's `<prefix>-runtime-config` ConfigMap from the same comment-only body; `adam-firstman.yaml`
is hand-written but also empty. Nothing in `genesis/orchestrator/scripts/` or
`deployments.json` feeds a per-human value into that ConfigMap. So after the storage roll that
delivers the adoption controller, the fleet can only follow a channel uniformly (edit the
template → every peer gets the same `ELOHIM_RELEASE_CHANNELS` line) — there is no way to say
"james = canary, matthew/jessica = apply, shem hosts = observe", which is exactly rung 5's
first fleet ceremony (canary on one household peer before promotion).

## Fix (bounded; no runtime change)

1. `deployments.json` per-human field, e.g. `runtimeConfig: { ELOHIM_RELEASE_CHANNELS: "runtime:coordinators:elohim:<channel>=canary" }`.
2. deploy-render substitutes a `RUNTIME_CONFIG_BODY_PLACEHOLDER` in the template's ConfigMap
   from that field (TOML `KEY = "value"` lines; absent → the current comment-only body).
3. `scope-reconcile`/render tests: a human with the field renders the line; one without renders
   byte-identical to today.

The rung-4 watcher already mounts the ConfigMap (`ELOHIM_RUNTIME_CONFIG_PATH`), so the follow
lands on RUNNING pods with no restart once rendered (mesh receipt: 35 s to 3/3 on the first follow).

## DoD

Render for james shows the canary line; a `kubectl`-free read via Prometheus
(`elohim_release_adoption_decisions_total` on james's pod gains the channel's series) or the
node-local `/admin/adoption` (operator) shows `mode: canary`.

## Landed in-tree (2026-09-02)

1. `deployments.json` per-human `runtimeConfig` (map KEY -> string), documented under
   `$runtimeConfigComment`; NO value set — the channel id is the ceremony's to mint.
2. `RUNTIME_CONFIG_BODY_PLACEHOLDER` line inside `runtime-config.toml: |` in both the
   consolidated template and `adam-firstman.yaml`; `elohim/holochain/Jenkinsfile`
   `runtimeConfigSedExpr(humanConfig)` (top-level def, pipeline{} block unchanged at 62328 B)
   emits either `/RUNTIME_CONFIG_BODY_PLACEHOLDER/d` (absent/empty → byte-identical to today) or
   one `s|…|K = "v"\n    K2 = "v2"|` substitution; keys env-var shaped, values refuse `| & \ ' "`
   and newlines, a bad entry fails the render loudly.
3. `genesis/orchestrator/runtime-config-render.test.mjs` (in `pnpm test`, suite 107/107): static
   pin of both arms + the sed-list call; the real template and adam manifest rendered with the
   real `sed`: absent → only the placeholder line gone, no blank line; present → each key an
   indented TOML line and everything else untouched; every declared `runtimeConfig` renderable.

Fleet-unproven until a human carries the field and the next edge deploy renders it; the DoD's
Prometheus / `/admin/adoption` read is the integrator's post-roll watch.
