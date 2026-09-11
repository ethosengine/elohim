---
title: Bounded recall mastery — the native memory entry serves the next agent
id: bounded-recall-mastery-sprint
status: landed
landed: 2026-09-11
verified_by: |
  just gate memory-ceremony EXIT=0 on the final tree (51/9/10 native, 34 hooks, 6+4 scenarios); fresh reader 3 reached authority entry-only (3 ops, 8,058 B metered, 0 unmetered, 0 mistaken); habit recall-reaches-authority RED→GREEN on that evidence; chronicle genesis/data/timeline/chronicle/2026-09-11-bounded-recall-mastery.md.
class: devflow
serves: recall-reaches-authority
date: 2026-09-11
cites:
  - "memory-kit-replacement-finish | the sprint this one follows — every parity row native, kit deleted, verdict rule left unmet | sha256:1237a12cebb97666 | path: genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md"
  - "ceremony-efficacy | the five fresh-start questions the habit check answers | sha256:23eb372f4ba66135 | path: genesis/docs/analysis/2026-09-09-memory-ceremony-efficacy.md"
  - "unified-memory-loop-design | the design that names the collective and EPRFS as the two owners of memory | sha256:07e941a325cc49c2 | path: genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md"
---

# Bounded recall mastery

The memory-kit replacement landed 2026-09-11 with every parity row native and the kit deleted.
It served `dev-system-equilibrium`, whose invariant is a stock-drain rate, so the replacement had
no green it could reach: the plan's own verdict rule (a context-reset reader reaches the
authoritative source with fewer mistaken assertions at equal or lower bytes) measures a different
habit. This sprint declares that habit, `recall-reaches-authority`, and drives the native entry
toward it by removing the friction a test-drive on 2026-09-11 found in the agent's seat.

## What the test-drive found (2026-09-11, session `test-drive-20260911`)

| Friction | Evidence | Cost to the agent |
|---|---|---|
| `open --need '<question>'` drops the question | orientation `intent` is the recipe purpose; only `--intent` sets it, and the skill/gospel document `--need` | the session never carries the agent's actual question; every later `remember`/`finish` is against a borrowed intent |
| `.claude/` is outside `source_roots` | `search --search-scope .claude/skills` and `read --path .claude/skills/memory-ceremony/SKILL.md` refuse `outside declared source scope` | the tooling an agent most often needs to recall (skills, hooks, commands, measures) is unreachable through the governed entry; direct reads then go unmetered (station six: honest totals 85 KB vs 18 KB metered) |
| every refusal is a JSON envelope repeating ~900 B of orientation | three refusals in one packet each re-printed intent, guiding context, constraints | noise; the remedy line is buried |
| the human `open` view is 12.6 KB with raw JSON blobs inline | `Continuation:`, `Cumulative:`, `Frontier:`, `Measurement:`, `Page:`, `Omissions:` are `to_string_pretty` dumps | the emitted Linked choices (the good part) sit under a wall; `--json` already serves machines |
| help is an error | `epr`, `epr flow`, `epr flow memory` print usage on stderr with a non-zero exit; only `epr flow` and leaf verbs honour `--help` | an agent discovering the surface pays a refusal per layer |
| the parity fold is 53/53 but `.md` surfaces still instruct deleted scripts | `mempalace-currency.py --remine`, `placement-audit.py --ledger/--focus/--headline/--epr-meta`, `decompose.py`, `scope-reconcile.py`, `memkit-retention.py`, `focus-baseline.py`, `memory-index-projector.py` in the ceremony skill, four memory agents, converge/delivery-stasis/agentic-developer/elohim-epr-metafile/p2p-design-gate skills, brainstorm/plan/shift commands, hook docstrings, `.claude/memory/.epr-meta`, memory entries, the lenses operating map | an agent following gospel runs a command that does not exist and reports having acted |
| `mempalace:` headline red with no named remine verb | 213 of 544 surface files newer than `.last-mine`; the remine wrapper was deleted with the kit and nothing native stamps the marker | the front-link recalls a stale index and the ceremony's Phase 4 names a deleted script |

## One journey, two doors (operator direction, 2026-09-11)

Bootstrapping a session into a concern and reconciling a ceremony's assertion are the same
experience: one algorithm (`recall-contract.json`), one set of mishpat (the `recall-journey`
ceilings in `.claude/epr-meta/measures.yaml`), one set of middot (`recall-metered-bytes`,
`recall-unmetered-bytes`, `recall-mistaken-assertions`, `recall-screens-to-shape`). The
orchestrator's own bootstrapping into this sprint is the first sample: 14 probe rounds and
about 100 KB of unmetered direct reads before the shape was nameable, while the entry meant to
serve that question refused the surfaces it needed. The SessionStart headline is a dashboard of
true numbers with no path; the recall `open` view is the ceremony's stale-edge scan, not the
newcomer's ranked first screen. Both are the same defect seen from two doors.

