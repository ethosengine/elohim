---
decided: 2026-05-13
purpose: orient contributors to the project's memory system — storage, hygiene tools, edit-time source-regulation, hooks, subagents, and the time×meaning perspective frame they serve (three temporal + one meaning)
---

# Memory System

This is the navigational map for the project's memory tooling. The system is structured around perspectives on a single substrate (the epic-graph over time): **three temporal perspectives** — past, present, future — plus a **meaning axis** orthogonal to time. Each perspective is served by a dedicated Opus-tier subagent and toolkit. The shape is **time × meaning**: four agents reading one substrate from different angles.

## ⚖ Pair-off boundary — native memory is Claude's; this discipline governs the ARTIFACTS (decided 2026-06-03)

`.claude/memory/` is Claude's **native** memory store: the `.claude-config/.../memory` config slot symlinks to
this in-repo primary. It holds the **light** native index — `MEMORY.md` plus the per-fact entry files — saved per
Claude's own native spec (only non-obvious facts useful across sessions). It is Claude-self-managed, NOT a curation
target for the heavy discipline below, and durable knowledge does not pile up here.

**`MEMORY.md` is a GENERATED projection** (decided 2026-07-02, after a hand-maintained index bloated past
the harness ~24.4KB load cap and loaded truncated): each topic file carries `title:` + `description:`
frontmatter (the one-line recall hook), and `memory-index-projector.py` renders the index from them —
PostToolUse-wired, budget-signaled into the `cleanup:` gate via `memory-index-drift.json`, compose-gated by
`.claude/memory/.epr-meta` (deny at birth without name/title/description; ask on hand-edits of the index).
Budget relief is population work — umbrella entries (`index: false` on folded members) and graduation —
never index hand-editing.

This kit's discipline (cleanup · dedupe · comet [the head/tail/memorialized-core lifecycle model] · cites ·
coherence · the `MEMORY.md` lightness budget · the time×meaning team) governs the **MANAGED ARTIFACTS** —
`genesis/docs/` · specs · plans · `genesis/data/stories/` · the CLAUDE.md gospel surfaces — routed by
`.claude/subject-routing.yaml`. That is where durable architecture/vision/wisdom and stable how-to-work rules live,
content-addressed and scope-aware.

**Durable knowledge GRADUATES to the managed artifacts; it does not accumulate in native memory.** Route a durable
fact to its managed home (per subject-routing), not to a new memory entry: architecture/vision → the protocol
corpus; stable rules → specific agent/skill/CLAUDE.md homes. `MEMORY.md` is the native index file the budget keeps
light (≤24KB); it grows as orphans get re-linked, but that is index traffic, not dump-pressure. (The 2026-06-03
pair-off founded this discipline — it drained a 260-entry dump down to a light core, graduating architecture/vision
into the corpus and stable rules into their homes, and dropping duplicates/stale. The lesson is the *discipline*,
not a fixed count: native memory stays light by graduating outward, not by capping entries.)

| Perspective | Subagent | Toolkit | Purpose |
|---|---|---|---|
| **Past** | [`historian`](../../agents/historian.md) | MemPalace MCP (wired) + archive walks + git log | Surface precedent/risk patterns from mined corpus (shifts/memory/plans/elohim-protocol wings) + archive + epic git history into present work |
| **Present** | [`librarian`](../../agents/librarian.md) | this directory's scripts + MemPalace MCP curate-grade | Keep the native `MEMORY.md` index light; tend the CLAUDE.md surfaces + skill catalog. `mempalace_sync` complements `cleanup-scan`; `mempalace_check_duplicate` replaces the TF-IDF dedupe approximation. |
| **Future** | [`cartographer`](../../agents/cartographer.md) | `/converge` skill + scripts at `.claude/scripts/converge/` | Synthesize memkit reports into a ranked next-actions menu, hand off to `/shift` |

All four agents — the three temporal perspectives above plus the **storyteller** (meaning axis, below) — are
Opus-tier subagents. The mechanical work (running scripts, parsing reports) is cheap; the orchestration + judgment
is what they encode.

### Meaning axis (orthogonal to time)

