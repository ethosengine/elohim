---
id: "backlog-progressive-discovery-substrate-emergent-concern"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Emergent concern: composition/seam discovery leans on the model's mental map, not on a query the repo can answer — the eprfs · epr-meta (uncompressed) · palace (compressed) convergence into a native progressive-discovery substrate over the local filesystem, whose concepts graduate one layer up onto the p2p network (privacy, reach, provenance). Not designed yet; breadcrumbs for a deep research pass"
slug: "progressive-discovery-substrate-emergent-concern"
written: "2026-09-08"
author: "operator (mid-sprint thought, 2026-09-08) via orchestrator@fable-5.1"
status: "envisioned"
priority: "medium"
cluster: "agentic-context-tooling-consolidation-queue"
relatedNodeIds:
  - "habit:dev-system-equilibrium"
  - "habit:epr-atom-home"
  - "spec:epr-meta-compose-gate"
  - "spec:cite-fingerprint-cid-convergence"
  - "spec:eprfs-witnessed-interaction-primitive"
  - "spec:epr-meta-native-capability-dogfood-and-graph"
  - "spec:peer-discovery-fractal-federation"
  - "backlog-object-open-with-runtime-facet-and-install-ladder"
tags: [composition, seams, valueflow, attention, context-management, cites, epr-meta, eprfs, mempalace, discovery, reuse, duplication, research-pass, graduation, provenance, reach]
shift_objective: |
  NOT a build objective yet. When picked up: run a deep research pass (historian + cartographer +
  rust-architect read-only) over the breadcrumbs in §4, produce ONE design spec in
  genesis/docs/superpowers/specs/ that answers the four concerns (§2) and the graduation rules
  (§3) with a p2p-design-gate record, then the smallest slice: `epr seams --write-set <paths>`
  in Python over the existing graphs + git co-change, retro-tested against the six corrected
  facts of the 2026-09-08 sprint plan (all six paths must appear in the brief).
---

## 1. The concern, and how it surfaced

**Operator, 2026-09-08 (essence):** the cite links are in effect the composition and the seams of
the whole. Asking an agent to "take care of the composition" in a sprint is leaning on the
quality of that agent's native mental map and context search over the codebase. The meta
question: what is the *complementary search algorithm* that reveals the seams to agents during
development, so whoever is asked to consider composition has a robust scaffold of where the
links are, which edges are fresh or stale-but-suddenly-relevant, so recall and progressive
re-discovery happen, bespoke implementations are avoided, reuse is promoted, duplication is
reduced, and more value is captured in every flight. It feels like a convergence with **eprfs**:
**epr-meta** as the uncompressed layer and something palace-like as the compressed layer, with
three concerns beneath the raw (compression, algorithmic discovery, graph/vector
representation), forming a native take on progressive discovery over a local filesystem whose
concepts all *graduate one layer up* onto the p2p network, which adds privacy, reach, and
provenance.

**How it came up (measured, same day):**
- The first draft of the sprint plan (`2026-09-08-sprint-velocity-quiescence-holochain-close`)
  needed an Opus reviewer to correct **six** load-bearing facts (T2 env names, T3 seeder path,
  T4 the adopted check is TypeScript not Rust, T7 station-4 cause and pending-step diagnosis,
  T8 gate-runner has no generic cargo key, T11 the gate already ships). Every one was on disk.
- In the same session two `.epr-meta` `inject` nudges fired on the *right* seams
  (`interface-first-reuse-ts` on a codegen commit naming the canonical CID homes;
  `dev-lifecycle-script-sync` on hc-mesh.sh edits), proving edit-time seam surfacing works.
- The p2p design audit fired on the literal word "schema" inside a plan file (string match, not
  a seam), and the palace pickup surfaced tx5 notes at cosine 0.52 with the index 247 files
  behind the front-link.
- Conclusion: the knowledge was present in the repo; the **query** was missing. Recall was the
  primary and it is probabilistic and stale; composition is structural.

## 2. Design concerns to carry into the research pass (not decisions)

Four layers the operator named, with the questions each must answer:

