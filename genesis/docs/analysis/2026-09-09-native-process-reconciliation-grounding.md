---
title: "Native grounding for reconciliation, search, and governed algorithms"
id: native-process-reconciliation-grounding
kind: analysis
status: draft
written: 2026-09-09
author: codex
serves: dev-system-equilibrium
cites:
  - elohim/epr-rea/src/model.rs
  - elohim/epr-rea/src/store.rs
  - elohim/epr-rea/src/epistemic.rs
  - elohim/eprfs/epr-cli/src/flow/walk.rs
  - elohim/eprfs/epr-cli/src/flow/read.rs
  - elohim/eprfs/epr-cli/src/flow/context.rs
  - elohim/eprfs/epr-cli/src/govern.rs
  - elohim/eprfs/eprfs-meta/src/evaluation.rs
  - .claude/hooks/pickup-semantic-surfacing.py
  - .claude/scripts/memory-kit/mempalace-currency.py
  - "requisite-variety-guidestar-epr-family-composition | Admission discipline for deciding whether reconciliation and retrieval justify a new native primitive; two examples alone do not establish two independent frameworks. | sha256:e1cf9e52fbe95c11 | path: genesis/docs/superpowers/specs/2026-08-12-requisite-variety-guidestar-epr-family-composition.md"
---

# Native grounding for reconciliation, search, and governed algorithms

The native substrate already represents processes over EPR atoms. The immediate
opportunity is to reconcile the meanings and bindings of those primitives before
introducing a general Algorithm kind or another work register. Reconciliation and
context retrieval provide two concrete uses against which to test their adequacy.

This is a dated source-grounding brief, not a feature specification or completion
ledger. It serves the existing `dev-system-equilibrium` concern; that habit remains
red and this investigation supplies no evidence for flipping it.

## Scope and evidence boundary

Inspected 2026-09-09 at repository HEAD
`45e54e7d7ba8fafcf0cb5ef674d6bfefc5934ffb`, with substantial concurrent worktree
changes and a mutable local flow sidecar. The doorway example includes uncommitted
work. Source inspection and existing test inspection establish implementation and
intended coverage, not fresh test success or fleet deployment.

Atlas routing: SDK grammar owns composition of existing primitives; domain/process
semantics own recipes and their interpretation; the filesystem projection seam owns
local context and governance adapters. Temporal ordering is examined where those
readers implement it. Bridge, cluster, transport, and runtime distribution changes
are outside this investigation. These concerns lack a matching current readiness
assessment in atlas §6, so source files are the build-state authority here.

Read-only probes: `epr doctor` succeeded (native governance floor coherent; Codex
hooks absent); `epr flow context` on the doorway continuity plan and `epr flow status
--json` succeeded. Installed CLI behavior was sampled, not proven binary-identical
to HEAD. No Rust suites, search quality evaluations, or memory ceremony ran.

## Existing ontology and binding strength

| Surface | Implemented primitive | Practical limit |
|---|---|---|
| `epr-rea` | Canonical payload CIDs; ProcessSpec stages/edges; Intent, Commitment, FlowEvent, Process; event-derived resource state | Byte identity does not establish that two differently worded behavioral assertions mean the same thing. |
| Recipes | Process carries `PinnedRef { id, version }`; hashing includes recipe semantics and excludes placement globs | Inspected registry lookup resolves by ID; an immutable `id@version → CID` association was not established. |
| Fulfillment | Quantity fold sums events naming a commitment | Generic open-work selection removes a commitment after any event names it in `fulfills`; that is not a quantity threshold or independent verification. |
| Epistemic standing | Separate fold consumes Cite/Affirm/Dismiss and ignores Produce; canon needs governance | CLI reviewer verdict notes are a different representation; their automatic composition with this fold was not established. Threshold defaults still carry a policy-binding TODO. |
| Dependency seals | Current edges fold by seal time with append-order ties; readers classify stale/held/governed edges | A matching fingerprint proves byte continuity, not semantic validity of a claim. |
| `.epr-meta` | Native ancestor cascade, versioned policy references, content-pin checks, validator provider | Repository validator dispatch is a hardcoded composition-root match. Content-addressed WASM dispatch is documented future intent. |
| `epr govern` | Decision includes the executing evaluator binary's content address | Identity enables attribution of disagreement; it does not prove fitness for purpose. |
| `epr flow context` | Combines identity, intents, commitments, notes, seals, habits, gate, governance | Its open-work filters do not yet express one shared reconciliation semantics. |

Source entry points: `epr-rea/src/model.rs:27`, `store.rs:130`, `fold.rs:133`,
`epistemic.rs:7`; `epr-cli/src/flow/registry.rs:65`, `project.rs:493`,
`context.rs:198`; `eprfs-meta/src/evaluation.rs:177`; `epr-cli/src/govern.rs:46`
and `repository_validators.rs:59`.

Existing tests inspected include `epr-rea/tests/fabric.rs` (CID stability, partial
fulfillment, sidecar integrity) and `epr-cli/tests/flow_integration.rs` (projection
and idempotence), plus `governance_parity_vectors.rs` (shared evaluator fixtures).
These tests were not executed in this pass.

## Concrete reconciliation failure: doorway continuity

`epr flow context genesis/docs/superpowers/plans/2026-09-09-doorway-continuity-proof-chain.md --json`
returned six `gap:open` intents, no commitments, and an approved gap-6 review note
describing completed household sibling-reading proof. This is not proof of whole
doorway continuity: the note explicitly excludes the remaining apex/WAN stations.

