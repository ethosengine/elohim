---
id: "backlog-harbor-retention-strips-doorway-sha-tags"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Harbor tag-retention strips elohim-doorway SHA tags after push — deploys chase nonexistent tags (ImagePullBackOff NotFound)"
slug: "harbor-retention-strips-doorway-sha-tags"
written: "2026-07-31"
author: "claude (integrator shift — operator cluster-side diagnosis)"
status: "open"
priority: "high"
jobs: [elohim-edge]
tags: [harbor, registry, tag-retention, doorway, deploy, image-pull, ci]
cites:
  - genesis/orchestrator/manifests/doorway/alpha-b.yaml
  - genesis/data/timeline/backlog/doorway-apex-only-hosted-provisioning-policy.md
---

# Harbor retention policy deletes doorway commit-SHA tags, stranding deploys

Observed 2026-07-31 (edge #1269 wave, dc73c5d0): the doorway image build and
push SUCCEED — the artifact lands in Harbor seconds after doorway-app's — but
the `ethosengine` project's tag-retention policy (retention_id 3) strips the
commit-SHA tags (`<sha>`, `1.0.0-dev-<sha>`) from `elohim-doorway` afterwards,
keeping only `dev-latest`. `doorway-app` tags survive, so the rule scoping is
repo-uneven. Every doorway pod then sits 2/3 with ImagePullBackOff — the
`elohim-doorway:1.0.0-dev-<sha>` reference 404s (NotFound, not auth) — while
sibling containers pull fine.

Broken for ~21h before diagnosis: the last surviving SHA-tagged doorway image
was b495d81e (07-30 05:10). Consequences that masqueraded as other failure
classes:

- `kubectl rollout status` reports `0 out of 1 new replicas updated` /
  `1 old replicas are pending termination` → read (wrongly) as a stuck-pod /
  node condition. Zero pods were terminating.
- Doorway A kept serving 200 from an old-RS pod on the day-old image; B had
  two ReplicaSets both chasing stripped tags → no ready endpoint → sustained
  503 on elohim.host that looked like the (separate) conductor-churn story.

## Triage rule (why this entry exists)

"Deploy timed out but the image build stage was green" → check Harbor for the
exact tag the rendered manifest references BEFORE any cluster theory:
`GET /api/v2.0/projects/ethosengine/repositories/elohim-doorway/artifacts?q=tags%3D<tag>`.
Build-green + tag-missing + dev-latest-present = retention stripping, not CI.

## Fix path (operator, in order)

1. Fix retention rule: retain `1.0.0-dev-*`/SHA tags for `elohim-doorway`
   (mirror whatever keeps doorway-app's tags alive). Fix BEFORE re-tagging or
   the next retention execution strips it again.
2. Re-tag the existing digest (no rebuild needed) via Harbor tag-create API /
   `crane tag` — manifests then resolve as-is and deployments reconcile.
3. Check untagged doorway artifacts aren't GC'd out from under step 2.

## Open question

Why the rule catches `elohim-doorway` but not `doorway-app` — scoping audit of
retention_id 3 pending (needs Harbor admin creds).
