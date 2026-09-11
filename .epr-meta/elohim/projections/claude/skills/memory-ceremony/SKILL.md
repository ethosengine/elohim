---
name: memory-ceremony
description: Carry a memory concern through purpose, governed collective context, evidence, action, independent review and reconciliation, with resumable projections and explicitly paired burden measurements.
metadata:
  sourceRuntime: claude
  master: package
  governance: "epr:elohim-agent/skills/memory-ceremony"
---
# Memory ceremony — preserve purpose through reconciliation

The ceremony helps the next agent act from current evidence without losing the purpose of the work. Its unit is an assertion and its relationships. Its finish is a warranted, reviewed outcome tied to the original intent, with uncertainty and remaining work visible. Rewrites and recall-preserving head compaction are actions within this journey. Audit counts and smaller memory heads are supporting observations, never acceptance criteria.

## Shared memory and deterministic lenses

The `memory-kit` skill was RETIRED on 2026-09-11 (station six of the memory-kit replacement)
and this ceremony is its only entry point. Shared knowledge belongs to the repository collective;
files/docs/algorithms/code are EPRFS objects governed by `.epr-meta`. Inspect the
static local relationship with `epr flow memory collective`, which carries the
declaration and its native input guides. Live session attribution remains in the
existing actor store. This is a local collaboration policy, not authentication or
network membership.

Collective memory has its own verbs, each taking an authored request file:
`epr flow memory contribute|project|feedback|graduate --input <request.json>`, with
`--session <registered-session>` on every write. They are deliberately NOT recall
operations, so one governed act never has two addresses. `project` exposes qualified
assertions, exact evidence, selection and omissions, and declares itself ephemeral:
save its exact output and pin the saved bytes with `epr flow memory pin` before any
consequential use, because feedback names those pinned bytes as its target. A resumed
investigation re-derives the projection from the same authored request rather than
caching it. Graduation is a read-only repository-reach rehearsal; it never publishes.
The legacy Claude memory symlink already points into tracked `.claude/memory`;
private/native separation is not retroactively established.

Entry captures a burden baseline; finish closes the explicit pair. Set
`--measure-scope authored:<relative-path>` (repeat with authored/projected/retained/
operational categories) at entry to declare the cohort. Otherwise the ceremony scope
is measured, with the recipe's default cohort for repository-wide work. Resume does
not replace the baseline. `measure --phase close` closes measurement independently
of a judgment; `measure --phase baseline` inspects the original observation.
Incomplete or incompatible pairs report no delta. Closing artifacts and unknown
model-token costs remain visible; reduced bytes never substitute for warranted judgment.

### The deterministic lenses, after the kit

`.epr-meta/elohim/lenses/` no longer exists. What it did is in three places, and knowing
which one answers a question is most of the navigation:

**Native verbs — the state machine.** `epr flow report --headline` (the SessionStart budget
line), `epr flow report placement --ledger | --coverage | --focus [--brief] | --stasis` (the
per-file budget, the un-captured queue, the env-scoped testable surface, the composite
context-coverage readout with its ratchet), `epr flow project` (plan/spec to bounded gap-items),
`epr flow report scope` and `epr flow hold --scope` (the env-scope reading and the mover),
`epr flow memory project --index --budget memory-index-bytes@1` (MEMORY.md), `epr flow cites
seal | describe | verify | stamp` (the cite writer), `epr flow concerns --corrections`
(unresolved corrections by identity — Phase 0 below reads this).

**Relocated lenses — no native replacement, declared as foreign measures** in
`.claude/epr-meta/measures.yaml` and living under `.epr-meta/elohim/lenses/`:

