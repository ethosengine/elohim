---
title: Concurrent agent attribution — implementation plan
id: concurrent-agent-attribution-plan
status: Draft
class: process-meta
context-tier: disclosed
steward: agent:architect@gpt-6
graduation-trigger: Six station commitments discharged with named tests, real three-harness evidence, and contextual valueflow discovery from a cold start.
date: 2026-09-05
serves: dev-system-equilibrium
cites:
  - "concurrent-agent-attribution-design | The primitive-reuse and invisible-mechanics contracts each implementation station must satisfy. | sha256:e1ba98b4f3ca4b47 | path: genesis/docs/superpowers/specs/2026-09-05-concurrent-agent-attribution-design.md"
---

# Concurrent agent attribution — plan

Use the valueflow-authoring, valueflow-implementer and valueflow-reviewer skills. One checkbox
per station is one mintable intent. First implement station 1; later stations remain explicit
work. User authorization covers spec, plan and starting implementation, not publishing or
deploying. Review the scoped worktree diff against the base SHA; commits are not required in
this shared worktree. This plan adds no active habit and does not claim equilibrium.

Operator steering: remove mechanical friction from in-flight work. Worker registration,
scope propagation and evidence references belong inside existing verbs and lifecycle adapters.
Progressive discovery through flow context and native packages supplies relevant research,
contractual wisdom, Mishpat obligations, tools, telos and values. Preserve deliberate friction
for evidence, consent, negotiation, review and contested judgment. No new manual setup ritual.
Normal operation should be quiet like dependable insurance; failure must expose a verifiable
depth of existing evidence and obligations for care, repair and accountability. Distinguish
local claims/integrity from signatures and peer witnessing; name unavailable evidence honestly.
Agent understanding is shared with Imagodei/Sophia/Psephos self-knowledge, not a separate model
profiler. Follow the spec's grounded interpretation contract: stated inputs, observed outcomes
and inferred patterns remain distinguishable, revisable and governed by consent and disclosure.

## Station 1 — Keep the actor claim in the evidence

Execution evidence (2026-09-05): implementation complete; `just gate eprfs` EXIT=0; independent
review approved and the commitment fulfilled. See `concurrent-agent-attribution/task-1-report.md`.
The checkboxes below declare intents; fulfillment and review are recorded in the valueflow.

Files: `elohim/eprfs/epr-cli/src/govern.rs`, `src/flow/{note,claim,fulfill}.rs`,
`elohim/eprfs/epr-cli/tests/actor_claim.rs`, `tests/flow_edges.rs`, and the CLI seam registry.
Paths abbreviated with `src/` belong to `elohim/eprfs/epr-cli/`.

Retain the CID already returned by ActorStore. Add `actor.claimCid` to session-bearing governance
stamps and `actor-claim:<CID>` before the final steward slot on session-resolved flow records.
Reuse existing attribution resolution and atom encoding. Explicit `--as` does not look up or
invent a claim pin. Keep missing/corrupt fallback and all old atom bytes readable.

Proof: independent fixture claims for two identical role/model workers yield different pins;
a later model change leaves prior event references intact; note, claim, task-report fulfill,
and govern all retain the appropriate pin. Test explicit override and absent/corrupt sidecars.
Do not call sequential fixture interleaving a concurrent storage proof.

Gate: obtain from `epr flow context <file>`; the owning manifest currently says `just gate eprfs`.
Claim/release the cargo berth around cargo work; retain the actual EXIT line in the report.
Record a review verdict and fulfil this station only with the gate evidence. Update the existing
habit with an evidence delta and regenerate the habit projection without a status flip.

- [ ] Station 1: governance and flow evidence retain the exact actor claim address, with interleaved-worker and compatibility tests passing.

## Station 2 — Make shared sidecars safe for concurrent authors

