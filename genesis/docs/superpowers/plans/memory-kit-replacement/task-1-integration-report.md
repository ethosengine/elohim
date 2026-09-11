---
id: memory-kit-replacement-task-1-integration-report
title: Station one, integration seat — thresholds declared, drift signals rewired to observations
status: DONE_WITH_CONCERNS
class: devflow
gap: plans__2026-09-10-memory-kit-replacement-finish#1
actor: agent:implementer@claude-opus-5
commits: []
cites: []
---

# Station one — integration seat

The 35 thresholds the memory kit hard-codes as module constants are now declared rows in
`.claude/epr-meta/measures.yaml`, the six drift-signal hooks append structured observations
through `epr flow note --kind observation --measure` (keeping their JSON write as the
fallback while that flag is absent from the pinned binary), and both SessionStart headline
consumers ask `epr flow report --headline` first.

The native seat's `epr flow report` landed mid-session and reads these rows: a `--json` run
against the live tree returns **32 outcomes (1 passed, 0 failed, 31 skipped)** with a method
pin of `measuresCid: bafkreieb27ojol2b5fop5633u3a3mucz42wmejsn2eubcnth7spzkuh7ki` /
`policiesCid: bafkreiegoqni66hxe3xzovrv3k7mgy63hwyjxxpvjfgz5ivvml2jljtkqy`. That is the seam
proven across both seats — every bound this seat declared is resolvable by the native report,
and `skipped` (never zero) is what a bound with no fold reports.

## Gate evidence

`epr flow context` on every file in the write set prints `GATE (no gate project covers this
path)` — `.claude/**` has no `gate.projects` entry in any `build-manifest.json`, so the gate
line the context printed is the one honoured here: the two suites the brief names, plus the
drift-observation suite added by this station.

```
python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
Ran 13 tests ... OK
EXIT=0

# just gate memory-ceremony — the two legs this seat can run without taking the
# parallel native seat's cargo berth (legs 2/4/5 rebuild elohim-epr-cli and drive
# cucumber against it; that binary is the native seat's, in flight this session)
python3 -m unittest discover -s genesis/scripts/__tests__ -p memory_balance_test.py
Ran 9 tests ... OK
EXIT=0

EPR_BIN=/tmp/eprfs-gate-target/debug/epr \
  python3 -m unittest discover -s .claude/scripts/memory-kit/__tests__ -p 'recall*_test.py'
Ran 60 tests ... OK
EXIT=0

python3 .claude/scripts/memory-kit/placement-audit.py --headline
EXIT=0

python3 -m unittest discover -s .claude/scripts/_lib/__tests__ -p '*_test.py'
Ran 14 tests ... FAILED (errors=3)
EXIT=1
```

The three `_lib` errors are **pre-existing and structural, not caused by this station**:
`capacity_ratification_test.py`, `ci_harvest_controller_restart_test.py` and
`seam_matrix_test.py` call `sys.exit()` at module scope, so unittest's loader raises
`SystemExit` while importing them. All three are unmodified in git (`git status --short` on
those paths is empty), none of them reads `measures.yaml` or any rewired hook, and run
directly the first two exit 0 while `seam_matrix_test.py` fails on its own live assertion
(`every crate with a seam-registry.yaml is routed to some column`).

## 1. Declared bounds

### `measures:` rows declared (34)

| id@version | family | unit | provenance (the constant it replaces) |
|---|---|---|---|
| `memory-index-bytes@1` | memory-index | bytes | .claude/scripts/memory-kit/memory-index-projector.py:53 SOFT_BUDGET_BYTES=20_000; :54 HARD_BUDGET_BYTES=24_000; .claude/scripts/memory-kit/memory-review.py:45 MEMORY_BUDGET_BYTES=24000; .claude/scripts/memory-kit/placement-audit.py:98 MEMORY_MD_BUDGET=24000 |
| `memory-index-lines@1` | memory-index | lines | .claude/scripts/memory-kit/memory-review.py:41 MEMORY_BUDGET_LINES=200 |
| `memory-index-title-chars@1` | memory-index | characters | .claude/scripts/memory-kit/memory-index-projector.py:56 TITLE_MAX=80 |
| `memory-index-desc-chars@1` | memory-index | characters | .claude/scripts/memory-kit/memory-index-projector.py:55 DESC_MAX=200 |
| `gospel-bytes@1` | gospel | bytes | .claude/scripts/memory-kit/placement-audit.py:101 CLAUDE_MD_BUDGET=12000 |
| `gospel-lines@1` | gospel | lines | .claude/scripts/memory-kit/claude-md-audit.py:55 LINE_BUDGET_WARN=200 |
| `claude-md-edit-signal@1` | gospel | edits | .claude/hooks/claude-md-drift-signal.py:145-172 (scope_edits / direct_edits accumulator) |
| `claude-md-structural-signal@1` | gospel | structural-ops | .claude/hooks/claude-md-structural-signal.py:164-191 (structural_edits accumulator) |
| `claude-md-drift-score@1` | gospel | drift-score | .claude/scripts/memory-kit/claude-md-audit.py:56 DEFAULT_DRIFT_THRESHOLD=3.0; .claude/hooks/claude-md-drift-signal.py:53 DEFAULT_THRESHOLD=3.0; .claude/hooks/claude-md-structural-signal.py:44 DEFAULT_THRESHOLD=3.0 |
| `claude-md-rescore-interval@1` | gospel | edits | .claude/hooks/claude-md-drift-signal.py:52 RESCORE_EVERY_N_EDITS=5 |
| `claude-md-walk-depth@1` | gospel | directories | .claude/hooks/claude-md-drift-signal.py:54 MAX_WALK_DEPTH=12; .claude/hooks/claude-md-structural-signal.py:43 MAX_WALK_DEPTH=12 |
| `memkit-report-tier-mb@1` | report-tier | megabytes | .claude/scripts/memory-kit/memkit-retention.py:16 SIZE_CAP_MB=8 |
| `retention-head-days@1` | report-tier | days | .claude/scripts/memory-kit/memkit-retention.py:14 HEAD_DAYS=30 |
| `retention-tail-days@1` | report-tier | days | .claude/scripts/memory-kit/memkit-retention.py:15 TAIL_DAYS=90 |
| `cleanup-pressure@1` | cleanup-pressure | pressure-points | .claude/scripts/memory-kit/cleanup-pressure.py:47 THRESHOLD=120; .claude/scripts/memory-kit/prep-brainstorm.py:30 DRIFT_THRESHOLD=120 |
| `cleanup-cycle-history@1` | cleanup-pressure | cycles | .claude/scripts/memory-kit/cleanup-pressure.py:52 MAX_CYCLES=24 |
| `decompose-threshold@1` | placement | gap-items | .claude/scripts/memory-kit/decompose.py:46 THRESHOLD=40 |
| `placement-drift-due@1` | placement | documents | .claude/hooks/placement-drift-signal.py:78 DEFAULT_THRESHOLD=1 |
| `stasis-margin@1` | placement | fraction | .claude/scripts/memory-kit/placement-audit.py:97 STASIS_MARGIN=0.15 |
| `map-currency-drift@1` | map-currency | seeds | .claude/hooks/map-drift-signal.py:70 DEFAULT_THRESHOLD=1 |
| `entry-stale-days@1` | memory-entry | days | .claude/scripts/memory-kit/memory-review.py:46 STALE_DAYS=90 |
| `memory-coherence-drift@1` | memory-entry | entry-hits | .claude/hooks/memory-coherence-signal.py:116-129 (entries accumulator; no hard-coded threshold) |
| `package-description-chars@1` | package-quality | characters | .claude/scripts/memory-kit/agent-audit.py:49 MIN_DESCRIPTION_CHARS=80; .claude/scripts/memory-kit/skill-audit.py:36 MIN_DESCRIPTION_CHARS=60 |
| `surface-stale-mtime-days@1` | package-quality | days | .claude/scripts/memory-kit/agent-audit.py:50 STALE_MTIME_DAYS=90; .claude/scripts/memory-kit/skill-audit.py:37 STALE_MTIME_DAYS=90 |
| `trigger-overlap@1` | package-quality | shared-words | .claude/scripts/memory-kit/agent-audit.py:51 TRIGGER_OVERLAP_THRESHOLD=3; .claude/scripts/memory-kit/skill-audit.py:38 TRIGGER_OVERLAP_THRESHOLD=3 |
| `dynamic-stopword-fraction@1` | package-quality | fraction | .claude/scripts/memory-kit/agent-audit.py:52 DYNAMIC_STOPWORD_FRACTION=0.25; .claude/scripts/memory-kit/skill-audit.py:69 DYNAMIC_STOPWORD_FRACTION=0.25 |
| `rationale-window-lines@1` | package-quality | lines | .claude/scripts/memory-kit/agent-audit.py:76 RATIONALE_WINDOW=3; .claude/scripts/memory-kit/claude-md-audit.py:65 RATIONALE_WINDOW=3 |
| `exempt-substr-window-chars@1` | substrate-currency | characters | .claude/scripts/memory-kit/substrate-currency-audit.py:366 EXEMPT_SUBSTR_WINDOW=80 |
| `negation-window-chars@1` | substrate-currency | characters | .claude/scripts/memory-kit/substrate-currency-audit.py:368 NEGATION_WINDOW=60 |
| `bare-negation-window-chars@1` | substrate-currency | characters | .claude/scripts/memory-kit/substrate-currency-audit.py:369 BARE_NEGATION_WINDOW=20 |
| `mempalace-mine-grace-seconds@1` | mempalace | seconds | .claude/scripts/memory-kit/mempalace-currency.py:27 GRACE_SECONDS=120 |
| `rename-window-days@1` | path-currency | days | .claude/scripts/memory-kit/path-update-scan.py:61 DEFAULT_RENAME_WINDOW_DAYS=365 |
| `story-vision-weight@1` | story-coverage | weight | .claude/scripts/memory-kit/story-coverage-audit.py:106 DEFAULT_VISION_WEIGHT=1.0 |
| `sovereignty-landings@1` | sovereignty-guard | landings | .claude/hooks/sovereignty-guard-signal.py:71 _ESCALATE_AT=3 |

