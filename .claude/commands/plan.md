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

## Step 2 — Scope rule (binding for this plan)

- Plan **ONLY** the `OPEN` gaps (implement) + `CLAIMED` gaps (VERIFY via ci-investigator — a checked box is
  a claim, never trusted as done).
- Do **NOT** plan work that is `BLOCKED-BY-ENV` (held — you can't validate it) or already verified-done.
- Compose from the prior plans surfaced; extend, don't fork.

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

Then decompose the plan into task-level gap-items (the budget line-items the implement→verify loop drives):

```bash
python3 .claude/scripts/memory-kit/decompose.py <new-plan-path>
python3 .claude/scripts/memory-kit/placement-audit.py --ledger | tail
```

Each task becomes a budget line-item; `BLOCKED-BY-ENV` tasks drop out of `--focus` automatically, and
`CLAIMED` tasks stay in the queue until ci-investigator verifies them — so "done" is earned, not asserted.
