---
title: "History/ADR: Archetype-primary a2o taxonomy migration — proposed, never executed"
id: archetype-primary-a2o-taxonomy-not-executed
type: history-gotcha
status: Accepted
tier: history
created: 2026-06-02
topic: [a2o, taxonomy, archetype, coherence-substrate, cites-edges, path-not-taken]
# DISTILLS a bulk a2o directory-restructure that was proposed (Sprint 0.5) and ABANDONED
# (0/10 decisions landed). The same archetype-traceability goal was met by the coherence
# substrate (cites: edges + coverage-spine) WITHOUT file moves. Raw plan bodies retire to git.
distills:
  - genesis/docs/plans/2026-05-22-scenario-archaeology-and-archetype-map.md
  - genesis/docs/plans/2026-05-22-archaeology-decisions-digest.md
canonical:
  - ../../../superpowers/specs/2026-06-01-coherence-substrate-design.md   # the replacement mechanism
memory_anchors:
  - feedback_a2o_is_human_experience_not_dev_bugs
  - project_memory_cites_edge
  - project_wisdom_resolves_into_epics
---

# Archetype-primary a2o taxonomy migration — proposed, never executed, superseded by the cites:/coverage-spine approach

> **One-sentence lesson:** Do NOT re-propose a bulk a2o directory restructure — taxonomy-by-relocation
> was tried and abandoned; express the archetype axis as tags / `cites:` edges over the existing tree,
> per the coherence-substrate design.

**Dated arc:** 2026-05-22 (Sprint 0.5 archaeology proposed) → 2026-06-01 (coherence-substrate-design
supersedes the migration mechanism) → 2026-06-02 (curated to history; migration confirmed abandoned,
0 of 10 decisions landed).

Sprint 0.5 ran a 10.3K-word archaeology pass over the 76 a2o `.feature` files, mapped each to the
gospel-tier collective-archetype catalog (Tier-0 household / faith-community / life-group /
wisdom-commons + cross-cutting + infrastructure), and proposed (a) a new archetype-primary directory
taxonomy (`features/{archetypes,cross-cutting,infrastructure}/`), (b) a 76-row file-by-file migration
plan, (c) a `@archetype:/@cc:/@inf:/@tier:` tag rename, and (d) a 10-decision digest the operator
confirm-all'd.

**WHY WE TURNED:** none of it was executed and the mechanism was superseded. Verify-gate evidence
(2026-06-02): no `archetypes/` directory exists; the corpus GREW 76→90 under the original mixed-axis
taxonomy; zero `@archetype:/@cc:/@inf:` tags were adopted; the named dispositions (split
relationship-idempotency, graduate the 3 smoke files, the MVP-BLOCKING witness-rewrite of
collective-governance) never landed — collective-governance still carries 62 vote/proposal mentions and
0 witness mentions. The replacement is the 2026-06-01 coherence-substrate-design, which achieves the
SAME archetype-traceability goal WITHOUT physical file moves: frontmatter `cites:` edges + a
coverage-spine walker + `@epic:value_scanner` scaffolding + `pending`-vs-`regressed` status, treating the
corpus as an edge-annotated graph rather than a directory to restructure.

**WATCH-OUT for future planners.**
1. Do NOT re-propose a bulk a2o directory restructure — taxonomy-by-relocation was tried and abandoned;
   express the archetype axis as tags / `cites:` edges over the existing tree, per coherence-substrate-design.
2. The archaeology's standing diagnostic remains TRUE and re-derivable: faith-community, life-group, and
   wisdom-commons have ZERO primary scenarios — that Tier-0 authoring gap is the live finding worth
   carrying, and it cannot be harvested from the value-scanner corpus (categorically absent there).
3. The "a2o is human experience not dev bugs" instinct that flagged 3 smoke files for graduation is
   sound and already canonical in memory — apply it at authoring time, not via a migration sprint.
4. The value-scanner content audit (`plans/2026-05-22-value-scanner-content-audit.md`) is a SEPARATE,
   still-live document — do not sweep it up with this; its "keep-in-place + index" recommendation IS the
   current reality.

## Bidirectional links

- **This record → canonical:** [coherence-substrate-design](../../../superpowers/specs/2026-06-01-coherence-substrate-design.md) (the replacement mechanism — `cites:` edges + coverage-spine, no file moves).
- **Distilled-from (raw bodies in git history):** scenario-archaeology-and-archetype-map + archaeology-decisions-digest (linked in frontmatter).
