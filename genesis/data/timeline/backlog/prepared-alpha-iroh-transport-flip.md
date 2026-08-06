---
id: "backlog-prepared-alpha-iroh-transport-flip"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Prepare the alpha namespace-atomic iroh image repoint and render gate"
slug: "prepared-alpha-iroh-transport-flip"
written: "2026-08-06"
author: "codex"
status: "wip"
priority: "high"
relatedNodeIds: []
tags: [iroh, wave-2, alpha, conductor, deployment, flip-day]
shift_objective: |
  Make Stage 1 a ratification decision by preparing the exact alpha-only image
  repoint and enabling the iroh conductor-config gate without changing staging
  or production.
---

# Prepared alpha iroh transport flip

Claimed by Codex on 2026-08-06 from the Wave-2 relay-sovereignty design Stage 1
and operator checklist items 7-8.

## Claim fence

- `elohim/holochain/Jenkinsfile` (`deployHumanManifest` only)
- `genesis/orchestrator/manifests/humans/adam-firstman.yaml`
- `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml`
- this backlog claim and flip-day notes

Doorway service behavior, conductor/fork sources, storage behavior, and live
cluster actions are outside this claim.

## Prepared result

The two active human StatefulSet sources now carry one full-image placeholder.
The render function resolves it to
`elohim-storage-iroh:hc-elohim-0.6.3-iroh` only for alpha and preserves
`elohim-storage:${STORAGE_TAG}` for staging/prod. The same alpha predicate sets
`IROH_TARGET=1` around `validate-conductor-config.sh`, so Stage 1 cannot render a
missing, public-n0, or wrong-primary-doorway relay URL. The diff is deliberately
uncommitted pending operator ratification.

## Flip-day sequence

1. Ratify the namespace-atomic window and retention of both tx5 and iroh image
   tags for the rollback wave (operator checklist 7-8). Confirm every live alpha
   human is present in the render set; a down peer is acceptable only if its
   StatefulSet spec will receive the same iroh image before it returns.
2. Stage the two human sources, this claim, and the prepared Jenkinsfile. The
   same push must include the ratio-dashboard manifest and its claim unless they
   landed earlier, because alpha's explicit infra list now references it. Keep
   the beacon and media-retirement preparations independently reviewable.
3. Push once. The alpha render must show all seven human StatefulSets using the
   exact iroh image, and every alpha validation invocation must print the
   `IROH-TARGET relay_url ... matches primaryDoorway` success. Any tx5 image or
   default-off validation on an alpha human aborts the window.
4. Let the one alpha deploy window finish across the whole namespace. Do not
   intentionally canary one live-DHT peer: tx5 and iroh peer URLs cannot dial
   each other, so a partial rollout is a transport partition.
5. Require the relay-reachability, peer-store, n0-contamination,
   no-lingering-tx5, bootstrap-sharing, and canonical-head probes to name the
   post-flip state. A failed/partial rollout rolls back by reverting the image
   repoint to the retained tx5 line for the same whole namespace.
6. Start the Stage-2 soak only after the fleet is wholly iroh. Set Grafana to the
   exact soak window; read the direct percentage only beside its per-pod sample
   count. The event-weighted `direct=true|false` result decides whether Phase-B
   QAD is warranted.

The live push, rollout, and soak evidence remain open; this entry stays `wip`.