The source explains the mixed presentation. `flow/walk.rs:186` collects every
intent scoped to the current document CID without checking satisfaction. Its
commitment filter at :172–193 excludes anything ever named by an event's `fulfills`.
`flow/context.rs:198` intentionally includes notes on discharged commitments.
Thus the evidence exists and is discoverable, while the intent still reads open.
The source checklist also retains unchecked assertions.

There is an additional historical sensitivity: scope is a document body CID and
gap identity uses document/ordinal classification. A later iteration must examine
edits, reordering, and genuine assertion changes rather than equating either paths
or byte identity with enduring behavioral meaning. No general semantic identity
solution is established by this investigation.

Scenario commitments have a different sensitivity: `flow/project.rs:791–817`
computes feature-body identity for labels, but builds the commitment from its
scenario path and classification under repository scope. Its identity can survive
changed feature bytes. Historical discharge must therefore not be assumed to
verify a revised assertion without examining evidence freshness.

## Search and algorithms already make consequential choices

1. **Latest event.** `flow/read.rs:99` collects Produce events followed by Dismiss
   events, stable-sorts by timestamp, then chooses the last. Equal-time cross-verb
   ties therefore favor Dismiss, despite the preceding summary describing append
   order. This corrects the earlier conversational shorthand: global append order
   is not the exact implementation. Notes use a separate time/append-index order.
2. **Next work.** `flow/walk.rs:428` iterates a HashSet of scopes and truncates the
   resulting unfulfilled list to ten. This is a sample without a stable priority
   ranking; the name `top_unfulfilled` should not be read as ranked importance.
3. **Context retrieval.** `.claude/hooks/pickup-semantic-surfacing.py:34` selects
   `TOP_K = 4`, a first-three-prompts window, and a 300-character query limit before
   invoking MemPalace. These are existing attention-allocation choices.
4. **Corpus and freshness.** `mempalace-currency.py:23` names protocol content,
   working memory, and stories as the indexed surface. Its freshness check compares
   timestamps. Freshness within that corpus is not coverage of source code, all
   plans, or the complete declared feature set.
5. **Plural lenses.** `sdk/schemas/v1/commitments/author-lens.schema.json` declares
   rule, purpose (`telos`), scope, school, role, and version lineage.
   `elohim-facings/src/folds/lens_selector.rs:27` ranks valid lenses by affinity with
   a CID tie-break and preserves alternatives. This is an adjacent precedent;
   native flow search does not thereby inherit lens governance or fitness proof.

The inspected retrieval path does not establish a result receipt binding corpus,
model/index version, selection method, and purpose-specific evaluation. This is a
bounded finding, not a claim that no search machinery elsewhere has such fields.
The selector of algorithms needs scrutiny too: making candidates governable while
leaving the default selector implicit can move enclosure up one level.

## Authority and complexity boundaries to preserve

The normal Claude compose-gate path takes native `epr` decisions as authority,
while retaining Python advisories and a fallback when native execution fails
(`.claude/hooks/epr-meta-resolver.py:396`). Native evaluation still reads a registry
under `.claude/epr-meta/`. Native execution and harness-neutral authoring are
different completion claims.

Current `epr/src/kind.rs` has more variants than the nine-kind schema/comment
suggests; neither inspected definition includes Algorithm. Do not infer that an
algorithm needs a new top-level kind, or that the enum and schema already agree.
Process recipes, commitments, observations, lenses, and identified evaluators are
the reuse candidates. The `epr-rea/.epr-meta` admission rule asks for a second
independent framework before generalizing a primitive. Two examples within this
developer workflow do not by themselves satisfy that stronger admission rule.

The inspected flow store is a local append-only sidecar, not proof of DHT
notarization. Its `records()` reads history into memory and folds scan it. Recording
every retrieval as a permanent network head would require a separate P2P design
gate, retention/granularity decision, and measured cost budget. No persistent
entity, schema, route, or new notarized kind is proposed here.

## Bounded next iteration

Use two existing processes as examples: recovering remaining work for the doorway
chain, and retrieving the context needed to judge that work. Preserve distinctions
between source assertion, promised work, produced evidence, admitted verification,
communal standing, and remaining uncertainty.

| Order | Question to settle | Observable acceptance target for later design |
|---|---|---|
| 1 — Reconciliation | Which existing fold decides residual work, and how does it interpret satisfaction, review, regression, and changed assertions? | One finished narrow assertion stops appearing as new work; broader unproved assertions remain visible; contrary later evidence is not hidden by an old discharge. |
| 2 — Retrieval | What corpus and selection semantics are fit for that reader's purpose? | Relevant evidence and contradictory evidence are reachable; omissions and index scope are explicit; relevance never silently confers verification. |
| 3 — Algorithm governance | Which existing recipe/lens/evaluator bindings can describe both processes faithfully? | A reader can identify the applied method/version, purpose, input scope, governing choice, and evidence of fitness; alternative selection remains inspectable. |

The first acceptance target should drive a focused scenario and native reader
tests before any corpus-wide rewrite. The two examples then determine whether a
new primitive is justified. The memory ceremony can consume their correction
records and regenerate context once the interpretation is sound; refreshing prose
first would preserve the underlying ambiguity.

This brief is the dated evidence handoff. Ongoing status belongs in native flow
records and owning habit atoms, not an independently maintained checklist here.