### `lenses:` ceiling rows declared (31)

| id@version | consumes | class | context | soft | hard |
|---|---|---|---|---|---|
| `memory-index-bytes-ceiling@1` | `memory-index-bytes@1` | measure | memory-index-projection | 20000 | 24000 |
| `memory-index-lines-ceiling@1` | `memory-index-lines@1` | measure | memory-review | — | 200 |
| `memory-index-title-chars-ceiling@1` | `memory-index-title-chars@1` | measure | memory-index-projection | — | 80 |
| `memory-index-desc-chars-ceiling@1` | `memory-index-desc-chars@1` | measure | memory-index-projection | — | 200 |
| `gospel-bytes-ceiling@1` | `gospel-bytes@1` | inject | session-headline | — | 12000 |
| `gospel-lines-ceiling@1` | `gospel-lines@1` | measure | claude-md-audit | 200 | — |
| `claude-md-drift-score-ceiling@1` | `claude-md-drift-score@1` | inject | session-headline | — | 3.0 |
| `claude-md-rescore-interval-ceiling@1` | `claude-md-rescore-interval@1` | measure | drift-accumulator | — | 5 |
| `claude-md-walk-depth-ceiling@1` | `claude-md-walk-depth@1` | measure | drift-accumulator | — | 12 |
| `memkit-report-tier-mb-ceiling@1` | `memkit-report-tier-mb@1` | inject | session-headline | 8 | — |
| `retention-head-days-ceiling@1` | `retention-head-days@1` | measure | report-tier-retention | — | 30 |
| `retention-tail-days-ceiling@1` | `retention-tail-days@1` | measure | report-tier-retention | — | 90 |
| `cleanup-pressure-ceiling@1` | `cleanup-pressure@1` | inject | session-headline | — | 120 |
| `cleanup-cycle-history-ceiling@1` | `cleanup-cycle-history@1` | measure | cleanup-pressure | — | 24 |
| `decompose-threshold-ceiling@1` | `decompose-threshold@1` | measure | decompose | — | 40 |
| `placement-drift-due-ceiling@1` | `placement-drift-due@1` | inject | session-headline | — | 1 |
| `stasis-margin-ceiling@1` | `stasis-margin@1` | measure | placement-stasis | — | 0.15 |
| `map-currency-drift-ceiling@1` | `map-currency-drift@1` | inject | session-headline | — | 1 |
| `entry-stale-days-ceiling@1` | `entry-stale-days@1` | measure | memory-review | — | 90 |
| `agent-description-floor@1` | `package-description-chars@1` | measure | agent-audit | — | 80 |
| `skill-description-floor@1` | `package-description-chars@1` | measure | skill-audit | — | 60 |
| `surface-stale-mtime-days-ceiling@1` | `surface-stale-mtime-days@1` | measure | package-quality-audit | — | 90 |
| `trigger-overlap-ceiling@1` | `trigger-overlap@1` | measure | package-quality-audit | — | 3 |
| `dynamic-stopword-fraction-ceiling@1` | `dynamic-stopword-fraction@1` | measure | package-quality-audit | — | 0.25 |
| `rationale-window-lines-ceiling@1` | `rationale-window-lines@1` | measure | package-quality-audit | — | 3 |
| `exempt-substr-window-chars-ceiling@1` | `exempt-substr-window-chars@1` | measure | substrate-currency-audit | — | 80 |
| `negation-window-chars-ceiling@1` | `negation-window-chars@1` | measure | substrate-currency-audit | — | 60 |
| `bare-negation-window-chars-ceiling@1` | `bare-negation-window-chars@1` | measure | substrate-currency-audit | — | 20 |
| `mempalace-mine-grace-seconds-ceiling@1` | `mempalace-mine-grace-seconds@1` | inject | session-headline | — | 120 |
| `rename-window-days-ceiling@1` | `rename-window-days@1` | measure | path-update-scan | — | 365 |
| `sovereignty-landings-ceiling@1` | `sovereignty-landings@1` | inject | post-tool-guard | — | 3 |

### Coverage of the 35 constants

`grep -nE '^[A-Z_]{4,}\s*=\s*[0-9.]+' .claude/scripts/memory-kit/*.py` returns exactly 35
lines. Every one is cited by a `provenance:` field above. Five measures absorb more than one
constant because the constant is literally duplicated across scripts, and declaring it once
is the point of the registry:

- `memory-index-bytes@1` ← `memory-index-projector.py:53,54`, `memory-review.py:45`,
  `placement-audit.py:98` (three scripts, one 24000).
- `package-description-chars@1` ← `agent-audit.py:49` (80) and `skill-audit.py:36` (60) — one
  measure, two FLOOR rows (`agent-description-floor`, `skill-description-floor`, both
  `compare: below`), which is
  the registry's own answer to a per-context threshold.
- `surface-stale-mtime-days@1` ← `agent-audit.py:50`, `skill-audit.py:37`.
- `trigger-overlap@1` ← `agent-audit.py:51`, `skill-audit.py:38`.
- `dynamic-stopword-fraction@1` ← `agent-audit.py:52`, `skill-audit.py:69`.
- `rationale-window-lines@1` ← `agent-audit.py:76`, `claude-md-audit.py:65`.
- `cleanup-pressure@1` ← `cleanup-pressure.py:47`, `prep-brainstorm.py:30`.
- `claude-md-drift-score@1` ← `claude-md-audit.py:56`, `claude-md-drift-signal.py:53`,
  `claude-md-structural-signal.py:44`.
