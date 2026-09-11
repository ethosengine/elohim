---
id: memory-kit-replacement-task-4-integration-report
title: Station four, integration seat — the corpus imported at repository reach, the analyzers relocated as declared measures
status: DONE_WITH_CONCERNS
class: devflow
gap: plans__2026-09-10-memory-kit-replacement-finish#5
actor: agent:implementer@claude-opus-5
commits: []
cites:
  - "memory-kit-replacement-finish | The plan this station drains | sha256:dbf92bf02195bfaf | path: genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md"
  - "parity-inventory-2026-09-10 | The inventory rows this station converts from missing to relocated | sha256:48aa6d31bffeef22 | path: genesis/docs/superpowers/plans/memory-kit-replacement/parity-inventory-2026-09-10.md"
  - "private-thought-governed-fruit | The constitutional privacy line the import gate enforces | sha256:ea19f49a6700ad04 | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - "memory-kit-replacement-task-4-native-report | The native half of this station — the two verbs, the frontmatter gate, and the six refusals this seat healed | sha256:eea1c2b63f78645c | path: genesis/docs/superpowers/plans/memory-kit-replacement/task-4-native-report.md"
---

# Station four — integration seat

The native seat's two verbs are now load-bearing on this tree. All 229 `.claude/memory/*.md`
entries are contributions in a **git-tracked** store at **repository reach**, the six memory
analyzers live under the memory owner as declared foreign measures, `cleanup-apply.py` is
retired, and the PostToolUse hook routes the index projection to `epr flow memory project
--index` with the kit projector as the fallback.

Status is `DONE_WITH_CONCERNS` for one measured reason, stated up front: **the native index
projection takes ~119 s on this corpus and the PostToolUse budget is 10 s.** The swap is wired,
proven byte-identical and self-healing, but today the router measures the native leg over budget
and takes the kit leg. That is recorded in §4 rather than hidden behind a hook that would time
out silently on every memory write. Two executable call sites outside this seat's write set
still name relocated scripts (§7); they are the other half of the concern.

## 1. (A) The contributions store is git-tracked

`flow/memory/import.rs:64` — `DEFAULT_CONTRIBUTIONS_DIR = ".eprfs/status/memory/contributions"`.
The `.gitignore` ladder gained a second exception over the parent of that path, shaped exactly
like the `gap-items/` one above it, with the reason written where the rule is:

```
!/.eprfs/status/gap-items/
# SECOND EXCEPTION: `.eprfs/status/memory/` is the collective-memory CONTRIBUTIONS STORE …
!/.eprfs/status/memory/
```

The exception is declared over `.eprfs/status/memory/` rather than the deeper
`…/contributions/` so that the tracked directory and the reach rule in §2 name the same
directory and cannot disagree about what is durable.

**`git check-ignore -v` on the three shapes:**

| Path | Verdict |
|---|---|
| `.eprfs/status/memory/contributions/MEMORY.md.json` | **not ignored** — no rule matches; the store is tracked |
| `.eprfs/status/flows.jsonl` | ignored by `.gitignore:138 /.eprfs/status/*` — the derived sidecar stays ignored |
| `elohim/eprfs/.eprfs/status/x.json` | ignored by `.gitignore:134 **/.eprfs/` — nested sidecars stay ignored |

`git status --porcelain -uall .eprfs/status/memory/` lists **229** untracked-and-not-ignored
contribution files. No commits in this sprint, so they stand as working-tree additions.

## 2. (B) The reach rule, and the reach it actually produced

`.epr-meta/collective.json` gained one rule, most-specific-prefix-wins over the native seat's
`.eprfs → workspace`:

```json
{"path": ".eprfs/status/memory", "maxReach": "repository"}
```

**The brief said `maxLocality:`; the declared schema key is `maxReach:`**
(`eprfs-agent/src/memory.rs:41` `max_reach`, serialized camelCase; `validation.rs:191
policy_reach` reads it). I used the key the reader parses — a `maxLocality:` line would have been
silently ignored and the store would have imported at `workspace` while the declaration claimed
otherwise.

`epr flow memory collective` accepts the declaration and echoes all seven rules (EXIT=0). The
effect on the import is the point:

| Run | source | request | **effective** |
|---|---|---|---|
| before the rule | repository | workspace | **workspace** |
| after the rule | repository | repository | **repository** |

`reach = min(policy(entry path), policy(request path))`, so the request leg was the binding
constraint and moving it is what lifted the effective reach. This closes the native report's
open item 2 — the contributions home did not need to move, its declaration and its tracking did.

