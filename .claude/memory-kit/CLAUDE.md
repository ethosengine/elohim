# `.claude/memory-kit/` — the context-coverage DATA surface

This directory holds the **generated reports + the one hand-tuned config** for the memory-stasis /
context-coverage system. The **tools** that write these live in [`../scripts/memory-kit/`](../scripts/memory-kit/CLAUDE.md)
(read that gate for the toolkit). The **contract** is [`genesis/docs/PLACEMENT.md`](../../genesis/docs/PLACEMENT.md).

> **Before you touch anything here, know the one rule:** only `context-coverage.yaml` is hand-edited.
> Every `*.json` is a GENERATED report — hand-editing it is a lie, exactly like editing a coverage
> report instead of writing the test. Regenerate via the tool that owns it.

Each entry below is a **link = path + a 1-2 sentence explainer** (the project link rule, applied to this dir):

## Tune here (the only hand-edited file)
- **`context-coverage.yaml`** — the tuning manifest: stasis `margin`, byte budgets, per-dimension weights,
  exclusions, each with a `why:` methodology block. `placement-audit.py --stasis` and `context-ratchet.py`
  read it. Edit here to retune the targets — *with* the reasoning beside each.

## Generated reports (regenerate via the tool; never hand-edit)
- **`state-ledger.json`** — the BUDGET: every file → its position + state + next-action. Written by
  `placement-audit.py --ledger`.
- **`context-coverage-baseline.json`** — the RATCHET FLOOR: per-dimension coverage that must not decrease.
  `context-ratchet.py` checks against it and ratchets it up; the gate fails on any regression.
- **`spec-coherence-index.json`** — the prior-art index (topic → spec, token overlap) the `/brainstorm`
  canonical-check reads. Written by `spec-coherence-index.py`.
- **`gap-items/`** — per-doc decomposed gap-items (OPEN = implement, CLAIMED = verify), written by
  `decompose.py`; rolled into the budget under "DECOMPOSED GAPS".
- **`cites-index.json`** — the memory `cites:` edge index (which entry cites which source). Built by
  `memory-coherence-audit.py`.
- **`memory-coherence-drift.json`**, **`claude-md-drift.json`** — PostToolUse drift accumulators: counters
  bumped when cited code / a CLAUDE.md changes in-flight (the in-flight coherence signal).
- **`delivery-status-distribution.json`** (+`.prev`) — per-plan delivery-status snapshot; `.prev` is the
  last run, kept for the diff section.
- **`story-coverage-audit.json`** (+`.prev`) — stories ↔ features coverage data each ceremony lens reads.
- **`next-ceremony-inputs.md`** — the librarian's hand-off into the next memory ceremony.

## Sub-directories
- **`balance-sheets/`** — per-cycle memory balance sheets.
- **`horizon-scans/`** — the cartographer's quarterly LLM-memory horizon scans (`mem-horizon-scan`).
- **`<YYYY-MM-DD>/`** — dated ceremony snapshots (that cycle's audit reports, cleanup proposals, etc.).

## Retention — this is the PROCESS-ARTIFACT tier (comet shape, bounded)
Unlike plans/specs, these reports do **NOT** decompose into the documentation — their only durable value is the
**trajectory** (the dated sequence). So they follow the comet (`project_memory_lifecycle_comet_shape`), not
decompose-to-zero: `memkit-retention.py` keeps the recent cycles full (head), compacts the tail to a one-line
`_digest.md`, and memorializes the core as a single line in **`TRAJECTORY.md`** (the permanent spine; bodies stay
in git) — so the dir can never grow indefinitely yet the arc stays inferable. Loose top-level reports are filed
into their dated cycle. All of this is **gitignored except `CLAUDE.md` + `TRAJECTORY.md`**; the
`memkit:` line in `placement-audit.py --headline` surfaces overflow, and the `memory-stasis-loop` drains it
(librarian → `memkit-retention.py --apply`) as one more discipline.

## How it fits together
`placement-audit.py` measures → `--ledger` (per-file budget) / `--coverage` (un-captured backlog) /
`--stasis` (the composite **context-coverage score** vs the manifest's benchmark, ±margin). `context-ratchet.py`
is the gate (coverage can't decrease). `memory-stasis-loop` drains the surface toward stasis. The score is
the one number to watch; the manifest is where you tune it.
