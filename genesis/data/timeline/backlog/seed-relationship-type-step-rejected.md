---
id: "backlog-seed-relationship-type-step-rejected"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "`just seed apply mesh content` drops the whole relationship graph again: the seeder emits relationship_type 'step' and storage refuses it (HTTP 400 'step' is not valid) — the EXTENDS fix (6e4fa4389) covered one vocabulary drift, not the class"
slug: "seed-relationship-type-step-rejected"
written: "2026-08-29"
author: "M4 corpus load"
status: "open"
priority: "medium"
jobs: [elohim-genesis, elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
tags: [seed, relationships, manifest-vocabulary, local-mesh]
---

Measured 2026-08-29 03:48Z on the household mesh: `Loaded 3433 concepts (28 failed)`, then
`Step relationships bulk create failed: HTTP 400 relationship_type 'step' is not valid`, then the
post-flight sample check `None of the 5 sample entries found` (the prologue post-flight quirk, still
unexplained — content IS present: `contentCount 3454`). Cure shape: the seeder and
`relationship_service::VALID_RELATIONSHIP_TYPES` must read ONE manifest vocabulary
(`elohim/sdk/domains/lamad/manifest.json` relationships) — the 6e4fa4389 test asserts the manifest ids are
accepted; add the seeder's emitted ids to the same assertion so the next drift fails in the gate, not on
the mesh.
