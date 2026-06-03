---
name: project_gap_granular_substrate_scope
description: "Substrate scope (held/ + budget) is gap-granular not document-granular — iroh≠shem; a mixed plan keeps testable gaps on the plate, only cross-node gaps held"
metadata: 
  node_type: memory
  type: project
  originSessionId: 7c732b67-b888-46d5-a52e-6372cedb7b53
---

The substrate-scope mechanism (the held/ tree + the planning gap-budget) is **gap-granular**, isomorphic with a2o's per-scenario `@requires:<cap>` tags — NOT document-granular. This came from the operator's correction "iroh by itself is still relevant — it still moves blobs": iroh is the blob-moving transport (household-testable on one node, loopback `MultiStackFixture`), while shem is only the cross-node *canvas* you verify against. Holding a whole plan because it has *some* cross-node gaps benches the real iroh transport work.

**The model** (`.claude/scripts/_lib/env_scope.py`):
- `resolved_requires_env(gap) = gap.@requires-tag if set, else the doc-level frontmatter default`.
- A gap is **BLOCKED-BY-ENV iff `resolved ⊄ available`** — *regardless of which directory its doc sits in* (physical `held/` location is decoupled from the budget signal).
- **Convention:** a UNIFORMLY-blocked plan sets a doc-level `requires_env` (every gap inherits → held whole); a MIXED plan declares NO doc-level `requires_env` and tags only its divergent gaps inline `@requires:<cap>`.
- Consumers all read the doc's **live frontmatter** as the inheritance default (not the possibly-stale cache copy): `decompose.py` records `doc_requires_env` + parses per-gap `@requires:` into the gap-items cache; `placement-audit.py --ledger` counts a blocked gap as BLOCKED-BY-ENV (not active OPEN) — this also closed the gap-budget leak where a held doc's gaps leaked as pickable; `scope-reconcile.py` `_scope_verdict` holds a doc whole only if EVERY gap is blocked ('live' = has a satisfiable gap → belongs on the plate; 'ambiguous' = no scope info → held stays held, the exfiltration guard).

**First instance:** `iroh-recovery-e2e` was held whole (33 gaps all blocked by doc-level `requires_env:[shem]`). Re-classified: dropped the doc-level req, tagged only the 4 live-cross-node-stack tasks (Steps 6.2/6.3/9.2/9.4) `@requires:shem`, moved back to the plate → **23 open / 6 claimed / 4 held**. The loopback fixture, Rust round-trip tests, observability, and catalog work are pickable now. `iroh-delivery-master` stays uniformly held (`requires_env:[harbor-registry, alpha-cluster-6peer]`).

Relates to [[feedback_build_move_safety_before_bulk_relocation]] (content-addressed cites make the held↔live move free) and the substrate-toggle work in [[project_ci_reconciles_to_substrate_signal]]. Spec: `genesis/docs/superpowers/specs/2026-06-02-scope-tree-reconciler-design.md`.