- `claude-md-walk-depth@1` ← `claude-md-drift-signal.py:54`, `claude-md-structural-signal.py:43`.

Beyond the 35, the six drift accumulators' own thresholds are declared:
`placement-drift-signal.py:78`, `map-drift-signal.py:70`, `claude-md-drift-signal.py:52,53,54`,
`claude-md-structural-signal.py:43,44`, `sovereignty-guard-signal.py:71`.
`memory-coherence-signal.py` hard-codes no threshold — it is a pure accumulator — so
`memory-coherence-drift@1` is declared with no ceiling row rather than inventing a watermark.

### Rows deliberately without a ceiling

`story-vision-weight@1` (a default weight applied to an unranked axis, not a watermark),
`memory-coherence-drift@1` (no threshold exists), and `claude-md-edit-signal@1` /
`claude-md-structural-signal@1` (inputs the drift score is derived from; the watermark lives
on `claude-md-drift-score@1`, not on its terms).

### The plurality ruling in force on this gap

`epr flow context` on the plan surfaces a `run:ruling` note (operator, 2026-09-10): this
policy set is **one lens** over the substrate, chosen as an intentional act; plurality is the
default, rows carry a binding level and lineage so a set can fork and be ratified upward, and
env-keyed folds keep efficacy claims qualified by where they were observed. What this station
owes that ruling is satisfied — every lens row carries a `PRECEDENT_BINDING` level, no row
mutates or overwrites another, and the emitters pass `--env` on every observation
(`status=`, `artifact=`, `kind=`, `op=`, `changed=`, `rule=`, `tool=`) so folds are env-keyed
from the first one. What it does **not** yet have is a declared default *recipe* — the rows
sit directly under `measures:`/`lenses:` with no recipe grouping, so simultaneous A/B
evaluation of two recipes over the same records has no place to read from. That is a
report-side and registry-shape question the native seat and a later station should settle
together; flagging it rather than inventing a grouping unilaterally.

### Registry contract as kept

- Every measure carries `default-authority: observation` and no watermark — teeth-free by
  construction, as the registry's contract requires.
- Every lens carries `binding: binding-local` from `PRECEDENT_BINDING`
  (`elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:36`), the enforcement
  `class` the consuming surface has today (`inject` where the value reaches an operator as
  injected context — the SessionStart headline lines and the sovereignty guard's PostToolUse
  message; `measure` everywhere else), and a `context`.
- Ceiling lenses are named `<measure-id>-ceiling` and consume exactly that measure, so a bound
  resolves from either end. The two per-context floors over `package-description-chars@1` are
  the deliberate exception: they are named for their context and compare `below`, not above.
- `soft:`/`hard:` follow the brief: single-threshold rows use `hard:` only, except the two
  whose source constant names itself soft — `memkit-report-tier-mb@1` (`SIZE_CAP_MB`, "soft
  cap for the whole tier", and named soft in the brief) and `gospel-lines@1`
  (`LINE_BUDGET_WARN`, a per-file *warning* threshold). Reading either as hard would
  misdescribe what the kit does.
- Version pins are declared, nothing was deleted, and the top-of-file comment states in one
  sentence that these files are projections of a policy set whose graduated home is the
  Mishpat Precedent entry.
- **`supersedes:` is absent by construction, and this is a finding, not an omission.** The
  registry held only the clippy family before this batch, so no row here replaces an earlier
  *registry row*; the constants each row replaces are cited in `provenance:`, which is a
  citation and never part of a row's identity (identity is `<id>@<version>` plus the row's own
  bytes). Minting a `supersedes:` edge to a Python line number would put a local path inside
  identity — exactly what the correction forbids.
- **`subsumes:` is likewise absent.** The registry defines it as an evidence-backed
  byte-containment claim verifiable by conformance probe. No probe has been run between any
  two of these measures, so declaring one would be an unbacked assertion.

## 2. Hook rewiring — every line changed

