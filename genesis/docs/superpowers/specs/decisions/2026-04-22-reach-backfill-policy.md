# ADR: Reach Backfill Policy for Existing Content

**Status:** Accepted
**Date:** 2026-04-22
**Supersedes:** none
**Context:** The graph substrate's EPR envelope requires a `reach` field on every atom. Existing elohim-storage rows (content_nodes, humans, economic_events, etc.) were written before `reach` was envelope-level and therefore lack any stored reach value. Phase 2b's projector must assign a reach when converting existing rows to EPRs.

**Decision:** Existing rows project to EPRs with `reach = "community"` until the owner explicitly re-asserts a different reach through a Phase 2b endpoint.

**Alternatives considered:**
- `commons` — rejected. Widens visibility beyond what the original author consented to.
- `public` — rejected. Same concern — public is broadcast-level on the substrate.
- `private` — rejected. Too restrictive; existing content was visible to network consumers.
- `self` / `intimate` / `trusted` / `familiar` — rejected. No evidence in the existing data to pick one over another.
- Per-content-type default (e.g., content_nodes → public, economic_events → community) — rejected. Adds complexity without proportional gain; the author re-assert path handles nuance.

**Consequences:**
- No existing consumer experiences visibility expansion at projection time.
- Authors who want their content broader-reach must explicitly act.
- Phase 2b MUST ship the re-assert path before the projector is enabled in production.
- The ADR is binding on Phase 2b's projector code and any future migrations that add reach to a projected table.
