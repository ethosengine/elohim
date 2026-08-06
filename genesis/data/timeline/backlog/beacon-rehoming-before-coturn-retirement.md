---
id: "backlog-beacon-rehoming-before-coturn-retirement"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Re-home the two premise DNS beacons before coturn retirement"
slug: "beacon-rehoming-before-coturn-retirement"
written: "2026-08-06"
author: "codex"
status: "wip"
priority: "high"
relatedNodeIds: []
tags: [iroh, wave-2, beacon, dns, coturn, retirement]
shift_objective: |
  Prepare standalone, premise-pinned Cloudflare beacon Deployments so coturn can
  later retire without making the elohim.host zone stale.
---

# Beacon re-homing before coturn retirement

Claimed by Codex on 2026-08-06 from relay-sovereignty design §6.2 and operator
checklist item 9.

## Claim fence

- `genesis/orchestrator/manifests/infra/alpha-relay-addr-beacons.yaml`
- this backlog claim

The coturn manifests and the pipeline's explicit infra apply list remain outside
the claim until the operator ratifies the standalone option. Live reconciliation
is operator-owned.

## Prepared result and remaining gate

The ready-to-apply manifest contains two separate one-replica Deployments because
the premises have different node pins, exclusive records, and shared-record owner
slugs. Each keeps host-network premise-local egress and the current Cloudflare
secret/resource contract while dropping every coturn-only coupling: no coturn
sink, init container, process-namespace sharing, config mounts, or `CAP_KILL`.

After ratification, wire/apply the new file while the legacy sidecars still run.
For each named standalone pod, deliberately drift its exclusive Cloudflare A
record and observe that pod log `exclusive record DRIFTED` and restore the record
within 30 seconds. Only that attributed proof permits removing the beacon from
the coturn pod. Coturn itself remains until no tx5 conductor exists.
