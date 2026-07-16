---
name: memory-ceremony
description: Orchestrate the four-agent memory team (librarian, historian, cartographer, storyteller) through a four-phase substrate-currency ceremony — population-wide drift triage, four-lens deep-read on 1-2 picked surfaces, storyteller-pen synthesis, apply + downstream coherence verification. The deliverable is substrate-grounded gospel-tier rewrites. Invoke when the substrate-currency audit accumulates drift signal, when a major substrate landing happens that the gospel-tier hasn't absorbed, or on operator request.
metadata:
  sourceRuntime: claude
  master: package
  governance: "epr:elohim-agent/skills/memory-ceremony"
---

# /memory-ceremony — Four-Phase Substrate-Currency Ceremony

This skill orchestrates the four-agent memory team (librarian, historian, cartographer, storyteller) through a coordinated ceremony whose **deliverable is substrate-grounded gospel-tier rewrites**. The ceremony exists because four lenses on a surface produce a better rewrite than one — and because Run #6 proved that the audit-numbers-moving ≠ substrate-coherence-improving. Audit-number hygiene (CLAUDE.md byte budgets, cleanup-scan flags, archive ratios, MEMORY.md size) lives in `/hygiene-sweep` on a separate cadence.

**When to invoke**:
- Signal: `substrate-currency-audit.py` ranks the gospel-tier surfaces by drift count (path-existence + process-status findings, highest first) and the top of that ranking carries enough drift to warrant a rewrite — there is no fixed numeric threshold; the audit emits a ranked list (plus headline counts and an uncited-recent-memory list) and the operator picks 1-2 off the top
- Substrate landing: a phase ships, a substrate parallel rolls in, a vocabulary lands — and the gospel-tier hasn't absorbed it
- Operator: when a gospel-tier surface feels stale, before a /shift that will read it as primer

**Substrate references**:
- `.claude/scripts/memory-kit/substrate-currency-audit.py` — Phase 1 triage script
- `.claude/scripts/memory-kit/LIFECYCLE.md` — ownership matrix, dispositions
- `.claude/agents/{librarian,historian,cartographer,storyteller}.md` — lens-job sections describe each agent's currency-mode role
- `.claude/skills/memory-kit/SKILL.md` — `/hygiene-sweep` (byte-budget + archive-ratio cadence; separate from this ceremony)
- `genesis/data/timeline/CONVENTIONS.md` — chronicle schema

## Execution modes — conversational (default) or dynamic workflow

The phases below define the ceremony's *content* regardless of how it's executed. Two modes:

- **Conversational (default, 1-2 surfaces).** The orchestrator dispatches each phase via Task, holding the lens reports in its own context and surfacing the Phase 3 rewrites to the operator gate inline. Right for the standard small cycle.

- **Dynamic workflow (3+ surfaces, or when you want the orchestration codified and rerunnable).** Phases 2-3 are a textbook fan-out + synthesis — per picked surface: librarian-prologue → three lenses in parallel → storyteller-pen. Run them as a dynamic workflow (`Workflow` tool) so the lens reports live in **script variables instead of the orchestrator's context**. That is what lets you pick more than two surfaces: the 1-2 cap is partly a context-budget limit, not only an attention one (see the "Picking too many surfaces" footgun). Pipeline shape, one item per picked surface:

  ```
  pipeline(pickedSurfaces,
    s   => agent(prologue,      {agentType: 'librarian',   phase: '2a', model: 'opus'}),
    fac => parallel([                                          // phase 2b, after prologue
             agent(missing-citations, {agentType: 'historian',    phase: '2b'}),
             agent(coverage-gaps,     {agentType: 'cartographer', phase: '2b'}),
             agent(narrative-lens,    {agentType: 'storyteller',  phase: '2b'})]),
    len => agent(compose-rewrite, {agentType: 'storyteller', phase: '3-compose'}))
  // returns paste-ready rewrites + diff-rationales
  ```

  `opts.agentType` invokes the real memory-team agents (so each keeps its MemPalace MCP grants and lens-job definition); `opts.model` honors the tier discipline (`[[project_agentic_loop_economics]]`). **The workflow stops before the Phase 3 operator gate** — workflows take no mid-run input, so it returns the rewrites and the operator gates each one in conversation (approve / revise / decline). Phase 4 (apply + coherence-verify) then runs inline or as a second workflow, with Phase 4b's fresh-context coherence check as the adversarial-verify stage (never one of the four lenses). Only launch a workflow when the operator has opted in — it spawns many agents.

## Phase 1 — Population-wide triage (~2 min)

Run the substrate-currency audit. Cheap, deterministic, idempotent:

