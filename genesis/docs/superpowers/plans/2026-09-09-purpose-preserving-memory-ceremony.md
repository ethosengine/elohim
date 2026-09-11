---
title: Purpose-preserving memory ceremony — first reconciliation journey
id: purpose-preserving-memory-ceremony
status: proposed
class: devflow
serves: dev-system-equilibrium
date: 2026-09-09
tags:
  - memory-ceremony
  - reconciliation
  - decision
  - constraint
cites:
  - "governed-retrieval-execution | Governed retrieval execution and progressive discovery | sha256:14cafaec2b815d08 | path: genesis/docs/superpowers/plans/2026-09-09-governed-retrieval-execution.md"
  - "acceptance-plan | Acceptance-aware native reconciliation | sha256:d794ff0ca171a660 | path: genesis/docs/superpowers/plans/2026-09-09-acceptance-aware-reconciliation.md"
  - "ceremony-efficacy | Memory ceremony efficacy | sha256:23eb372f4ba66135 | path: genesis/docs/analysis/2026-09-09-memory-ceremony-efficacy.md"
  - "elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
---

# Purpose-preserving memory ceremony — first reconciliation journey

## Sprint outcome and authority

Deliver an agent-facing ceremony that carries purpose through **Orient → choose a concern → understand its story → follow evidence → act → review → reconcile**. An appointed fresh agent must complete an evidenced repair, recognize a contested assertion, survive a deliberate context reset, and finish with a warranted outcome. Sacred Attention means correct, explainable decisions with preserved uncertainty and less avoidable investigation; token cost is supporting evidence.

The first product surface is one coherent reconciliation view over remaining stale edges, with compact human-readable and structured renderings behind a single ceremony entry. It groups shared evidence, exposes exact passages and offers contextual next actions. The bounded executor runs underneath. A collection of independently usable search commands does not satisfy this sprint.

This plan serves the existing `dev-system-equilibrium` habit. Its outcomes improve the quality of stock drainage; neither one accepted journey nor fewer stale edges establishes the habit's rate invariant. Keep the habit RED until its own measured criterion passes. The plan's assertions become commitments through existing flow projection/claim; boards are derived views, never another queue.

Existing operator rulings on the recall algorithm provide the governing context:

- `bafyreieicbex3dkkgounj4latxya32ezpl6622lpij327mzndwptfqkf7q`: governed concern views compose scope, retrieval, filtering, grouping, aggregates and linked actions.
- `bafyreig7os3vfj4z63vgy7lpfgcnzzr2z26vrvdtheapoegvgzhnnhxkr4`: purpose, guiding context, omissions, progression, authority, dependencies and execution provenance belong to the experience; portability of bytes is insufficient.

These records were inspected in `.eprfs/status/flows.jsonl`. User direction in this session authorizes sprint planning. Implementation and experiential acceptance are not claimed by this document.

## Grounded baseline and entry prerequisite

Recorded foundation: scoped local retrieval accepted; one doorway assertion accepted within the observed household bundle; MAP placement adopted; stale edges reduced 147 → 28. The dated foundation chronicle retains conflicts, missing evidence, incomplete reviews and broader delivery limits. Those counts are historical, not a fixed sprint denominator.

Verified source seams:

| Existing surface | Reuse and structural gap |
| --- | --- |
| `elohim/eprfs/epr-cli/src/flow/context.rs` | Composes identity, intent, notes, seals, habit, gate, governance and reconciliation. Extend its composition; avoid a parallel authority reader. |
| `flow/reconciliation.rs`, `flow/edges.rs` | Exact assertion evidence and acceptance; computed edge states with source/target, explanation and governor. Fingerprint drift alone cannot classify a repair as evidence-ready. |
| `flow/acceptance.rs` | Appointment, independent attribution, production, technical review and pinned experiential report. Local accountability does not establish peer authority. |
| `elohim/epr-rea/src/model.rs`, `flow/registry.rs` | Existing Intent and ProcessSpec, canonical atom identity and path-independent recipe identity. New mandatory fields can change existing CIDs. |
| `.claude/scripts/memory-kit/recall-contract.json` and executor | Bounded sources, excerpts, declared scope and accounting. Session state lacks intent, findings, decisions and next action. Semantic subprocess selection is hardcoded to MemPalace. |