Execution evidence (2026-09-05): implementation and both owning gates pass, including explicit
sidecar-disabled tests and a red/green descriptor-lifetime regression. Independent review by
the operator-selected gpt-5.6-sol approved with no actionable findings; the existing commitment
is fulfilled. The earlier review-runtime hold and its resolution remain in
`concurrent-agent-attribution/task-2-report.md`. No commits or pushes were made.

Files: `elohim/epr-rea/src/{actor,store}.rs`, their existing tests and owning seam registry;
CLI claim/fulfill transaction boundaries if needed. Inspect gate ownership before editing.
Reproduce concurrent first-open truncation, then make creation non-truncating. Coordinate full
record writes/reads and read-check-append idempotence using existing store seams and filesystem
locking. Specify crash/partial-record recovery without silently deleting evidence. Test separate
processes, simultaneous same-intent claims, same-role distinct workers, and interrupted writes.
Document durability level; lock acquisition failures must surface rather than imply acceptance.

- [ ] Station 2: independent processes preserve actor and flow history under concurrent creation, append, duplicate claims and interrupted writes.

## Station 3 — Carry the worker scope consistently

Files: `epr-cli/src/{actor,govern}.rs`, `src/flow/mod.rs`, CLI integration tests; governing adapter
packages only where needed. Reuse opaque ActorClaim.session. Define one shared CLI scope resolver
with explicit worker scope taking precedence over inherited vendor session. Preserve legacy
session-only behavior and no-parent-fallback for a named but unclaimed worker. Avoid joining
identity namespaces or encoding model identity into a new registry key.

- [ ] Station 3: all actor, governance and flow entry points use the same explicit worker binding without stealing a parent or sibling claim.

## Station 4 — Project and prove real harness bindings

Files: owning `.epr-meta/elohim/packages/` agent/hook/projection packages and
`elohim/sdk/domains/elohim-agent/` renderer/tests. Inspect metadata.master and current runtime
contracts before edits. Generate only touched projections. Complete stock Codex and Gemini
bindings appropriate to installed products; Antigravity is not a synonym for Gemini CLI.
Replace shared-session persona overwrite/reclaim where the runtime supplies worker provenance.
Verify package fidelity and actual lifecycle events from each installed harness, including a
shell-less worker. Document unavailable capabilities with evidence, without simulating support.
Registration and scope transport must be automatic for supported runtimes, with no per-action
manual flags; explicitly count attribution repairs during switch/restart/handoff (target zero).
Exercise refusal, referral and changed-obligation fixtures through each runtime; verify context
arrives before the relevant action and unavailable support never grants permission. Persist
unavailable/unreadable attribution once per worker scope using existing observation events.

- [ ] Station 4: Claude, Codex and Gemini each preserve native governance and worker attribution in observed runtime lifecycle calls.

## Station 5 — Close the resource, evidence and dispute loop

Files: existing elohim-agent manifest/package binding consumers, epr CLI context/flow consumers,
and `genesis/a2o/features/devflow/agent-identity-claim-and-acceptance.feature` with its steps.
Reuse capability-used, use/consume, resource CIDs, Magnitude and measurement evidence. Keep
requested modelHints separate from measured effort. Add the spec's multi-agent acceptance story
and run the required fresh-context story review. Resolve records through existing flow context
and actor history; any added reader must have a concrete story consumer. Show a third-party
correction against a pinned historical claim, with no false fulfillment or authority promotion.
Verify progressive context/tool discovery through existing flow context and package bindings:
the responsible agent can identify the relevant research, telos, values and Mishpat obligations
without fiddling with transport identifiers or reading the entire corpus. Review confirms that
automation preserved the meaningful consent, evidence and judgment boundaries.
Join the existing ActorStore into note-target/context readers by historical claim CID, and test
`epr flow note --on <historical-claim-CID>` directly. After a deliberate failed outcome and loss
of the original process, a reviewer must recover the exact actor/promise/governance/evidence
chain, identify unwitnessed or unavailable links, and record a non-discharging correction.
Test disclosure limits and routed responsibility without treating audit evidence as automatic
blame or creating a second audit store.
Report local claims versus witnessed identity/delegation separately. Scope remote graduation to
the existing protocol contract, not a new agent system.