New file `.claude/hooks/_observation.py` (147 lines): `resolve_bin()` (`$EPR_BIN` → 
`/tmp/eprfs-gate-target/debug/epr` → `epr` on PATH), `available()` (probes
`epr flow note --help` for `--measure`, falling through to the bare `epr flow` usage dump
because today `note --help` answers with an argument error rather than a usage listing),
`emit()`, `reset_cache()`. The probe is cached in-process **and** on disk keyed by the
binary's path+mtime+size, because these hooks run on every `Edit|Write`: an unchanged binary
never spawns a probe again, and a rebuilt one re-probes automatically (which is exactly what
happened when the native seat's binary landed mid-session).

| hook | line(s) | change |
|---|---|---|
| `placement-drift-signal.py` | 58 | import `_observation as _obs` |
| | 181-185 | `_obs.emit("placement-drift-due@1", rel, 1 if status else 0, …)`; returns before the JSON write on success |
| | 22-24 | docstring: storage is the fold, JSON named as the fallback |
| `map-drift-signal.py` | 60 | import `_obs` |
| | 157-164 | `_obs.emit("map-currency-drift@1", rel, 0 if name == WALK_ARTIFACT else 1, …)` — `0` on a MAP.md refresh preserves the accumulator's self-healing reset |
| | 27-29 | docstring |
| `claude-md-drift-signal.py` | 53 | import `_obs` |
| | 181-196 | one `claude-md-edit-signal@1` observation per enclosing scope, `--env kind=direct\|scope`; falls back whole if any emit fails |
| | 18-20 | docstring |
| `claude-md-structural-signal.py` | 45 | import `_obs` |
| | 162-181 | one `claude-md-structural-signal@1` observation per affected scope, `--env op=<op>` |
| | 18-21 | docstring |
| `memory-coherence-signal.py` | 53 | import `_obs` |
| | 134-144 | one `memory-coherence-drift@1` observation per matched entry, subject `.claude/memory/<slug>.md`, `--env changed=<rel>` |
| | 19-21 | docstring |
| `sovereignty-guard-signal.py` | 42 | import `_obs` |
| | 130-150 | `sovereignty-landings@1` observation with `--env rule=<rule@version> tool=<tool>`; the private tally is now the `else` branch |
| | 19-22 | docstring |

The `.claude/data/sovereignty-guard.jsonl` landing ledger is untouched — this station moved
the *tally*, not the ledger. Nothing prints anything it did not print before, on either path.

## 3. Headline rewiring

| file | line(s) | change |
|---|---|---|
| `load-project-context.py` | 74-96 | new `_epr_headline()` — resolves the binary through `_observation.resolve_bin()` and runs `epr flow report --headline --root <project>`; returns `""` on absent verb, non-zero exit or timeout |
| | 99-129 | `get_memory_budget()` asks native first; on `""` falls back to `placement-audit.py --headline` and appends a `(fallback: memory-kit)` line; the `/tmp` headline cache is written on **either** path, unchanged in location and format |
| `delivery-gate.py` | 24-31 | new `_epr_bin()` (same resolution order) |
| | 34-69 | `headline_text()` keeps the <120s `/tmp` cache read first, then native, then `placement-audit.py` with the `(fallback: memory-kit)` line |

`placement-audit.py` was **not modified**. The `(fallback: memory-kit)` marker is appended by
the two consumers rather than printed by the kit, which keeps
`placement-audit.py --headline` byte-identical to what it printed before this station — the
brief's own verification asks for exactly that.

## 4. Tests

`.claude/hooks/__tests__/drift_observation_test.py` — 13 tests, unittest, no pytest. A stub
`epr` recording every argv as JSON, with `MEASURE_VERB` / `REPORT_VERB` switching the two
sides of each probe, and an isolated `TMPDIR` so the probe's disk cache never touches the
real one.

- verb absent → the JSON accumulator is still written, stdout is empty, and no `--measure`
  call was attempted (the probe *did* run);
- verb present → the observation lands with `--kind observation`, the right `--measure`,
  `--subject` and `--value`, and **no** JSON file is written;
- a re-opened doc observes `0` (the self-heal path survives the rewire);
- each of the six hooks emits its own declared measure id in native mode
  (`placement-drift-due@1`, `map-currency-drift@1`, `claude-md-edit-signal@1`,
  `claude-md-structural-signal@1`, `memory-coherence-drift@1`, `sovereignty-landings@1`) and
  the sovereignty guard still prints its PostToolUse message and still appends its jsonl;
- a subTest sweep proving all six still write their JSON artifact when the verb is absent —
  the fallback branch is the real regression risk;
- the headline prefers native, falls back with a visible `(fallback: memory-kit)` marker, and
  writes the `/tmp` cache on both paths;
- a missing binary costs zero subprocesses.

## Concerns

1. **The native headline is currently thinner than the kit's, and it now wins.** With
   `epr flow report --headline` live, SessionStart prints five lines (`memkit:`,
   `mempalace:`, `cleanup:`, `scope:`, `memory-budget:`), four of them `skipped (no fold for
   …)` and `scope: skipped (no bound declared)`. The kit's headline carries eleven lines
   including `debt:`, `testable env:`, `review:`, `decompose:`, `path:` and `epr-meta:`, and
   the actionable `cleanup: pressure 250/120` and `scope: ⚠ 3 to hold` values that root
   `CLAUDE.md` declares as session triggers. The brief's fallback rule is "only when the verb
   is absent or fails", and a `skipped` bound is neither, so this seat did not add a
   thinness heuristic. **Until the native seat folds observations and ports the remaining
   headline lines, SessionStart loses those triggers.** The cheapest interim is for the
   native report to emit nothing (exit non-zero) while every bound is `skipped`, which the
   fallback already handles.
2. **`scope:` has no bound to declare.** It is a state readout derived from
   `cluster-state.yaml` and the `requires_env` resolvers, not a numeric threshold, so no
   measure row fits it. Station two owns `epr flow report scope`; noting it here so the empty
   line is read as routed, not forgotten.
3. **Two pre-existing `policies.yaml` rows carry `binding: observation`**
   (`source-file-loc-ceiling@1:31`, `capability-governance@1:115`), which is not a
   `PRECEDENT_BINDING` value — the same drift the operator's correction fixes for this batch.
   Both are operator-ratified (`established_by: operator-2026-07-02` / `operator-2026-07-10`),
   so this seat did not rewrite them. They need one operator-authorised pass to `binding:
   observation → binding-local` (or whichever the operator intends) before the registry can
   lift as a whole.
4. **Sovereignty escalation semantics shift when the verb lands.** In native mode the hook has
   no private tally, so its `_ESCALATE_AT` comparison reads this edit's net-new rather than
   the running total. The threshold is declared as `sovereignty-landings-ceiling@1` (hard 3,
   `class: inject`), so aggregation belongs to `epr flow report`. Today the fallback path runs
   and behaviour is unchanged; the delta arrives with the `--measure` flag.
5. The three `_lib` suite loader errors above are pre-existing (evidence in the gate section).

## Not done, by boundary

`elohim/eprfs/**`, packages, skills, and `.claude/scripts/memory-kit/*` are the parallel
seat's or a later station's. `.claude/settings.json` was not edited (station four swaps the
`memory-index-projector.py --hook` registration). No `.claude/memory-kit/*.json` file was
deleted — station six owns that. No commits, no pushes.

---

# Follow-up — the reader migrated, so the producer is bridged

Concern 1 above ("the native headline is thinner than the kit's, and it now wins") is closed
by a **producer bridge** rather than a thinness heuristic, per the coordinator's direction. The
reader moved to the native report in the first pass; the kit is still the thing that *computes*
these values until station two ports them, so the bridge runs the kit once, folds each headline
line onto the bound that line reports, and hands the kit's text back so no caller pays for a
second run.

## 5. The producer bridge

`.claude/hooks/_observation.py` gains `parse_headline()` and `bridge_headline()`.

| headline line (producer) | measure fed | value read |
|---|---|---|
| `cleanup: pressure N/120` (`cleanup-pressure.py:212`) | `cleanup-pressure@1` | `N` |
| `memkit: over cap (N.NMB > 8MB cap)` / `memkit: N.NMB / K cycles` (`memkit-retention.py:123,124`) | `memkit-report-tier-mb@1` | `N.N` |
| `mempalace: ⚠ N surface file(s) changed` / `mempalace: fresh ✅` (`mempalace-currency.py:89,91`) | `mempalace-surfaces-changed@1` | `N` / `0` |
| `scope: ⚠ N to hold · N to return to plate · N deployment flag(s)` / `scope: aligned ✅` (`scope-reconcile.py:425-455`) | `scope-pending-moves@1` | sum of parts / `0` |

Subject `.` (repository-wide) on all four, `--env head=<short sha> producer=placement-audit.py --headline`.

Three properties the bridge holds:

- **A dead sub-gate yields no fold.** `placement-audit.py` prints `⚠ gate-error (…)` when a
  sub-gate dies; that line matches no value pattern, so the bound reports `skipped` — never
  `0`. Zero and unmeasured are different answers and the bridge keeps them different.
- **Idempotent.** Values stay strings exactly as the kit printed them, and a `/tmp` memo keyed
  by `HEAD` suppresses a re-emit of an unchanged value, so re-running SessionStart does not
  grow the log even before native CID dedupe is exercised.
- **Inert until the verb exists.** `bridge_headline()` returns `""` when
  `epr flow note --measure` is absent, so today it costs nothing and changes nothing.

Wiring: `load-project-context.py:118-125` and `delivery-gate.py:50-60` run the bridge **before**
`epr flow report --headline`, and reuse its returned text for the `(fallback: memory-kit)` path
so `placement-audit.py` still runs at most once per SessionStart.

### A refusal the tests would not have caught, and now do

Reading `elohim/eprfs/epr-cli/src/flow/mod.rs:685-691` (the native `run_observation`, already in
source though not yet in the pinned binary) turned up a hard incompatibility: **`--measure`
refuses `--on`** — *"a structured observation's subject IS its target."* The first pass's
`emit()` passed `--on <subject>` unconditionally, so every native emission would have been
refused the moment the binary was rebuilt, silently falling back forever. `emit()` now omits
`--on`, and the test stub was hardened to mirror the real verb's refusals (`--on` present,
`--subject`/`--value` missing, non-numeric `--value`) so this class of drift fails loudly in
the suite instead of in production.

## 6. The declared default recipe

`.epr-meta/manifest.md` frontmatter now carries `policy-recipe:` and a `policy-recipes:` map.

The coordinator's shape (`policy-recipe: default` + a map naming the two files) did not match
what the native seat implemented: `report.rs:358-367` `declared_default()` reads
`policy-recipe:` as a **directory** and appends `measures.yaml` / `policies.yaml`
(`Recipe::at_dir`). `policy-recipe: default` resolved to `<root>/default/measures.yaml` and the
report refused every run with *"recipe `default` has no bound registry to report over."* So the
scalar is `.claude/epr-meta` — the directory holding exactly the two files the coordinator
named — and the `policy-recipes:` map keeps the recipe under its declared name `default` with
both explicit paths, which is where a second recipe gets added. One consequence to note: the
report labels the recipe from the directory's basename, so the headline prints
`recipe: epr-meta@<cid>` rather than `recipe: default@<cid>`. Giving the recipe its declared
name needs either a `name:` key in `declared_default()` or a directory rename — the native
seat's call, not this seat's.