Atlas routing: native composition/governance corresponds to the SDK grammar seam; presentation and intent capture correspond to the client seam, here an agent-facing local interface. No new Angular application, bridge, runtime plugin, doorway service, cluster operation or transport work is needed for this first slice. A later graphical renderer can consume the same derived view.

Before claiming implementation, restore a verified `epr` executable; run setup/doctor, explain each owning path, then read the exact scoped context in JSON and human form plus current edge inventory. This planning environment has neither documented `/tmp` binary; onboarding cargo failed resolving `index.crates.io`. Do not interpret unavailable probes as an empty graph. Record executable identity, worktree revision and dirty-source fingerprints. Inspect the two rulings and map the sprint to existing intent/recipe references without minting duplicate intent for already represented work.

## P2P design gate and ownership decisions

The gate precedes schemas or route design. This is local EPRFS authoring, matching the accepted retrieval slice's scope; no Holochain witnessing or cross-device durability is asserted.

| Entity / view | Classification, identity and source of truth | Projections and cost |
| --- | --- | --- |
| Authored journey recipe and guiding references | Private B, local authored content under existing EPRFS governance. Reference existing ProcessSpec, Intent, ruling and content CIDs; fingerprints verify bytes and are not new content addresses. | Extend the governed algorithm artifact, with an explicit version and declared dependencies. No shared table, sync or publication. |
| Continuation state | Private B, local agent/session-scoped operational state. Reuse the existing recall execution area; session labels are locators, not actor identity. Authoritative judgments remain native notes/reviews. | Retain intent/recipe pins, selected references, evidence receipts, unresolved questions and next action. Rebuild acceptance/edge state on resume. No shared work queue. |
| Orientation, concern groups, node story, action choices and totals | Ephemeral C, reconstructable from native records, recipe and verified evidence. Preserve exact native identities; paths are navigation locators. | Disposable human/structured projections. Aggregate only the declared observed set. No additional content head per row or navigation event. |
| Repairs, reviews, appointments and acceptance | Reuse existing native flow events and governors without changing their authority or identity model. | No new persistent verdict model; effects go through current verbs and admission checks. |

Network stages: local tooling does not select a runtime stage or emulate notarization. Evidence integrity, explicit authorization and honest absence remain floors under any eventual stage. No stage-based discount is introduced. Network head count added at seed and year one is **0**; local receipts grow with actual investigation and reuse the existing retention/accounting mechanism. No added quiesce work.

Integrity check: `dna/elohim/zomes/content_store_integrity/src/lib.rs` declares existing `EntryTypes::Content`; `dna/elohim/dna.yaml` names `content_store_integrity` and coordinator `content_store`. This sprint calls no content-creation coordinator, emits no post-commit signal, introduces no SQLite/Automerge projection or HTTP route, and is DNA-hash-neutral. Bytes remain local; transport affinity is not applicable. The back-fill questions therefore resolve to no new coordinator return/route hash, unchanged integrity zome, and zero new network heads. Publishing this experience later needs its own witnessed-path gate.

Reuse existing CID/path/citation references. Grounding did not locate a literal `ContentReference` type in the sampled native source; do not invent a parallel type based on that conceptual name. Preserve canonical bytes/CIDs for existing ProcessSpec and Intent atoms. Any necessary contract evolution must explicitly version new recipe content and retain old-reader/old-session behavior.

All applicable concern answers are **partial** until implementation has contract tests:

| Canon | Required behavior and proof obligation |
| --- | --- |
| C0 | Presentation derives truth/authority; tests distinguish cached view, current source and admitted acceptance. |
| C1, C5 | Ranking or an implementer's recommendation cannot confer acceptance. Independent technical and appointed experiential judgments remain separate. |
| C2 | No newest-file or arrival-order inference of authoritative scope. Preserve declared pins, historical candidates and existing evaluator ordering. |
| C3 | Every unfinished view offers an executable inspection, continuation or responsible-stop transition. Waiting for human judgment preserves the frontier and permits independent work; it never manufactures approval. |
| C4 | Missing, refused, unreachable, unmeasured and contested stay distinct. Partial traversal cannot render a population total. |
| C6a, C11 | Bound each retrieval/provider operation, retain cumulative cost and failures, and expose deferred/retry/refused outcomes under imposed load. |
| C6b | Reopening/resuming a view has no authoring effect. Retried actions re-read native receipts and cannot duplicate closure. |
| C7 | Every offered action is executable under its stated prerequisites; disabled actions explain the unmet condition. |
| C8 | Record action/result reasons, successes and failures, scope and sample/census semantics. Do not introduce a second telemetry ledger. |
| C9 | Preserve exact native actor/appointment references; never join by path, gap label or session name. Peer lineage resolution is outside this local slice. |
| C10 | Recipe/provider/source changes are visible and require explicit continuation or revalidation; old accounting and provenance survive. |
| C12, C13 | Existing local authority gates govern source repair and holds. Identify local appointments as local scaffolding; network authority requires a separately witnessed capability path. |
| C14 | Unclassified conflicts retain evidence, uncertainty and a next step or stopping reason. No unknown branch silently disappears. |

Read the guarantees from `.claude/epr-meta/{concerns,policies}.yaml` when implementing. Register any new Rust decision predicates in the owning seam registry with contract tests; current planning answers are not an enforcement claim.

## Delivery stations

One sprint, six ordered stations; at most two implementation commitments in flight. Stations 1–2 establish shared contracts; station 3 makes the first full journey usable; stations 4–5 complete continuity and portability; station 6 accepts the integrated experience. Stations 4 and 5 may proceed in parallel after station 3. Four memory lenses share one evidence packet; independently dispatch contested judgments and final readers. Working batches continue while useful authorized work remains.

### 1. Establish the concern and acceptance contract

- [ ] A fresh entrant can recover the ceremony's intent, source-backed guiding context, scope, constraints and worthwhile finish from one entry without reconstructing the repository.

Owner: orchestrator with historian/cartographer grounding. Identify current exact edge references and select real evidenced, contested and missing-evidence examples. Reviewers establish expected outcomes from sources before measuring the new journey. Preserve captured bytes for repeatable controls without calling them current live state. If a live category is absent, disclose it and use a labeled fixture for that negative case.

Write the runnable story in `genesis/a2o/features/devflow/ceremony-reconciliation.feature`, tagged `@concern:dev-system-equilibrium`, with executable steps in the existing devflow family. It covers the whole journey; later stations add passing behavior rather than new competing finish lines. Bind the governing recipe to existing intent/ruling/ProcessSpec references. Expose defaults, selection/filter/order rules, omissions, dependencies, authored-by and applicable change authority, permitted overrides, execution identity and alternatives. Unknown author/authority/ranking stays explicitly unknown.

### 2. Derive concern groups and local stories

- [ ] The entrant can choose an assertion from a source-grouped concern view, explain why it appeared, and open its purpose, claim, relevant passages, changes, contrary evidence and affected relationships progressively.

Owner: native flow implementer. Compose from current context/reconciliation/edges and bounded excerpts. Group by the shared upstream evidence source while retaining each consuming assertion/edge identity and decision. Unnotarized citation edges still appear as edges; lack of a commitment is disclosed, not solved by manufacturing one.

Show observed coverage and omitted scope separately from population counts. Distinguish unreviewed drift, evidenced repair candidates, substantive conflict and absent evidence with supporting reasons. Candidate classification is advisory until a qualified judgment supports it. Default ordering should be explicit and deterministic over those declared groups; user overrides record their rationale. Semantic ordering remains provider-supplied and opaque unless the provider exposes verifiable reasons.

