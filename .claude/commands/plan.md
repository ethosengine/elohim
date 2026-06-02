# Plan (coherence-wrapped)

Wraps `superpowers:writing-plans` with a deterministic **pre-step** (target real gaps, not re-plan settled
work) and **post-step** (land the plan auditable + decompose into tasks). Same seam as `/brainstorm`.

Plans the spec at: `$ARGUMENTS`

## Step 1 — PRE: decompose the spec into gaps + scope (deterministic)

```bash
python3 .claude/scripts/memory-kit/decompose.py "$ARGUMENTS"        # OPEN gaps = implement, CLAIMED = verify
python3 .claude/scripts/memory-kit/placement-audit.py --focus        # what's testable vs BLOCKED-BY-ENV
python3 .claude/scripts/memory-kit/spec-coherence-index.py --query "<spec topic>"   # prior plans (compose, don't fork)
```

Read `.claude/memory-kit/gap-items/<spec-slug>.json`. If decompose says **"needs AGENT decomposition"**
(a prose spec), extract 5–15 bounded gap-items yourself first (each citing a spec line).

This is the **FRONT fire point** of the Spec/Plan Compaction Loop
(`genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md`, §4) applied to plans: surface
the canonical seeds + prior plans this work descends from so the plan is **born linked** (compose, don't fork).

## Step 1b — DISCOVERY: semantic surfacing (JIT-scoped MemPalace, recall lens)

`spec-coherence-index.py --query` (above) is the always-available lexical floor, but it matches token overlap
and is **provably blind to vocabulary drift** — same-concept-different-words prior art returns zero matches
(§4.1). So run a **second, semantic lens** before writing the plan. Pull MemPalace **just-in-time**, scoping
exactly the two tools, then release them (an always-on ~18-tool MCP is itself a dump, §4.2):

```
ToolSearch "select:mempalace_search,mempalace_check_duplicate"
```

- `mempalace_search` — recall the nearest prior plans / canonical seeds / history lessons by embedding similarity.
- `mempalace_check_duplicate` — a near-duplicate plan ⇒ COMPOSE (extend it), do not fork a parallel plan.

**Staleness guard (§4.4):** the MemPalace index is frozen at mine-time. If it returns nothing, or is older than
the last BACK-fire dissolve, treat the semantic lens as **STALE — degraded to lexical-only** and say so
explicitly; never present stale recall as authoritative "no prior plan." The lexical floor always stands. Carry
the surfaced seeds/plans into the plan as **plain text**; release the scoped MemPalace tools after this step.

## Step 2 — Scope rule (binding for this plan)

- Plan **ONLY** the `OPEN` gaps (implement) + `CLAIMED` gaps (VERIFY via ci-investigator — a checked box is
  a claim, never trusted as done).
- Do **NOT** plan work that is `BLOCKED-BY-ENV` (held — you can't validate it) or already verified-done.
- Compose from the prior plans/seeds surfaced by **both** lenses (Step 1 lexical + Step 1b semantic); extend,
  don't fork. A CANONICAL/near-duplicate match from *either* lens binds you to compose.

## Step 3 — Write the plan

Invoke `superpowers:writing-plans`, scoped to exactly those gaps.

## Step 4 — POST: land auditable + decompose into tasks

The plan MUST carry PLACEMENT frontmatter so it's instantly auditable (never a no-status orphan):

```yaml
---
title: <name>
status: Draft
cites: [<spec path>, <gap-items it covers>]
# requires_env: [<env>]   # if its tasks can only be validated on a specific node/cluster
---
```

**BORN LINKED (§4.3):** `cites:` is **front-loaded** from the seeds surfaced at Steps 1 + 1b, not threaded in
afterward — list the spec being planned, the prior plans (lexical + semantic) you composed from, and the
gap-items covered. `cites: []` is legitimate only when both lenses came back empty and the semantic lens was
not stale.

Then decompose the plan into task-level gap-items (the budget line-items the implement→verify loop drives):

```bash
python3 .claude/scripts/memory-kit/decompose.py <new-plan-path>
python3 .claude/scripts/memory-kit/placement-audit.py --ledger | tail
```

Each task becomes a budget line-item; `BLOCKED-BY-ENV` tasks drop out of `--focus` automatically, and
`CLAIMED` tasks stay in the queue until ci-investigator verifies them — so "done" is earned, not asserted.