## 7. The mempalace headline binding

`report.rs:108-115` derives a headline slot from the measure **id prefix**, so both
`mempalace-*` rows claimed the `mempalace:` line and it resolved to
`mempalace-mine-grace-seconds@1` — a grace window, not the staleness signal. Two changes:

- the grace tolerance is renamed `mine-grace-seconds@1` (still `family: mempalace`, class
  corrected to `measure`/`mempalace-currency` since it is not a headline line). Amended in the
  authoring session before landing, which the registry's contract explicitly permits;
- all four bridge-fed rows now carry an explicit `headline:` slot (`measures.rs:298-299`
  supports it and prefers it over the id-derived default), so the binding is **declared**
  rather than inferred and the whole prefix-collision class is closed:
  `memkit-report-tier-mb@1 → memkit`, `mempalace-surfaces-changed@1 → mempalace`,
  `cleanup-pressure@1 → cleanup`, `scope-pending-moves@1 → scope`.

Verified live: the native headline's four lines now name exactly the four ids the bridge feeds.

## 8. PRECEDENT_BINDING alignment and its pins

`source-file-loc-ceiling@1` and `capability-governance@1` move `binding: observation →
binding-local` (vocabulary alignment to `PRECEDENT_BINDING`, authorised by the operator's
plurality ruling `bafyreiakmc4b3ojogxcv3hphpr2arfqwdqugybqu2i3yv7xprwbgfclthi` recorded on the
plan). Re-pinned in place rather than versioned: `class: measure` — the operative field — is
unchanged, and a new version would orphan the root manifest's `policy: source-file-loc-ceiling@1`
binding.

The registry is tamper-detecting, so the edit stranded both `contentHash:` pins and routed
their bindings to `policy-pin-mismatch` review. Repaired with the canonical tools:

```
python3 .claude/scripts/epr-meta-pin.py --write
[epr-meta-pin] wrote 19 contentHash pin(s) to .claude/epr-meta/policies.yaml.
[epr-meta-pin] wrote 15 contentHash pin(s) to .claude/epr-meta/concerns.yaml.
EXIT=0
# only the two edited rows actually changed; concerns.yaml is byte-identical

epr canon-lift --write
canon-lift: 34 atoms · 27 declared heads · 34 standing records
EXIT=0
# wrote the 4 files under .claude/epr-meta/generated/canon-lift/ and nothing else
```

Both are idempotent on a second run, and `epr_meta.load_policies()` returns zero errors.

## Verification after the follow-up

```
python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
Ran 19 tests ... OK
EXIT=0

python3 .claude/scripts/memory-kit/placement-audit.py --headline
EXIT=0

python3 -m unittest discover -s .claude/scripts/_lib/__tests__ -p '*_test.py'
Ran 14 tests ... FAILED (errors=3)
EXIT=1        # the same three pre-existing module-level sys.exit loader errors as before

# discovery hides real results behind those three, so every _lib test was also run directly:
for f in .claude/scripts/_lib/__tests__/*_test.py; do python3 "$f"; done
only seam_matrix_test.py fails (pre-existing: "every crate with a seam-registry.yaml is
routed to some column"), and it failed the same way before this station

python3 -m unittest discover -s genesis/scripts/__tests__ -p memory_balance_test.py    EXIT=0
EPR_BIN=… python3 -m unittest discover -s .claude/scripts/memory-kit/__tests__ \
  -p 'recall*_test.py'                                                                 EXIT=0
python3 .claude/scripts/_lib/__tests__/epr_meta_cascade_test.py                        EXIT=0
every .claude/scripts/memory-kit/__tests__/*_test.py run directly                       all EXIT=0
```

Six new tests cover the bridge: the fixture headline parses to the expected four
`(measure, value)` pairs; the healthy shapes read `0` and the gate-error shapes read *nothing*;
a multi-part `scope:` sums; the emitted argv set carries the right measure, value and `.`
subject and **no `--on`**; a second run emits nothing; and — the point of the whole change — a
SessionStart with the native verb present prints `cleanup: pressure 250/120`,
`scope: 3 pending move(s)`, `memkit: 8.3MB` and `mempalace: 1 surface file(s) changed` with no
`(fallback: memory-kit)` line and no `skipped (no fold …)`.

## Standing concerns after the follow-up

1. **The bridge is armed but inert until the CLI is rebuilt.** `--measure` exists in
   `flow/mod.rs:633` but not in the pinned `/tmp/eprfs-gate-target/debug/epr`, so today's
   SessionStart still shows four `skipped` lines. Nothing further is needed from this seat: the
   next build of the native seat's binary activates the bridge, and the suite proves the result.
2. **`recipe: epr-meta@<cid>`, not `recipe: default@<cid>`** — see §6; the native seat owns
   whether a recipe can carry a declared name distinct from its directory.
3. Sovereignty escalation still reads per-edit net-new once the native path activates
   (aggregation belongs to the report) — unchanged from the first pass.
4. `seam_matrix_test.py` remains red for its own unrelated reason.

## Write-set note

`.epr-meta/manifest.md` and `.claude/epr-meta/generated/canon-lift/*` sit outside the write set
the original brief fixed; both were edited on the coordinator's explicit instruction (the recipe
declaration, and `epr canon-lift --write`). Every other `.epr-meta/elohim/**` path git reports as
modified was already dirty before this seat started and was not touched. No commits, no pushes.

---

# Review response — comparison semantics, a real golden, and the recipe label

Station one review returned changes-requested. Four integration-seat items, all closed. The
native binary was rebuilt mid-pass (`sha256:6cdbf177edac8ad2…`, 2026-09-10 15:20) and now
carries `--measure`, `compare:` and the map-key recipe label, so everything below is verified
against the real thing rather than argued.

## 9. `compare:` declared to match each kit constant exactly

`measures.rs:129-176` defines the vocabulary (`above` | `at-or-above`, with `gt`/`gte` aliases)
and `bound_from` reads `compare:` off the **bound row** — a `lenses:` row or a `class: measure`
policy row — defaulting to `above`. Nine ceiling lenses now declare it, each read off the kit's
own comparison rather than assumed:

| ceiling lens | kit comparison | declared |
|---|---|---|
| `cleanup-pressure-ceiling@1` | `cleanup-pressure.py:196` `due = a >= THRESHOLD` | `at-or-above` |
| `mempalace-surfaces-changed-ceiling@1` | `mempalace-currency.py:84` `stale = changed > 0` → fires at 1 | `at-or-above` |
| `scope-pending-moves-ceiling@1` | `scope-reconcile.py:453-455` — any part present warns | `at-or-above` |
| `placement-drift-due-ceiling@1` | `placement-drift-signal.py:78` "any past-due doc is worth surfacing" | `at-or-above` |
| `map-currency-drift-ceiling@1` | `map-drift-signal.py:70` "any seed changed … is worth surfacing" | `at-or-above` |
| `sovereignty-landings-ceiling@1` | `sovereignty-guard-signal.py:162` `total >= _ESCALATE_AT` | `at-or-above` |
| `claude-md-drift-score-ceiling@1` | `claude-md-audit.py:574,578` `drift_score >= threshold` | `at-or-above` |
| `entry-stale-days-ceiling@1` | `memory-review.py:163` `age_days >= STALE_DAYS` | `at-or-above` |
| `memkit-report-tier-mb-ceiling@1` | `memkit-retention.py:109` `mb > SIZE_CAP_MB` | *(default `above`)* |

