# Brainstorm (coherence-wrapped)

Wraps `superpowers:brainstorming` with a deterministic **pre-step** (compose-from-canonical) and
**post-step** (land it auditable), so brainstorming targets real gaps instead of minting duplicate specs.

This command IS the pre/post seam — the harness has no pre-/post-brainstorming hook, so the wrapper
provides it (same pattern as `/gap-analysis`, `/close-loop`, `/shift`).

Topic: `$ARGUMENTS`

## Step 1 — PRE: deterministic prior-art + scope preload (cheap, always run)

```bash
python3 .claude/scripts/memory-kit/prep-brainstorm.py --check-drift "$ARGUMENTS"
```

Read the preload it prints. It tells you, deterministically:
- **PRIOR ART** — specs already touching this topic, ranked, with state.
- **TESTABLE SURFACE** — what's in scope vs `BLOCKED-BY-ENV` (held, don't plan it).
- **BUDGET** — outstanding pressure, and a **drift advisory** if the surface is too messy to brainstorm against.

If the drift advisory fires, STOP and run a structuring pass first (classify `needs-triage`, link unlinked
memory) — do not brainstorm against a dumping ground.

This is the **FRONT fire point** of the Spec/Plan Compaction Loop
(`genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md`, §4): before any design
proposal, surface the canonical seed(s) this topic already descends from so the new artifact is **born linked**
to them (compose, don't fork) instead of minting a duplicate.

## Step 1b — DISCOVERY: semantic surfacing (JIT-scoped MemPalace, recall lens)

The lexical preload above is the always-available floor, but it is **provably blind to vocabulary drift** — it
matches token overlap, so the same concept under different words returns zero matches (the spec's own surfacing
probe got 0 lexical hits for "decompose-self / dump / forget" while the canonical `2026-05-10-memory-lifecycle-design.md`
sat right there under the names *compact* / *forget*). So run a **second, semantic lens** that catches
same-concept-different-words prior art the lexical floor misses (compaction-loop spec §4.1).

Pull MemPalace **just-in-time** — scope exactly the two tools needed at the surfacing step, then release them
(do NOT carry the full ~18-tool MCP as ambient context; an always-on MCP is itself a dump, §4.2):

```
ToolSearch "select:mempalace_search,mempalace_check_duplicate"
```

Then, with those two schemas loaded, query the palace semantically for the topic:
- `mempalace_search` — recall the nearest canonical seeds / history lessons / graduated stories by embedding
  similarity (defeats the lexical blindness above).
- `mempalace_check_duplicate` — confirm whether this topic is already covered (a near-duplicate ⇒ COMPOSE, do
  not fork).

**Staleness guard (§4.4):** the MemPalace index is frozen at mine-time and does not auto-update. If the search
returns nothing, or the index is older than the last BACK-fire dissolve, treat the semantic lens as **STALE —
degraded to lexical-only**, and say so explicitly ("semantic surfacing degraded; trusting lexical floor only").
Never present stale, incomplete recall as authoritative "no prior art" — that false confidence waves through a
fork. The lexical floor (Step 1) always stands regardless.

Carry the **surfaced seeds as plain text** into the brainstorm (exactly as the lexical preload is injected as
text); the scoped MemPalace tools are released after this step, not kept live through the session.

## Step 2 — Apply the composition rule (binding for this session)

Apply this rule to the seeds surfaced by **both** lenses (Step 1 lexical PRIOR ART + Step 1b semantic recall) —
a CANONICAL match found by *either* lens binds the session to compose:

- **CANONICAL / done match** → COMPOSE from it. Extend the canonical spec; do **not** fork a new one.
- **SUPERSEDED match** → do **NOT** revive. Open its history record, read the gotcha, design *around* it.
- **claimed-UNVERIFIED match** → treat as unverified; note the verification gap, don't assume it works.
- **PRIOR ART empty (BOTH lenses)** → a standalone spec is justified. Proceed. Only an empty result from *both*
  the lexical floor and a *fresh* (non-stale) semantic lens justifies a `cites: []` standalone (§4.3).

## Step 3 — Brainstorm

Invoke `superpowers:brainstorming` on `$ARGUMENTS`, carrying the Step-1 preload as binding context and the
Step-2 rule. Prefer "add a section to `<canonical spec>`" over "new spec" whenever a canonical match exists.

## Step 4 — POST: land it auditable (no orphan, no dump)

Whatever the brainstorm produces, it must be **instantly auditable the moment it lands** — never a no-status
orphan (that is the #1 debt). So the output spec MUST carry PLACEMENT frontmatter:

```yaml
---
title: <name>
status: Draft            # the lifecycle state — NEVER omit
topic: [<tokens>]        # what it's about (feeds the prior-art index)
cites: [<prior-art paths you composed from>]   # the verifiable links back
# requires_env: [<env>]  # if it can only be validated on a specific node/cluster
---
```

**BORN LINKED (§4.3):** `cites:` is **not** a retroactive afterthought — it is **front-loaded** from the seeds
surfaced at Steps 1 + 1b. Write the lexical PRIOR-ART paths *and* the semantic MemPalace hits into `cites:`, and
add the lineage edge that names the relationship: `refines: <seed>` when extending a canonical seed (the
preferred "add a section to `<canonical>`" path), or `derived_from:` / `compacted_from:` as appropriate. A spec
that surfaced a CANONICAL match but forks a standalone doc anyway is a placement violation the BACK fire point
will catch. `cites: []` is legitimate **only** when both lenses came back empty (and the semantic lens was not
stale).

Then re-audit so the new artifact shows up in the budget immediately:

```bash
python3 .claude/scripts/memory-kit/spec-coherence-index.py   # refresh prior-art index with the new spec
python3 .claude/scripts/memory-kit/placement-audit.py --ledger | head -20
```

**Decompose into gap-items** — run:

```bash
python3 .claude/scripts/memory-kit/decompose.py <new-spec-path>
```

It writes `.claude/memory-kit/gap-items/<slug>.json` — the bounded, cited gap list the next `/plan`
targets (**OPEN** = implement, **CLAIMED** = verify; a checked box is a claim, never trusted as done). If it
reports "needs AGENT decomposition" (a prose design spec with no checkboxes/requirements), extract the
spec's components yourself: 5–15 bounded items, each citing a spec line, `OPEN` unless already
implemented-and-verified. Then `placement-audit.py --ledger` shows the gaps rolled into the budget.
