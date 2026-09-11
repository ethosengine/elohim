---
id: bounded-recall-integration-report
status: DONE_WITH_CONCERNS
gap: plans__2026-09-11-bounded-recall-mastery-sprint#4
actor: agent:implementer@claude-sonnet-5
session: kit-residual-sweep-20260911
commits: []
cites:
  - bounded-recall-mastery-sprint | the sprint plan this report closes station 4 for | path: genesis/docs/superpowers/plans/2026-09-11-bounded-recall-mastery-sprint.md
---

# Station 4 — kit-residual sweep

Scope: no projected surface may instruct an agent to run a deleted memory-kit script
(`.claude/scripts/memory-kit/` and `.claude/memory-kit/`, deleted 2026-09-11). Fixed every
instructional hit found across `.claude`, `.agents`, `.epr-meta/elohim/packages`,
`.epr-meta/elohim/lenses/{CLAUDE.md,LIFECYCLE.md}` and `genesis/docs/PLACEMENT.md`, plus the
named memory entries. Left every dated, past-tense historical mention untouched.

## Surface → stale instruction → native replacement

Representative rows (grouped; the full list is ~50 files, git diff carries the exact bytes).

| Surface | Stale instruction | Native replacement |
|---|---|---|
| `skills/memory-ceremony.json` | `mempalace-currency.py --remine` / `placement-audit.py --headline` | `mempalace --palace .mempalace/palace sync --root . --apply` → `mine <surface>` per `mempalace-surfaces-changed-ceiling@1` → `date +%s.%N > .mempalace/.last-mine` (only if no step lock-blocked) + `epr flow report --headline` |
| `skills/memory-ceremony.json` | PostToolUse router "falls back to `memory-index-projector.py --apply`" | hook now **refuses** (never falls back) when a just-written entry has no contribution row yet or the native projection exceeds its latency budget — verified against the actual hook code, no fallback arm exists |
| `skills/memory-ceremony.json` "Enter the journey" | (missing) | added: `--need` alone sets intent (no `--intent` needed); `.claude/` is inside the entry's declared source scope |
| `skills/converge.json`, `agents/cartographer.json`, `agents/librarian.json`, `agents/storyteller.json` (body + metadata.description + triggerDescription + projections.{claude,codex,antigravity}.frontmatter copies) | "memkit reports" / "memkit ceremony" / "memkit hygiene pass" / "run scripts … (the memkit toolkit)" | "lens reports" / "hygiene ceremony" / "lens hygiene pass" / "(the lenses toolkit)" |
| `agents/librarian.json` "Budget / compaction tier" table | `placement-audit.py`, `decompose.py`, `cleanup-pressure.py`, `context-ratchet.py`, `memkit-retention.py`, `mempalace-currency.py`, `focus-baseline.py` | `epr flow report placement` / `--headline`, `epr flow project`, `epr flow report --bound cleanup-pressure-ceiling`, `epr flow report placement --stasis [--fold]`, — (retired, no replacement — noted honestly), `epr flow report --bound mempalace-surfaces-changed-ceiling`, `epr flow report placement --focus --brief` |
| `skills/agentic-developer.json` | `placement-audit.py --headline`; `decompose.py` (plus a stray `python3 epr flow project` typo) | `epr flow report --headline`; `epr flow project` (typo fixed too) |
| `skills/delivery-stasis.json` | `scope-reconcile.py --apply` | `epr flow hold --scope --apply` |
| `skills/elohim-epr-metafile.json` | "coverage walk (`placement-audit.py --epr-meta`)" | `epr flow report placement --stasis` (the `epr_meta_coverage` dimension) |
| `skills/p2p-design-gate.json` | "`placement-audit.py --epr-meta` … fails loud on a missing registration, an uncited contract test, or a mirrored test" | split honestly: `seam-audit.py --matrix` (registration + uncited contract test) and `--spot-check` (independent re-derivation, exit 2 on mismatch incl. mirrored test) |
| `agents/historian.json` | `placement-audit.py --ledger` | `epr flow report placement --ledger` |
| `commands/{brainstorm,plan,shift}.json` | `placement-audit.py --ledger`/`--focus`; `decompose.py` | `epr flow report placement --ledger`/`--focus`; `epr flow project` |
| `hooks/epr-meta-resolver.json` (master:package — edited package `source.body`, not the `.claude/hooks/*.py` projection) | "intervenor census (placement-audit --epr-meta)"; "full queue: `placement-audit.py --epr-meta`" | `epr flow report placement --stasis` (`epr_meta_coverage` dimension), both spots |
| `agentdocs/superpowers-held-stop-marker.json` (master:package) | "`scope-reconcile.py --apply`"; composition text naming `scope-reconcile.py` as the STOP-marker writer | `epr flow hold --scope --apply`; native writer named (`elohim/eprfs/epr-cli/src/flow/scope.rs`) |
| `.claude/hooks/_observation.py` | "Each drift-signal hook **keeps** a private JSON accumulator under `.claude/memory-kit/`" (present tense, false) | "**used to keep**" |
| `.claude/hooks/pickup-semantic-surfacing.py` | "staleness check via `mempalace-currency.py --status --json`" (docstring didn't match the actual code) | `epr flow report --bound mempalace-surfaces-changed-ceiling --json` (matches `currency_banner()`'s real subprocess call) |
| `.claude/hooks/{map-drift,placement-drift}-signal.py`, `load-project-context.py` | several "must mirror placement-audit.py's …" / "placement-audit --headline runs ONCE" present-tense claims; one self-contradicting leftover paragraph | historicized ("mirrored …, retired 2026-09-11") or pointed at `epr flow report --headline`; contradictory paragraph replaced with "There is no JSON fallback: the fold plane is the only accumulator now." |
| `.claude/hooks/.epr-meta` | "hook logic lives in `_lib/` … or `.claude/scripts/memory-kit/`" | "… or `.epr-meta/elohim/lenses/`" |
| `.claude/memory/.epr-meta` | `memory-index-projector.py` (x2), `dedupe-of: .claude/scripts/memory-kit/memory-index-projector.py`, `.claude/memory-kit/memory-index-drift.json` | `epr flow memory project --index …`, `dedupe-of: .claude/hooks/memory-index-projection.py`, native `memory-index-drift@1` fold |
| `.claude/epr-meta/concerns.yaml` | "decision-point CENSUS — `placement-audit.py --epr-meta`" | `python3 .claude/scripts/seam-audit.py --matrix` |
| `.claude/epr-meta/seam-catalog.yaml` | "loaded by … `placement-audit.py --epr-meta`" | "… `.claude/scripts/seam-audit.py --matrix`" |
| `.claude/subject-routing.yaml` | `decompose.py` (x2), "pipeline and placement-audit" | `epr flow project` (x2), "pipeline and the native placement report (`epr flow report placement`)" |
| `.claude/subject-focus.md` (generated artifact, stale since before the Rust fix landed) | "(via scope-reconcile)"; "`scope-reconcile.py --set … --apply`" | "(via epr flow report scope)"; "`epr flow hold --scope --set … --apply`" — hand-patched rather than regenerated because `--apply` would also `git mv` 3 unrelated a2o features (`genesis/a2o/**` is out of my write scope) |
| `.claude/scripts/seam-audit.py` header | present-tense "`placement-audit.py --epr-meta` RENDERS the census…" | historicized: "Before `placement-audit.py` was retired 2026-09-11, … RENDERED…" |
| `_lib/{seam_census,seam_cascade,intervenor_census,residual_channel,seam_matrix,epr_meta,epr_meta_git,subject_routing,cite_graph,env_scope}.py` docstrings/comments | present-tense "wired into / RENDERS / shared by placement-audit.py / decompose.py / scope-reconcile.py / cite-gen / cites-migrate / cite-propagate" | historicized, each naming the current runnable owner where one exists (`seam-audit.py --matrix`/`--cascade`, `epr flow project`, `epr flow cites seal\|describe\|stamp\|migrate`) or honestly stating "no runnable caller today, exercised only by its own test" where none does (`seam_census.render_census`, `epr_meta.subtree_coverage`, `_lib.env_scope`/`_lib.cluster_state` as a whole — see Known gaps) |
| `.claude/workflows/memory-stasis-loop.js` | (checked — already fully native; the one `placement-audit.py` mention is dated/historical) | no change needed |
| `.epr-meta/elohim/lenses/CLAUDE.md` | `memory-index-projector.py` renders the index (present tense); "Synthesize memkit reports"; cite tooling named as `cite-gen`/`cite-describe`/`cite-propagate`/`cites-migrate`; architecture-diagram line listing `cite-{gen,describe,propagate}.py` + `cites-migrate.py` as files "in this dir"; `_lib.env_scope`/`_lib.cluster_state` table rows naming dead callers | `epr flow memory project --index` (+ hook); "Synthesize lens reports"; `epr flow cites seal\|describe\|stamp\|migrate`; diagram line replaced with a native-verb note; both `_lib.*` rows historicized with the native Rust port path |
| `genesis/docs/PLACEMENT.md` | "## Enforcement (deterministic — **to build**, extends memory-kit)" — describes a not-yet-built hook, but the hook has existed and run natively since station one | rewritten to describe what's actually implemented: `.claude/hooks/placement-drift-signal.py` folding into `placement-drift-due@1`, surfaced via `epr flow report --headline` |
| 9 `.claude/memory/*.md` entries (`reference_memory_system`, `feedback_stale_record_feeds_memory_ceremony`, `feedback_managed_surface_edit_discipline`, `feedback_agent_prompts_no_process_status`, `reference_memory_ceremony_skill`, `project_valueflow_authoring_surface_landed`, `scope-flag-beats-prose-note`, `project_reach_enum_drift_reconciliation`, plus 2 extras found in the broad sweep: `feedback-identity-sovereignty-ontology-guard`, `project_seam_concern_architecture_landed`) | `stale-record.py`, `cite-gen --seal/--refresh`, `.claude/scripts/memory-kit/CLAUDE.md`, `decompose.py`, `cite-gen.py --refresh`, `scope-reconcile.py`, `/hygiene-sweep (memory-kit)`, `sovereignty-guard-drift.json` under `.claude/memory-kit/`, `placement-audit.py --epr-meta` | `epr flow concerns --corrections`, `epr flow cites seal/describe/stamp`, `.epr-meta/elohim/lenses/CLAUDE.md`, `epr flow project`, `epr flow cites stamp`/`verify`, `epr flow hold --scope`, "the `/hygiene-sweep` cadence, absorbed into `/memory-ceremony`", native `sovereignty-landings@1` fold, `.claude/scripts/seam-audit.py --matrix` |