Two the review did not list are included because the kit plainly uses `>=`
(`claude-md-drift-score@1`, `entry-stale-days@1`); leaving them on the default would have been a
silent off-by-one in the other direction. Byte budgets keep the default: every one is a strict
`>` in the kit (`memory-review.py:133`, `memory-index-projector.py:125,151,152`,
`memkit-retention.py:109,119`), and `gospel-bytes@1`'s source is `size <= CLAUDE_MD_BUDGET`
(`placement-audit.py:665`), i.e. over is strictly above. `decompose-threshold@1` is
`len(items) > THRESHOLD` (`decompose.py:196`) and `trigger-overlap@1` is documented ">N
distinctive shared words → flag" — both correct on the default.

The change is load-bearing, not cosmetic: before it, the rebuilt binary read
`mempalace: 1 files within hard 1 ✅` — a **false pass**, since the kit calls one changed file
stale. It now reads `⚠ failed — 1 files has reached the hard watermark 1`.

**The two floor rows now declare their direction.** `agent-description-floor@1` and
`skill-description-floor@1` are *floors* (`MIN_DESCRIPTION_CHARS`: a value **below** the
watermark is the finding). This seat left them on the default and flagged the gap rather than
mis-declaring a direction that would invert their meaning; the coordinator has since added
`compare: below` to the vocabulary and to both rows, which closes it. Verified in the registry:
`agent-description-floor@1 → compare: below, hard: 80` and
`skill-description-floor@1 → compare: below, hard: 60`.

### A note on provenance that survived its script

`agent-audit.py` and `skill-audit.py` were **deleted from the tree during this pass** (station
three's retirement). Five rows cite line numbers inside them. Those citations stay: `provenance:`
records where a constant came from, and the registry is now the constant's home — which is the
whole point of the station. The comparison for `trigger-overlap@1` was read from the surviving
documented behaviour rather than re-derived from a file that no longer exists, and that is
stated here rather than presented as a fresh reading.

## 10. Fabricated report formats replaced by a golden

The `epr` stub used to print invented report lines (`  cleanup: pressure {}/120`, …). A format
change in `report.rs` would have left that test green. Removed, and replaced by
`GoldenReportCase` — four tests that:

- resolve the binary (`$EPR_BIN` → gate target → PATH) and **`skipTest` with the reason** when
  it is missing or predates `--measure`. Verified: `EPR_BIN=/bin/true` yields
  `OK (skipped=1)`, never a pass;
- build a temp root holding the repository's **own** `measures.yaml`/`policies.yaml` and a
  minimal manifest, seed the four fixture folds through the **real** `epr flow note --measure`,
  and assert the **real** `epr flow report --headline` carries `8.3`, `1`, `250`, `3` on the
  `memkit:`/`mempalace:`/`cleanup:`/`scope:` lines with no `skipped`;
- assert a bound with no fold reports `skipped`, never ` 0 `;
- assert re-seeding identical folds does not change the reading;
- assert the argv `_observation.emit()` builds is one the **real** verb accepts — the direct
  regression test for the `--on` refusal found earlier.

The argv-recording stub survives only where the assertion is *what the hooks send*; its
`flow report` answer is now deliberately opaque (`REPORT_TEXT`), so it can say whether the hook
chose the native path but can never assert a line format.

Building the golden surfaced a real contract: the native verb refuses a root with no HEAD
(*"a note is dated by the tree it was written against, never by wall clock"*), so the fixture
root is a git repo with one commit.

## 11. Recipe label and the stale purpose sentence

- The recipe entry keeps the key `default` and now also carries `name: default`, so a label
  reader can take the name from either the key or the field, while `policy-recipe:` stays the
  directory `Recipe::at_dir` resolves. The join is the entry's `dir:`. **Verified on the
  rebuilt binary: the headline now prints `recipe: default@bafkreie…imhq`** — declared name and
  reported label agree, and §6's concern is closed.
- The frontmatter `purpose:` no longer calls the LoC ceiling "observation tier"; it reads
  "measure class — never blocks", which is the field (`class: measure`) that actually carries
  that meaning now the row is `binding-local`.

```
python3 .claude/scripts/epr-meta-pin.py --write
[epr-meta-pin] wrote 19 contentHash pin(s) to .claude/epr-meta/policies.yaml.
[epr-meta-pin] wrote 15 contentHash pin(s) to .claude/epr-meta/concerns.yaml.
EXIT=0

epr canon-lift --write
canon-lift: 34 atoms · 27 declared heads · 34 standing records
EXIT=0
```

Both idempotent on a second run; `load_policies()` returns `[]`.

## 12. Verification on the rebuilt binary

```
python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
Ran 23 tests ... OK                                                          EXIT=0
  (19 hook/bridge tests + 4 golden tests actually EXERCISED, not skipped)

SessionStart headline, end to end:
  memkit: ⚠ 8.3 megabytes is past the soft watermark 8
  mempalace: ⚠ failed — 1 files has reached the hard watermark 1
  cleanup: ⚠ failed — 250 pressure-points has reached the hard watermark 120
  scope: ⚠ failed — 3 count has reached the hard watermark 1
  memory-budget: ⚠ 23772 bytes is past the soft watermark 20000 (hard 24000)
  recipe: default@bafkreie…imhq
                                                                             EXIT=0

python3 .claude/scripts/memory-kit/placement-audit.py --headline               EXIT=0
python3 -m unittest discover -s .claude/scripts/_lib/__tests__ -p '*_test.py'  EXIT=1
  Ran 14 ... FAILED (errors=3) — the same three pre-existing module-level sys.exit
  loader errors; run directly, only the pre-existing seam_matrix_test.py fails
memory-ceremony gate leg 1 (memory_balance_test)                               EXIT=0
memory-ceremony gate leg 3 (recall*_test, 60 cases)                            EXIT=0
```

**Concern 1 is closed live.** The bridge folded the kit's values, the native report evaluated
them against the declared bounds with the declared comparison, and SessionStart now carries
`cleanup: … 250 … 120` and `scope: … 3 …` with no `(fallback: memory-kit)` line and no
`skipped`.

## Remaining, all outside this seat

1. **The remediation pointers the kit printed are gone.** The kit's lines ended with
   `→ /memory-stasis-loop` and `→ scope-reconcile.py --apply`, and named the capabilities
   (`3 to hold (local-conductor,owned-substrate)`). The native line carries the ⚠ and the number
   but not the next action. The trigger is legible; the *pointer* is not. Station two owns
   `epr flow report scope`, which is where that belongs.
3. Sovereignty escalation reads per-edit net-new on the native path; aggregation belongs to the
   report's fold.
4. `seam_matrix_test.py` remains red for its own unrelated reason.

---

# Round two — the fold was starving its own producer

Round-two review found one Important, and it was mine. Fixed, with a regression test that fails
against the reintroduced bug.

## 13. The bug: an early return that broke the chain it fed

The first pass had each rewired hook `return 0` after a successful `emit()`, on the premise that
a fold supersedes the JSON accumulator. It does not — not yet, and the premise was wrong in a
way that was self-defeating:

```
hook writes .claude/memory-kit/*.json
  -> cleanup-pressure.py:53-56 scores pressure as the CARDINALITY of those collections
     (ACTIVITY_FILES × COLLECTIONS = due | files | entries | items)
  -> the SessionStart bridge folds that score onto cleanup-pressure@1
  -> epr flow report evaluates it against cleanup-pressure-ceiling@1
```

Skipping the JSON write cut the first link, so the bridged value was frozen at whatever the kit
last counted — `cleanup-pressure@1` pinned at 250 and unable to rise — while
`placement-drift-due@1` read `0` natively against the kit's 8 past-due. The fold was starving
the producer whose number it reports.

**Fix: the hooks write BOTH.** All six emit-then-write unconditionally; the early returns are
gone and the `if landed` gating with them. This is not a fallback arrangement and the code no
longer calls it one — the kit remains the *producer* of the accumulated counts until the native
report derives them from folds (queued on the native seat after station two), and station six is
what deletes the JSON.

| hook | line | change |
|---|---|---|
| `placement-drift-signal.py` | 181-190 | `_obs.emit(...)` then always `locked_update` |
| `map-drift-signal.py` | 157-169 | same |
| `claude-md-drift-signal.py` | 181-195 | per-scope emit loop, no `landed` gate |
| `claude-md-structural-signal.py` | 162-186 | same |
| `memory-coherence-signal.py` | 134-148 | same |
| `sovereignty-guard-signal.py` | 130-158 | emit, then the tally unconditionally (its own escalation message reads that tally) |

## 14. The premise removed from the docs

`_observation.py`'s module docstring asserted the superseded-JSON premise; it now states the
producer relationship, names `cleanup-pressure.py:53-56` as the reason, and says plainly that an
emitter here is additive. Each hook's `Storage:` block changed from
"`<fold>` … Fallback while that verb is absent" to "BOTH — a fold … AND `<json>`", naming which
collection `cleanup-pressure.py` counts in that file. `sovereignty-guard-signal.py`'s inline
comment claiming "the native path stops keeping a private one" is corrected too.

## 15. Tests that fail against the bug

- `test_verb_present_appends_observation_AND_writes_the_json` — the inverted assertion. The old
  test asserted the JSON was **absent** on the native path, which is precisely how the bug
  passed review the first time.
- `test_pressure_rises_across_two_hook_runs_with_the_verb_present` — the end of the chain. Two
  distinct docs go past-due through the native path against a fixture copy of
  `cleanup-pressure.py` (its `_root()` walks up for a `.claude/memory-kit` dir, so the copy
  scores the fixture, not the repository); the score must be strictly higher after each run.
  Skips if the script is gone, so it retires cleanly at station six.
- The four per-hook native-emission tests flipped from `assertFalse(...exists())` to
  `assertTrue(...is_file())` with the reason on the assertion.

**Proven, not assumed:** re-introducing `return 0` after the emit in
`placement-drift-signal.py` makes both new tests fail
(`AssertionError: False is not true : the fold is ADDITIVE — the kit still produces the
accumulated count`); removing it makes all 24 pass.

## 16. Verification

```
python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
Ran 24 tests ... OK                                                          EXIT=0

SessionStart headline, end to end:
  memkit: ⚠ 8.3 megabytes is past the soft watermark 8
  mempalace: ⚠ failed — 3 files has reached the hard watermark 1
  cleanup: ⚠ failed — 250 pressure-points has reached the hard watermark 120
  scope: ⚠ failed — 3 count has reached the hard watermark 1
  memory-budget: ⚠ 23772 bytes is past the soft watermark 20000 (hard 24000)
  recipe: default@bafkreib…ptke
                                                                             EXIT=0

_observation.py import check: bin=/tmp/eprfs-gate-target/debug/epr measure_verb=True   EXIT=0
python3 .claude/scripts/memory-kit/placement-audit.py --headline                       EXIT=0
memory-ceremony gate leg 1 / leg 3                                             EXIT=0 / EXIT=0
_lib run directly: only the pre-existing seam_matrix_test.py fails
```

Producer and fold agree on live data — `cleanup-pressure.py --status` reads
`pressure 250/120` and the headline's bound reads 250; `mempalace-currency.py --status` reads
3 changed files and the headline reads 3 (it read 1 before this fix, because the accumulator
had stopped moving). All four accumulator files carry recent mtimes again.

## The producer tension, stated plainly

Until the native report derives these counts from folds, **the kit is load-bearing inside the
native headline.** The chain is hook → kit JSON → `cleanup-pressure.py` / `mempalace-currency.py`
/ `scope-reconcile.py` → bridge → fold → `epr flow report`. Three consequences worth holding:

1. **Deleting a kit script before its count is derived natively silently zeroes a bound** — it
   does not error, it reports `skipped`, or worse reports a stale frozen number if a fold was
   ever written. Station six's deletion order must follow the derivation, not the calendar.
2. **The fold is a projection of a projection right now.** `cleanup-pressure@1` is not measured
   from the flow plane; it is the kit's score, folded. It becomes a genuine derivation only when
   the native report counts distinct subjects from the observation folds the hooks already
   append — which those folds are already shaped for (subject = the drifted path, one per
   landing), so the input exists and nothing here needs re-authoring.
3. **`agent-audit.py` and `skill-audit.py` are already gone** (station three) while five rows
   still cite their line numbers. That is fine for `provenance:` — but it is the same pattern
   one step earlier, and it is why (1) matters.

## Standing, all outside this seat

1. The kit's remediation pointers (`→ /memory-stasis-loop`, `→ scope-reconcile.py --apply`, and
   the named capabilities) are still absent from the native lines — station two's
   `epr flow report scope`.
