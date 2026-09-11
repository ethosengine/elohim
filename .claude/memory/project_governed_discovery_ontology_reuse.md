---
index: false
name: project_governed_discovery_ontology_reuse
title: Governed discovery reuses existing kinds — recipe=ProcessSpec, lens=AttentionTending, sample=FlowEvent+Verdict
id: project-governed-discovery-ontology-reuse
description: "Design 2026-09-11: search/recall roles map to existing primitives — ProcessSpec/Manifest (recipe), Intent (need), AttentionTending (lens: TTL, tended, private), FlowEvent+Verdict+Observation (sample), ProjectionRequest/Feedback/FeedbackSignal (view+dispute), EpistemicStanding; social-reach five-constraint filter + anti-bubble floor govern lenses."
metadata:
  type: project
---

**Spec:** `genesis/docs/superpowers/specs/2026-09-11-governed-discovery-journey-lens-graduation-design.md`
(proposed, operator-approved in principle 2026-09-11; plan pending).

**The mapping (never mint these again):**
- recipe/algorithm → `ProcessSpec {stages: Vec<StageSpec>}` (`elohim/epr-rea/src/model.rs`) carried as a `Manifest` EPR; budgets → `Bound {limit, unit, threshold_pct, sense, source}`; the recall contract's `composition` list IS `stages`.
- need/question → `Intent {action: Consume, resource_spec, in_scope_of, raised_by}`; a question bank = Intents in scope of the recipe.
- reader lens preset → `AttentionTending {filter_subject, classification, ttl_seconds, tended_at}` (`elohim/epr/src/kind.rs`, private, never gossiped) — this IS the "time-limited, tended, values-forward" filter the social-reach architecture requires.
- journey sample → `FlowEvent {action: Consume, fulfills: [intent]}` + second-seat `Verdict {witness: Witness{checks}}` (`elohim/epr/src/verdict.rs`) + `Observation` folds; the seat's judgment is labour that carries flow (living memory: consolidation is judgment).
- view / dispute → `ProjectionRequest {purpose, audience, inputs, omissions}` and `Feedback {kind: PoorSelection|MisleadingProjection|OmittedContradiction|StaleSource}` (`elohim/eprfs/eprfs-agent/src/memory.rs`); network: `FeedbackSignal {Squelch|Correction|Retraction|Quarantine}`.
- provider ranking inputs → `EpistemicStanding`/`WitnessedInteraction` on CIDs, printed beside results, never summed into a score (standing is a shape).
- scope → `Scope::Directory` locally, `Scope::Ring(elohim_epr::Reach)` on the network; two `Reach` enums exist (8 rings vs Private/Workspace/Repository) — convergence frontier, no third enum.

**Heart-level rules carried from the epics:** reach earned at authoring is the floor and discovery never becomes a central filter; a lens must meet the five constraints (values-forward, time-limited, tended, anti-bubble floor of unfilterable classes, feeds collective wisdom); provenance on every candidate with sense/respond back along it; carrot before stick (the lens negotiation is the tender conversation); local first, rings outward; comet-shaped memory (recency ≠ authority); journeys sealed against the self.
Related: [[project_recall_reaches_authority_habit]], [[feedback_reader_context_shapes_recall_defaults]], [[feedback_inherit_substrate_ontology]].
