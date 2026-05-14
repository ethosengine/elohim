---
name: memory-balance-sheet-pattern
description: Standing ceremonial artifact — run memory-balance.sh at Wave 0 + Wave 6 of every memory-team ceremony. Persists JSON snapshots so each cycle can be diffed against the prior. Headline metric — Surface:Archive ratio — gives deterministic evidence that distillation is running.
type: feedback
---

The memory-team ceremony needs a deterministic "are we generating runaway documentation or reaching balance" measurement. Single-snapshot readings (e.g., "MEMORY.md is 28.5KB today") are ambiguous; the meaningful signal is the **delta across cycles**. `genesis/scripts/memory-balance.sh` captures a balance sheet across all tiers (code / gospel / surface-of-comet / working-memory / canonical / archive) and persists JSON + text to `.claude/memory-kit/balance-sheets/<ts>.{json,txt}`. Each run diffs against the most-recent prior snapshot.

**Why:** The substrate has weight, and the team's job is to keep that weight oscillating around a steady state, not monotonically growing. Without snapshots, we can't tell whether a "distillation pipeline" is running or just being talked about. The Surface:Archive ratio is the smoking-gun metric — healthy values trend toward <100:1; a stuck or growing ratio means archive isn't absorbing what surface produces.

**How to apply:**

- **Wave 0 of every ceremony**: run `genesis/scripts/memory-balance.sh` to capture baseline. The "Prior snapshot" line will show the last ceremony's end-state for free diff.
- **Wave 6 of every ceremony**: run again. The delta column shows what this ceremony actually moved. Paste the diff into the chronicle entry as the ceremony's evidence.
- **Healthy targets**: Surface:Archive trending <100:1; Working memory:Stories converging to 2-3:1; MEMORY.md ≤ 24KB; ≥1 canonical story; 0 memorialize-archive orphans missing `story_pointer`.
- **Runaway signals**: MEMORY.md >32KB and growing; archive line-count flat across multiple ceremonies; stories canonical=0 sustained; archive orphans >0 (memorialize without pointer = lost context, exactly the failure mode storyteller warned about).
- **Known caveat — gospel double-counting**: the script's CLAUDE.md count includes all nested CLAUDE.mds; the operationally meaningful budget is per-directory-chain-loaded, not the total footprint. For now, treat the total as an upper bound; tighten per-chain when the flag turns chronic.
- **Token-cost view** intentionally deferred: line-count proxy works well enough at this resolution; revisit if we want context-budget framing.

Pairs with [[feedback_first_memory_team_ceremony]], [[project_memory_lifecycle_comet_shape]], [[project_signal_driven_audit_ceremonies]], [[feedback_correct_reindex_grows_index]].