2. Native derivation of the accumulated counts from folds, which retires the bridge and the
   producer tension above.
3. Sovereignty escalation reads the kit's tally, which is now correct precisely because the
   hook writes both; it moves to the report's fold with (2).
4. `seam_matrix_test.py` remains red for its own unrelated reason.

---

# Round three — the derived world

The native seat's derive change landed (`epr` `sha256:f4b23255c9b5e36d…`, identical on PATH and
at the gate target). `cleanup-pressure-ceiling@1` now declares
`derive: distinct-subjects-since-reset` over seven measures — `cleanup-pressure@1` plus the five
drift measures the signal hooks already fold, plus `memory-index-drift@1` — and the seat removed
`cleanup-pressure@1` from the bridge's list in `_observation.py`. Seven of my tests were still
written for the pre-derive world. Updated, and the derived value is now asserted where it can
only be asserted honestly: against the real binary.

## 17. Tests moved to the derived world

The bridge must no longer fold `cleanup-pressure@1` — the report computes it, and bridging the
kit's pre-computed total as well would count it twice (once derived, once as a reading of a
measure nothing else writes). Five bridge tests now expect **three** folds, not four
(`memkit-report-tier-mb@1`, `mempalace-surfaces-changed@1`, `scope-pending-moves@1`), with an
explicit `assertNotIn("cleanup-pressure@1", …)` so a re-add fails loudly rather than silently
double-counting. The gate-error fixture switched from the `cleanup:` line to `memkit:`, since
`cleanup:` is no longer parsed at all.

`GoldenReportCase` gained the derivation itself — real binary, real registry, real folds:

- `test_cleanup_is_derived_from_distinct_subjects_across_measures` — seeds **5 distinct
  subjects across two measures** (3 on `placement-drift-due@1`, 2 on `map-currency-drift@1`,
  disjoint paths) and asserts the `cleanup:` line reads `5 … since the beginning`. Before any
  fold it asserts `skipped` — a derived bound with nothing to count is skipped, never zero.
- `test_a_reset_observation_witnesses_zero` — appends `cleanup-pressure-reset@1` and asserts
  the line reads `0 … since the last reset`.
- `test_only_new_distinct_subjects_count_after_a_reset` — the property that makes the reset
  meaningful: re-running the identical seed after a reset leaves the count at 0 (byte-identical
  observations mint one CID and are not new landings), while one genuinely new landing moves it
  to 1. This was verified by hand against the binary before it was written as an assertion.
- `test_the_hooks_emitter_is_accepted_by_the_real_verb` — retargeted from `cleanup-pressure@1`
  to `placement-drift-due@1`, so it proves the hooks' argv reaches the **derived** bound.

The fixture root now creates and commits its subject files: a subject must be a real path in the
tree (`failed to read <root>/docs: No such file or directory`) on top of the existing
HEAD requirement.

**Kept, unchanged:** the test that the hooks write **both** the fold and the JSON, and the
pressure-rises test. Station six deletes the JSON; until then the kit remains the producer of
the accumulators, and `cleanup-pressure@1` remains a declared measure the ceiling still consumes
even though the bridge no longer feeds it.