| Lens | Reads |
|---|---|
| `memory/memory-review.py` | memory-dir health: index lines vs budget, projected bytes, typed/untyped, stale, index↔file drift |
| `memory/memory-coherence-audit.py` | each entry's `cites:` still resolve; writes the `cites-index.json` the coherence hook reads |
| `memory/dedupe-memory-scan.py` | TF-IDF duplicate-candidate clusters across entries |
| `memory/cleanup-scan.py` | the archive-candidate proposal set (its apply half was RETIRED — accepted proposals route through `epr flow hold`) |
| `memory/path-update-scan.py` · `path-update-apply.py` | rename detection, and the healing edit |
| `gospel/claude-md-audit.py` | CLAUDE.md drift + rightsizing |
| `gospel/substrate-currency-audit.py` | gospel scanned for PATH-EXISTS / process-status phrasing |
| `gospel/locus-drift.py` | per-locus drift roll-up over the cite graph |
| `delivery/story-coverage-audit.py` | story ↔ a2o feature coverage |
| `delivery/delivery-status-distribution.py` | the delivery-status gradient |
| `prior-art/spec-coherence-index.py` | prior-art index — run `--query '<topic>'` BEFORE proposing, and compose rather than re-spec |
| `prior-art/prep-brainstorm.py` | the `/brainstorm` deterministic preload |

Every lens writes under `.eprfs/status/lenses/` — derived, regenerable, untracked, with
`_lib.paths.reports_root` as its one authority. None of them keeps an accumulator; the
accumulated counts are DERIVED from the fold plane by the report.

**The operating map** — tiers, cadence, the disposition discriminators, the hook inventory and
the hard-won gotchas — is `.epr-meta/elohim/lenses/CLAUDE.md` and its sibling `LIFECYCLE.md`,
which moved there with the lenses. Read those rather than reconstructing them here.

## Enter the journey

Start with the delivered agent-facing entry:

```sh
epr flow memory recall open --session <ceremony-id> --need '<the question you carry>'
```

The executor is native: sixteen operations — `open`, `select`, `context`, `read`,
`remember`, `recipe`, `search`, `source`, `history`, `compare`, `resume`, `adopt`,
`prepare`, `reconcile`, `measure`, `finish` — under one session label. Its receipts and
continuation are PRIVATE session records under `.eprfs/status/recall/<session>/`, mode
`0600`, in a terminally ignored directory; they are never imported, projected, witnessed
or targeted by feedback, and every path-shaped input (`--path`, `--scope`,
`--search-scope`) is refused if it reaches into that store. What a session exposes is
what was read and what was concluded, never a reasoning trace.

It supplies **Orient → choose a concern → understand its story → follow evidence → act → review → reconcile**. Orientation carries intent, source-backed guiding values, scope, constraints and a worthwhile finish across navigation. The concern view groups native stale/dangling edges by shared source, with per-edge identities, coverage, omitted scope and linked choices. A stale fingerprint is unreviewed drift; it does not establish either a safe repair or a substantive conflict.

Follow an emitted choice rather than reconstructing command syntax. Use `--json` for structured views. `select` opens the source's section outline and current claim; `read` opens a bounded passage and returns its receipt key. `context` follows native intent, governance, review and acceptance through bounded sections. Follow its emitted section and continuation choices to expand individual records; indexed choices pin the observed context and refuse if it changes. Native `--section <dot-path>` is a progressive projection of the existing context, not a second authority reader. Views expose input-dependent choices for recording findings, preparing an action, or finishing. These choices name their required inputs; they do not author a placeholder judgment.

`remember` retains an investigator finding, inspected receipt references, an unresolved question and the next justified action. Classifications (`unreviewed`, `evidence-ready`, `conflict`, `missing-evidence`) are attributed observations, not authority. Each assertion keeps its own judgment even when evidence is shared. `prepare` produces a scoped native action for review and execution by the agent; preparation does not execute, approve, or bypass its governor. After an authorized effect, `reconcile` rereads affected edges, and `finish` rereads native acceptance while recording the bounded reported outcome. An unresolved stop is valid and does not require a normative hold.

The same concern projection is readable directly as `epr flow context <path> --concerns [--offset N --limit N]`; `--all-states` includes healthy/governed/held edges for exact-slot revalidation. These are read-only projections of the existing graph, not a new queue — and the ceremony reads them in-process, so there is no second executable to locate and no separate stderr budget. If `epr` itself is missing or stale, build it through the owning `just gate memory-ceremony` and inspect `epr doctor`; do not interpret a failed query as no work. For repository Git ownership, use the command-scoped safe.directory environment described by the workspace governance rather than modifying shared global configuration.