| Layer | Concern | Open questions |
|---|---|---|
| L0 uncompressed raw | files, `.epr-meta`, cite envelopes with authored descriptions, manifests, habits, `@concern:` tags, git history | Is L0 complete? What authored edge is missing today (e.g. an "acts-on" declaration, a canonical-home declaration outside `inject` rules)? |
| L1 compression | derived, lossy, rebuildable: fingerprints/CIDs, summaries, drawers, co-change, KG triples | Content-addressed to the L0 bytes (cite-fingerprint ↔ CID convergence)? Incremental? Byte-identical rebuild on two machines? |
| L2 algorithmic discovery | the query: given a write-set, which edges (canonical home · cite in/out · co-change · sync rule · gate · concern · package · rule), ranked, with freshness marks | What is "fresh", "stale", "surfacing" (two hops from today's write-set)? Cost bound per brief? Where is it injected (Agent dispatch? session start? hooks as consumers)? |
| L3 representation | graph + vector store, one index, two renderings | Does the vector rank only, never generate the candidate set? Can the palace be one backend behind a trait? |

Cross-cutting concerns:
- **Authority.** Everything above L0 must be derived and rebuildable; the only authored things
  are L0 files and the one-sentence description on a cite. A stale index must degrade to a
  banner, never to a wrong answer.
- **Reuse / duplication as missing edges.** A bespoke implementation is an edge that should
  exist and does not; can shape-match (same symbol stem / manifest vocabulary in a canonical
  home) and orphan-cite (a doc citing nothing in a cluster with siblings) be emitted as
  findings, reported never auto-fixed, with the compose gate staying the enforcement class?
- **Git co-change** is free, deterministic, and unread today; it is the best honest proxy for
  "stale but suddenly relevant".
- **Graduation.** Each L0 concept has a network twin and graduation adds exactly three concerns:
  a file → object CID (eprfs); a cite → an EPR link with fingerprint = CID; a `.epr-meta` rule →
  a governance policy on the directory-as-object; a drawer/summary → a derived-view attribute
  (A2) carrying its source CID; a seam edge → a witnessed interaction on the object CID; the
  brief → a federated query over reach-visible peer indexes. Rules to test: privacy is a property
  of L1 (compress private content on-device; only CID + reach-gated summary graduates); reach is
  inherited by derivation, never widened; provenance is the witnessed-interaction ledger.
- **Context management is a valueflow (operator, 2026-09-08 follow-up).** Every discovery act is
  attention spent: a brief consumed, a cite followed, a drawer derived, a reviewer's correction,
  a model's context window filled. The eprfs primitive already denominates interactions on the
  object CID and aggregates them as REA; `epr flow` already projects the developer valueflow from
  the repo (claim/fulfil/verify/rule). The research pass must state how a seam brief's consumption
  and the derivations it triggers become REA events on the objects they touch (who spent what
  attention discovering what, and which reuse it produced), so that context management stops being
  a hidden cost and becomes a measurable flow with a stock (what is known, cached, stale) and rates
  (re-discovery, decay). This is also the honest denominator for "more value captured per flight":
  value captured / attention spent, per object, per flight. Ties to `dev-system-equilibrium`
  (stocks and drain rates) and to the palace's per-subagent scope as an attention budget.
- **Placement.** By the seam map: L1/L2 are a crate concern in the eprfs family (never inside
  `eprfs-core` for git semantics; brit adapts the repository); Python `_lib` graphs are the
  oracle until a Rust derivation matches byte-for-byte (the parity discipline brit used).
- **P2P design gate** (to be answered in the spec, not here): seam edge local = C; derived view
  local = B, graduated = A2 attribute of the object CID; cite description = A via its doc; zero
  new DHT entry types expected; identity content-derived (CID of the edge tuple); no new HTTP
  route (a CLI verb over local L0+L1).

## 3. What "done" for the research pass looks like

One spec, cite-sealed, with a p2p-design-gate record, that (a) joins the five existing seam
graphs (§4) into one query contract, (b) states the four-layer invariants, (c) states the three
graduation rules, (d) names the smallest slice and its retrospective test (the six corrected
facts of 2026-09-08 all appear in the brief for that plan's write-set). The spec must cite at
least the breadcrumbs below and must not mint a new register: every edge is derived from L0
files that already exist.

## 4. Breadcrumbs to bootstrap the pass (read in this order)

**The five seam graphs that exist and are not joined**
1. Cites: `cites:` envelopes (slug · description · fingerprint · path), `cite_graph.py`,
   `cite-gen.py --seal/--verify/--into`, `cite-describe.py` (`.claude/scripts/_lib/`,
   `.claude/scripts/memory-kit/`). Convergence of fingerprint with CID:
   `genesis/docs/superpowers/specs/2026-07-12-cite-fingerprint-cid-convergence-design.md`.
2. Directory governance: `.epr-meta` cascade, deny/dispatch/inject classes, `covers:`,
   resolver — `genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md`,
   `.claude/scripts/_lib/epr_meta.py`; policy registry measure
   `2026-07-02-epr-meta-policy-registry-measure-design.md`; kinship/lineage
   `2026-07-12-epr-meta-kinship-lineage-reconciliation-design.md`.
3. File relationships: `.claude/file-relationships.json` + the edit-time sync hooks.
4. Concern chain: `.epr-meta/*.habit.md` `checks:` → `@concern:` tags in `genesis/a2o/features`
   → `build-manifest.json gate.projects` (`genesis/orchestrator/gate-runner.mjs`).
5. Package graph: eprfs package graph and projections —
   `genesis/docs/superpowers/specs/2026-07-10-epr-meta-native-capability-dogfood-and-graph-design.md`
   (Implemented), `elohim/eprfs/CLAUDE.md` (boundary: projection, never truth; no git semantics
   in `eprfs-core`), brit's content-addressed epr-meta foundation + composition snapshot
   (memory `project_brit_next_gen_epr_meta_foundation`,
   `elohim/brit/docs/specs/2026-07-12-shared-crate-consolidation-design.md`).

**The compressed store and its currency discipline**
6. MemPalace: ChromaDB + SQLite, wings/rooms/drawers, KG triples, tunnels, per-subagent scope
   (memory `reference_memory_system`); `mempalace-currency.py` staleness tripwire;
   `.claude/hooks/pickup-semantic-surfacing.py` (cosine floor 0.35, once per session, degrades
   to a banner). `.claude/skills/memory-kit/SKILL.md` §pickup-time complement.

**The graduation targets**
7. eprfs witnessed-interaction primitive (local witness → peer-validated → REA-aggregated on the
   object CID; attention-denominated):
   `genesis/docs/superpowers/specs/2026-07-15-eprfs-witnessed-interaction-primitive-design.md`.
8. Peer discovery across reach: `2026-07-09-peer-discovery-fractal-federation-design.md`.
9. Where a concern lives (manifest → SDK seam, crate → bridge, native code → mod):
   `genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md`.

**The evidence of the failure and the two sibling specs written the same day**
10. Sprint plan `genesis/docs/superpowers/plans/2026-09-08-sprint-velocity-quiescence-holochain-close.md`
    (the ⟲ marks are the six corrections).
11. `genesis/docs/superpowers/specs/2026-09-08-package-composition-at-the-holochain-seam.md`
    and `2026-09-08-seamless-groupware-ux-addendum.md`: both are cite-dense composition documents
    whose authoring exercised exactly the manual process this substrate would replace.

**Prior art already surveyed in-repo (do not re-survey; cite)**
12. `agentic-harness-borrows-backlog` (context-engineering survey 2026-08-13) and the memory
    horizon scans under `.claude/memory-kit/horizon-scans/`.

## 5. Placement

Queue item 23 in `agentic-context-tooling-consolidation-queue`. Not in sprint 2026-09-08. Pick
up as a research pass (read-only, three lenses) before any code; the first slice after the spec
is `epr seams --write-set` in Python over graphs 1–5 plus git co-change.