After each memory-entry edit: re-imported (`epr flow memory import .claude/memory --session kit-residual-sweep-20260911`, all 93 entries `contributed`) and re-projected (`epr flow memory project --index --budget memory-index-bytes@1 --out .claude/memory/MEMORY.md --session kit-residual-sweep-20260911` → 22,436 bytes, 93 entries, `indexUnloaded: 0`).

Package-first edits (16 packages: memory-ceremony, converge, agentic-developer, delivery-stasis,
elohim-epr-metafile, p2p-design-gate, cartographer, librarian, historian, storyteller, brainstorm,
plan, shift, epr-meta-resolver [hook, `master: package`], superpowers-held-stop-marker [agentdoc,
`master: package`]) were made with a small Python helper (`apply_edits` in the scratchpad) that
loads the JSON, does exact-string replacements on named fields, bumps `metadata.version` (patch),
and writes back `indent=2` — matching the existing file style. Then projected:

```
node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs project --write-fixtures --write-runtime
node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify
```

Both ran clean; `.claude/skills/*/SKILL.md`, `.claude/agents/*.md`, `.claude/commands/*.md`,
`.claude/hooks/epr-meta-resolver.py`, `genesis/docs/superpowers/held/CLAUDE.md`, and their
`.agents/`/`.codex`/antigravity mirrors regenerated from the fixed package bodies (confirmed via
`git diff --stat`, 63 auto-projected files in addition to the 50 hand-edited source files).

