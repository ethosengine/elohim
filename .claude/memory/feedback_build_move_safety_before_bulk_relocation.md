---
id: feedback-build-move-safety-before-bulk-relocation
name: feedback_build_move_safety_before_bulk_relocation
description: "Build the move-safe substrate (content-addressed links) BEFORE bulk file relocations — don't move-then-path-update (rework)"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 7c732b67-b888-46d5-a52e-6372cedb7b53
---

When a bulk reorganization (relocating many specs/plans to their proper homes) is on the table, **build the move-safety mechanism first** — the content-addressed citation graph (`semantic-computable-links`: slug-resolved cites that survive moves) — then apply it to the whole corpus, THEN move things programmatically. Do NOT `git mv` first and patch inbound path-cites with `path-update-scan/apply` after.

**Why:** path-based `cites:` break on every move, so move-then-repair is "a bunch of rework" across the corpus and risks a dead-link storm. Once cites resolve by slug (not path), the moves are FREE — no breakage, no repair pass. The enabling substrate de-risks the bulk operation; sequence it first.

**How to apply:** before a bulk operation that a durable mechanism would make safe, build + apply that mechanism first. Surfaced 2026-06-02 when the corpus-classification produced a 43-plan relocation map: the operator chose to build the computable-links compute + migrate all piles before any `git mv`. See [[project_memory_cites_edge]], the semantic-computable-links spec/plan, and [[project_principle_p1_reconciliation_controller]] (build the controller before relying on reconciliation).
