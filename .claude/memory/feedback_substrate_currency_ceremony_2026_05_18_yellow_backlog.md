---
name: substrate-currency-ceremony-2026-05-18-yellow-backlog
description: YELLOW findings from the 2026-05-18 substrate-currency ceremony's Phase 4b coherence check that were larger than mechanical and got backlogged for next-cycle pickup. Three load-bearing pattern-examples are missing from gospel-tier Rust surfaces and a controller-state claim in the resilience epic may overstate what code does. Next ceremony should consider these for inline closure or operator-elevation as a focused sprint.
metadata:
  type: feedback
---

The 2026-05-18 substrate-currency ceremony (rust-architect.md + doorway-service/CLAUDE.md rewrites) had Phase 4b return YELLOW. Four findings were mechanical and closed inline; three were larger-than-mechanical and are documented here for the next memory-ceremony or cartographer pass.

**Backlog 1 — Doorway-manifest route pattern needs a minimal worked example.**
- Where it surfaces: `doorway/doorway-service/CLAUDE.md` "Adding New Routes" section (lines 41-49) and `.claude/agents/rust-architect.md` web2-bridge guidance (line 729).
- Why it matters: a future agent writing the `GET /storage-stewardship/summary` route (resilience-epic roadmap item 10) has the discipline named but no template for the three-part dance (implement handler in storage's `http.rs` → add to `build_manifest()` → doorway serves on next boot).
- Closing edge: add a 5-10 line code example to either `doorway/doorway-service/CLAUDE.md` or to `doorway/CLAUDE.md`'s "Adding New Routes" decision table. Parent doorway/CLAUDE.md is already named as the next-cycle pick by the cartographer's responsibility-split; this gap merges with that work naturally.

**Backlog 2 — Query-composition aggregation pattern missing from rust-architect.md.**
- Where it surfaces: the surface provides Path A/B/C patterns for entity creation but no template for *aggregation* routes that compose across multiple tables (the storage-stewardship-summary route is the canonical case — it aggregates `rea_commitments` by reach × `resource_classified_as` into three buckets).
- Why it matters: aggregation-shape services are different from CRUD-shape services and will recur — recovery-class progress dashboards, contributor-presence summaries, patron-CDN visibility surfaces all share this shape.
- Closing edge: add a §Query Composition Pattern subsection to `.claude/agents/rust-architect.md` after Path A/B/C, with the storage-stewardship-summary route as the worked example. Estimated ~15 min addition.

**Backlog 3 — Resilience-epic Part VII claim about ReconcileController state may overstate what's wired.**
- Where it surfaces: `genesis/docs/content/elohim-protocol/resilience/README.md` Part VII line ~450 claims the ReconcileController has "real handlers for imagodei/M5 recovery signals (`on_key_rotation`, `on_key_revocation`, `on_agent_peer_binding`, `on_revocation_attestation`, `on_portal_host_created/removed`) and stubs for the rest." The Phase 4b coherence-check agent reported that `elohim-storage/src/reconcile/controller.rs` describes itself in its top-of-file comment as "A.4 skeleton" with "All four handlers... no-op stubs."
- Why it matters: this is exactly the kind of gap matrix maintenance discipline `feedback_living_doc_honesty_matrix_maintenance` was created to catch. The matrix in Part IX says "LIVE — imagodei/M5 handlers"; if the handlers are actually still stubs, that's a Part IX row migration in the wrong direction.
- Closing edge: read `elohim-storage/src/reconcile/controller.rs` end-to-end and ground-truth the claim. If the controller does have wired handlers despite the "no-op stubs" comment, update the comment; if the comment is correct, migrate the resilience epic Part IX row from LIVE to DESIGNED-skeleton. Either way, the substrate-currency comes back into alignment.

**Operational disposition for next ceremony.**

Backlog 1 likely closes when the cartographer-named "parent `doorway/CLAUDE.md` rewrite" is picked in a future cycle — they overlap by topic. Backlog 2 is rust-architect-scoped and could close in a focused 20-minute pass without re-running the full lens dispatch. Backlog 3 is ground-truth work the librarian should handle as a Phase 2a deepening when ReconcileController is touched in any substrate-currency cycle.

None of these block the recovery-class signal_kind extension sprint (resilience-epic roadmap items 1-9). Backlog 1 + 2 are quality-of-life improvements for the agent doing that sprint; backlog 3 is matrix-honesty hygiene.