## Evidence

### Grep re-run (post-fix), scoped exactly as specified

```
grep -rn -E "memkit|memory-kit|placement-audit|mempalace-currency|memory-index-projector|cleanup-scan\.py|decompose\.py|scope-reconcile\.py|focus-baseline|recall-ceremony|recall-packet|cite-gen\.py|cite-propagate|cites-migrate|stale-record\.py|memkit-retention" \
  .claude .agents .epr-meta/elohim/packages .epr-meta/elohim/lenses/CLAUDE.md .epr-meta/elohim/lenses/LIFECYCLE.md genesis/docs/PLACEMENT.md
```

122 matching lines remain (down from 393 before this station's edits, same scope, worktrees /
`.claude/epr-meta/measures.yaml` / `.claude/shifts/` excluded as out of my write set — worktrees
are stale checkouts, `measures.yaml` and shift journals are explicitly out of scope). Every
remaining line is one of these classes, all non-instructional:

1. **Explicit dated/retired framing I wrote or found already correct**, e.g.:
   - `.claude/hooks/_observation.py:6`: "…accumulator under `.claude/memory-kit/`; the replacement shape is:" (continuation of the now-fixed "used to keep" sentence on the prior line).
   - `.claude/scripts/seam-audit.py:9`: "`placement-audit.py --epr-meta` RENDERED the census…" (continuation of "Before `placement-audit.py` was retired 2026-09-11," on line 8).
   - `.claude/hooks/placement-drift-signal.py:24`, `map-drift-signal.py:29`, `claude-md-structural-signal.py:19`, `_lib/paths.py:66`: "…was deleted with the kit at station six round (b) (2026-09-11)."