## 3. (C) The six refused entries, and the live import

The native seat reported six refusals, all `missing memory frontmatter metadata.type:`. Reading
the live refusal reasons rather than the summary shows **two defect classes**, not one:

| Entry | Defect | Fix |
|---|---|---|
| `feedback_human_loop_not_terminal_authority.md` | column-zero `id:` closes the `metadata:` block over `type:` | `id:` moved above `metadata:` |
| `feedback_private_thought_governed_fruit.md` | same | same |
| `feedback_sccache_failure_classes.md` | same | same |
| `project_reach_enum_drift_reconciliation.md` | same | same |
| `feedback-identity-sovereignty-ontology-guard.md` | **no `type:` anywhere** — valid YAML, undeclared kind | `type: feedback` added as the first `metadata:` sub-key |
| `project_prod_main_lag_vs_alpha_dev.md` | **no `type:` anywhere** | `type: project` added |

The last two are not YAML repairs: those entries never declared a kind. The kinds are the ones
their filename prefix and every one of their 227 siblings already carry (`type: project` ×142,
`feedback` ×77, `reference` ×6, `user` ×2). Nothing else in any of the six changed — no body, no
description, no title. Four of the six diffs are a single line moving; two are a single line added.

**Projector-neutrality, checked before and after.** `.claude/scripts/_lib/frontmatter.py` matches
top-level keys only, so moving a column-zero `id:` and adding an indented `type:` cannot change a
rendered row. Verified rather than argued: `sha256(MEMORY.md)` is
`8ee2e07eac63f45594cf18b6ab5f996594c5c25b0e511fd20bba198a627a83dd` before the edits and after,
98 rows, 23,993 bytes, and `memory-index-projector.py --check` reports `fresh ✅` (EXIT=0).

**The four runs, in order:**

| Run | entries | contributed | refused | refusedIndexedToday | eventsAppended | skipped |
|---|---:|---:|---:|---:|---:|---:|
| dry-run, before the fixes | 229 | 223 | **6** | 3 | 0 | 0 |
| dry-run, after the fixes | 229 | 229 | **0** | 0 | 0 | 0 |
| **live apply** | 229 | **229** | 0 | 0 | **229** | 0 |
| live apply, second run | 229 | 0 | 0 | 0 | **0** | **229** |

`.eprfs/status/flows.jsonl` went 7,557 → 7,790 lines on the apply and stayed at 7,790 across the
second run; the contributions directory holds 229 files after both. Idempotence is by content, as
the native seat built it — the second run appended nothing and rewrote nothing.

The live apply was this seat's call, per the native report's §5 precondition: it ran only once
`refusedIndexedToday` was 0, so no row could be lost from the index unnoticed.

## 4. (D) The hook swap — wired, proven, and measured over budget

`.claude/settings.json:206` now runs `.claude/hooks/memory-index-projection.py --hook` (new,
this seat) in place of `.claude/scripts/memory-kit/memory-index-projector.py --hook`. The new
file is a **router**, not a second projector — it renders no row itself.

**Parity, against the real binary and the real corpus:**

```
epr flow memory project --index --budget memory-index-bytes@1 --out <scratch>
sha256  8ee2e07eac63f45594cf18b6ab5f996594c5c25b0e511fd20bba198a627a83dd   (native)
sha256  8ee2e07eac63f45594cf18b6ab5f996594c5c25b0e511fd20bba198a627a83dd   (.claude/memory/MEMORY.md, kit-projected)
cmp → identical                                                            EXIT=0
```

98 rows, 23,993 bytes, equal to the digest the native seat pinned. The budget resolves through
the declared `memory-index-bytes-ceiling@1` bound (soft 20,000 / hard 24,000, state `over-soft`)
rather than a constant, which is the whole point of the swap.

**Why the router exists, and why it takes the kit leg today.** Three things a bare command line
in `settings.json` cannot do:

1. **Probe before spending** — resolve the binary the way `_observation.py` does (`$EPR_BIN`, the
   gate target, `PATH`), try the verb once, cache the verdict on disk keyed by the binary's
   path+mtime+size. A rebuilt binary re-probes; a missing binary costs zero subprocesses.