Preserve the existing REA observations, resource/package/model configuration references and
proof status needed by the shared understanding consumer. Trace these to the existing Sophia
Recognition/instrument semantics without recasting every work event as a psychometric answer.
Test that a self-report and an observed outcome remain distinguishable and that superseding
an interpretation cannot rewrite its source evidence. Development evidence must not enter the
public reputation fold, whose phase filter deliberately excludes it.

- [ ] Station 5: the full three-harness story produces walkable resource use, fulfillment and a contestable historical attribution, with honest measurement and authority status.

## Station 6 — Discover relevant valueflows from a cold start

Start with `elohim/epr-rea/src/{walk,scope,fold,stock}.rs` and existing
`elohim/eprfs/epr-cli/src/flow/{context,walk,stocks}.rs` consumers. Reuse these primitives; add the
missing contextual entry and graph projection in the owning consumer. Ground existing search
and graph renderers before selecting a presentation surface; apply the frontend skill for any
actual UI work. Do not add a parallel graph store or relevance ontology.

Discovered prerequisite (station 2 inspection): source path → manifest ownership → actionable
gate context has a missing ownership lookup. `epr flow context elohim/epr-rea/src/store.rs`
reports no gate because its reader walks ancestors and matches project.dir; the existing
manifest change detector correctly selects `elohim-epr` from sibling-declared source inputs.
Probe: context and `gate-runner.mjs --changed-file-list --print` agree on that same source path.
Current state: verification coverage exists; contextual discovery is incomplete. Reuse the
manifest-owned detection semantics; do not add duplicate gate declarations to hide the gap.

From workspace/scope and purpose alone, progressively show relevant intentions, agents,
commitments, resources, dependencies, fulfillment, governance and evidence-bearing signals.
Every suggested focus explains why it matters in the declared telos and values, what signal
supports it, and which obligations/tools are next. Distinguish declared graph edges from optional
semantic proximity. Reuse typed quantities and declared windows; no centrality-as-value or
universal urgency number. Suggestions never claim work or confer authority.

Before model–task-fit suggestions, trace `sophia-core` Recognition, `psyche-survey` aggregation,
Lamad's existing InstrumentDefinition/registry/loader and `ElohimCapabilityProfile` to the actual
consumer. Make the existing interpretation seam reusable without a second registry or a core
dependency on a UI bundle. This is a missing station within the discovery chain: attributed
observations → portable, evidence-bearing interpretation → situated fit → governed suggestion.
Probe: the same pinned instrument and inputs produce the same interpretation in existing and
agent consumers, with missing/unsupported interpretation reported rather than scored as certainty.
Current state: contracts exist; portability, uncertainty semantics and model-fit consumer unproven.

Use task-appropriate instruments, not human personality norms applied to models by assumption.
Keep declared model capability/configuration separate from demonstrated performance and emerging
hypotheses. Test contradictory stated/observed inputs, sparse evidence, changed model/configuration,
superseded interpretation and denied disclosure. A suggestion explains its evidence and uncertainty;
unknown fit remains unknown, and selection cannot grant authority or public reach. Preserve exact
graph discovery when interpreted fit is unavailable. The known Psephos ballot naming cleanup is
separate scope, not a prerequisite for provenance repair or a reason to invent a replacement.

Tests: a fresh agent without an internal ID finds a relevant flow, traces a suggestion backward
to evidence and forward to affected promises, responds honestly to changed/stale/missing signals,
and respects scope/disclosure/WIP constraints. An unavailable semantic index must leave exact
local graph/text discovery usable. Add the story to the existing devflow acceptance family and
run its required blind-reader review; prove the selected presentation against actual consumers.

- [ ] Station 6: a fresh agent discovers relevant valueflows and explainable focus signals from context, with graph-to-evidence traversal and no manual identifier hunting.