2. **The lenses `CLAUDE.md` "Old table, for readers arriving from a pre-2026-09-11 citation"** (lines ~118-130): an explicit, labeled old-name→native-verb translation table — presenting the old names is the table's whole purpose, not an instruction to run them.
3. **`cluster_state.py` and its test** (`_lib/cluster_state.py`, `_lib/__tests__/cluster_state_test.py`, 21 lines together): a self-contained historical design-rationale document (git-`HEAD`-referenced, entirely past tense: "used different parsers", "HEAD `placement-audit.load_cluster_state`… were LAST-wins") explaining why three now-deleted parsers were unified into this one module. No instruction to run anything.
4. **`.claude/hooks/__tests__/drift_observation_test.py`** (10 lines): test assertions that the code does **not** emit `memkit:`/`(fallback: memory-kit)` strings and that `.claude/memory-kit/` does **not** exist — verifying absence, not instructing presence.
5. **Still-current tool names that happen to share a substring**: `memory/cleanup-scan.py` (exists today, only its `--apply` half was retired — 3 occurrences across librarian/memory-ceremony/lenses-CLAUDE.md, all phrased correctly).
6. **`.claude/data/island-recompose-gate/**`** (5 lines): dated inventory reports of a separate repair sweep, citing `.claude/memory-kit/<date>/...` paths as historical artifact locations under review, never as run instructions.
7. **One named-lesson citation** left as-is: `_lib/findings_ledger.py:79`, "(the placement-audit gate-liveness lesson)" — cites a precedent by name, doesn't instruct running it.
8. **`.claude/memory/MEMORY.md:2`** — see Known gap below; this is the one line I could not fix.

Zero instructional hits (a line telling an agent to run a deleted script as if it exists today) remain in the fixed scope.

### Verifier

```
node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify
```
`EXIT=0` — `elohim-agent package checks passed: 1972 passed` (88 packages: 73 package-first, 0 source-fidelity, 15 native).

### Hook tests

```
EPR_BIN="$(which epr)" python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
```
`Ran 34 tests in 10.420s` — `OK (skipped=1)` — `EXIT=0`. Changed files were docstrings only in
`_observation.py`, `pickup-semantic-surfacing.py`, `load-project-context.py`,
`placement-drift-signal.py`, `map-drift-signal.py`; no test reads those particular docstring
strings, and none regressed.

Also ran (not requested, extra verification): `_lib/__tests__` full discover. Two pre-existing
failures unrelated to this station's edits, confirmed by reverting `seam_matrix.py` and
re-running the same test in isolation (identical `AssertionError` with my change fully absent):
`seam_matrix_test` — `FAIL: LIVE: every crate with a seam-registry.yaml is routed to some column`
(live-data drift in the seam catalog, pre-existing). `capacity_ratification_test` and
`ci_harvest_controller_restart_test` also error on import — both are script-style test files that
call `sys.exit(bool(failures))` at import time, which `unittest.loader` reports as an ImportError
regardless of the boolean; neither file was touched by this station.

### `git diff --stat`

50 hand-edited source files (my write set) + 63 auto-regenerated projection files (from the
package-first edits, via `package-projections.mjs project --write-fixtures --write-runtime`):

