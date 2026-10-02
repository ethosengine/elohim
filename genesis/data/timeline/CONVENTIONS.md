# Timeline — Conventions

A single collection of dated, frontmatter-shaped entries that the memory team writes into. Three **kinds** (`chronicle`, `roadmap`, `backlog`) share one storage shape; the three classic **views** (timeline, roadmap, kanban) are projections over the collection, not separate stores.

This catalog is **placeholder-grade for now**. The shape will evolve when we revisit it after the storyteller's first canonical work. The current goal is one directory, one schema, three kinds — enough to start writing entries without devolving into ad-hoc files.

## Philosophy — one collection, many views

The pattern Linear, Notion, GitHub Projects, and Airtable all converge on: store items as first-class objects with a status enum, a kind, dates, and links to related objects. The "view" is a query.

| View | What it shows | The query |
|---|---|---|
| **Timeline** | What happened, chronologically | `kind=chronicle`, ordered by `occurred_at` |
| **Roadmap** | Where we're going, by horizon | `kind=roadmap`, grouped by `target_window` |
| **Kanban** | What's in flight, by status | `kind=backlog`, grouped by `status` |
| Cross-cut | Everything on a theme | filter by `tags` or `relatedNodeIds`, any kind |

One storage shape. Three rendering options. Add a fourth view later (Gantt, dependency-graph, valueflow attestation map) without changing storage.

## The three kinds

| Kind | Owner | Tense | Examples |
|---|---|---|---|
| `chronicle` | **historian** | Past — what happened that's worth remembering | "MinIO replaced Garage as sccache substrate (2026-05-09)", "iroh Phase 11 — all six backends wired", "James story landed as first canonical experience-story" |
| `roadmap` | **cartographer** | Future direction — theme-shaped, not task-shaped | "Complete iroh cutover by Q3 2026", "Memory team triad becomes triadic operating system", "Move sccache to elohim-native quilt" |
| `backlog` | **cartographer** | Near-future, ready-to-execute Objective candidates | "Write canonical story for James's recovery", "Add validate-stories.ts pre-push gate", "Wire mempalace_sync into librarian's cleanup ceremony" |

The librarian and storyteller don't write into this catalog directly. They produce signal:
- Librarian's cleanup/dedupe surfaces *what's stable enough to chronicle*.
- Storyteller's "needs memorialization" list (memory sprint output) feeds the cartographer's backlog as candidate Objectives ("write a story that covers X").

## File layout

```
genesis/data/timeline/
├── CONVENTIONS.md          # this file
├── INDEX.md                # cross-kind index (historian + cartographer co-maintain)
├── chronicle/
│   └── YYYY-MM-DD-<slug>.md     # date prefix for chronological scan
├── roadmap/
│   └── <slug>.md                 # date-agnostic; target_window in frontmatter
└── backlog/
    └── <slug>.md                 # date-agnostic; priority + status in frontmatter
```

Subdirectories are for human-scannable navigation. The underlying query model treats it as one flat collection — `find timeline/ -name '*.md'` walks all entries; frontmatter `kind` disambiguates.

## Frontmatter schema (common)

Every entry begins with YAML frontmatter, then a markdown body.

```yaml
---
# ContentNode-aligned identity (when we seed these into DHT, this becomes the ContentNode)
id: "<kind>-<slug>"                   # e.g. "chronicle-mempalace-wired" / "backlog-james-story"
kind: "chronicle" | "roadmap" | "backlog"
contentType: "<kind>-entry"           # chronicle-entry | roadmap-item | backlog-item (lamad manifest extension TBD)
contentFormat: "markdown"

title: "Human-readable title"
slug: "kebab-case-slug"               # matches filename minus extension/date-prefix
written: "YYYY-MM-DD"                 # when this entry was written
author: "historian" | "cartographer" | "<operator-name>"
status: <kind-specific; see below>

# composition — what this entry references (becomes :relates-to links when seeded)
relatedNodeIds: []                    # ids: human-*, story-*, epic paths, feature paths, memory entry slugs

# free-form tags for theme grouping (cross-cut views)
tags: []
---

# Body — markdown narrative (length matches kind; see below)
```

## Kind-specific frontmatter

### `chronicle` (historian)