Initial node detail is compact: original purpose (or unknown), current claim, change summary, evidence status and next useful action. Expansions open exact passages and provenance; histories remain available on demand. Changed/missing passage anchors must request renewed evidence rather than silently opening an unrelated section.

### 3. Complete one repair and one responsible unresolved outcome

- [ ] The agent follows linked actions to inspect, compare, test a prerequisite, repair, request judgment or stop, and obtains a reviewed outcome tied to the original intent with per-edge reconciliation.

Owner: journey implementer with librarian/storyteller judgments. Compose actions over existing flow/executor operations; keep prerequisites, affected scope and approval boundaries visible. Retain orientation while navigating. Mechanical re-verification requires evidence; substantive source rewrites use the existing approval discipline, and normative holds require operator confirmation. Explicitly unresolved is valid without authoring a hold.

Apply one evidenced real repair, independently review the changed claim against live source, and recompute every affected dependent edge. Grouping must not blanket-reseal neighbors. Keep technical review, fulfillment and experiential acceptance distinct. The completion view names what changed, its supporting evidence, residual uncertainty, downstream effects and reusable learning. Continue another justified batch from the same entry; one complete journey is not an invocation cap.

### 4. Preserve purpose through interruption

- [ ] After a deliberate context reset, a fresh agent resumes the same concern with intact intent, verified prior findings, unresolved questions and next justified action, revalidating changed evidence without repeating unchanged investigation.

Owner: recall/runtime implementer. Extend local continuation beyond accounting using exact native references and evidence receipts. Preserve reason-for-following, excerpts already inspected, relevant source/method identity, limits and unresolved frontier. Rehydrate native decisions; do not persist a competing acceptance status. Retain cumulative accounting across compaction, restarts and explicit recipe-version transitions, including the prior receipt.

Exercise two resets: unchanged evidence, then changed source or record set. The latter invalidates only affected conclusions and names why renewed investigation is required. Never silently replay a mutation from a saved next action. Failed/interrupted actions need a native-state check before retry.

### 5. Exercise governed alternatives

- [ ] The agent changes to an authorized retrieval alternative within the same ceremony, retaining intent, navigation, evidence obligations, uncertainty and cumulative provenance without changing ceremony implementation.

Owner: recall/provider implementer. Replace hardcoded provider selection with a versioned provider/dependency declaration added to the existing recipe, reusing bounded executor interfaces. The current provider-name field does not implement this selection mechanism. First support and exercise deterministic local source/graph traversal alongside the existing semantic route. Also use a second conforming provider test adapter to prove substitution does not depend on MemPalace-shaped output; a fixture proves interface compatibility, not another live service's retrieval fitness.

Expose provider identity, available version/freshness, restrictions, returned scope and actual selection source. Refused/unavailable alternatives have named outcomes. Arbitrary command execution is not an authorized provider mechanism. Demonstrate that declining an optional opaque provider leaves the evidenced repair journey usable. Recipe changes retain prior method receipts rather than silently resetting them.

### 6. Independently accept and reconcile delivery

- [ ] An appointed fresh agent exercises the delivered entry, evidenced and contested concerns, interruption/resumption, alternative retrieval and reviewed completion, and records a warranted acceptance or changes-requested outcome on the exact produced scope.

Owner: integration implementer, independent technical reviewer, then a separately appointed fresh experiential agent. First update the authoritative memory-ceremony package so its entry, primary work unit and finish criterion describe the delivered concern journey. Rewrites/head compaction remain available actions/lanes. Project only that surface and any specifically changed role package; verify projections. Preserve Phase 0 intake, useful successive batches, independent judgments, authority boundaries and population-versus-scope distinctions.