2. **A latency budget, not just a capability probe.** Measured on this tree:

   ```
   time epr flow memory project --index --budget memory-index-bytes@1   →  real 1m59.162s
   time python3 .claude/scripts/memory-kit/memory-index-projector.py    →  real 0m00.061s
   ```

   The native leg re-reads all 229 contributions and asks the flow plane to confirm each one's
   attributed observation against a 7,790-line sidecar. The PostToolUse timeout is 10 s. So the
   probe's verdict is `native projection exceeded the 8s hook budget` and the route is
   `memory-kit` — recorded, cached, and re-evaluated on the next rebuild with no edit here. The
   manual probe prints exactly that:

   ```
   $ python3 .claude/hooks/memory-index-projection.py
   {"route": "memory-kit", "binary": "/tmp/eprfs-gate-target/debug/epr",
    "reason": "native projection exceeded the 8s hook budget"}
   ```

3. **A freshness guard.** The native index projects **contributions**, not a directory scan — by
   design. An entry written seconds ago has no contribution, and `epr flow memory import` is
   directory-at-a-time, so installing that projection would silently delete the just-written
   entry's own row. The router renders to a scratch path first and installs only if the row for
   the edited entry survived; otherwise the kit projection stands and the advisory names the
   import.

**The native leg was exercised end-to-end**, not just parity-checked: with
`MEMORY_INDEX_NATIVE=1 MEMORY_INDEX_BUDGET_SECONDS=240` the router took the native path, the
freshness guard passed, `MEMORY.md` was installed unchanged at `8ee2e07e…`, the scratch file was
removed, and the advisory came from the native JSON report's own budget block.

**`memory-index-projector.py` was NOT deleted, deliberately.** The brief gates its deletion on
the hook swap being verified — but the swap's own contract names it as the fallback, and a
fallback that does not exist is not a fallback. It is also the oracle the native parity test runs
against when present. Its test `__tests__/memory_index_projector_test.py` therefore stays where it
is, still guarding a live script. Station six deletes both with the directory.

## 5. (E) Relocations, retirement, and the declared rows

**Moved (`git mv`, content unchanged except their own path constants) to
`.epr-meta/elohim/lenses/memory/`:**

| File | Path-constant fix |
|---|---|
| `memory-review.py` | `REPO_ROOT` `parents[3]` → `parents[4]` (four levels below root now, was three) |
| `path-update-apply.py` | same |
| `memory-coherence-audit.py` | none — walks up for `.claude/` + `.git` |
| `dedupe-memory-scan.py` | none — same walk |
| `cleanup-scan.py` | none — same walk; its printed next step now names `epr flow hold` |
| `path-update-scan.py` | none — same walk |
| `__tests__/memory_coherence_audit_test.py` | its `_audit_path` and run line follow the script |

Each script's own self-naming strings (`Generated by …`, docstring usage lines, the scan→apply
hand-off in `path-update-scan.py:407`) were re-pointed to the new home; nothing else in their
bodies moved. Smoke-run from the new location: all six answer `--help` at EXIT=0,
`memory-review.py` resolves `REPO_ROOT = /projects/elohim` and produces its report, and the
relocated test passes (`10 assertions passed ✅`, EXIT=0).

**Deleted:** `.claude/scripts/memory-kit/cleanup-apply.py` (`git rm`). Archival relocation is not
net removal, per the design; an accepted cleanup proposal is a decision, and decisions route
through `epr flow hold`. That routing is now written in three places that used to name the mover:
`cleanup-scan.py`'s own proposal footer, the memory-kit skill's Phase-3 line and weekly-sweep step
8, and the librarian's tool table and apply-authority list.

**Declared** in `.claude/epr-meta/measures.yaml`, six rows, `family: memory-lens`,
`default-authority: observation`, `binding: binding-local`, `procedure:` naming the relocated
script, `unit:` naming the shape it returns, `provenance:` citing the inventory row:

| id | unit | inventory row |
|---|---|---|
| `memory-review-health@1` | `memory-review-report` | :39 `memory-review.py` (missing) |
| `memory-coherence-findings@1` | `memory-coherence-findings` | :37 `memory-coherence-audit.py` (partial) |
| `memory-dedupe-clusters@1` | `dedupe-clusters` | :32 `dedupe-memory-scan.py` (missing) |
| `memory-cleanup-proposals@1` | `cleanup-proposals` | :29 `cleanup-scan.py` (missing) |
| `path-update-proposals@1` | `path-update-proposals` | :42 `path-update-scan.py` (partial) |
| `path-update-applied@1` | `path-update-applied` | :41 `path-update-apply.py` (partial) |