```yaml
kind: "chronicle"
contentType: "chronicle-entry"
status: "noted"                       # noted | superseded | retired
occurred_at: "YYYY-MM-DD"             # when the event happened (may predate `written`)
significance: "small" | "meaningful" | "significant"
```

Body length: 100–500 words. What happened, why it mattered, what it changed. Reference stories/epics/memory entries via `relatedNodeIds`.

**Horizon-scan reference (memory-ceremony chronicles only)**: when a memory-ceremony chronicle is written, append a `## Horizon-scan reference` section near the end with:
- **Latest scan**: link to `.claude/memory-kit/horizon-scans/YYYY-MM-DD.md`
- **Next recommended scan**: date (90 days from `scanned_at`) — this is the **trigger** future ceremonies read to decide whether cartographer should re-scan
- **Trigger** clause: one line stating "if today >= next-recommended and latest scan is still this one, invoke `/mem-horizon-scan` before Wave 1 surface"
- **Summary**: 4-sentence quote of the scan's Summary section (the deterministic short form so the chronicle is self-contained without reading the full report)
- **Top elevation candidates**: 1-3 line summary of the scan's elevation list

This makes every memory-ceremony chronicle a self-contained pointer-with-summary that tells the next ceremony's cartographer when and what to re-scan, without forcing them to read the full horizon-scan report. See `genesis/data/timeline/chronicle/2026-05-14-first-memory-team-ceremony.md` for the canonical pattern.

### `roadmap` (cartographer)

```yaml
kind: "roadmap"
contentType: "roadmap-item"
status: "proposed" | "active" | "achieved" | "abandoned"
target_window: "2026-Q3" | "2026-H2" | "1-3 months" | "open-ended"
themes: [...]                         # high-level groupings; cartographer-curated
```

Body length: 200–800 words. What direction, why it matters, what it would feel like to have achieved it. Roadmap entries are theme-shaped, not task-shaped — they describe a *direction*, not a *deliverable*.

### `backlog` (cartographer)

```yaml
kind: "backlog"
contentType: "backlog-item"
status: "envisioned" | "backlog" | "refined" | "wip" | "active.alpha" | "active.beta" | "active.latest-stable" | "stable" | "regression"
priority: "high" | "medium" | "low"
regression_from: "active.latest-stable"  # ONLY when status == regression; preserves the level to repair back to
shift_objective: |                    # ready-to-paste Objective for /shift
  <draft objective text>
```

Body length: 300–1000 words. The full Objective draft + readiness notes (what's blocking, what's ready, who knows the area). When `/shift` is invoked, the operator (or cartographer) can lift `shift_objective` directly into the shift kickoff.

