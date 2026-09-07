---
id: "backlog-clone-content-escapes-via-shared-projection"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Clone-cell isolation holds only in the authoring peer's DHT — content written to a lamad clone lands in the peer's shared storage projection with no cell qualifier, crosses the storage sync plane, and a RECEIVING peer's storage re-authors it into its BASE lamad cell (fresh DHT action) — measured on the 3-peer mesh 2026-09-07; this is the D6 gate for any group/fixtures clone"
slug: "clone-content-escapes-via-shared-projection"
written: "2026-09-07"
author: "overnight shift 2026-09-07 (fixtures-clone harness, slice 1b)"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:dataplane-convergence"
  - "habit:reach-enforced-everywhere"
tags: [clone-cell, isolation, projection, sync, group-spaces, D6, slice-1b]
---

**Evidence (receipt `genesis/a2o/reports/fixtures-clone/mesh-20260907T1000Z/`, report `genesis/a2o/fixtures-clone-2026-09-06-report.md` addenda, commits e9b64ca42 + e36ecc578):** test bundle = today's workdir happ with `lamad.clone_limit 0→15`; 6 clones per peer; CapGrants before baseline; two baselines 20 s apart byte-identical (quiescent). After one probe write into a clone: base-cell **source chain, ops, zome content, targets: EMPTY delta** (DHT isolation holds); storage **`content` projection: +2 rows** (`h_app_id: "lamad"`, no cell/DNA qualifier, `dht_anchor_hash` = the clone's action); **Automerge sync: +2 docs**. Eight minutes later jessica's DHT (base and her own clone) held neither probe action hash, but her projection held `fxclone-beta-20260907` under a DIFFERENT anchor that sits in her BASE lamad chain → her storage re-authored clone-originated content into her base cell; james carries the row with jessica's anchor, `dht_anchor_state: NULL`. Partial and ordering-dependent (only one of two probes completed in the window).

**Reading:** the storage projection is keyed by role (`h_app_id`), not by cell, and the sync plane + back-authoring path treat every projected row as base-cell content. Plan D1/D6: "state whether the experiment uses a separate storage instance or context-qualified shared storage — a cell selector alone does not isolate the downstream projection" — measured: it does not. **No group clone (household, collective, fixtures) can ship before the projection and sync plane are cell-qualified** (DNA hash or cell id on every projected row and sync doc, and the re-author path refusing to move content across cells).

**Also measured (gossip, 3 peers):** marginal per clone RSS +1.62 MiB, disk +2.11 MiB (~8× the idle figure), FDs +21, threads +10; clone create+enable median 148 ms.

**Done when:** the identity diff on a receiving peer's base cell is EMPTY across all six planes after a clone write, with the projection/sync rows carrying a cell qualifier; the harness (`genesis/a2o/scripts/fixtures-clone.mjs`) is the check.