```
python3 .claude/scripts/memory-kit/substrate-currency-audit.py
```

Output: `.claude/memory-kit/<TODAY>/substrate-currency-audit.{json,md}` — ranked drift list across all ~60 gospel-tier surfaces (agents + skills + CLAUDE.md), broken down by:
- **PATH-EXISTS** findings — backticked path-like tokens that don't resolve
- **PROCESS-STATUS** findings — temporal phrasing violating `[[feedback_agent_prompts_no_process_status]]`
- **MISSING-CITATION** findings — recent MEMORY.md slugs that the surface's scope plausibly touches but doesn't cite

Surface the top-5 by total drift to the operator. **Operator picks 1-2 surfaces** to rewrite this cycle. Default N=2; default pick the top-2 unless the operator overrides (e.g., "skip rust-architect, do code-reviewer and angular-architect — they touch the next sprint").

If the operator picks 0 (audit numbers are clean, or all top-ranked surfaces are bare-filename-heavy noise): announce that the ceremony has no work this cycle, surface the audit summary, and exit.

### Phase 1b — PATH + PRIORITIZATION currency readout (~3 min)

Alongside the gospel-tier drift triage, the ceremony carries two **standing canonical-surface
currency checks** — the LEGIBILITY/PATH map and the PRIORITIZATION/ROADMAP — so both are maintained
in-flight, not one-off. This sub-phase is a deterministic readout the operator sees next to the
substrate-currency audit; it does not consume a picked-surface slot. Dispatch (in parallel, one
message) two lightweight currency checks:

- **MAP-CURRENCY (librarian + historian).** The librarian verifies
  `genesis/docs/content/elohim-protocol/architecture/MAP.md` is current with the seeds — every D1–D10
  seed still on disk and matching INDEX.md, walk-links in §2 resolve, §3 gap-ledger pointers real —
  per its `MAP-CURRENCY mandate` lens-job. The historian verifies the §3 gap ledger is *substantively*
  honest (closed gaps worth a `history/` record; recurring gap-shapes worth a `feedback_*`), per its
  `MAP-CURRENCY mandate` lens-job. The **map-currency-drift accumulator**
  (`.claude/memory-kit/map-currency-drift.json`, written by the `map-drift-signal.py` PostToolUse hook and
  surfaced at SessionStart through the budget headline) names which seeds moved without the map
  following; if absent, fall back to an `architecture/` directory-diff against MAP §1.
- **ROADMAP-CURRENCY (cartographer).** The cartographer regenerates
  `genesis/data/timeline/roadmap/vision-readiness-sprint-roadmap.md` from the three live inputs
  (`placement-audit.py --ledger` × `--focus` × the re-mined vision axis) per its `ROADMAP-CURRENCY
  mandate`, re-stamping the dated regeneration header even when rankings are unchanged, and surfaces
  the single highest-leverage next move (§4) so it agrees with the next-actions menu.

Readout shape (one line each, surfaced under the Phase-1 triage summary):
> **PATH currency:** MAP.md — `<current ✅ | N seeds drifted / M gap rows stale / K dead walk-links>`
> **PRIORITIZATION currency:** roadmap — `<re-stamped <date>; top move: <one line> | stale vs today's --ledger>`

If both read current, the ceremony notes them green and proceeds to surface-picking. If either drifted,
the fix is the owning agent's standing duty (librarian/historian for MAP, cartographer for roadmap),
applied per their structural-vs-substantive authority split — it does **not** require picking MAP or the
roadmap as one of the 1-2 gospel-tier rewrite surfaces unless the drift is a substantive re-framing (in
which case it joins the candidate list and the storyteller pens it in Phase 3, same as any surface).

## Phase 2 — Four-lens deep-read (~25-30 min)

Each picked surface goes through the four lenses. The librarian runs first as prologue; the other three run in parallel against the librarian's verified-facts ground.

### Phase 2a — Librarian prologue (~5 min per surface)

