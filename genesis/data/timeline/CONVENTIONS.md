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
- `genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md` — Tier 1 narrative anchor design (sibling pattern for chronicle-entry if we go that route)
- `genesis/plans/` — implementation specs (downstream of backlog)