The **storyteller** ([`storyteller`](../../agents/storyteller.md)) operates orthogonally to the three temporal
perspectives. They don't tend a slice of time; they decide which memory artifacts **graduate** to canonical story,
which are **memorialized** in deep archive (the Isildur's-diary tier — dormant but findable), and which are **held**
for later. They own [`genesis/data/stories/`](../../../genesis/data/stories/), the catalog of canonical human
stories composed with humans, devices, epics, and Gherkin scenarios.

**Forgetting is by design, not failure.** History becomes legend, legend becomes myth — but the small, well-storied
artifact stays findable in the deep archive when the story leads back to it. The promise is not omniscient recall;
it is that the right diary surfaces at the right moment because a story made it retrievable. So the storyteller
never deletes: graduation and memorialization are *markings*, not destructions. (This rewrite demonstrates the
discipline on itself — see "Related memory entries" below.)

During disposition triage (the librarian's hygiene-sweep hands archive candidates to the storyteller — see the
storyteller agent's "Disposition triage" section), the storyteller produces a graduate/memorialize/hold decision
that the cartographer can fold into the next-actions menu.

## State & Coherence Tooling — the deterministic memory-stasis kit (decided 2026-06)

A second toolkit sits alongside the hygiene scripts: a **deterministic (no-LLM) state machine** over the doc +
memory surfaces, governed by the placement contract at **`genesis/docs/PLACEMENT.md`**. It exists so every file
is instantly auditable for *position + state*, and so the surface trends toward **stasis** — equilibrium:
pressure dirs empty, no dumps, every artifact has a next state.

| Tool | What it gives you |
|---|---|
| `placement-audit.py` | scoreboard + `--ledger` (the BUDGET: every file → position + state + next-action) + `--focus` (the env-scoped testable surface; scope follows `cluster-state.yaml`) + `--headline` (the SessionStart budget line) + `--epr-meta` (**directory-governance coverage**: the remediation queue of structurally-substantial regions not yet OWNED by a `covers: subtree` `.epr-meta` — feeds the `epr_meta_coverage` stasis dimension + the `epr-meta:` headline token; reuses the subtree-coverage walk in `_lib/epr_meta.py`, tuned in `context-coverage.yaml`'s `epr_meta_governance` block) + structural anti-dump check + **stray-doc census** (the `STRAY-DOC` counter + `stray:` headline token: ungoverned `.md` parked at the repo-root or `genesis/docs/` root — the REMEDIAL counterpart to the preventive `.epr-meta` gate, since the gate only makes *new* docs born-governed and never sweeps residue already on disk; must trend to 0). `--epr-meta` is the PREVENTIVE-coverage complement to that remedial census: are the codebase's substantial directories *thoughtfully owned* by a self-responsible manifest, the way CLAUDE.md owns progressive init context? |
| `spec-coherence-index.py [--query "<topic>"]` | deterministic prior-art index (token overlap) → "have we spec'd this?" so you compose from canonical instead of re-speccing |
| `decompose.py <doc>` | spec/plan → bounded, cited gap-items (OPEN = implement, CLAIMED = verify) — the gap budget |
| `state-machine-gen.py` | regenerates the `genesis/docs/_state/<state>/` pressure-dir CLAUDE.md gates (blockers / regression / unverified / needs-triage) |
| `prep-brainstorm.py "<topic>"` | the `/brainstorm` pre-step preload (prior-art + focus + budget) |
| `genesis/manifests/cluster-state.yaml` | the env-reality input — flip a node/cluster/registry → `--focus` scope cascades |

The lifecycle, the four verification states (VERIFIED-STABLE / CLAIMED-ONLY / REGRESSED / BLOCKED-BY-ENV), the
feedback graph (regression *warms* a doc; env-unavailable *holds* it), and the home taxonomy (CANONICAL /
HISTORY / ACTIVE + pressure dirs) all live in `genesis/docs/PLACEMENT.md`. Read it once.

### Gotchas (hard-won — do not relearn the expensive way)

- **A "landed" / checkbox / ✅ claim is NOT done.** Checkboxes lie (the iroh delivery-master showed every gate
  ✅ while CI held landing commits for *unrun soaks*). "Done" is EARNED by the verification gate
  (ci-investigator / CI / soak), never self-asserted. Treat `status: landed`-without-evidence as CLAIMED.
- **Env-unavailable ≠ regression.** A node/cluster/registry down is `BLOCKED-BY-ENV` — *held, not broken* (a
  partial cluster is the steady state). It must NOT fire a regression cascade. Only *red on an available env*
  is a regression. Update `cluster-state.yaml`; scope cascades; no false alarm. **Trust the structured
  `available:` flag + the `--focus` baseline over any prose `note:` field** — the `note:` is unreconciled and is
  the least-authoritative, most-likely-stale source (a stale `shem: offline` note propagated a false "held"
  through a 4-wave arc for weeks). `@requires:<cap>` means *satisfiable-when-available*, never *held*. [[scope-flag-beats-prose-note]]
- **No dumping grounds, anywhere.** Runaway docs, runaway directory trees (no `blocked-by-env/shem/…` nesting —
  reason goes inline), and growing archives are all debt. `_retired/` holds ONLY verified-stable work. The
  pressure dirs are MEANT to be empty.
- **Don't archive trash.** mem-palace ingestion is a deliberate graduation act (landed-canonical +
  distilled-history only); raw/abandoned/superseded is never embedded. Coherence comes from the cheap
  deterministic floor, not from mining everything.
- **Compose, don't fork.** Run `spec-coherence-index.py --query` before proposing — extend canonical; revive
  nothing superseded (read its history gotcha first).
- **Fix deployed ≠ fix converged.** When a ceremony deploys a substrate fix, the immediate post-deploy counter
  shows the cascade unmasked — not convergence. Record it as the *starting baseline*. The next ceremony's
  same-counter reading is the convergence evidence. One-cycle observation is insufficient; two cycles = minimum
  proof. Applies to any drift counter (claude-md, cleanup-scan, dedupe clusters).
- **Re-indexing orphans grows `MEMORY.md`.** After tightening entries AND re-indexing orphans, expect the index
  to grow — orphans are load-bearing, just invisible. Tightening is bounded by entry-shape floor (~100–150
  chars). Real compression comes from: umbrella entries (fold related items under one link), graduation to
  stories (one story carries 6+ entries), or archive-with-pointer. Per-entry tightening is a tiny correction,
  not a budget path.

### Ceremony discipline — balance sheet

Run `genesis/scripts/memory-balance.sh` at **Wave 0** (baseline) and **Wave 6** (close) of every memory ceremony.
Snapshots persist to `balance-sheets/<ts>.{json,txt}` and auto-diff against the prior run. The **Surface:Archive
ratio** is the smoking-gun metric — healthy: trending <100:1; runaway: flat archive across multiple ceremonies.
Paste the delta into the chronicle entry as the ceremony's evidence. Healthy ceremony targets: `MEMORY.md` index
≤ 24KB; ≥1 canonical story; 0 memorialize-archive orphans missing `story_pointer`.

### Your mandate (broad — you decide the how)

You own a slice of this surface (past / present / future / meaning). **Use these tools to drive your domain
toward memory stasis** — fewer no-status orphans, fewer unlinked memory entries, claims verified or moved to
regression, dead paths distilled to history, pressure dirs empty. The budget (`placement-audit.py --ledger`)
is your scoreboard; the debt numbers must fall. *How* you get there is your judgment — these are instruments,
not a script.

## Pollution regulation for agentic-developer context — source regulation at the emitter

The State & Coherence kit above is **end-of-pipe remediation**: cleanup, dedupe, audit, and the stray-doc census
catch drift *after* it lands. This concern is its peer in the other direction — **prevention at the point of
emission**.

Agentic agents — subagents, workflow agents, the main loop — **emit externalities** into a shared **context
commons**: ungoverned text, drift, stray docs, dead cites, dumped memory. That commons (the gospel surfaces, the
managed artifacts, `MEMORY.md`, the doc corpus) primes *every* downstream agent, so one agent's stray emission
becomes the next agent's bad prime — the classic externality, paid by everyone who reads next. The governance
answer is **source regulation**: internalize the externality AT THE EMITTER — edit-time, per-directory,
agentic-agent-facing — so the text is **born governed** instead of cleaned up later. The two modes are peers, not
rivals: the sweep cleans residue; these gates keep residue from being emitted at all. The loop closes on the
ceremony's own coherence-verify — a RED verdict is, literally, context pollution that would derail the next sprint.

**The catch is shifted LEFT of compile — before a single byte compiles.** Regulation runs in two beats around the
write. *Before* (PreToolUse: the `.epr-meta` compose-gate + `managed-surface-context`) catches at edit-time — the
malformed doc is denied and the cite discipline is injected before the file is even saved. *During / after* (the
PostToolUse signal hooks) catches what did land and accrues it. Both beats are **deterministic and cheap** — a
denied write or a bumped counter costs single-digit milliseconds, where the same defect caught downstream costs a
CI red, an expensive compile, or a future agent's wasted context. The whole point is to catch the pollution to the
LEFT of the compile/CI loop, not at it.

**Governance is DISTRIBUTED over the filesystem substrate, not a central registry.** The rules are co-located: each
directory carries its own `.epr-meta` manifest, and the gate resolves the nearest-wins ancestor cascade
(`_lib.epr_meta`) — composable, per-directory, with no central authority. This is the protocol's own
distributed-governance ethos applied to the repo: the **directory is the locus**, the rule lives *with* the text it
governs, and governance composes outward from the leaves rather than down from a registry.

Three coupled edit-time mechanisms carry this. The first two are PreToolUse gates that fire *before* the write; the
third is the cheap PostToolUse accumulator layer that watches what got emitted and triggers remediation only once
it accrues.

**`.epr-meta` directory-local compose-gates — new docs born-governed, and a surface agents WRITE.** The lightest
co-located rule that drives a directory toward stasis: a live PreToolUse deny-gate
(`.claude/hooks/epr-meta-resolver.py`) reads the nearest `.epr-meta` manifest and DENIES authoring a NEW `.md`
under a governed tree unless it carries the required `id`/`kind` frontmatter (existing-file edits are exempt via
`new: true`). Governed trees include `genesis/data/timeline/` (the cartographer's `backlog/` + `roadmap/` home) and
`genesis/docs/{specs,plans}`. The gate is strict-but-recoverable — a malformed manifest downgrades the subtree
`deny → ask` rather than hard-blocking — and the manifest is a projection of earned reach, never the authority. The
STRAY-DOC census in the State & Coherence table is its **remedial counterpart**: the gate makes *new* docs
born-governed; the census sweeps the residue already on disk.

But `.epr-meta` is more than a static gate — it is **generative, and agents author it**. An agent that hits a
gotcha closes the loop by writing the co-located rule that prevents recurrence (the whole point of the
`elohim-epr-metafile` skill): the rule you author today **deterministically** regulates *every* future agent's
edits in that directory. The **content** of that regulation is precisely the hard-won hazards — gotchas,
anti-patterns, trap flags — carried in the manifest so the next agent catches the trap instead of re-discovering it
the expensive way. Pay-it-forward governance: each agent leaves the directory a little more self-regulating than it
found it, and the smart move on hitting a wall is to encode the wall, not just route around it.
[[project_epr_meta_compose_gate]]

**`managed-surface-context.py` — discipline injected before the edit.** Editing a gospel surface (this file, any
CLAUDE.md, spec, or plan) is itself a governed emission. The `managed-surface-context` PreToolUse Edit/Write hook
injects the touched surface's discipline + tooling BEFORE the edit lands, so you flow the change through the cite
tooling (`cite-gen` / `cite-describe` / `cite-propagate` / `cites-migrate`) rather than hand-writing a slug,
fingerprint, or path — a hand-written cite silently drifts the content-addressed graph. Managed-surface scope lives
in ONE place, `_lib/managed_surfaces.py`, never per-hook (per-hook scope is how cite-seal drift recurred).
[[feedback_managed_surface_edit_discipline]]

**PostToolUse signal hooks — the DURING / AFTER beat: emission tracked, born-linked nudged.** The same layer
watches emission cheaply after the fact and converts it into the trigger for remediation: `claude-md-drift` /
`claude-md-structural` count CLAUDE.md edits and structural mv/cp/rm; `memory-coherence` bumps a memory entry when
edited code matches its `cites:`; `cite-seal` nudges born-linked on graph members (doc-roots + gospel CLAUDE.mds)
carrying cite debt. These counters feed the signal-driven audit (it fires when `drift_score ≥ threshold`), so the
heavy sweep is summoned by *accumulated emission*, not a fixed calendar — prevention and remediation share one
signal bus.

## Architecture

```
storage tier        .claude/memory/                      ← primary (in repo, git-tracked, PVC-recoverable)
                    .claude-config/.../memory  →  symlink to primary

scripts (this dir)  cleanup-{scan,apply}.py              ← archive stale specs/plans/memory
                    path-update-scan.py                  ← propagate renames into stale citations
                    path-update-apply.py                 ← apply approved replacements
                    dedupe-memory-scan.py                ← surface merge candidates (TF-IDF)
                    memory-review.py                     ← MEMORY.md index size/drift/growth/types
                    skill-audit.py                       ← always-loaded skill descriptions
                    claude-md-audit.py                   ← CLAUDE.md drift + fit + missing + opted-out
                    memory-coherence-audit.py            ← memory↔code cites: edge; DEAD-CITE/CITE-CANDIDATE; builds cites-index
                    cite-{gen,describe,propagate}.py + cites-migrate.py ← content-addressed cite envelopes (slug|desc|sha256|status:|path:)
                    _lib/                                ← shared helpers (paths, store, frontmatter, drift_score, cite_graph, managed_surfaces, env_scope, subject_routing, epr_meta, ci_trigger, runtime_harvest, bootstrap)

hooks               .claude/hooks/pre-tool-memory.py     ← PreToolUse * — injects MEMORY.md across subagents
                    .claude/hooks/managed-surface-context.py    ← PreToolUse Edit/Write — injects a managed surface's discipline+tooling BEFORE the edit (_lib/managed_surfaces registry)
                    .claude/hooks/epr-meta-resolver.py          ← PreToolUse Edit/Write — the .epr-meta compose-gate (new governed doc needs frontmatter or DENY; fail-open)
                    .claude/hooks/claude-md-drift-signal.py     ← PostToolUse Edit/Write — counters
                    .claude/hooks/claude-md-structural-signal.py ← PreToolUse Bash — mv/cp/rm signal
                    .claude/hooks/memory-coherence-signal.py    ← PostToolUse Edit/Write — bumps a memory entry when edited code matches its cites:
                    .claude/hooks/placement-drift-signal.py     ← PostToolUse Edit/Write — terminal-status tripwire (feeds placement-audit / state-machine-gen)
                    .claude/hooks/cite-seal-signal.py           ← PostToolUse Edit/Write — born-linked nudge on graph members (doc-roots + gospel CLAUDE.mds) with cite debt

skills              .claude/skills/memory-kit/SKILL.md   ← user-facing toolkit doc
                    .claude/skills/converge/SKILL.md     ← future-projection synthesis

subagents           .claude/agents/librarian.md          ← present-tending (operates this dir)
                    .claude/agents/historian.md          ← past-surface (operates archive + epic git)
                    .claude/agents/cartographer.md       ← future-projection (operates /converge)
                    .claude/agents/storyteller.md        ← meaning axis (operates genesis/data/stories/)

reports / state     .claude/memory-kit/<YYYY-MM-DD>/     ← dated reports (operator review surface)
                    .claude/memory-kit/claude-md-drift.json ← signal accumulator state
                    .claude/archive/<YYYY-MM-DD>/        ← cleanup destinations (preserves trajectory; materializes on first archival)
```

## Operating principles

**Memory in repo is team-shareable and PVC-recoverable.** Primary lives at `.claude/memory/` (git-tracked). Personal observations could optionally stay at `.claude-config/`, but the corpus is overwhelmingly project knowledge. Recovery from a fresh PVC: `git clone` + recreate the symlink.

**Gospel surfaces describe stable architecture, not where-we-are.** A CLAUDE.md / spec / plan records durable rules and decisions, never sprint-state. Use the decision-date convention (`decided:` frontmatter, dated event markers) for *when a rule was set* — never "as of [date]", "in flight", "Phase N closed", or a live magnitude (a count, a percentage) that ages into falsity the moment the substrate moves. A founding event may be cited as a dated parenthetical; the rule it founded is what the body asserts.

**Signal-driven ceremonies, not fixed cadence.** Hooks accumulate cheap counters; the audit ceremony runs only when `drift_score ≥ threshold` for a given CLAUDE.md (or when the operator invokes it). This mirrors the operator's event→automation architecture: a deterministic ledger flag → background Opus dispatch → cite-sealed backlog with documented status → deterministic suppression on re-encounter → ceremony-pattern stasis sweep (the EPR `signal_kind` → threshold → mandatory-review shape). [[feedback_deterministic_flag_agent_canon_stasis_pattern]]

**Trust-compute gradient.** Cheap accumulators in hot paths (PostToolUse hooks: single-digit ms). Expensive ceremony only at operator invocation. Heavier-impact signals (structural ops like mv/cp/rm) weight ~6× direct edits. Re-tunable in `_lib/drift_score.py` without changing the protocol.

**Counters are source of truth.** `drift_score` is derived. Hooks update it lazily; the audit recomputes live from counters. No risk of stored-score drift.

**Opt-out markers preserve decision chains.** `.no-claude.md` in a directory excludes it from MISSING-CLAUDE-MD candidacy. Frontmatter (`decided`, `revisit-if`) + markdown body capture rationale. Audit surfaces these in their own section so surrounding-doc updates can reference what's been considered.

**Three-perspective separation; the storyteller crosses them.** Librarian, historian, cartographer are peers, not nested — each owns its temporal slice. The storyteller cuts across all three on the meaning axis (graduate / memorialize / hold). The operator (or a higher-level orchestrator) decides which to invoke when.

## When to invoke what

| Operator question | Invoke |
|---|---|
| "Run a memory hygiene pass" | `librarian` (or `/memory-kit`) |
| "Is memory healthy?" | `librarian` for a quick `memory-review.py` summary |
| "What's next?" | `cartographer` (or `/converge`) — assumes recent memkit reports exist |
| "Pre-shift readiness check" | `librarian` for hygiene, then `cartographer` for objective selection |
| "I'm about to do X; anything from history?" | `historian` |
| "This caching bug feels familiar" | `historian` |
| "Should this lesson become a story / be archived?" | `storyteller` (disposition triage: graduate / memorialize / hold) |
| "Are CLAUDE.md files drifting?" | `librarian` runs `claude-md-audit.py` |
| "Audit found false positives" | `librarian` triages, places `.no-claude.md` markers |

## Workflow — weekly hygiene + synthesis (~25 min)

1. `librarian` invoked → runs `memory-review.py` first (baseline)
2. `librarian` checks drift store, decides scope (light vs full)
3. Full pass: `cleanup-scan.py` → judgment subagent → `cleanup-apply.py`
4. Then: `path-update-scan.py` → `path-update-apply.py`
5. Then (monthly): `dedupe-memory-scan.py`, `skill-audit.py`, `claude-md-audit.py`
6. Reports land in `.claude/memory-kit/<today>/`
7. `cartographer` invoked → reads memkit reports → runs `converge-scan.py`
8. Cartographer's synthesis subagent produces per-theme proposals + `next-actions.md`
9. Operator reads `next-actions.md`, picks recommendation, invokes `/shift` or `/deliver`

## Shared helpers (`_lib/`)

Pure-stdlib modules used by scripts AND hooks. Bootstrap pattern: walk up from `__file__` looking for `.claude/scripts/_lib`. See `_lib/__init__.py` for the snippet to copy. Discipline: extract only when 3+ callers share a pattern.

| Module | Use |
|---|---|
| `_lib.paths` | `repo_root_from_file`, `reports_root`, `reports_dir_for_today`, `memory_dir` |
| `_lib.store` | Best-effort JSON load/save with safe defaults — for accumulator state |
| `_lib.frontmatter` | Minimal YAML-frontmatter parser for memory entries + opt-out markers |
| `_lib.drift_score` | Canonical drift-score formula (counters → score) |
| `_lib.cite_graph` | Content-addressed cite envelopes (`slug \| desc \| fingerprint \| status \| path`); slug-identity survives file moves |
| `_lib.managed_surfaces` | Edit-time SURFACE axis: given a touched file, which managed-surface class + discipline + tooling apply (ONE registry, never per-hook) |
| `_lib.env_scope` | Gap-granular substrate-scope resolver — the BLOCKED-BY-ENV / `requires_env` discriminator (honors `iroh ≠ shem`); shared by decompose / placement-audit / scope-reconcile |
| `_lib.subject_routing` | Cascade-resolver for `.claude/subject-routing.yaml` — routes a durable fact to its managed home by deliverable-target |
| `_lib.epr_meta` | `.epr-meta` manifest cascade/merge (nearest-wins ancestor walk) — the compose-gate's author-time rule engine |
| `_lib.ci_trigger` | The `ci-trigger:` leg of `.epr-meta`, projected into the flat `.ci-ignore` (build-time; orthogonal to the rule engine) |
| `_lib.runtime_harvest` | Pure core of the elevate-arm runtime poller (exhaustion predicates; no I/O — the `runtime-harvest.py` shell does the I/O) |
| `_lib.bootstrap` | Self-locating `_lib` import walk-up for any `.claude/*` script |

## Related memory entries

The architectural insights the subagents once internalized as separate native-memory entries
(`project_three_temporal_perspectives`, `project_memory_in_repo_two_tier`, signal-driven ceremonies, the `_lib/`
extraction discipline, opt-out markers, the historian role, wisdom→epics) were **graduated into this surface's
body** by the 2026-06-03 pair-off — the exact discipline this kit teaches, demonstrated on itself: durable
knowledge lives in the managed artifact (this CLAUDE.md), not as a pile of native entries. The live memory entries
the team still leans on:

- [[reference_mempalace]] — the wired recall substrate (historian read-mostly; librarian dedupe/sync); complements archive walks + git log, does not replace them
- [[feedback_managed_surface_edit_discipline]] — how the team edits gospel CLAUDE.md / spec / plan surfaces (this file included): cite tooling + PreToolUse injection, never hand-written slugs (the source-regulation layer above)
- [[project_epr_meta_compose_gate]] — born-governed doc authoring; the `.epr-meta` deny-gate behind the STRAY-DOC census (the source-regulation layer above)
- [[scope-flag-beats-prose-note]] — trust the structured `available:` flag + `--focus` over prose `note:`/memory; what feeds the env-scoped budget
- [[feedback_deterministic_flag_agent_canon_stasis_pattern]] — the deterministic flag → background-Opus → cite-sealed-backlog → ceremony-stasis loop this kit embodies

## Specs

- `genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md` — lifecycle primitives (`promote`, `compact`, `merge`, `submerge`/`surface`, `close-interval`, `memorialize`, `forget`, `quarantine`)
- `genesis/docs/superpowers/specs/2026-05-10-converge-skill-design.md` — converge design rationale + end-state vision
- `genesis/docs/superpowers/specs/2026-05-13-historian-and-epic-timeline.md` — historian role + epic-graph-as-timeline + MemPalace substrate

## What this system is NOT

- Not a general memory consolidator (no `merge`, `promote`, `compact` primitives implemented — those need their own design)
- Not autonomous — operator approval is structural for any file modification
- Not always-active — skills are deferred-loaded; subagents dispatched on demand
- Not MemPalace's replacement, nor MemPalace its: MemPalace is **wired** (read-mostly recall for the historian, dedupe/sync for the librarian) and **complements** the archive-walk + git-log substrate rather than supplanting it
- Not the auto-memory replacement — it complements Claude's native chat-side memory