Dispatch the librarian via Task with currency-mode framing (the agent's lens-job section in `.claude/agents/librarian.md` carries the full method). The dispatch prompt:

> "**Substrate-currency ceremony Phase 2a — prologue.** Picked surface: `<path>`. Run mechanical fact-verification per your `Substrate-currency ceremony — Phase 2 prologue lens-job` section.
>
> Output a verified-facts report for `<path>` listing every factual claim (path, crate/module/DNA, cited file, internal-citation slug) tagged `verified` / `not-found` / `drift` / `forbidden-phrasing` with line numbers. Process-status phrasing flagged per `[[feedback_agent_prompts_no_process_status]]`. Cap ~5 min per surface. Bare-filename-heavy surfaces get a heads-up note de-rating that class.
>
> Do NOT interpret findings or propose rewrites — the historian, cartographer, and storyteller consume your report next."

Run picked surfaces in parallel if more than one (one Task dispatch per surface, single message).

### Phase 2b — Three lenses in parallel (~10 min per surface)

After the librarian-prologue reports return, dispatch historian + cartographer + storyteller in parallel — one set of three dispatches per picked surface, all in a single message. Each agent's currency-mode lens-job is defined in its agent file; the dispatch hands them the librarian's verified-facts report verbatim.

**Historian dispatch**:
> "**Substrate-currency ceremony Phase 2b — missing canonical-discipline citations lens.** Picked surface: `<path>`. Librarian's verified-facts report attached: `<librarian Phase 2a output verbatim>`.
>
> Per your `Substrate-currency ceremony — missing canonical-discipline citations lens-job` section, surface up to 10 missing canonical-discipline citations the surface should carry but doesn't. Walk MEMORY.md filtered to the surface's scope; walk the palace if you suspect uncodified-but-recurring shape. Each finding: `[[slug]]` + one-sentence discipline + one-sentence bug-shape-risk. Silence is valid output."

**Cartographer dispatch**:
> "**Substrate-currency ceremony Phase 2b — substrate-coverage gap lens.** Picked surface: `<path>`. Librarian's verified-facts report attached: `<librarian Phase 2a output verbatim>`.
>
> Per your `Substrate-currency ceremony — substrate-coverage gap lens-job` section, surface up to 10 substrate-coverage gaps. Walk MEMORY.md entries from the last ~30 days; git log relevant repo paths from the last 60 days; check coverage-completeness (all 5 DNAs, both transport stacks, major substrate components, canonical vocabulary). Each finding: substrate citation + one-sentence gap + one-sentence suggested-claim. Silence is valid output."

**Storyteller dispatch (Phase 2 lens only — synthesis pen comes in Phase 3)**:
> "**Substrate-currency ceremony Phase 2b — narrative coherence & framing lens.** Picked surface: `<path>`. Librarian's verified-facts report attached: `<librarian Phase 2a output verbatim>`.
>
> Per your `Substrate-currency ceremony — rewrite-synthesis pen lens-job` Phase-2 sub-section, surface 5-10 narrative-coherence findings: vocabulary consistency, causality/framing, memorable shape, narrative-shape of process-status leaking in. Do NOT compose the rewrite yet — that's Phase 3. Output findings only."

After all three return per picked surface, you have the four reports (librarian facts + three lens findings). Move to Phase 3.

## Phase 3 — Storyteller pens rewrite + operator single-gate (~20 min per surface)

Dispatch the storyteller (one Task per surface; in parallel if more than one) with all four inputs:

> "**Substrate-currency ceremony Phase 3 — synthesis pen.** Picked surface: `<path>`. Four inputs attached verbatim: (1) librarian verified-facts, (2) your Phase 2 narrative-coherence findings, (3) historian missing-citations, (4) cartographer coverage-gaps.
>
> Per your `Substrate-currency ceremony — rewrite-synthesis pen lens-job` Phase-3 sub-section, compose the paste-ready rewrite of `<path>`. Preserve structural skeleton (frontmatter, section headings) unless lens findings argue for restructure. Each addition must cite at least one input. Apply canonical vocabulary; apply `[[feedback_agent_prompts_no_process_status]]`.
>
> Output: (a) full rewritten surface body, ready to paste, (b) 1-paragraph diff-rationale citing which findings drove which changes. Cap ~15 min. If the surface needs more, return what you have plus a one-sentence two-cycle-rewrite note for the operator to elevate as backlog."

Surface each storyteller rewrite to the operator as ONE single gate per picked surface — approve / revise-with-direction / decline:

- **Approve**: orchestrator applies the rewrite via Write or Edit, moves to Phase 4
- **Revise-with-direction**: re-dispatch the storyteller with operator's direction folded in; one revision round, then re-surface
- **Decline**: discard the rewrite; file a backlog entry naming what went wrong (operator's call) so the next ceremony picks differently

This is the only operator gate in the ceremony. Surface only the rewrites, not the lens reports — the operator reviews the deliverable, not the process. Per `[[feedback_no_menu_punt_in_auto_mode]]`, no four-option ritual menus; one decision per surface.

## Phase 4 — Apply + downstream-coherence verification (~10 min)

### Phase 4a — Apply approved rewrites

For each approved rewrite, apply via Write (full-file replace) or Edit (section replace, when the rewrite preserves enough structure). Git diff IS the chronicle for what changed; do not duplicate diff content elsewhere.

### Phase 4b — Coherence verification (fresh-context Explore agent)

Per `[[feedback_first_memory_team_ceremony]]` Phase 6d lesson — the only piece of the old ceremony worth preserving verbatim. Dispatch a fresh-context Explore agent (NOT one of the four memory agents, which carry confirmation bias from inside the ceremony):

> "You are a downstream implementation-sprint agent about to start work on **<plausible-next-topic>**. Read these files as your primed context:
> - MEMORY.md
> - <each rewritten surface>
> - <any CLAUDE.md or memory entries that the rewrite cites or is cited by>
>
> Report: (a) contradictions across these files that would mislead your sprint; (b) gaps where context references something you cannot find; (c) stale citations (e.g., a CLAUDE.md mentions a pattern, the rewritten agent prompt cites the pattern, but the pattern is referenced inconsistently); (d) redundancies where the same fact lives in N places, unclear which is authoritative.
>
> You are NOT executing the sprint — evaluating whether the context primes you well or poorly. Return a coherence score: GREEN (clean), YELLOW (minor noise, recoverable), RED (contradictions/gaps that would derail). Cite specific files + line evidence for any non-GREEN finding."

Pick `<plausible-next-topic>` from cartographer's `/converge` backlog if active, otherwise from the substrate the rewritten surfaces describe (e.g., rewriting rust-architect.md → topic "elohim-storage view authoring" or "doorway endpoint addition").

**Verdict handling**:
- **GREEN**: ceremony closes. Brief chronicle entry per Phase 4c.
- **YELLOW**: the librarian closes the noise inline if it's mechanical (typo, dead-path, citation fix); larger-than-mechanical noise becomes a single backlog entry for next cycle. Then ceremony closes; chronicle records the YELLOW + resolution.
- **RED**: the ceremony is NOT done. Re-dispatch the storyteller (or appropriate lens) to resolve the contradictions; re-apply; re-verify. RED is the highest-value signal the ceremony can produce — it means the four-lens deep-read missed cross-substrate impact. Do not paper over it.

### Phase 4c — Lightweight chronicle write

Chronicle stays as a deterministic record between ceremonies — **not** the heavy stasis-table-with-rationale-columns the old ceremony made it. Per operator direction (2026-05-15): the chronicle should surface drift deterministically without being leaned on as crutch.

Write `genesis/data/timeline/chronicle/<YYYY-MM-DD>-substrate-currency-<slug>.md`. Body is concise (~150-300 words):

```yaml
---
kind: chronicle
status: noted
date: <today>
ceremony: substrate-currency
surfaces_rewritten:
  - <path>
  - <path>
coherence_verdict: GREEN | YELLOW | RED
next_topic_sampled: <topic>
---

## What changed

<2-4 sentences naming the surfaces rewritten and the dominant lens-driver: which lens contributed the most weight to each rewrite. E.g., "rust-architect.md rewritten — cartographer's coverage-gap on iroh parallel stack was the driver; historian surfaced 4 missing canonical-discipline citations; librarian flagged 3 fictional path claims.">

## Coherence-check sampling

<1-2 sentences naming the topic sampled, the verdict, and any non-GREEN findings + resolution.>

## Wisdom worth carrying forward

<Only write this section if the cycle taught something cross-cutting that the next ceremony should know. Otherwise omit. NOT a ritual section — silence is the default.>
```

The chronicle is forensic record, not narrative scaffolding. Future ceremonies grep it to recall "have we rewritten this surface recently?" and "did the coherence-check find anything we should remember?" — that's the deterministic drift-surface role it plays. If a cycle taught nothing cross-cutting, the Wisdom section is omitted. Most cycles will land there.

### Phase 4d — MemPalace re-embed (the ceremony just rewrote the canonical surface)

The ceremony's whole point is substrate-true rewrites of gospel-tier canonical content — which **changes the semantic surface the front-link recalls from**. A frozen index would keep surfacing the *old* gospel. So the final act, once 4b is GREEN, is to re-embed the cleaned surface:

```bash
python3 .claude/scripts/memory-kit/mempalace-currency.py --remine   # sync-prune + mine canonical+history+memory+stories + record
```

This is the same back-fire-point re-mine the shift runs, fired eagerly here because the ceremony is *exactly* the event that stales the index. Confirm `placement-audit.py --headline` then reads `mempalace: fresh ✅`. (The `memory-stasis-loop` would also catch this via its `mempalace` dimension; closing it in-ceremony means the next session's front-link recalls the new gospel immediately, not after the next drain.)

## End state criteria

- Each picked surface has either (a) an applied rewrite + GREEN/YELLOW coherence verdict, or (b) a declined-with-rationale entry filed as backlog
- All Phase 4b RED verdicts resolved before chronicle write
- Chronicle entry exists with the three required frontmatter fields
- **PATH + PRIORITIZATION currency read (Phase 1b)**: MAP.md verified current (or its drift handed to the librarian/historian as a standing fix) and the roadmap re-stamped this cycle (or its staleness closed by the cartographer). Both readouts appear in the ceremony summary.
- **MemPalace re-embedded (Phase 4d)**: the index re-mined after the rewrites land; `--headline` reads `mempalace: fresh ✅`.
- All TaskCreate tasks closed

The operator should leave the ceremony with substrate-grounded gospel-tier surfaces — picked, rewritten with four lenses' contribution, coherence-verified, recorded. Audit-number wins (byte budgets, archive ratios, cleanup-scan flags) are NOT the ceremony's job; those land in `/hygiene-sweep` on a separate cadence.

## Boundaries — what this ceremony does NOT do

The 6-wave predecessor accumulated machinery that masked its purpose. Explicitly out of scope here:

- **Byte-budget enforcement** (CLAUDE.md over-budget, MEMORY.md size) → `/hygiene-sweep`
- **Archive ratios + cleanup-scan flags + agent-audit / skill-audit / claude-md-audit** → `/hygiene-sweep`
- **Stasis-plan / defer-budget / 3-cycle-clock / 20%-floor machinery** → deleted; the ceremony's measure is rewrites delivered, not dimensions advanced
- **Wave 5 retrospective every cycle** → runs quarterly on the ceremony itself, not every cycle
- **Multi-question operator menus** → one gate per surface (approve/revise/decline); no four-option ritual

If the operator asks for byte-budget hygiene, invoke `/hygiene-sweep`. If they ask for the substrate to absorb a recent landing, this is the right skill.

## Known footguns

- **Bare-filename audit noise**: surfaces like agent prompts cite many bare filenames that match against many subdirs. The audit's PATH-EXISTS check is conservative on purpose; the librarian-prologue de-rates bare-filename findings when reporting. Don't over-react to a surface flagged at 30 findings if 25 are bare filenames.
- **Storyteller over-running the 15-min cap**: if a rewrite genuinely needs >15 min, return what you have plus a two-cycle-rewrite note. Don't blow the cap silently — the next cycle picks up the remainder.
- **RED verdict paper-over**: tempting to close ceremony despite RED because "we already rewrote it." Don't. RED means the cross-substrate impact was missed; the value of the verdict is precisely that it catches what the four-lens missed. Resolve before chronicle.
- **Audit-number-as-success conflation**: substrate-currency-audit numbers go down when surfaces are rewritten well, but that's a side-effect. The measure is coherence-verify verdict + operator's "yes this matches today's substrate" approval. Number-watching is `/hygiene-sweep`'s job.
- **Picking too many surfaces**: in conversational mode, 1-2 per cycle — three is the upper edge, four guarantees one will be under-served, because the lens reports compete for the orchestrator's context. The dynamic-workflow execution mode (see "Execution modes") keeps those reports in script variables, relaxing the *context-budget* half of the cap — you can pipeline the top ~5 drifted surfaces in one run. The *attention* half still applies at the operator gate: review the returned rewrites in batches, not all at once. The audit's ranked list survives across cycles regardless — there is always more.

## Related

- `.claude/scripts/memory-kit/substrate-currency-audit.py` — Phase 1 triage
- `.claude/scripts/memory-kit/LIFECYCLE.md` — ownership matrix
- `.claude/skills/memory-kit/SKILL.md` — `/hygiene-sweep` cadence (byte-budgets etc.)
- `.claude/agents/{librarian,historian,cartographer,storyteller}.md` — currency-mode lens-jobs
- `.claude/memory/feedback_first_memory_team_ceremony.md` — wisdom from inaugural 6-wave ceremony
- `.claude/memory/feedback_agent_prompts_no_process_status.md` — gospel-tier surfaces describe stable architecture
- `genesis/data/timeline/CONVENTIONS.md` — chronicle schema
- `genesis/docs/content/elohim-protocol/architecture/MAP.md` — the LEGIBILITY/PATH walk (Phase 1b MAP-CURRENCY target; librarian + historian own its currency)
- `genesis/data/timeline/roadmap/vision-readiness-sprint-roadmap.md` — the PRIORITIZATION/ROADMAP home (Phase 1b ROADMAP-CURRENCY target; cartographer regenerates it)