Registry now: 48 measures, 34 lenses; the file parses and `epr-meta-pin.py --verify` reports
`19 pinned row(s) verified clean` + `15 pinned row(s) verified clean`, EXIT=0.

**One thing to flag about `binding:` on a measure row.** `flow/measures.rs:548` reads `binding:`
off a **bound** (a `lenses:` row or a `class: measure` ceiling), never off a measure, and carries
it through uninterpreted. So `binding: binding-local` here is declarative only: it states the rung
a consuming bound inherits, and gives the measure no teeth — which is the registry's own standing
contract that a measure is teeth-free. I wrote that reading into the section header rather than
letting a future reader discover it. **No ceiling lens is declared for the six**: a lens needs a
watermark, none of these six has a threshold that is not already its own row
(`entry-stale-days@1`, `rename-window-days@1`, `memory-index-bytes@1`), and a bound whose
procedure the report cannot yet invoke would report `skipped` forever.

**Three station-one `procedure:` paths corrected** (`memory-index-lines@1`, `entry-stale-days@1`,
`rename-window-days@1`) — they name where a procedure lives, and it moved. A deliberate step
beyond "declare new rows only", and narrow: `provenance:` on those rows was left untouched,
because provenance cites where the constant *was lifted from*, which is a historical fact.

**Packages, package-first, then projected** (`node …/package-projections.mjs project
--write-fixtures --write-runtime`, i.e. `just codegen agents write`):

- `skills/memory-kit.json` — 8 command paths re-pointed, the Phase-3 `cleanup-apply` line and
  weekly-sweep step 8 rewritten to `epr flow hold`, the PostToolUse projector paragraph rewritten
  to the router + native verb + declared bound + the contributions-not-directory caveat.
- `skills/memory-ceremony.json` — the reprojection step names the native verb and the router, and
  says a projected row comes from a contribution.
- `agents/librarian.json` — the tool table names the new home for five lenses, the
  `cleanup-{scan,apply}` row splits (apply retired), the apply-authority bullet routes to
  `epr flow hold`.

The runtime diff after projecting was inspected against a pre-projection copy: **the only changes
are the intended ones** — no runtime-newer-than-package content was clobbered (the standing
projection-drift trap). Verifier before: `32 failed, 1982 passed`, all 32 the stale projections of
exactly these three packages. After: **`1998 passed`, 0 failed, EXIT=0**.

**Gate wiring** (`justfile` `_gate-memory-ceremony`, `genesis/build-manifest.json` inputs): the
hooks suite and the relocated lens test now run as gate legs. Nothing ran `.claude/hooks/__tests__`
before — the same verification-that-never-runs shape pre-push names for
`.claude/scripts/_lib/__tests__`. Inputs added: `.claude/hooks/memory-index-projection.py`,
`.claude/hooks/_observation.py`, `.claude/hooks/__tests__/*_test.py`, `.claude/settings.json`,
`.epr-meta/elohim/lenses/memory/**`, `.claude/epr-meta/measures.yaml`.

## 6. (F) Tests and gate evidence

`.claude/hooks/__tests__/memory_index_projection_test.py` — 9 tests. `RouterCase` asserts the
route through the real module against a stub binary; `GoldenParityCase` asserts formats against
the **real** binary and the **real** corpus, or skips with the reason.

| Test | What it pins |
|---|---|
| `a_binary_without_the_verb_falls_back_to_the_kit` | the kit renders; the index is the kit's bytes |
| `the_native_projection_is_what_lands_when_the_verb_is_present` | native install; `--budget memory-index-bytes@1` on the argv |
| `the_probe_is_cached_so_a_second_write_costs_one_native_run` | one trial per binary, then installs only |
| `a_verb_over_the_latency_budget_routes_to_the_kit_and_says_why` | over-budget ⇒ kit, and the cached reason names the budget |
| `an_unimported_entry_keeps_the_kit_projection_and_names_the_import` | the freshness guard; advisory names `epr flow memory import` |
| `a_write_outside_the_memory_directory_is_a_no_op` | zero subprocesses |
| `a_hand_edit_of_the_index_itself_never_projects` | `MEMORY.md` writes never trigger a projection |
| `the_native_projection_equals_the_kit_projection` | **golden** — byte-identical on the live tree |
| `the_native_projection_matches_the_recorded_station_four_digest` | **golden** — sha256 `8ee2e07e…`; skips (never reds) if the corpus has moved past the snapshot, because parity above is the drift-proof half |

