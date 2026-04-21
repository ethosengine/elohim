# R&O Lessons → Elohim — Cross-Wave Guidance

**Date:** 2026-04-21
**Authors:** Matthew Dowell + Opus 4.7
**Status:** Guidance (shared context for wave execution sessions)
**Companion docs:**
- Source roadmap: `genesis/docs/plans/2026-04-21-rno-lessons-roadmap-handoff.md`
- Wave 1 plan: `genesis/docs/plans/2026-04-21-rno-lessons-wave-1-execution-plan.md`
- Wave 2 plan: `genesis/docs/plans/2026-04-21-rno-lessons-wave-2-execution-plan.md`

---

## Purpose

The roadmap handoff decomposes 9 R&O-inspired sub-projects. The wave plans execute the first four of them in two separate sessions. This doc holds the cross-cutting context both sessions need: the strategic frame, the design principles, the gate definitions, and the memory rules that govern decisions inside each sprint.

Read this first when opening a wave plan. It is short on purpose.

---

## Strategic frame

**We are not graduating R&O. We are preparing elohim to be worthy of being graduated into.**

R&O is the canonical example of a coordination hApp that will one day outgrow its Moss-group DHT and want a protocol-wide home. Our job for these waves is to make elohim that home — internally coherent, credibly shipped, economically intelligible to REA-shaped hApps — so that when the graduation conversation becomes real, elohim is the obvious landing zone.

This reframe matters for every design decision:

- **We do not work backwards from R&O's needs.** We build elohim's own coherence. R&O (and any other coordination hApp) graduates into that coherence, not the other way around.
- **The pitch is the destination (Wave 5), not the kickoff.** Writing the graduation narrative is the *last* act — it describes a bridge that already exists.
- **Credibility is cumulative.** Release discipline, test discipline, vocabulary discipline, and economic-interop discipline each contribute to a protocol that external observers can trust. None of them are "just process."

---

## Wave structure

Five waves, small-to-large, with explicit retrospective gates between each strategic pivot.

| Wave | Scope | Focus |
|---|---|---|
| 0 | Graph substrate (parallel work) | Foundation — not part of this roadmap |
| **1** | **#7 DNA manifest hygiene + #3 Sweettest** | **Internal quality foundations** |
| **2** | **#1 Release discipline + #2 Feature flags** | **Credibility & safe-rollout** |
| — | **Gate B — retrospective + vision check** | **Re-brainstorm before Wave 3** |
| 3 | #4 hREA / VF-GraphQL alignment | Strategic substrate (multi-week, XL) |
| — | **Gate C — retrospective + vision check** | **Re-brainstorm before Wave 4** |
| 4 | #5 Tauri multi-platform + #6 Launcher listing + #8 Moss Weave Tool | Distribution & visibility |
| 5 | #9 R&O graduation path doc | Destination — no sprint tasks |

Waves 1 and 2 are straight-ahead execution (this planning round). Waves 3 and 4 are re-brainstormed at their gates because their strategic weight and dependency on upstream work (graph substrate, Wave 3 outcomes) make up-front detailed planning premature.

---

## Gates

Each gate is an explicit pause for retrospective review and forward-looking brainstorm. Gates are not status meetings — they are design sessions.

### Gate A — Wave 1 retrospective (light)

After Wave 1 completes. Captured inside the Wave 1 plan.