**Status enum extension (2026-05-14 Run #2)**: backlog `status:` is extended from the legacy {proposed → ready → in-shift → completed → closed} to the full unified delivery-status gradient:

```
envisioned → backlog → refined → wip → active.alpha → active.beta → active.latest-stable → stable
                                                                                     ↑
                                                                              regression (sideways)
```

This is the **same lifecycle** stories and feature files use. One vocabulary, three artifact types; see [[feedback_story_delivery_status_axis]] and `.claude/scripts/memory-kit/LIFECYCLE.md` "The author/delivery axis split" section.

**Legacy migration map** (cartographer reads when encountering pre-Run-2 backlog entries):
| Legacy | New |
|---|---|
| `proposed` | `backlog` |
| `ready` | `refined` |
| `in-shift` | `wip` |
| `completed` | `active.alpha` (`/deliver` verdict expected next; may bump higher) |
| `closed` | `stable` (or archived if abandoned without delivery) |

**Authority boundary**: cartographer owns the upstream states (`envisioned`, `backlog`, `refined`); sprint/agentic-developer brings entries into `wip`; **`/deliver` is the only authority that can mint `active.*`, `stable`, or `regression`**. The memory-ceremony group does not author these states; it reads `/deliver`'s tier-3 verdicts via the `deliver-bridge` auto-poller (`.claude/scripts/memory-kit/delivery-status-poll.py`), which writes the new status onto the backlog frontmatter. See `.claude/scripts/memory-kit/LIFECYCLE.md` for the full ownership matrix.

`regression` is orthogonal-sideways: when `/deliver` re-judges a previously-delivered feature as `partial`/`error_state`/`missing` after a prior `delivered`, the entry flips to `regression` and `regression_from` preserves the prior level. The historian surfaces these as risk-precedent annotations; the cartographer ranks them high in next-actions for repair.

## In-flight classification — reusable categories on existing sources

Classification is part of maintaining a concern while working on it. Reuse the source's
existing `tags` vocabulary; do not create another backlog, roadmap, or copied narrative
just to make a discovery retrievable. These are cross-cutting categories, distinct from
an entry's `kind`, lifecycle status, and the `@concern:` identifier joining assertion to proof.

Use two questions: **what is this about?** (reuse the subject tag already used by its cluster)
and **what does it contribute?** The following contribution tags have narrow meanings:

| Tag | Apply when the source contains |
|---|---|
| `risk` | A foreseeable failure condition; retain its trigger and mitigation (see below) |
| `decision` | A choice with rationale and a link to its actual decision authority |
| `constraint` | An evidenced operating limit or required precondition, with applicability |
| `lesson` | A causal explanation of what happened and what a future agent should do differently |
| `open-question` | A specific unresolved question and the evidence or reviewer needed to answer it |

Use only tags supported by the content. An emerging concern without a measurable trigger
can be an `open-question`; do not manufacture a number to qualify it as a risk. These labels
are a starter vocabulary, not an exhaustive ontology. Before adding a synonym, inspect nearby
sources and reuse their term. Add a category when a concrete retrieval question needs it;
record its meaning here rather than silently assigning several meanings to the same label.

Any agent may classify an existing source within its authorized editing scope while doing
useful work. Preserve the source's format: do not add unsupported metadata to code or invent
native EPR kinds. If the source cannot carry tags, classify its existing owning document and
point to the exact assertion, section, or symbol. A document tag selects a candidate document;
it does not classify every row inside it. Keep decisions and acceptance in their authoritative
records, linked from the categorized source. Tags never confer verification or acceptance.

For recall, select candidates by category and subject, then inspect the matching assertion,
its evidence date, and authority. Semantic similarity can supplement discovery; it cannot
prove exact tag membership. Check that the changed source is in the index's actual ingestion
scope before claiming it searchable, and refresh only the relevant supported index when needed.
A useful ceremony checks whether a fresh reader can answer a concrete question from these
sources, and merges redundant labels encountered in flight while preserving historical meaning.
No population-wide retagging is required before continuing useful work.

## Performance — a view, not a kind (tag `performance`)

A **performance concern** is an entry whose subject is the resource cost, elapsed time, throughput,
capacity or scaling behaviour of a system we run or build with, or the inability to measure such a
cost. A correctness bug that merely surfaced as a timeout is not one. The **performance view** is the
query `tags contains performance`; the mechanism tags below narrow it. Each entry carries
`performance` plus one to three mechanism tags, and keeps the layer tag its cluster already uses
(`conductor`, `kitsune2`, `elohim-storage`, `doorway`, `ci`, `a2o`, …).

| Tag | The entry is about |
|---|---|
| `perf-cpu` | CPU saturation, spin, throttling, repeated unnecessary compute |
| `perf-memory` | Heap or RSS growth, leaks, OOM, per-agent memory cost |
| `perf-io` | SQLite, disk or lock contention, WAL, store growth, pool saturation |
| `perf-latency` | Per-call cost or wall-clock on a path someone waits on |
| `perf-queue` | Admission, backpressure, starvation, retry storms, livelock — work that amplifies or blocks other work |
| `perf-convergence` | Time for peers, heads or projections to converge; gossip rounds, quiesce, catch-up |
| `perf-scale` | A growth law: cost ∝ chain length, peers, agents, corpus or history |
| `perf-telemetry` | A cost we cannot observe or attribute; a missing or misleading metric, probe or profile |

The prefix exists because the bare words were already taken (`memory` means the memory team here).
The tags describe the mechanism an entry reports, not a verdict: they never mean measured, confirmed
or cured. Read the entry's evidence for that. A `perf-scale` entry with a measurable trigger that has
not fired also carries `risk`. Index and profiling bootstrap:
`backlog/performance-concern-index.md`.

## Trust context and friction — the gradient view

Trust prices compute: verification is paid once, close to the human judgment, and cost should fall
as trust rises (`trust-as-efficiency-signal`). Two more tag families make that readable against the
performance view. Each answers one question about **the path an entry is about**.

**Who stands at each end?** The relationship that already holds there — not the content's reach, and
not whether the code honours it today.

| Tag | The two ends are |
|---|---|
| `trustful-self` | One steward: a process and its own store or source chain, a storage peer and its own conductor, a doorway and the node that backs it, one human's own registered devices. Nothing is left to earn |
| `trustful-declared` | Distinct parties in a standing declared relationship: household peers, the genesis pair, steward peers, fleet nodes that pre-agreed to replicate, a hosted human and their doorway |
| `trustful-earned` | Parties whose standing was earned on the commons: attested stewards, earned reach, a delegate under a scoped grant |
| `trustless` | Strangers: first contact, an unknown or newly joined peer, an anonymous browser, unwitnessed or contested content. Full ceremony belongs here |

An entry whose path serves both kinds without separating them, or that has no relationship on it
(build cost, tooling, hardware), carries none of these.

**What kind of cost is being paid?**

| Tag | The cost is |
|---|---|
| `friction-verify` | Re-deriving or re-checking a fact one end already holds or that was already verified: per-call authorization lookups, re-reading a whole chain, signature or witness verification, re-resolving instead of carrying a proof |
| `friction-wait` | Waiting for agreement or readiness: a second convergence, a poll or retry ladder, quiesce, catch-up, a gossip round, a cold warm-up |
| `friction-mechanical` | Plain resource engineering: leaks, lock or pool contention, livelock, backpressure storms, write amplification, OOM. It would cost the same between any two parties |
| `friction-blind` | We cannot see or attribute the cost |

**Reading the gradient.** A `trustful-*` entry paying `friction-verify` or `friction-wait` is a
candidate for compression: a carried proof, one trust act per relationship instead of per item. The
ceremony is compressed, never skipped — a proposal that goes fast by weakening a check is the wrong
shape. `friction-mechanical` is owed unconditionally whatever the relationship, and is never deferred
to trust. `trustless` with `friction-verify` is the ceremony standing where it belongs.

The dataplane's trust handshake is a stub that classes every sync edge `public`
(`sync-edge-susan-timeouts-per-edge-observability`), so every path is trust-blind today. These tags
name the relationship that holds, which is what a priced path would read.

### Custody shape — who carries it

Sharing load is the other way cost falls. A third question, asked only where the answer drives the
cost or the design: **how widely is the thing held, and is that breadth bought as headcount or as
independence?** Custody is its own plane — it is neither reach (audience) nor head (version).

| Tag | The entry is about |
|---|---|
| `custody-everyone` | A cost paid because every participant holds or validates all of it: full arc, flood gossip, a copy per hosted agent |
| `custody-subset` | Holding by a chosen subset: an arc, a shard, a ring — as a lever tried or a design proposed |
| `custody-diverse` | Choosing holders for independent failure domains instead of for number |

A `custody-everyone` entry asks what the smallest holder set meeting the object's resilience need
would be. An entry that records breadth was *not* the driver should say so in its body; that is as
useful as the tag. The operator's statement of the principle is
`backlog/commons-holonic-stewardship-backlog.md` row 30.

### Plane — which part of the object is doing the work

An object on the network is several planes, and each has its own natural price. Bytes are large and
self-certifying, so they are cheap to verify even from a stranger. Meaning is small and takes the
ceremony. Separating them lets each take its cheapest shape. The tag names **the plane whose work
consumes the resource**.

| Tag | The plane |
|---|---|
| `plane-bytes` | Content bytes: blobs, shards, transfer |
| `plane-head` | Which version is current: declaration, election, adoption, anchors |
| `plane-authority` | Who may act: capability grants, delegation, signatures, identity binding |
| `plane-notary` | DHT ops: publish, validation, receipts, gossip, the source chain as machinery |
| `plane-custody` | Who holds what: inventory, provider records, arcs, placement |
| `plane-projection` | The derived local view: SQL projection, projector, reconcile, render |
| `plane-reference` | Edges between objects: relationships, paths, links |
| `plane-attention` | Views, feedback, private observation |
| `plane-value` | Economic events, commitments, flows |

`fused-planes` marks an entry where one plane's price is charged on another plane's path: an
authority lookup paid on every data read, a head convergence delaying bytes that already arrived.
It is the marker to read first, because the cure is usually to let the cheap plane stop waiting on
the expensive one. The operator's worked example of separating custody from identity and meaning is
`backlog/arch-confidentiality-plane-backlog.md` row 12.

### Cost unit, phase and lane

Three shorter questions complete the view.

| Question | Tags |
|---|---|
| **What does the cost multiply by?** | `unit-once` (a boot, a deploy, a build) · `unit-call` (every call, request or sweep tick) · `unit-item` (per item, blob, shard, op or row) · `unit-peer` (per peer or edge) · `unit-agent` (per hosted agent or cell) · `unit-history` (grows with a chain, a log or accumulated history) |
| **When is it paid?** | `phase-steady` (at rest or ordinary load) · `phase-transition` (restart, cold start, deploy, roll, a peer joining, catch-up) · `phase-growth` (appears only as corpus, chain, peers or agents grow) |
| **Who is waiting?** | `lane-interactive` (a person or foreground caller) · `lane-background` (only sweeps, reconcile, gossip or heal are delayed) · `lane-borrowed` (background work consuming capacity a foreground path then waits on) · `lane-operator` (a developer, pipeline or gate) |

The unit is where conversions show: per item to per peer, per call to once, per history to a bounded
range. Phase matches the cure to the moment — steady cost is optimised, transition cost is staggered
or warmed, growth cost needs a different algorithm. `lane-borrowed` is the lane to read first.

The whole family — performance, trust context, friction, custody, plane, unit, phase, lane — is the
**gradient view**. The `gradient-reading` skill holds the method for reading costs and for declaring
an object at design time, and says how to brief a reader for a bulk labelling pass.

## Risks — a view, not a kind (tag `risk`)

A **project risk** is a concern with a *measurable trigger that has not fired yet*. It is not a
fourth kind: it is a `backlog` row (in a cluster, per `backlog/CLUSTERS.md`) carrying the tag
`risk`, and the **risk view** is the query `tags contains risk` — exactly as the timeline, roadmap
and kanban views are queries over the one collection. First cluster: `backlog/arch-scale-risk-backlog.md`.

**Row shape** (a cluster table, one row per risk): `# | Risk | Where (file + symbol) | Trigger
(measurable) | Horizon | L / I | Mitigation → graduates to | Status`. The trigger is a number
someone can read off a receipt or a probe, never an adjective. The horizon names the roadmap
`target_window` or the milestone that first exercises it. The mitigation names the smallest change
that retires the row and the surface it graduates to (an epic gap id, a plan task, a habit `guard:`).

**Filed when the code lands, not when it bites.** The reader is a reviewer or an implementer, so a
risk is written at the moment its shape is visible in a diff — a full-chain walk per page, a
per-record DHT read, a fan-out ∝ peers × records — with the scale at which it was measured.

**Surface and recall — five fire points, all pre-existing.** Storing a risk is not recall; a row is
wired into each surface it deserves, in the same pass:

1. *At the point of touch* — an `inject` rule in the nearest `.epr-meta` (`when: { write: <file>,
   contains-any: [<symbols>] }` + `dedupe-of: <cluster path>`) so the next editor of that code
   sees the row. Never blocks; fires only on that code.
2. *At planning* — the `risk` tag is what `/converge` and the historian's precedent search read;
   the p2p-design-gate's head-plane question cites the matching row. Run the MemPalace sync after
   filing so the pickup hook can recall it.
3. *At session start* — a risk that threatens a declared habit is a numbered entry in that habit
   atom's `guard:` (precedent: `governance-plane-single-evaluator`), rendered by `just status
   habits --full`. A risk with a runnable trigger probe may instead be born as an `unwired` habit.
   No new headline line: the habit register is the surface with a reader.
4. *Every measured run* — the a2o receipt that exercises the code carries the trigger number
   (elapsed, count, RSS), so the row's evidence accrues without anyone re-reading it.
5. *When it fires* — the row flips to `status: regression` with `regression_from` and the receipt,
   and the historian writes the chronicle entry (two entries, one moment). The row is retired only
   when the mitigation lands, never because it fired.

The cartographer owns the rows (backlog authority); any agent may file one at landing time.

## Ownership and write rules

- **historian** writes under `timeline/chronicle/` only. May not write under `roadmap/` or `backlog/`.
- **cartographer** writes under `timeline/roadmap/` and `timeline/backlog/`. May not write under `chronicle/`.
- **storyteller** does not write into timeline at all. Storyteller surfaces "needs memorialization" candidates that the cartographer may convert to backlog entries.
- **librarian** does not write into timeline. Librarian produces signal (drift counters, cleanup-scan output, dedupe candidates) that feeds historian (chronicle-worthy moments) and cartographer (backlog candidates).
- **operator** may write any kind, override any status, retire any entry.

Entries start at the upstream end of their kind's lifecycle: chronicle at `status: noted`, roadmap at `status: proposed`, backlog at `status: backlog` (or `envisioned` if it's a vision-tier idea not yet refined). They require operator confirmation before they shape downstream behavior — a backlog entry shouldn't be picked up by `/shift` until it reaches `status: refined`, and `/deliver` won't promote past `wip` until its tier-3 verdict fires.

## INDEX.md

A single cross-kind index, co-maintained by historian and cartographer. Minimum sections:

- **Chronicle by significance** — `significant` events surfaced first, then `meaningful`, then `small`
- **Active roadmap themes** — `status: active` roadmap items, grouped by `target_window`
- **Backlog ranked** — `status: ready` items first, then `proposed`, by `priority`
- **Recent achievements** — `status: achieved` roadmap items and `status: completed` backlog items from the last 90 days (the system's own success log)
- **By related ContentNode** — for each story, epic, or feature with timeline entries, the entries that reference it

Like the stories INDEX, this is curator-tended for now. A future generator can derive it from the entries.

## Relationship to other catalogs

- **Stories** (`genesis/data/stories/`) — narrative anchors. Timeline entries reference stories via `relatedNodeIds`. A chronicle of "the James story landed" links to the story id; a backlog entry of "write a story for X" links forward.
- **Memory** (`.claude/memory/`) — operational knowledge. Chronicle entries may graduate memory entries (the librarian's archive becomes safe when a chronicle preserves the moment). Backlog items may resolve memory entries flagged as TODOs.
- **Plans** (`genesis/plans/`) — implementation specs. Backlog entries are *upstream* of plans: the backlog says "do X"; the plan says "here's the design for X." A backlog entry's `status: in-shift` typically pairs with a plan being authored or already complete.
- **Epics** (`genesis/docs/content/elohim-protocol/`) — manifesto philosophy. Roadmap entries reference epics they advance toward.

## Validator (future)

`genesis/seeder/src/validate-timeline.ts` (mirroring `validate-stories.ts`). Checks:

- `id` matches the filename
- `kind` matches the subdirectory
- `relatedNodeIds[]` resolve to existing entities
- Kind-specific required fields are present (`occurred_at` on chronicle, `target_window` on roadmap, `shift_objective` on backlog)
- `status` is valid for the kind

Out of scope until the catalog is populated.

## Status — placeholder

This catalog is starting empty. The shape above is a working hypothesis; expect revision as the memory team produces real entries and we see what's awkward. Two known open questions deferred for revisit:

1. **ContentNode contentTypes**: `chronicle-entry`, `roadmap-item`, `backlog-item` are not yet declared in the lamad manifest. Either we add them, or we lean on `work-story` / `work-project` (existing avodah types) for backlog and roadmap, and add only `chronicle-entry`. Decision deferred until we know what's load-bearing in the prose vs. what wants to be queryable as protocol data.

2. **Multi-kind transitions**: when does a backlog item become a chronicle entry (after `status: completed`)? Likely the chronicle entry is *separate* — it records the completion as a moment, while the backlog item retires. Two entries, one moment. Confirm when we see real cases.

## Related

- `genesis/data/stories/CONVENTIONS.md` — sibling catalog (canonical narrative anchors)
- `.claude/agents/historian.md` — chronicle owner
- `.claude/agents/cartographer.md` — roadmap + backlog owner
- `.claude/agents/storyteller.md` — produces signal that flows into backlog
- `genesis/docs/content/elohim-protocol/architecture/2026-04-18-experience-story-epr-design.md` — Tier 1 narrative anchor design (sibling pattern for chronicle-entry if we go that route)
- `genesis/plans/` — implementation specs (downstream of backlog)
