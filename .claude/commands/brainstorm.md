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

## Step 2 — Apply the composition rule (binding for this session)

- **CANONICAL / done match** → COMPOSE from it. Extend the canonical spec; do **not** fork a new one.
- **SUPERSEDED match** → do **NOT** revive. Open its history record, read the gotcha, design *around* it.
- **claimed-UNVERIFIED match** → treat as unverified; note the verification gap, don't assume it works.
- **PRIOR ART empty** → a standalone spec is justified. Proceed.

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