Independently review the final integrated tree, including projected instructions, and pass its gates before the final experiential trial. Supply the appointed fresh agent only the entry locator and task intent; do not provide investigation transcripts, source maps or expected verdicts. Record actual actor independence. Use the native actor/appointment/fulfillment/review/acceptance sequence and pinned report format already implemented. Pin the exercised implementation, recipe and projected instructions; changes after the trial require scoped revalidation. Sample downstream use before the chronicle.

## Acceptance and efficiency scorecard

| Trial | Required observation |
| --- | --- |
| Cold entry | Agent states purpose, relevant guiding sources, constraints and finish without a bespoke repository tour. |
| Evidenced concern | Correct per-assertion repair and reviewed downstream reconciliation; untouched conflicting neighbor remains unresolved. |
| Contested / missing evidence | Agent recognizes disagreement or absence, retains contrary evidence and seeks judgment or stops with a named frontier. Zero unsupported acceptance or holds. |
| Explain selection | Agent identifies the applicable rule, evidence relation and excluded scope; says ranking is unknown where appropriate. |
| Context reset | Intent and findings recover; unchanged evidence is not reinvestigated without a named reason. Changed evidence triggers scoped revalidation. |
| Alternative / failure | Recipe-selected alternative completes the same journey; unavailable provider and budget exhaustion remain visible and preserve continuation. |
| Completion | Exact production, review, appointment and acceptance chain; outcome addresses initial intent, with reusable learning and bounded residuals. |

Measure matched tasks before and after with independent fresh readers, consistent revision/environment and declared fixtures. Retain decision correctness, unsupported certainty, source/excerpt reads, repeated investigation with reasons, unique versus total bytes, elapsed time, measurable tokens, review rounds and downstream corrections. Record failures and operator interventions. Do not add overlapping scan/source counters or infer token cost from bytes. Outside-executor reads are separately disclosed; unavailable total tokens remain unknown.

Accept structural delivery only when every required trial has been personally exercised without an unresolved misleading outcome. Report cost alongside correctness; if avoidable rereads or rework persist, identify and repair their cause before calling the experience efficient. Small paired trials support only the sampled tasks. Sustained downstream rework and the habit's stock rates require follow-up observations in existing reports/chronicles; no new dashboard or ledger is created.

## Gates, evidence and stopping

Implementation checks follow touched ownership: `just gate eprfs`; `just gate elohim-epr` if its owning tree changes; `just gate genesis-a2o`; `just gate gherkin-prepush-lint`; focused recall checks via `python3 -m unittest discover -s .claude/scripts/memory-kit/__tests__ -p 'recall*_test.py'`; and `pnpm run elohim-agent:packages:verify` for package changes. Wire and explicitly execute the new devflow scenarios: a lint gate or a registered feature with no steps is not behavioral proof. Add them to the owning gate through its existing manifest/runner if the integration currently omits them. Include adversarial projection, source-change, replay and provider-failure tests without duplicating existing acceptance evaluator internals.

Finish with the exact scoped native reconciliation readout, reviewed source/edge repairs, appointed experiential report, updated ceremony package, one concise ceremony chronicle and a one-line evidence delta in `.epr-meta/dev-system-equilibrium.habit.md`, then re-project habits. Re-mine changed canonical memory surfaces only after their verification as required by the ceremony. No population-wide stasis claim from this sample.

If independent review finds a misleading outcome, repair and rerun the affected journey. If evidence or authority is unavailable, preserve the blocked assertion and complete independent useful work. Stop with scope, evidence and reason when no justified authorized work remains. Do not drain real conflicts merely to reach zero or substitute recipe checks for experiential acceptance.

## Planning verification and limits

Two read-only seam agents grounded native composition and agent interaction; the planner checked their load-bearing source claims, operator rulings, concern canon and ownership constraints. Bounded recall was used for documentary evidence. Governance/code reads and agent contexts were outside executor accounting; total context/tokens are unknown. The setup attempt failed before native execution because dependency DNS was unavailable. Planning does not revalidate current acceptance, edge counts, MAP currency or MemPalace freshness.

The sprint begins with the execution prerequisite above. This document is the implementation plan, not a report that any station has shipped.