The two doors, named (operator, 2026-09-11): the **ceremony door** is the systematic review of
the whole — walk the ship, find the two or three bespoke cooling systems in the engine room,
converge their specs, choose a head, file the backlog item. The **sprint door** is one focused
area — cooling for this room: find the competing candidates, name the best one, implement it,
validate it. One entry serves both; what differs is the first screen. Whole-scope `open` shows
stale edges grouped by shared source (the convergence question). Focused `open` (a `--scope
<dir>` or a question that names an area) shows the area's habit and last delta, the competing
sources ranked, and the choices to compare them, choose a head and file the residue. Optimizing
that first screen is optimizing the communication, and thereby the agility, of the whole.

## Delivery stations

- [x] The recall entry carries the agent's question and reaches the tooling layer: `open --need` becomes the session intent when `--intent` is absent (the documented entry works as documented), `source_roots` gains `.claude/` (skills, hooks, commands, agents, epr-meta, scripts, workflows, memory) with the private recall store and `.claude/worktrees/` still refused, and the contract version moves so every receipt pins the new method.

Native owner: `elohim/eprfs/epr-cli/src/flow/memory/recall.rs` and `.epr-meta/elohim/algorithms/recall-contract.json`. Tests in `elohim/eprfs/epr-cli/tests/flow_memory_recall.rs`: a session opened with `--need` and no `--intent` shows that text as `orientation.intent`; a `read` under `.claude/skills/` succeeds and a `read` under `.eprfs/status/recall/` and `.claude/worktrees/` refuses; a receipt's method CID equals the new contract CID.

- [x] Refusals and the human view are legible: in human mode a refusal is two lines (`refused: <message>` and `next: <remedy>`) with exit 2 preserved and `--json` unchanged; the human `open`/`select`/`read`/`resume` renderings summarize continuation, cumulative accounting, frontier and measurement in one line each and print omissions as bullets, with `--json` carrying the full payload.

Native owner: `render()` and the refusal path in `run()`. Tests: human-mode refusal output has no `{`; human `open` output is under 6 KB on the station-five fixture while `--json` still carries `cumulative.totals`; every existing assertion on the JSON view stays green.

- [x] Help is a question at every layer: `epr --help`, `epr flow --help`, `epr flow memory --help`, `epr flow memory recall --help` print usage on stdout and exit 0; a bare `epr flow memory` prints its operation list.

Native owner: `main.rs`, `flow/mod.rs`, `flow/memory/mod.rs`. Tests: one integration test per layer asserting stdout usage and exit 0.

- [x] No projected surface instructs a deleted kit script: the memory-ceremony, converge, agentic-developer, delivery-stasis, elohim-epr-metafile and p2p-design-gate skill packages, the cartographer/librarian/historian/storyteller agent packages and the brainstorm/plan/shift command packages name only native verbs (package-first, versions bumped, projected with `just codegen agents write`, verifier green); hook and `_lib` docstrings, `.claude/memory/.epr-meta`, `.claude/epr-meta/concerns.yaml`, `.claude/subject-routing.yaml`, `genesis/docs/PLACEMENT.md`, the lenses operating map and the memory entries that name kit scripts are corrected; the re-mine discipline is written as the three MemPalace CLI commands plus the marker stamp.

Integration owner: `.epr-meta/elohim/packages/**`, `.claude/**` (not `elohim/eprfs`), `.agents/**`, `.epr-meta/elohim/lenses/*.md`, `genesis/docs/PLACEMENT.md`. Evidence: `grep -rn` for the kit script names over those surfaces returns only historical mentions (dated, past tense) and `node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify` exits 0.

- [x] The first screen answers the question at its door: a whole-scope `open` keeps the grouped stale-edge view (compact); a focused `open --need '<question>' [--scope <dir>]` renders, before any concern groups, the habit(s) the question's terms touch with each one's last delta line, a ranked handful of candidate sources found by the contract's own metadata discovery over `source_roots` (frontmatter title/description match on the question's terms, budget-bounded, provider named), and the Linked choices to open them; the `memkit` headline slot is retired in `report.rs` `HEADLINE_ORDER` in favour of `recall`, which renders the latest `recall-journey` fold so the session's first line is the last journey's honesty reading.

Native owner: `recall.rs` (`open` view composition, reusing `discover` for the ranked sources) and `report.rs` (slot rename). Tests: an `open` with a `--need` naming a habit id shows that habit and its last delta; the ranked sources are inside `source_roots` and carry the discovery receipt; `epr flow report --headline` prints `recall:` as its first line and `skipped` when no journey fold exists.