## Recipe, evidence and continuity

The `recipe` choice exposes the authored defaults, selection/omission/order rules, source scope, dependencies, authorship, applicable change authority, permitted changes, actual method identity and provider restrictions from `.epr-meta/elohim/algorithms/recall-contract.json`. That EPRFS-governed algorithm content artifact owns the machinery under the journey, and its raw CID is the `method` pinned on every session and every receipt. Existing ProcessSpec, Intent, content references, rulings, review and acceptance records remain authoritative; no new protocol kind or peer authority is implied.

`search` chooses an authorized recipe provider. Deterministic local traversal remains available when optional MemPalace is unavailable or declined. Opaque ranking/version/freshness stay visibly unknown. A local provider fixture proves interface interchange only, never live semantic fitness. Returned candidates require source verification; tags locate evidence and never grant acceptance.

Use one session for a coherent investigation. `resume` restores intent, the selected concern, findings, evidence receipts, open questions and next action; it rereads native standing and checks bounded batches of source receipts. Follow the next revalidation offset when a batch is incomplete. Changed evidence invalidates dependent findings. Findings pin the inspected bytes; rereading changed evidence cannot validate an old judgment. The latest explicit judgment controls repair readiness. Use `history` to page earlier findings and questions, and `compare --evidence <receipt-key>` to expand the prior and current inspected passages. Current native state always wins over the cached selection; an absent, changed or ambiguous slot needs renewed investigation. Reopening never replays a mutation.

Method changes refuse silent continuation. Use `adopt --from-session <prior-id> --session <new-id>` to retain prior method/accounting receipts explicitly, then revalidate. Local continuation has a bounded storage envelope; if it refuses growth, keep the existing receipt, start a scoped follow-up and reference the prior investigation. Never reset accounting to hide repeated work. Continuation is private local accounting/investigation, not another work queue or acceptance ledger.

For additional bounded evidence questions, open a separate session label on the same
executor: `epr flow memory recall open --session <packet-id> --need '<question>'`, then
`search --search-scope <dir> --name '<glob>' --query <term>` for metadata candidates,
`source --path <path>` for a section outline, and `read --path <path> --lines START:END`
for the bounded passage and its receipt key. One executor now serves both the ceremony
and its evidence packets, so the method pin is the same contract CID; keep the session
LABELS distinct so each investigation's accounting stays its own. Retain their receipts
together in the final scope/cost account. Widen only for a named unresolved question;
budgets bound each context load, not the ceremony's number of useful batches. Direct
shell/MCP/code reads remain outside executor enforcement and must be accounted
separately. Unknown token/context measurements stay unknown.

## Phase 0 — witnessed corrections first

Read `epr flow concerns --corrections [--since YYYY-MM-DD] [--json]` before population scans. It lists every unresolved correction BY IDENTITY — a `run:correction` with no later note carrying `closes:<its exact CID>` — so a date, a newer chronicle or a `failed-approach` note never closes anything, and `--since` filters the display only (suppressed rows are counted, not resolved). Witnessed staleness outranks scan guesses. Notes use:

```sh
epr flow note --on <surface> --kind correction --reason 'STALE (<date>): "<exact claim>" → <current truth> (<evidence>)'
```

Keep one note per claim, including the quoted claim, replacement truth and evidence. Every unresolved correction remains a candidate across successive working batches. For the selected scope, read `epr flow context <path|cid> --json` and its human rendering. Follow actual fulfillment, technical review, appointment and acceptance evidence. Preserve `acceptance-unestablished`, `revalidation-required`, `changes-requested`, `contested`, historical candidates and missing evidence. Paths and gap labels do not establish identity; local appointments do not establish peer authority.

Correction closure is an explicit act. Close one with a later note on the SAME subject
naming the exact correction:

```sh
epr flow note --on <surface> --kind observation --closes <correction-CID> \
  --reason '<verified explanation, its evidence path and that evidence body fingerprint>'
```

A closure may be an `observation`, `ruling`, `verdict` or `correction`; a
`failed-approach` closes nothing. Retain correction CID, exact target, evidence
fingerprint and a nonempty reason; changed or missing evidence reopens it. Because a
repair changes the bytes the correction was written against, closure matches on the
subject label rather than the resource CID — the `closes:` CID is what names the exact
correction. **A closure written as chronicle frontmatter is no longer read**: the eight
historical `stale_record_resolutions` entries were migrated into native closure notes,
and any NEW closure must be a note. Never hand-author fingerprints.

## Phase 1 — triage and orient without losing the selected concern

Run `substrate-currency-audit.py` and read native `epr flow status --json`. Report population drift and native sealed/governed/stale/held/dangling counts separately from the selected concern and returned page. Show the top five scanned surfaces as information; select justified work autonomously, prioritizing witnessed corrections. Bare-filename path noise is not proven drift. A clean or unjustified candidate set can warrant stopping without another approval ritual.

Carry two standing checks alongside the selected concern:

- **PATH currency:** librarian/historian verify `genesis/docs/content/elohim-protocol/architecture/MAP.md` against seeds/INDEX, walk links and the substantive gap ledger. Use `the folds on map-currency-drift@1` or bounded seed evidence. Mechanical links alone do not prove conceptual coverage.
- **PRIORITIZATION currency:** cartographer checks `genesis/data/timeline/roadmap/vision-readiness-sprint-roadmap.md` against placement ledger/focus, guiding vision and native selected-scope context. Regenerate/restamp only from the actual inputs; a new date cannot strengthen stale evidence. Separate production, technical review and experiential acceptance. Apply structural corrections within authority; substantive reframing follows Phase 3.

Read ``epr flow memory project --index --json` (unloadedRows)`. `index_unloaded > 0` selects head compaction for the next batch; otherwise use concern reconciliation/gospel repair. Re-evaluate after each batch. Work in 1–2 surfaces per batch by default, continuing justified authorized work autonomously. No one-item invocation cap and no silent budget resets.

## Phase 2 — four judgments, shared investigation

Use one bounded evidence packet: exact claim/correction, source path and passage or CID, verified fact, uncertainty and unresolved frontier. Four lenses are required judgments, not four repeated source-loading passes:

- **Librarian:** verify factual claims, paths, citations and current evidence; distinguish missing, drifted, unverified and forbidden process-status phrasing.
- **Historian:** check causal precedent and missing discipline; preserve historical facts rather than promoting them to current acceptance.
- **Cartographer:** check affected relationships, current substrate coverage and implications for next work.
- **Storyteller:** check purpose, narrative coherence, vocabulary, framing and recall preservation.

Routine uncontested work may use one investigator carrying all four lenses. Contested facts require independently dispatched judgments, with shared excerpts and only each lens's new delta. Role details live in `.claude/agents/{librarian,historian,cartographer,storyteller}.md`. Explicitly name an additional evidence need before expanding a packet. Maintain existing subject/contribution tags per `genesis/data/timeline/CONVENTIONS.md`; assertions, not whole tagged documents, are the evidence.

