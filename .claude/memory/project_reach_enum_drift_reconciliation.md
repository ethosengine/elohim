---
name: reach-enum-drift-reconciliation
title: Reach reconciliation — canonical spec exists
description: "Reach 5-way vocabulary drift now has a canonical guiding-principles spec (2026-07-22) — sprints plan AGAINST it, never re-derive; schema-8 canonical, geographic-8→locality, Part-V-5→custody, two die."
metadata:
  node_type: memory
  id: project-reach-enum-drift-reconciliation
  title: "Reach reconciliation — canonical spec EXISTS, plan against it"
id: project-reach-enum-drift-reconciliation
  cites:
    - genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md
    - genesis/research/ontology-systems-survey-reach-reconciliation-2026-07-22.md
    - genesis/research/letter-to-rea-practitioners-observed-presence-2026-07-22.md
    - genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
  type: project
  originSessionId: 8c91e512-7e3f-4704-8595-8a9333cfc24b
  modified: 2026-07-22T15:56:03.215Z
---

**The reconciliation is now SPECED — do not re-derive.** The 2026-07-22 session (ontology-systems deep-research survey, adversarially verified + operator-adjudicated thesis) produced the canonical spec `2026-07-22-reach-ontology-vocabulary-split-spec.md`. Its §7 is the sprint Definition of Done. Companion artifacts: the research survey and the REA-practitioner letter in `genesis/research/`.

**Core rulings (from the spec — read it for the full table):**
- Schema-8 (`private…commons`) is the canonical DECLARED reach; Rust services-8 and `VALID_REACH_LEVELS`-6 die; TS geographic-8 is renamed to a locality/placement vocabulary; resilience Part-V-5 becomes custody vocabulary (Mishpat::Commitment lineage).
- Effective reach = derived verdict `verdict(content, viewer?, announcement?, freshness)` with evidence + opt-in explain; narrow-never-widen composition; declared floor + key envelope sovereign.
- Freshness anchors on amber/green; revocation must order before what it protects (new-enemy problem).
- Announcement slot from v1 (anonymous→commons; session→viewer-lens; announced→negotiable); said-vs-did variance feeds standing, never widens access; fresh identities start at the floor.

**Adjacent design seed NOT in the spec (don't lose):** the observer-relative lens is one primitive family — imagodei profile lens (subtractive: what to conceal) and lamad path personalization (generative: what to fit) are the same read-time fold over shared substrate keyed on viewer evidence. Two-person learning paths = derived from the INTERSECTION of two evidence sets (A's mastery ∩ B's goals → teaching edge + recognition flow; shared frontier → companion path); mentorship = a materialized lens. Also: delegated-agent variance reflects up onto the dispatching principal.

**How to apply:** a sprint picking up roadmap item 13 / the frontend strand starts at the spec §7 DoD. Blast radius warning stands (72+ files, 3 separately-built bundles — see the backlog strand). T4-4 reach-governed serving is sequenced BEHIND the reconciliation, out of its scope.

**Execution state (2026-07-23):** slices 1–3 BUILT and review-READY on stacked un-pushed branches `shift/reach-vocab-slice1` → `slice2` → `slice3` (merge in order; integrator owns push). §7 DoD: items 1+2 SATISFIED (canonical enum + locality rename + aliases DELETED), 5 PARTIAL (storage done incl. one-time `content.reach` canonicalization migration `2026-07-23-140000` — public→commons ordering load-bearing; doorway `can_serve_at_reach` residue remains), 3/4/6 OPEN. Slice-4 queue lives in the strand doc's slice-3 dispositions section. The suspected parse divergence on stored "public" never existed in code (stale comment only). Sibling reach columns (`epr_atoms.reach`, `humans.profile_reach` DEFAULT 'public', `portal_hosts.reach`) NOT yet migrated. Operator-seeded follow-on: [[alias-density-governance-signal]] (backlog doc, classify+stack-rank alias clusters).