- [x] The habit is declared, checked and measured: `.epr-meta/recall-reaches-authority.habit.md` names an a2o `@concern:recall-reaches-authority` scenario (a fresh agent opens with its own question, reads a passage under `.claude/skills/`, and finish reports the question and bounded bytes) plus `just gate memory-ceremony` and the four recall-journey folds; the MemPalace index is re-mined and the headline clears; a context-reset reader answers one concrete repository question through the entry alone and its bytes and correctness are recorded as the first evidence delta; `habits-project.py` reprojects.

Orchestrator: habit atom, `genesis/manifests/habits.yaml` projection, chronicle, memory entries for what was learned.

## Reader context (operator direction, 2026-09-11 — held frontier, not a station)

Who is using the tool matters to the defaults it applies: stated and revealed preferences,
capabilities and intent, across agents (haiku → sonnet → opus → fable) and humans of varying
capability. A journey designed without that intent fails in known shapes — audience capture,
filter bubbles, rabbit-hole/radicalization pipelines, epistemic bifurcation. That is why the
recipe is a governable EPR artifact and its application is a mishpat/middot negotiation rather
than a fixed default. The seam is already half-present: the actor sidecar knows
`agent:<role>@<model>` per session and the recipe declares `defaults`, `authored_by` and
`change_authority`, but nothing joins them, so every reader gets the same defaults today. This
sprint's folds are keyed `reader=<who>` and show the same question producing three different
journeys across Fable and two Sonnet readers — the first evidence for the design. Defaults and
presets for a human or agent are a continuous negotiation (capabilities, health, review of
direction, feedback) whose goal is to maximize the agency and usefulness of the one using the
tool. Recorded as a native `ruling` note on this plan; design pass deferred.

Compose, do not re-invent: the client surface already carries a capability spectrum tuned for
the accessibility dimension — the lens gradient `minimal | simple | standard | detail | debug |
trace` (`app/elohim-library/projects/graphos/src/designed/qahal/_lib/types.ts`, atlas §3.8 in
`app/elohim-library/CLAUDE.md`), the viewer `CapabilityTier` (`visitor | engaged | contributor |
steward | elohim-support | child`, `mock-imagodei-profiles.ts`), and the element capability
contract read from `@capabilityMaxLens / @capabilityMaxStimulus / @capabilityTextuality /
@capabilityContrast / @capabilityLocales / @capabilityRequiredStandings` tags by
`app/elohim-elements/elohim-imagodei/cem-plugins/capability-contract.mjs`, with a11y / i18n /
uaPrefs gate fields written by the test runner. The recall reader-context design should key the
recipe's defaults on that same vocabulary (a lens per reader: a haiku-tier or child reader gets
`minimal`, a fable-tier or steward reader may open `detail`/`trace`; stimulus and textuality bound
the first screen's density) rather than minting a second spectrum. The target is each reader's zone of proximal development:
a Fable-tier reader and a Sonnet-tier reader should have tangibly different default experiences
of the same tool — the smaller reader gets the passage located and the one justified next command,
the larger reader gets the ranked candidates and the omissions and is trusted to choose — while
honesty (selection rule, omissions, receipts) stays constant across lenses; only density and
choice-count move. Model presets are comparatively static (per model per domain, shifting with
context budget and memory contributions); human presets are dynamic — a continuous negotiation
between imagodei (who the person is, what they have stated) and sophia (what they have revealed
through mastery, discovery and reflection), reviewed on the folds, revealed to the person so it
can be contested, and re-folded. Note: the plugin header still
cites `specs/2026-05-20-capability-profile-element-contract-design.md`, which no longer exists
(correction recorded).

## Graduation (operator direction, 2026-09-11)

This concept graduates from eprfs to p2p. Today the governable artifact is
`recall-contract.json` serving agents that recall bytes on one machine. It graduates when humans
search, surface and consume content and discover meaning in their communities through the same
kind of artifact, with its mishpat (declared bounds) and middot (witnessed measures) inspectable
and interpretable, so that its fit-for-purpose in context can be judged against the core goal of
human flourishing for the humans and agents of the larger network. The sprint's constraints are
therefore the graduation's constraints rehearsed at machine scale: the algorithm is a
content-addressed EPRFS artifact with declared authorship and change authority; its bounds are
declared rows, never code constants; its measures fold as observations anyone can re-derive; the
first screen prints its selection rule and omissions. None of that changes when the reader is a
human on the p2p side. Only the substrate and the reach do. Recorded as a native `ruling` note on
this plan.

## Execution boundaries

Two seats at once, disjoint write sets as named per station. Native gate:
`env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --test flow_memory_recall --test flow_concerns_corrections --test flow_memory_footprint`
plus `cargo fmt --check` and clippy `-D warnings` on the crate, `EXIT=$?` echoed on its own line.
Integrated gate: `just gate memory-ceremony`. Commits are path-limited to this sprint's write
sets; the dirty elohim-storage tree beside it belongs to another lane and is never staged.
