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

## Step 1c — FRONT-DISCOVERY: locate the plan on the MAP and the ROADMAP

The lenses above surface *prior plans* (what to compose from). This step surfaces *position + priority* — the
two standing maps the compaction-loop machinery keeps continuously current — so the plan is born oriented and
scoped to the right rung of the roadmap, not just born linked. Both are plain-text reads; fold their answers
into the plan's opening context.

**(1) MAP-PATH — where on the canonical surface does this plan live?** Read
[`architecture/MAP.md`](../../genesis/docs/content/elohim-protocol/architecture/MAP.md) and name, in one line:
- the **concern-domain D# (Section 1)** this work owns — *"this plan implements in domain D#"* — and the
  **owning architecture seed(s)** the gaps trace to (these are the `informed-by:` edges the plan cites);
- the **pillar(s)** the code lands in and the **per-pillar reading order (Section 2's walk)** the implementer
  follows — **default to the Household Living Core path** for care/recovery/memory work at the household seed;
- any **Gap Ledger row (Section 3)** these gaps correspond to — if this plan *closes* a tracked OPEN /
  STRADDLE / CODE-NO-DOC / GUIDE-GAP, name it, so the MAP row can flip when the plan drains. MAP is the
  **walk** over INDEX's **graph** — orient on MAP first.

**(2) ROADMAP-PRIORITY — which sprint rung is this, and is it pickable?** Read
[`vision-readiness-sprint-roadmap.md`](../../genesis/data/timeline/roadmap/vision-readiness-sprint-roadmap.md)
(the maintained prioritization home) and name, in one line:
- the ranked **Sprint-N (§1)** this plan drains (quote the live OPEN/CLAIMED counts it lists for the plan, and
  the readiness verdict) — these are the budget line-items Step 2 scopes to;
- the **verification track (§2)** — if the plan is CLAIMED-ONLY (built, checkboxes unticked), the work is
  *verify via ci-investigator*, not re-plan-and-rebuild;
- **BLOCKED-BY-ENV (§3)** — if the plan (or some of its tasks) needs harbor / alpha-cluster / shem, those
  tasks are HELD: **do not plan them now** (they drop out of `--focus` by design); plan only the
  testable-on-`household-nodes` legs;
- or **vision-deferred** (a #6-rung network-scale plan ranked DOWN of the single-household seed — confirm it
  is the right pick before draining it ahead of the seed).

**(3) CAPTURE COMPLEMENTARY WORK — keep the plan's scope genuine.** Planning a sprint rung surfaces *adjacent*
work the gaps brush — a dependency, a supportive fix, a neighboring gap-ledger row. Do **not** widen the plan to
swallow it (scope-bloat is how a plan becomes a dump), and do **not** drop it (a dropped discovery is a dump).
**Capture it**: a one-line item to [`genesis/data/timeline/backlog/`](../../genesis/data/timeline/backlog/),
linked to its domain D# + roadmap rung, so it queues as a future sprint instead of bloating this one. The plan
stays scoped to *one* rung; the complementary work plays nice as the roadmap's next entry.

**Staleness guard (mirror of Step 1b §4.4):** both maps regenerate each ceremony, not live. If the roadmap
body disagrees with today's `placement-audit.py --ledger` / `--focus` (Step 1 already ran them), **trust the
ledger/focus numbers over the roadmap prose** and say so — the audit is the live source; the roadmap is its
readout. Never plan a rebuild of CLAIMED-ONLY work the verification track owns, or a HELD item, off stale prose.

See the compaction-loop spec
(`genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md`, §4): MAP-PATH (legibility)
and ROADMAP-PRIORITY (prioritization) are promoted into the same FRONT-fire discovery as the lexical+semantic
lenses — surfaced in-flight, additively, on every plan.

## Step 2 — Scope rule (binding for this plan)

- Plan **ONLY** the `OPEN` gaps (implement) + `CLAIMED` gaps (VERIFY via ci-investigator — a checked box is
  a claim, never trusted as done).
- Do **NOT** plan work that is `BLOCKED-BY-ENV` (held — you can't validate it; ROADMAP §3) or already
  verified-done. Scope to the rung the ROADMAP (Step 1c) places this plan on — drain the named Sprint-N, or
  route CLAIMED-ONLY work to the verification track rather than re-planning it.
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
domain: D#               # the MAP-PATH concern-domain (Step 1c)
sprint: <Sprint-N | verify-track | vision-deferred>   # the ROADMAP rung (Step 1c)
# requires_env: [<env>]   # if its tasks can only be validated on a specific node/cluster (a ROADMAP §3 HELD leg)
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