```
50 files changed, 157 insertions(+), 136 deletions(-)   # hand-edited sources
```

Two additional files changed in the working tree during this session are **not mine** — concurrent
work from the other station on this sprint (station 1, same plan): `.epr-meta/elohim/algorithms/recall-contract.json`
(version 4→5, `source_roots` gains `.claude/`, `--need`-as-intent language — exactly station 1's
scope, and explicitly on my DO-NOT-EDIT list) and `.claude/memory/feedback_push_branch_discipline.md`
(an unrelated memory-ceremony edit, frontmatter reshape + a new dated finding, `originSessionId`
9f7bc2ea-…, not `kit-residual-sweep-20260911`). Neither was touched by me; noted here so the diff
isn't misattributed.

## Known gaps (found, not invented, not fixed)

1. **`elohim/eprfs/epr-cli/src/flow/memory/entries.rs:26`** — the native `epr flow memory project
   --index` command's own output template hardcodes the header
   `<!-- GENERATED — do not hand-edit. MEMORY.md is projected from .claude/memory/*.md\n
   frontmatter (title: + description:) by memory-index-projector.py --apply. -->`, naming the
   deleted script. Every regeneration of `.claude/memory/MEMORY.md` reproduces this line — I
   confirmed it by running the projector myself. This is inside `elohim/eprfs`, explicitly on my
   DO-NOT-EDIT list; I did not hand-edit the generated `MEMORY.md` (that would just be
   overwritten next projection). This needs a one-line fix in `entries.rs` by whichever seat owns
   `elohim/eprfs` — flagging it here rather than inventing a workaround.
2. **`seam_census.py`'s `render_census()`** and **`epr_meta.py`'s `subtree_coverage()`** (the
   "descending census" walker) have no runnable caller today — their former caller
   `placement-audit.py --epr-meta` was retired, and nothing in `seam-audit.py` invokes them (it
   imports `seam_cascade`, `seam_forecast`, `seam_matrix` only). I documented this honestly in
   both docstrings rather than inventing a false current owner, per the task's own instruction.
   The SessionStart-facing `epr_meta_coverage` stasis dimension is served by a **separate** native
   Rust implementation (`elohim/eprfs/epr-cli/src/flow/stasis.rs:800`), so the census is not
   unserved end-to-end — but these two Python functions specifically are dead code outside their
   own tests. Not fixed (no script to point them at); named as a finding for whoever next touches
   that module.
3. **`_lib/env_scope.py` and `_lib/cluster_state.py`** are, as modules, no longer imported by any
   runnable script in this tree (only by their own `__tests__/`) — their native ports live in
   `elohim/eprfs/epr-cli/src/flow/scope.rs`. I historicized their docstrings rather than deleting
   the modules (deletion is a bigger call than this station's scope, and their tests still pass;
   leaving dead-code removal to a follow-up).
4. **`.claude/subject-focus.md`** is a generated artifact; the Rust generator (`scope.rs:1575`)
   already emits the correct native text, so the two stale lines I hand-patched will be
   overwritten correctly on the next real `epr flow hold --scope --apply` — I did not run
   `--apply` myself because it would also `git mv` 3 currently-live a2o features into `held/`
   (pre-existing drift unrelated to this station, and `genesis/a2o/**` is on my DO-NOT-EDIT list).

## Summary for the caller

- Report path: `genesis/docs/superpowers/plans/bounded-recall-mastery/integration-report.md`
- `package-projections.mjs verify` EXIT=0 (1972 passed)
- Hook tests EXIT=0 (34 tests, 1 skipped)
- Instructional-hit count: 393 matching lines before (same grep, same scope) → 122 after, **all
  122 classified historical/non-instructional** (table above) → zero instructional hits remain.
- One native-side gap found and named, not fixed (out of write scope): `entries.rs:26`'s hardcoded
  MEMORY.md header still names `memory-index-projector.py --apply`.
- No cargo run. No commits made. No files touched under `elohim/eprfs/**`,
  `.epr-meta/elohim/algorithms/**`, `.claude/epr-meta/measures.yaml`, `genesis/a2o/**`,
  `.epr-meta/*.habit.md`, `genesis/manifests/**`, or `elohim/elohim-storage/**`.