Checks:
- Are all 5 DNAs manifest-hygienic? (#7 DoD met)
- Is sweettest running in Jenkins with baseline coverage? (#3 DoD met)
- Did anything surface in execution that reshapes Wave 2?

If nothing reshapes Wave 2, proceed directly. No brainstorm required.

### Gate B — Pre-Wave 3 retrospective + vision check (heavy)

After Wave 2 completes, before any #4 hREA work begins.

**Invoke `superpowers:brainstorming` fresh on sub-project #4.** Do not assume the strategic framing from the roadmap handoff still holds — the graph substrate may have resolved questions, Wave 1/2 may have surfaced new constraints, VF team coordination may be further along.

Required brainstorm inputs:
- Wave 1/2 outcomes — what quality bar does elohim now have?
- Graph substrate status — is it solid enough to build VF-GraphQL views on top?
- Path (a) / (b) / hybrid decision — re-evaluate, don't assume
- Which 3-5 VF types to start with (Agent, EconomicEvent, Commitment, ResourceSpecification, Agreement)?
- VF team coordination — has Lynn Foster / Bob Haugen been engaged?
- Impedance between VF's Agent-centric model and elohim's Human-vs-Agent distinction

Output: a fresh spec at `genesis/docs/superpowers/specs/` and plan at `genesis/docs/superpowers/plans/` for Wave 3.

### Gate C — Pre-Wave 4 retrospective + vision check (heavy)

After Wave 3 completes, before distribution work begins.

**Invoke `superpowers:brainstorming` fresh on sub-projects #5, #6, #8.** Distribution decisions depend heavily on what Wave 3 delivered:

- Is elohim stable enough to list publicly?
- Is Moss the right trial surface, or has Wave 3 made direct steward distribution simpler?
- Identity handoff story for Launcher-only users
- Mobile targets — in scope for Tauri, or defer?
- What does the Moss-group → elohim-steward graduation UX look like (this feeds Wave 5)?

Output: fresh spec(s) and plan(s) for Wave 4.

---

## Wave 5 — Destination

Wave 5 is not a sprint. It is the moment when sub-project #9 (R&O graduation path doc) can be written as a **credible invitation** rather than a speculative pitch.

**No tasks.** The marker is: Waves 1–4 have delivered a quality foundation, a credible release cadence, an hREA-intelligible economic layer, and a Moss presence. At that point, writing the graduation narrative is a documentation exercise that describes a bridge which already exists.

If someone tries to draft #9 before Wave 4 lands, reject it. The doc only has weight when it describes reality.

---

## Cross-cutting design principles

These principles apply inside every sprint. They are not repeated per-sprint; they are honored by default.

### 1. Stewardship, not ownership, not sovereignty

No component is sovereign. No actor *owns* data, authority, or identity — they *steward* it.

- Reject vocabulary: `own`, `ownership`, `sovereign`, `sovereignty`, `admin-of-X`.
- Use vocabulary: `steward`, `contributor`, `agency`, `authored`, `custodian`.
- **Applied to #7:** R&O's "progenitor pattern" is technically "the pubkey that first seeds a network and bootstraps authority." In elohim docs, CLI output, UI strings, and log messages → call it **bootstrap steward** or **founding steward**. The Holochain schema field name may still be `progenitor_pubkey` (we can't rename their primitives), but our surface language is stewardship.
- **Applied broadly:** anywhere a sprint names an actor with authority, check for sovereignty vocabulary and substitute.

Memory rule: `project_no_sovereignty_stewardship_over_ownership.md`.

### 2. Observed, not flagged

State derives from observation of real signals, not from config booleans. Flags declare *intent*; state reflects *reality*.

- **Flags** (declared, build-time): `VITE_MOCK_BUTTONS_ENABLED`, `PEERS_DISPLAY_ENABLED`, experimental UI toggles. Pure config. Safe to surface via a FeatureFlagsService.
- **State** (observed, runtime-derived): `Phase::ElohimActive`, `Peer::Healthy`, `Household::Resilient`. Never a flag. Always a computation over real signals (inference actually happening, peer actually present, contract actually held).
- **Applied to #2:** the FeatureFlagsService handles flags only. Observed state is a separate concern (state machines / gates in elohim-agent). Do not conflate them — naming matters.
- **Applied broadly:** any time a sprint proposes a boolean toggle, ask: does this declare intent, or describe reality? If reality, it doesn't belong in a flag registry.

Memory rule: `project_elohim_active_observed_not_flagged.md`.

### 3. Sense-and-respond split

Discernment and gates live in Rust (elohim-agent). Manifests declare which gates apply. TypeScript is sense-and-respond only, never the evaluator.

- Flags (TS) do not participate in gates (Rust).
- Gate logic never leaks into Angular services.
- Manifests are the contract between the two.

Memory rule: `project_elohim_agent_sense_respond_architecture.md`.

### 4. Schema-first is IoC

For any wire contract, write the JSON schema first. Rust and TypeScript comply. Never guess at implementation.

- **Applied to #7:** DNA manifest schema changes (modifiers block, lineage field) are schema-first — the schema is canonical, manifests are generated/validated against it.
- **Applied broadly:** any new wire format, API response shape, or manifest field starts as a schema.

Memory rule: `feedback_schema_first_ioc.md`.

### 5. Measures live in Jenkins

Eclipse Che has no docker / k8s / holochain locally. Any "verify this works" step that needs a running stack goes through Jenkins MCP, not local shell.

- **Applied to #3:** sweettest runs are Jenkins stages, not local commands. Sprint DoD includes "pipeline passes on Jenkins," not "tests pass on my machine."
- **Applied broadly:** never close a sprint on local green. Close on Jenkins green.

Memory rule: `feedback_shift_measure_jenkins.md`.

### 6. Doorway is manifest-driven

Doorway is a registry-driven proxy. App manifests declare HTTP routes. Direct doorway code changes are only for web2 concerns (federation, CDN, DNS).

- **Applied to any sprint touching endpoints:** add routes to the relevant domain manifest, not to doorway's handler code.

Memory rule: `project_doorway_manifest_driven_routes.md`.

---

## Memory rules summary (indexed)

The following memories apply across these waves:

| Rule | File | Applies to |
|---|---|---|
| Stewardship not ownership | `project_no_sovereignty_stewardship_over_ownership.md` | #7 especially, all sprints |
| Observed not flagged | `project_elohim_active_observed_not_flagged.md` | #2 especially, any state discussion |
| Sense-and-respond split | `project_elohim_agent_sense_respond_architecture.md` | #2, any agent/gate work |
| Schema-first | `feedback_schema_first_ioc.md` | #7, any wire contract |
| Measures in Jenkins | `feedback_shift_measure_jenkins.md` | #3, any verification step |
| Manifest-driven routes | `project_doorway_manifest_driven_routes.md` | any endpoint change |
| Stewardship philosophy | `project_stewardship_philosophy.md` | #7 progenitor framing |
| Collective is stewardship unit | `project_social_compute_collective_is_stewardship_unit.md` | #9 (Wave 5) |

---

## How to use this doc from a sprint session

When opening a wave execution plan:

1. **Read this guidance doc first** (~5 min). It is the shared context.
2. **Read the wave plan.** It contains sprint-specific decisions and tasks.
3. **Do not re-read the roadmap handoff** unless a question arises that the wave plan doesn't answer.
4. **Honor the design principles in §3 above by default.** They are not repeated per-sprint.
5. **At gate time** (Gate A inside Wave 1 plan; Gate B after Wave 2; Gate C after Wave 3), follow the gate definition in §2 above. Heavy gates require `superpowers:brainstorming` invocation.

If you find yourself deciding something that contradicts a cross-cutting principle here, stop and flag it. Either the principle needs updating (rare) or the decision is wrong (common).

---

## Out of scope for these waves

- Graph substrate spec/implementation (parent session)
- hREA / VF-GraphQL work (Wave 3)
- Tauri / Launcher / Moss (Wave 4)
- R&O graduation narrative (Wave 5)
- Apollo Client adoption as a general choice (resolved during roadmap session)
- Effect-TS 7-layer architecture adoption (not on roadmap)
- Any deep R&O internals analysis (done; see roadmap §2)