**Mutation check, run and reverted.** Short-circuiting the freshness guard reddened exactly
`an_unimported_entry_keeps_the_kit_projection_and_names_the_import` ("the freshness guard did not
fall back") and nothing else. Returning `True` from the timeout branch reddened exactly
`a_verb_over_the_latency_budget_routes_to_the_kit_and_says_why` ("True is not false") and nothing
else. Both probes removed; the file is byte-identical to its pre-mutation copy.

**Every command, with its own `EXIT=` line:**

```
python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
  Ran 38 tests in 124.259s — OK
EXIT=0

python3 .epr-meta/elohim/lenses/memory/__tests__/memory_coherence_audit_test.py
  10 assertions passed
EXIT=0

python3 .claude/scripts/epr-meta-pin.py --verify
  19 pinned row(s) verified clean (19 total). / 15 pinned row(s) verified clean (15 total).
EXIT=0

node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify
  89 packages: 74 package-first, 0 source-fidelity, 15 native — 1998 passed
EXIT=0

# `just gate memory-ceremony` legs that do not rebuild cargo:
python3 -m unittest discover -s genesis/scripts/__tests__ -p memory_balance_test.py
  Ran 9 tests — OK
EXIT=0

EPR_BIN=/tmp/eprfs-gate-target/debug/epr python3 -m unittest discover \
  -s .claude/scripts/memory-kit/__tests__ -p 'recall*_test.py'
  Ran 60 tests — OK
EXIT=0

cd genesis/a2o && EPR_BIN=… pnpm exec cucumber-js --profile ceremony
  5 scenarios (5 passed) · 22 steps (22 passed)
EXIT=0

cd genesis/a2o && EPR_BIN=… pnpm exec cucumber-js --profile collective-memory
  4 scenarios (4 passed) · 17 steps (17 passed)
EXIT=0

python3 .claude/scripts/memory-kit/memory-index-projector.py --check
  memory-index: 98 entries · 23993B (soft 20000 / hard 24000: over-soft ⚠) · 0 violation(s) · fresh ✅
EXIT=0
```

`cargo build --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli` (the gate's first leg) was
skipped per the brief; the existing `/tmp/eprfs-gate-target/debug/epr` was used for every leg that
needs a binary. No cargo was invoked, so no berth was claimed.

`epr flow context` was run on every file this seat wrote; the gate line it prints for the manifest
is `just gate memory-ceremony`, which is what the block above runs.

## 7. Concerns, and what the next seat owns

1. **The hook swap is routed to the kit today (§4).** The native projection is ~119 s against a
   10 s budget. Nothing is broken and nothing is hidden — the router measures, caches, reports the
   reason, and flips itself when the native side gets under budget. The native-side work is an
   `observation_recorded` scan per contribution over a 7,790-line sidecar: an index, or a
   projection that reads the sidecar once, would take it from O(n·m) to O(n+m). Native seat.
2. **Two executable call sites outside this seat's write set still name relocated scripts.** Both
   files are dirty from concurrent seats, so editing them here is exactly the collision the write
   set is fencing. The replacements are one substring each:
   - `.claude/scripts/_lib/managed_surfaces.py:78` —
     `"python3 .claude/scripts/memory-kit/memory-coherence-audit.py"` →
     `"python3 .epr-meta/elohim/lenses/memory/memory-coherence-audit.py"`
   - `.claude/workflows/memory-stasis-loop.js:79` — same substring, same replacement.

   Until then, the coherence audit does not run from the stasis loop or from the managed-surface
   command table. Every other reference in the corpus is prose or history (`genesis/docs/**`
   specs, the task-1 report, cite-writer fixtures, `governance-findings.jsonl`) and correctly
   describes the world as it was.
3. **`memory-index-projector.py` survives station four** (§4) and so does its test. Its deletion
   belongs with the fallback's, in station six.
4. **`refusedIndexedToday` is now a permanent 0 on this corpus**, so the three index rows the
   native seat warned about are safe — but the guard only holds while frontmatter stays
   well-formed. The two entries that had no `type:` at all suggest the entry template is not
   enforced at birth for every dialect; `.claude/memory/.epr-meta`'s `memory-frontmatter-at-birth`
   rule checks `name`/`title`/`description`, not `type`. Adding `type` to that rule would close
   the class. Not done here — `.claude/memory/.epr-meta` is outside this seat's write set.
5. **Independent technical review** — the plan requires a separately-appointed reviewer for this
   station, plus a fresh-agent observation with context reset. Not self-certifiable.
