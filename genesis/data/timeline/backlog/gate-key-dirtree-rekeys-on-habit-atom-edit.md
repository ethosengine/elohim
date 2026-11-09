---
id: "backlog-gate-key-dirtree-rekeys-on-habit-atom-edit"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A native gate's closure key takes the project directory's whole tree OID (dirTree), so a docs-only habit-atom edit under <crate>/.epr-meta re-keys the gate and a merge of dev re-runs elohim-storage's 70-90 min gate on unchanged source"
slug: "gate-key-dirtree-rekeys-on-habit-atom-edit"
written: "2026-11-08"
author: "artifact-store sprint 2 orchestrator (claude-opus-5-5, shem)"
status: "open"
priority: "high"
jobs: [pre-push]
cluster: "push-gate-pays-once"
tags: [gate, reuse, closure-key, wall-clock, pays-once]
---

**Observed 2026-11-08** landing the artifact-store storage closure: `elohim-storage` was green and
attested at 86e1395e0 (86 min). Merging dev 2c413084f, a docs-only close whose one file under the
crate is `elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`, produced 1b76c2327; its
pre-push printed `elohim-storage evidence: miss … not reused: no attestation for this closure yet`
and re-ran the whole gate (`push-L3.log`).

**Mechanism:** `gate-evidence.mjs` (~:532) sets `dirTree = manifestBlob(root, commit, project.dir)`,
the tree OID of `elohim/elohim-storage`, which includes `.epr-meta/**`. A habit DELTA — the
deliverable CLAUDE.md asks every finished lane to write — therefore invalidates the crate's gate.

**Cure:** key `dirTree` on the project tree minus `.epr-meta/**` (and other declared non-input
governance paths), or let `gate-input-audit.mjs` declare `.epr-meta` a non-read for native gates;
add a test: a commit touching only `<dir>/.epr-meta/*.habit.md` keeps the closure key. Habit
`push-gate-pays-once`.
