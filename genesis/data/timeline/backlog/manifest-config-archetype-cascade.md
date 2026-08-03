---
id: "backlog-manifest-config-archetype-cascade"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Deploy manifests should derive configuration from device archetypes cascading into declarative surfaces — not three hand-kept homes per value"
slug: "manifest-config-archetype-cascade"
written: "2026-08-03"
author: "adopt-before-author flip session (operator directive)"
status: "backlog"
priority: "medium"
tags: [deploy, manifests, orchestrator, device-archetype, config-cascade, edge, k8s-models-not-architecture]
cites:
  - elohim/holochain/Jenkinsfile
  - genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml
  - genesis/orchestrator/manifests/humans/adam-firstman.yaml
  - genesis/orchestrator/data/deployments.json
---

# Manifest config should cascade from device archetypes, not live in three hand-kept homes

## Operator directive (2026-08-03, verbatim intent)

"The manifests need consistency — they should be getting their configurations
from device archetypes and those values cascading down into their declarative
surfaces." Context frame: "we shouldn't ever be relying on k8s for anything but
to model the network and infra, implied by real world peers" — k8s models what a
real-world peer's operator would have configured; genesis declares it; the
pipeline reconciles it.

## The drift, made concrete by the ELOHIM_ADOPT_BEFORE_AUTHOR flip

Landing ONE per-env boolean today required **three hand-synchronized edits**:

1. `_edgenode-consolidated.template.yaml` — env var + `ADOPT_BEFORE_AUTHOR_PLACEHOLDER`
2. `elohim/holochain/Jenkinsfile` `deployHumanManifest` — a Groovy ternary
   (`env == 'prod' ? 'false' : 'true'`) + a sed substitution line
3. `adam-firstman.yaml` — a literal value, because adam is the one hand-rendered
   manifest (`manifest:` field, no placeholders)

Same triple-home shape already exists for `ALLOW_DNA_REINSTALL` and
`ALLOW_SEED_SHARD_MANIFEST`; per-node tunables (`edgenodeDbPoolSize`,
`edgenodeArcFactor`, memory/cpu) live as a FOURTH pattern (deployments.json
per-human fields with Jenkinsfile fallback defaults). Every new knob forks
four ways, and adam silently diverges from the fleet unless someone remembers
his hand-rendered copy (the genesis-pair partition risk makes that divergence
expensive, not cosmetic).

## Target shape

Config values derive from **device archetypes** (deployments.json already
carries `deviceArchetype` per human) and cascade:

archetype defaults → env overlay (alpha/staging/prod) → per-human override
→ rendered declarative surface (the manifest the pipeline applies).

- One declared home per value (archetype/env data, not Groovy ternaries).
- The Jenkinsfile render step becomes a dumb projector of the cascade — no
  policy encoded in build code.
- adam migrates off the hand-rendered manifest onto the same cascade (the
  "historical consolidated reference" role moves to docs, not to a divergent
  runtime surface).
- Aligns with the seam-map disambiguator: what you ADD is a *manifest* → SDK
  seam, compose inward. The operator-tunable surface is data, not pipeline code.

## Boundary honored

This is the k8s-modeling layer only (`feedback_k8s_is_not_the_architecture`):
node-behavior semantics (e.g. adopt-before-author itself) stay protocol-side
(per-call zome param; future policy entries via p2p-design-gate). The cascade
governs how the *infra model* renders — never a second home for protocol policy.