## 18. `ready to expand` removed from the parser

`_observation.py`'s `_SCOPE_PARTS` listed `ready to expand`, which `scope-reconcile.py` never
prints. Its three real part spellings are `to return to plate` (`:434`), `to hold` (`:438`) and
`deployment flag(s)` (`:444`), plus `aligned ✅`. The phantom pattern is gone; `deployment flag`
stays, because dropping it would under-count a part the kit genuinely emits. A new assertion
pins the absence: `parse_headline("  scope: ⚠ 4 ready to expand (shem)")` must yield `[]`.

`CLAUDE.md:247` still carries the `scope: ⚠ N ready to expand (cap)` wording — a gospel surface,
so that edit belongs to **station six through the managed-surface tooling**, not here. Left
untouched deliberately. (The same line's remediation pointer is already stale in the other
direction: it names `scope-reconcile.py --apply`, while the native headline now prints
`→ epr flow hold --scope --apply`.)

## 19. Verification

```
sha256sum $(which epr)                       f4b23255c9b5e36d…  (identical at the gate target)

EPR_BIN=$(which epr) python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
Ran 27 tests ... OK                                                          EXIT=0
python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'   (default resolution)
Ran 27 tests ... OK                                                          EXIT=0

SessionStart import check:
  memkit: ⚠ 8.3 megabytes is past the soft watermark 8
  mempalace: ⚠ failed — 3 files has reached the hard watermark 1
  cleanup: 96 pressure-points since the beginning within hard 120 ✅
  scope: ⚠ 3 to hold (local-conductor,owned-substrate)  →  epr flow hold --scope --apply
  memory-budget: ⚠ 23772 bytes is past the soft watermark 20000 (hard 24000)
  recipe: default@bafkreia…67o4
                                                                             EXIT=0

_observation.py import check: bin resolved, measure_verb True,
  bridged = memkit-report-tier-mb@1 · mempalace-surfaces-changed@1 · scope-pending-moves@1
                                                                             EXIT=0
python3 .claude/scripts/memory-kit/placement-audit.py --headline               EXIT=0
memory-ceremony gate leg 1 / leg 3                                     EXIT=0 / EXIT=0
_lib run directly: only the pre-existing seam_matrix_test.py fails
```

**The remediation pointer is back.** The `scope:` line now reads
`⚠ 3 to hold (local-conductor,owned-substrate) → epr flow hold --scope --apply` — capabilities
named and next action given. That closes the standing concern from round two.

## New finding: the derived `cleanup:` currently disagrees with its producer

```
kit      cleanup: pressure 250/120   (⚠ cleanup due → /memory-stasis-loop)
derived  cleanup: 96 pressure-points since the beginning within hard 120 ✅
```

Same gate, opposite verdicts — the kit says over, the native line says under. This is not a bug
in either: they measure different things right now.

- The kit counts the **current cardinality** of the accumulator collections, which have been
  filling since the last cleanup cycle — months of history in a snapshot.
- The derivation counts **distinct subjects in the fold history since the last reset**, and the
  fold history only began when these hooks started folding, earlier today. There has never been
  a `cleanup-pressure-reset@1`, so it reads "since the beginning" — but its beginning is hours
  old, not months.

So the native `cleanup:` gate — one of the two triggers root `CLAUDE.md` declares — **currently
under-reports**, and will keep under-reporting until fold history catches up with the kit's
accumulated window. Two ways out, both outside this seat: backfill the fold plane from the
existing accumulators once, or stamp a `cleanup-pressure-reset@1` at the next real cleanup so
both sides start counting from the same mark. Whichever is chosen should land before the kit's
`cleanup-pressure.py` is deleted, or the gate goes quiet at exactly the moment it stops having a
second opinion.

## Standing, all outside this seat

Refreshed after station one round three (native seat, 2026-09-10). Items that closed are listed at
the bottom rather than restated as open.

1. **The derived/producer divergence** — the native `cleanup:` gate reads 94 against the kit's 250.
   **The backfill has landed** (90 folds, 163 dead paths — coordinator correction on the native
   report), so the round-two recommendation to *backfill or stamp a reset before deleting
   `cleanup-pressure.py`* is **superseded and withdrawn**; do not act on it. What remains is a
   window difference, not a missing measurement: the kit's accumulators hold a snapshot filled
   since the last cleanup cycle, the derivation counts distinct subjects in fold history, and the
   two converge as folds land (4 → 11 → 94 across one day). Re-measure before station six deletes
   the producer; no further intervention is queued.
2. `CLAUDE.md:247`'s `ready to expand` wording and its `scope-reconcile.py --apply` pointer are
   both stale; station six owns them through the managed-surface tooling.
3. Sovereignty escalation still reads the kit's tally; it moves to the report's fold with the rest
   of the derived family.
4. `seam_matrix_test.py` remains red for its own unrelated reason.

**Closed since first raised** — do not re-open:

- **The floor rows carry `compare: below`** — `agent-description-floor@1` (hard 80) and
  `skill-description-floor@1` (hard 60), at `measures.yaml:883` and `:894` (the coordinator cited
  `:872`/`:883`; the eleven lines added to `cleanup-pressure-ceiling@1`'s `why:` in round four
  shifted them). Both now mean what they say.

- The monotone derived count (a re-opened doc stayed counted; a MAP refresh raised the count).
  Native seat addendum 5: a subject whose latest fold is 0 is retired, and the MAP-refresh path is
  routed to `map-currency-drift-reset@1` through `_observation.py`'s measure-routing table.
- The bridge/producer tension on `cleanup-pressure@1` — the bridge no longer folds it; the report
  derives it.
- The stale `/opt/rust/cargo/bin/epr` shadowing the rebuilt binary: the hooks suite runs green
  (27 tests) against the gate-target binary via `EPR_BIN`.

- **The kit's `changed`-key omission**, raised as round four's minor (1). Not closed — *decided*:
  `cleanup-pressure.py:56`'s `COLLECTIONS` tuple lists `files`/`entries`/`due`/`items` but not
  `changed`, the only collection `map-currency-drift.json` has, so the kit reads **0** from that
  file by omission while `cleanup-pressure-ceiling@1` consumes `map-currency-drift@1` and counts
  it (3 of today's 94). Coordinator decision: the kit's omission is a defect the replacement
  should not inherit, so the native row keeps map drift and is deliberately higher than the gate
  it replaces by exactly that count. Disclosed in one sentence on the row's own `why:`. No
  `supersedes:` and version stays 1: the registry's never-mutate rule binds at first landing, this
  row has never landed on `dev` and is amended in place by its authoring session with the
  contentHash re-pinned, and there is no earlier registry row to supersede — the constant it
  replaces is cited in `provenance:`, which is a citation, not lineage. (Note the direction: the
  defect makes the *kit* read low, so it does not explain the 94-vs-250 gap in item 1, which runs
  the other way.)

- **The Python half of the bulk-clear contract is now pinned.**
  `test_map_walk_refresh_stamps_the_reset_not_a_per_subject_zero` asserts that a `MAP.md` edit
  through `map-drift-signal.py` reaches the binary as `map-currency-drift-reset@1`, value `1`,
  subject `.`, with no per-subject zero and no `map-currency-drift@1` in the argv — and that the
  kit's own bulk clear (`store["changed"] == {}`, `last_map_refresh` stamped) still happens.
  `test_a_seed_change_still_folds_its_own_subject` pins the other arm. **Proven, not asserted:**
  emptying `_BULK_CLEAR_ON_ZERO` makes the first test fail
  (`'map-currency-drift@1' != 'map-currency-drift-reset@1'`), which is exactly the hole the review
  named. The routing stays in the emitter alone — a hook-level duplicate was written and then
  reverted, because two implementations of one rule is how they drift apart.