For head compaction, storyteller owns disposition: memorialize into an existing umbrella, graduate a lesson to a canonical story, or retain it. Librarian applies topic-file metadata and reprojects with `epr flow memory project --index --budget memory-index-bytes@1 --out .claude/memory/MEMORY.md` (the PostToolUse router `.claude/hooks/memory-index-projection.py` takes the same leg, falling back to `memory-index-projector.py --apply` while the native projection is over the hook's latency budget). A projected row comes from a CONTRIBUTION, so an entry the ceremony just wrote reaches the index through `epr flow memory import .claude/memory`. Preserve candidate-to-surviving-source mappings and the corrected lesson. Staleness alone does not authorize removal. Feedback remains unless a surviving entry genuinely duplicates it. Routine byte-budget and archive hygiene belong to the hygiene lenses at `.epr-meta/elohim/lenses/`.

## Phase 3 — concrete decisions and authorized action

Storyteller synthesizes substantive rewrites or compaction dispositions in a clean context carrying the target, verified packet, lens deltas and authorized scope, not investigation transcripts. Preserve useful structure, hedges and source-backed meaning; do not turn uncertainty into manufactured confidence or add boilerplate. Each addition must have evidence. Present the concrete rewrite with a short change rationale.

Apply already-authorized substantive work without asking again. New substantive choices require approve/revise/decline on the concrete result. If approval is needed, name this skill's Phase 3 requirement and explain the uncovered scope. Mechanical adjacent evidenced corrections proceed within existing authority. A proposed normative `epr flow hold` always requires operator confirmation; a hold declares policy, not hygiene. Keeping an unresolved frontier is different and needs no policy mutation.

One evidence-ready edge never authorizes blanket resealing a shared-source group. Reverify and reconcile each affected assertion. Classifications, prepared commands, passing tests and stale counts cannot substitute for independent review or acceptance. Optional dynamic workflows require operator opt-in because they spawn many agents; conversational successive batches remain the default.

## Phase 4 — verify the experience, reconcile and retain learning

After applying an authorized change, two independent verification lenses must clear GREEN or YELLOW-resolved:

1. **Diff regression:** independent code-reviewer checks changed/added/dropped claims against live source, including production versus fixture paths and citation targets.
2. **Downstream coherence:** a fresh-context implementation reader receives the changed primer and relevant source routes for a plausible next task. It checks contradictions, missing evidence, authority ambiguity and misleading duplication. It does not inherit the investigation or execute the imagined sprint.

RED means the work is not done. Repair, reapply and reverify; resolve mechanical YELLOW inline and route substantive findings through existing authority. Preserve uncertain scope instead of declaring population-wide coherence from a sample.

For ceremony-interface changes, project/review the final authoritative package and pass owning gates **before** appointed experiential acceptance. Register a distinct acceptor, appoint it to the exact commitment, and have it personally exercise entry, evidenced and contested concerns, selected-edge explanation, context-reset resumption, provider alternatives and warranted completion. For native writes attributed to a registered actor, use `--session <registered-actor-session>` without `--as`: that literal override omits the exact actor-claim linkage. A recall session is an investigation locator; it does not register an actor. Pin implementation, recipe, projected instructions, report, revision/environment, observations and limits. Later material changes require scoped revalidation. A technically approved build cannot supply that experience.

Chronicle once justified authorized work reaches its stopping condition in `genesis/data/timeline/chronicle/<date>-substrate-currency-<slug>.md`. Keep the body about 150–300 words; git diff is the detailed change record. Frontmatter records: kind/status/date, ceremony, surfaces_rewritten, diff_review_verdict, coherence_verdict, sampled topic, the correction CIDs this run closed (the closure itself is the native note, not the frontmatter; `stale_record_resolutions` remains only as the historical field), measured agent minutes/surfaces/context/tokens (unknown where unavailable), recall correctness, review rounds and rework. Report selected scope, native acceptance states, evidence examined, unresolved frontier and PATH/PRIORITIZATION readouts. Record one evidence delta in the owning habit and reproject; never flip its status on intention.

When verified canonical memory surfaces change, run `mempalace-currency.py --remine` and check `placement-audit.py --headline` for freshness. A failed refresh stays visible; do not claim the provider current or erase successful local work. No automatic diary/curation outside this authorized maintenance scope. Reuse the existing chronology and flow records rather than creating another ledger.

Measure correct decisions, unsupported certainty, unnecessary context loading, repeated investigation with reasons, preserved uncertainty and downstream rework. Separate unique/total source bytes and overlapping scan counters; metered tokens support the result but never define Sacred Attention. Stop when no justified useful authorized work remains, a genuine authority/environment boundary prevents it, or diminishing returns justify it. Name the frontier and reason, and close orchestration tasks. A smaller index or a lower stale count alone never establishes success.
