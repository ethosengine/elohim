---
id: memory-kit-replacement-task-6-integration-report
status: DONE_WITH_CONCERNS
cites: []
gap: plans__2026-09-10-memory-kit-replacement-finish#7
actor: agent:implementer@claude-opus-5
session: mk-replace-integ-6a
commits: []
---

# Station six (integration seat, round (a)) — the ledger re-taken, the fold moved

The native seat's headline was *"the inventory is 17 rows stale, and that is the station's real
blocker."* This round repaired the ledger, cleared the references that were in its write set,
re-pointed the gospel at the native verbs, deleted the memory-balance set, and consolidated the
memory index under its cap. **Nothing under `.claude/scripts/memory-kit/` or `.claude/memory-kit/`
was deleted** — that is round (b).

## Headline

**The parity fold moved from 1 passed / 43 failed / 9 skipped to `21 passed · 22 failed · 10
skipped` of 53 rows.** Every one of the 32 non-passed rows names its blocker, and every blocker is
one of exactly three kinds: a script round (b) deletes (with its declaration, its JSON or its
docstring), a file outside this round's write set, or a relocated lens with no test. None is a
missing replacement that this round could have supplied.

```
$ epr flow report parity --inventory genesis/docs/superpowers/plans/memory-kit-replacement/parity-inventory-2026-09-10.md
parity  …/parity-inventory-2026-09-10.md@bafkreiftoln7gpd23sweatdn37tfu4qhz3vwvknaozdhgchnjjyzt4bfna  (53 rows: 21 passed · 22 failed · 10 skipped)
scan    5130 files, 49673015 bytes; roots .claude genesis elohim/sdk justfile; extensions py js mjs json yaml sh
EXIT=0
```

Inventory bytes 55,233 (was 30,046). The method pin is the inventory's raw CID, so this reading and
the native seat's 2026-09-10 reading are legibly different measurements rather than invisibly
different ones.

## Gate evidence

Every exit status echoed on its own line. **No cargo was run and no berth was claimed** — the brief
scopes this seat to the non-cargo legs; the native seat owns `elohim/eprfs/**`.

```
$ EPR_BIN="$(which epr)" python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
Ran 38 tests — OK (skipped=1)
EXIT=0

$ python3 .epr-meta/elohim/lenses/memory/__tests__/memory_coherence_audit_test.py
10 assertions passed
EXIT=0

$ cd genesis/a2o && EPR_BIN="$(which epr)" pnpm exec cucumber-js --profile ceremony
5 scenarios (5 passed) · 22 steps (22 passed)
EXIT=0

$ cd genesis/a2o && EPR_BIN="$(which epr)" pnpm exec cucumber-js --profile collective-memory
4 scenarios (4 passed) · 17 steps (17 passed)
EXIT=0

$ python3 .claude/scripts/epr-meta-pin.py --verify
19 pinned row(s) verified clean (19 total)
15 pinned row(s) verified clean (15 total)
EXIT=0

$ node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify
89 packages: 74 package-first, 0 source-fidelity, 15 native
elohim-agent package checks passed: 1998 passed
EXIT=0

$ just codegen agents write
PASS: wrote package-derived projection fixtures / runtime projections
EXIT=0
```

Those four test legs plus the two cargo legs ARE `_gate-memory-ceremony`. The gate's first line —
`python3 -m unittest discover -s genesis/scripts/__tests__ -p memory_balance_test.py` — is gone,
deleted with the suite it ran (§5).

PATH binary pinned for every native run: `/opt/rust/cargo/bin/epr`, sha256
`66f9db3fbbfcebf98f6f1069437e6c01b07357b4771873e9a0012b9533d98000` — the digest the brief names.
(The hooks' own `resolve_bin` prefers `/tmp/eprfs-gate-target/debug/epr`, which during this round was
the native seat's concurrent rebuild, `bed5c3b0…`. Named rather than hidden: it is a newer build of
the same tree, and every number quoted above was taken with the PATH binary.)

**One pre-existing red, disclosed and untouched:** `.claude/scripts/_lib/__tests__/seam_matrix_test.py`
fails on *"every crate with a seam-registry.yaml is routed to some column"*. It concerns seam
registries, nothing this round edited, and `_lib/__tests__` is not a leg of `_gate-memory-ceremony`.
The other two `unittest discover` errors in that directory (`capacity_ratification_test`,
`ci_harvest_controller_restart_test`) are loader artefacts — both pass standalone, exit 0.

---

## 1. The inventory, re-taken

`parity-inventory-2026-09-10.md` now carries two sections. The 2026-09-10 tables are kept **verbatim**
under `## 2026-09-10 — the original take (kept verbatim, superseded)`, with one byte changed in each:
their first column header reads `artifact (2026-09-10 snapshot)` instead of `artifact`.

That single edit is the load-bearing one and it deserves its own sentence. `parse_tables` reads
*every* markdown table whose header carries a cell spelled exactly `artifact`. Leaving the old
tables live would have folded 106 rows and reported the superseded reading beside the current one —
the ledger contradicting itself in one payload. Neutralising the header keeps the record readable by
a human and invisible to the fold. It is stated in the file, above the tables, so the next reader
does not mistake it for damage.

The re-taken tables use six columns — `artifact · state · replacement · parity · tests · evidence`.
Three reading changes, each deliberate:

**The `artifact` cell names the FULL kit path.** The 2026-09-10 take used bare filenames, and that
became wrong the moment station four relocated six lenses: a `procedure:` naming
`.epr-meta/elohim/lenses/memory/memory-review.py` *contains* the substring `memory-review.py`, so a
bare token made a **successful relocation read as a surviving consumer**. Four of the six relocated
rows were failing for that reason alone. The full path asks the question the row actually means.

**`state` is measured, not declared** — `present`/`deleted`/`relocated` are `git status` facts.

**`parity` uses the plan's four station-six values** — `native · relocated · retired · pending`. The
2026-09-10 vocabulary (`partial`/`missing`) described *capability*, not *disposition*, which is
exactly how nineteen rows read as unfinished work when the work was done. `pending` now means one
thing only: **round (b) deletes this**.

Counts after the re-take: **13 retired · 6 relocated · 34 pending**; zero rows declare `partial` or
`missing`. The `cite-propagate.py` row's verb was corrected from `epr flow concerns --stamp` to
`epr flow cites stamp` — the native seat's own Finding-1 example.

**The evidence column is generated from the fold, not hand-written.** After the table settled I
re-ran `report parity --json` and wrote each row's `evidence` cell from that payload's own
`residue`, leg by leg. A hand-typed residue list is a second authority that drifts the first time a
line number moves; this one cannot disagree with the fold, because it *is* the fold.

### What each non-passed row blocks on

Nineteen distinct files hold the 32 non-passed rows' residue. Grouped by why they are still there:

| Blocker | Rows | Where |
|---|---|---|
| **The script is still on disk and its `procedure:` still names it truthfully** | 8 | `.claude/epr-meta/measures.yaml` (15 hits) — `claude-md-audit`, `cleanup-pressure`, `memkit-retention`, `mempalace-currency`, `placement-audit`, `story-coverage-audit`, `substrate-currency-audit` |
| **The hook still WRITES the JSON** | 6 | the five drift-signal hooks + `memory-coherence-signal.py`, plus 6 fixture strings in `drift_observation_test.py` |
| **Outside round (a)'s write set** | 6 | `.claude/settings.local.json` (3), `.claude/subject-routing.yaml` (2), `.claude/epr-meta/recipes.yaml` (2), `_lib/__tests__/subject_routing_test.py` (3), `.claude/shifts/…objective.json` (1) |
| **A relocated lens with no test** | 5 | `cleanup-scan`, `dedupe-memory-scan`, `memory-review`, `path-update-scan`, `path-update-apply` — leg 3 is clean, leg 2 is `skipped` |
| **No native replacement exists at all** | 5 | `claude-md-audit`, `spec-coherence-index`, `story-coverage-audit`, `substrate-currency-audit`, `delivery-status-distribution` — genuine coverage gaps, not wiring gaps |
| **Deliberately kept** | 1 | `.claude/hooks/memory-index-projection.py:65` (§3) |

The five relocated-lens rows are the one class I could not close and would have liked to:
`.epr-meta/elohim/lenses/**` is not in this round's write set, so I could not write the tests that
would let leg 2 pass. Each is `skipped`, never `failed` — the fold is right to distinguish them, and
the repair is one test per lens.

## 2. `measures.yaml` — eleven `procedure:` rows re-pointed

`provenance:` was left alone throughout: the registry declares it a **citation** of the constant a
row replaces, and the fold honours that (`CITATION_KEYS`). `procedure:` names *the producer that must
exist for the measure to be takeable*, so a `procedure:` naming a deleted script is a dangling
method pin. Eleven were re-pointed, each because the named producer is **actually** the new one:

| Row | Was | Now |
|---|---|---|
| `package-description-chars@1` | `agent-audit.py` + `skill-audit.py` | `package-quality.mjs` (run by `package-projections.mjs verify`) |
| `trigger-overlap@1` | same pair | same verifier |
| `dynamic-stopword-fraction@1` | same pair | same verifier |
| `surface-stale-mtime-days@1` | same pair | **`none`** — staleness was deliberately not ported; the row is declared and unproduced **on purpose** |
| `rationale-window-lines@1` | `agent-audit.py` + `claude-md-audit.py` | the `claude-md-audit.py` half only, with the dead half named |
| `memory-index-bytes@1` | `memory-index-projector.py` | `epr flow memory project --index --budget memory-index-bytes@1` |
| `memory-index-title-chars@1` | same | `epr flow memory project --index` (`entries.rs TITLE_MAX`) |
| `memory-index-desc-chars@1` | same | `epr flow memory project --index` (`entries.rs DESC_MAX`) |
| `memory-index-drift@1` | same | `epr flow memory project --index --json` (`unloadedRows`) |
| `decompose-threshold@1` | `decompose.py` | `epr flow project` |
| `scope-pending-moves@1` | `scope-reconcile.py --report` | `epr flow report scope` |

**The fourteen I did NOT re-point are the finding, not the omission.** `memkit-report-tier-mb@1`,
the two retention-days rows, `cleanup-pressure@1`, `cleanup-cycle-history@1`, `gospel-bytes@1`,
`gospel-lines@1`, `stasis-margin@1`, `story-vision-weight@1`, `mine-grace-seconds@1`,
`mempalace-surfaces-changed@1` and the three `substrate-currency-audit` window constants still name a
kit script **because the kit script is still the producer.** Re-pointing them at a native verb would
have cleared eight parity rows by writing something false into the registry. Round (b) must re-point
each of these *in the same act that deletes its script*, or the measure silently loses its producer
— which is the exact failure class the native seat named as "a dangling method pin".

## 3. References re-pointed, and the three fallbacks that stay

### Switched to the native owner

| File | Was | Now |
|---|---|---|
| `.claude/hooks/load-project-context.py` | last-resort `placement-audit.py --headline` re-run | removed; `epr flow report --headline` is the owner |
| `.claude/hooks/delivery-gate.py` | same last-resort re-run | removed |
| `.claude/scripts/delivery-scoreboard.py` `doc_budget()` | `placement-audit.py --headline` | `epr flow report --headline` (binary: `$EPR_BIN`, gate target, PATH) |
| `.claude/scripts/_lib/managed_surfaces.py` | `decompose.py <file>` ×2 | `epr flow project` |
| same | `placement-audit.py --ledger` ×2 | `epr flow report placement --ledger` |
| `.claude/workflows/memory-stasis-loop.js` | `memory-index-projector.py --apply` | `epr flow memory project --index --budget … --out …` |
| same | `memory-index-drift.json` reads ×2 | `epr flow memory project --index --json` → `unloadedRows` |
| same | `gap-items/` write target | `.eprfs/status/gap-items/` |
| same | `memkit-retention.py --apply` | the bound, read via `epr flow report --headline` — because `--apply` **refuses every move** |
| `.claude/hooks/delivery-gate.py` docstring | `.claude/memory-kit/gap-items/` | `.eprfs/status/gap-items/` |

The `memkit-retention.py --apply` line is worth naming separately: the stasis workflow was
instructing an agent to run a command whose apply leg has been disabled since 2026-09-10. It was not
a stale *path*, it was a **stale instruction** — an agent following it would have done nothing and
reported having acted.

### The three fallbacks that stay, and why

The brief says remove a kit fallback **only** where the native path is proven live. Three are not.

**`_observation.bridge_headline` — a PRODUCER, not a fallback.** `epr flow report --headline` prints
five slots. `cleanup:` and `scope:` are natively derived (`derive: distinct-subjects-since-reset`,
and the cluster-state walk). `memkit-report-tier-mb@1` and `mempalace-surfaces-changed@1` have **no
native derivation at all** — the bridge is the only thing that folds those two values onto their
bounds, and without it the native headline reports `skipped` for two of the five slots. Removing it
would have been removing the only producer of a signal the root gospel declares as a session trigger.
Kept; its module docstring was corrected to say exactly this, because the old text claimed the kit
was still the producer of the *cleanup* count, which stopped being true when the derive landed.

**`.claude/hooks/memory-index-projection.py` KIT_PROJECTOR — kept for a different reason than the one
recorded.** The docstring said the native projection takes ~119s against a 10s hook budget. **Re-measured
2026-09-11: 1.0s.** The probe now routes native by default (`{"route": "native", …, "reason": "native
projection ready in 1.0s"}`), and latency is no longer a reason for anything. What *is* still a
reason is the **freshness guard**: the native index projects CONTRIBUTIONS, a memory entry written
seconds ago has none, and installing the native projection would delete that entry's own row from
the index. Until a fresh entry can be contributed inside the hook budget, the kit's directory scan
is the only leg that can render a just-written entry. The docstring now says that instead.

**The still-live JSON double-write in the five drift-signal hooks.** The native derive means the
`cleanup:` headline no longer needs the JSON — but `cleanup-pressure.py` is still on disk and still
reads it. Dropping the write now would leave a live reader without its file for the length of one
round. Round (b) removes script, JSON and write in one act; that is a smaller, safer diff than two
half-acts.

### Two hook tests rewritten with the fallback they pinned

`drift_observation_test.py::test_headline_falls_back_with_a_visible_marker` and
`::test_headline_cache_is_written_on_both_paths` asserted the branch I removed. They now assert the
*absence*: with the kit script deliberately installed in the fixture, a tree without the native verb
gets `""` and writes no cache. Installing the script and asserting it changes nothing is what makes
this a removal rather than a path that merely happens not to be taken today.

## 4. Gospel — three sections, through the package

Edited `.epr-meta/elohim/packages/agentdocs/elohim-root-gospel.json` and ran `just codegen agents
write`. `CLAUDE.md` and `AGENTS.md` were never touched directly.

- **§Bounded recall** — `recall-packet.py` replaced by the `epr flow memory recall open → search →
  source → read` ladder (every verb and flag exercised live before it was written down); the contract
  path corrected to `.epr-meta/elohim/algorithms/recall-contract.json`; and the **privacy line made
  explicit in gospel** — receipts and continuations are private session records under
  `.eprfs/status/recall/<session>/`, never imported, projected, witnessed or targeted by feedback.
- **§Memory cleanup trigger** — the old `cleanup: ⚠ … due` / `held ✅` spellings do not exist. The
  native line reads `cleanup: N pressure-points since the beginning within hard 120 ✅` under the
  watermark and `⚠ failed — … has reached the hard watermark 120` at or over it. The section now
  carries both spellings, says the value is DERIVED from folds, and names the drain:
  `epr flow note --kind observation --measure cleanup-pressure-reset@1 --subject . --value 1`.
- **§Substrate scope trigger** — `scope-reconcile.py --apply/--set/--env` → `epr flow hold --scope
  --apply/--set/--env`; `epr flow report scope [--docs]` named as the read; the bullet list cut to
  the three spellings the native mover actually prints (`aligned ✅  (plate matches substrate)`,
  `N to hold (cap)  →  epr flow hold --scope --apply`, `N to return to plate  →  …`). The
  `⚠ N ready to expand (cap)` bullet was **deleted because the native mover never prints it** — it
  folds that case into `to return to plate`; the meaning it carried was moved into that bullet
  rather than dropped. Resolver re-pointed to `flow/scope.rs`, budget to `epr flow report placement
  --ledger`. `just status habits` untouched.

**Diff verification.** `git diff CLAUDE.md` shows four hunks: `@@ -18` (Bounded recall), `@@ -222`
(both trigger sections), and two that are **not mine** — `@@ -32` (`just dev conductor alpha`) and
`@@ -301` (the conductor submodule pointer), both uncommitted work already in the tree when this seat
opened. `AGENTS.md` shows the same plus its own preamble hunk. No other section of either file moved.

> **One incident, disclosed in full.** Mid-edit I ran `git checkout` on the gospel package to undo a
> JSON re-encoding of my own, forgetting the file already carried *another* seat's uncommitted work.
> That discarded the Bounded-recall addition and two CI-prose corrections. They were recoverable and
> were recovered exactly: `plant-eprfs-agentdoc` makes the package body a **verbatim passthrough** of
> the projection, which I verified byte-for-byte at HEAD (`body == projection`, 40,993 chars each)
> before restoring the body from the still-intact working-tree projection. The four hunks above are
> the proof the restore was complete. The lesson is the one the projection-drift memory already
> carries, from the other direction: never `git checkout` a package that is dirty from someone else.

## 5. The memory-balance set, deleted

The native seat's closed six-edit set, applied:

1. `genesis/scripts/memory_balance.py` — deleted (untracked; `rm`)
2. `genesis/scripts/memory-balance.sh` — deleted (`git rm -f`; it had a working-tree modification)
3. `genesis/scripts/__tests__/memory_balance_test.py` — deleted with its directory
4. `justfile` `_gate-memory-ceremony` first line — removed, replaced by a comment naming the native
   method (`flow/memory/footprint.rs`, `bounded-memory-balance-v1`) and the test that now carries it
5. `genesis/build-manifest.json` — the three `memory-ceremony` `inputs.sources` rows removed
6. `elohim/eprfs/epr-cli/src/flow/memory/recall.rs:62` `BALANCE_LENS_REL` — **left**, per the native
   seat: its only use is a negative assertion, and `elohim/eprfs/**` is not this seat's write set

Every surviving mention of `memory_balance` is inside `elohim/eprfs/**` and is either that negative
assertion or a historical citation of the oracle the native method was transcribed from.

**Two consequences routed rather than left:** `.claude/memory-kit/balance-sheets/` now has **no live
writer** — its row says so, and round (b)'s relocation is an archival move of authored evidence, not
a redirect of a running producer. And the `memory-kit` skill package's step 9, which instructed a
ceremony to run the deleted shell wrapper, was re-pointed at `epr flow memory recall measure
--phase baseline|close`.

## 6. The memory index, consolidated

**23,993 → 22,412 bytes. 1,588 under the 24,000 hard cap** (the brief asked for ≥1,500; the
station-four review NOTE measured the headroom at **7 bytes**).

Five entries folded into **existing** umbrellas, each carrying its fact into the umbrella body
before its row left the index — no fact was deleted, only re-homed:

| Folded | Into | Why it is a near-duplicate |
|---|---|---|
| `project_host_psu_power_interrupts_2026_09` | `project_devspace_recovery` | the same failure shape one layer out — the umbrella already cited it as "not the same as a container recycle" |
| `feedback_pvc_deferral_hides_gate_debt` | `project_cargo_pvc_disk_discipline` | the umbrella already carries "act at 85% PVC" |
| `feedback_sccache_failure_classes` | `project_ci_build_infra` | CI substrate: images, caches, registries |
| `feedback_delegate_research_to_opus_sonnet_codex` | `feedback_agent_fleet_and_harness` | the umbrella already says "delegate narrow tasks to cheaper tiers" |
| `project_fresh_worktree_install_state_traps` | `feedback_push_branch_discipline` | the umbrella already carries the shared-worktree discipline |

Then six over-long descriptions shortened. A description is an index **pointer**, not the fact; each
trimmed specific was verified still present in its entry body before the trim landed (`8 members
per 60 s sweep`, `remote consumer`, `'struggle'`, `observe→canary`, `every subsequent cycle`,
`outran evidence` — all confirmed by grep in the body).

The native projection reads **contributions**, not the directory, so the folds only took effect
after `epr flow memory import .claude/memory` re-contributed the six changed entries (223 skipped,
6 contributed). Reprojected with the brief's exact command; `unloadedRows: 0`. One observation
appended so the headline stops reporting a number that is no longer true:

```
$ epr flow report --headline
  memory-budget: ⚠ 22412 bytes is past the soft watermark 20000 (hard 24000)
```

Still over the soft watermark, and it should be: 22,412 is real debt, just no longer a cap that
one more entry would breach.

---

## Concerns (why `DONE_WITH_CONCERNS`)

1. **The artifact is still not clearable, and this round was never going to make it so.** 32 of 53
   rows are non-passed. Every blocker is named, but 34 rows are `pending` by construction — they
   clear when round (b) deletes the scripts.
2. **Fourteen `measures.yaml` `procedure:` rows still name a live kit script.** Correct today,
   dangling the moment round (b) deletes them. **Round (b) must re-point each one in the same act
   as the deletion.** This is the single most likely thing to be forgotten, because the deletion
   will look complete without it.
3. **Two headline slots have no native producer.** `memkit-report-tier-mb@1` and
   `mempalace-surfaces-changed@1` are bridged from the kit. **Round (b) cannot delete
   `memkit-retention.py` or `mempalace-currency.py` without either porting those two values or
   accepting two `skipped` slots in the SessionStart headline.** This is a genuine ordering
   constraint on the deletion, not a wiring detail.
4. **Five relocated lenses have no test**, so five rows are `skipped` on leg 2 alone.
   `.epr-meta/elohim/lenses/**` was outside this round's write set.
5. **Two tracked historical records still gate two rows** (`.claude/shifts/…objective.json`,
   `governed-retrieval-acceptance.json`). I agree with the native seat: these are dated statements
   about a tree that existed, and editing them to pass a gate would be falsifying a record. The call
   is the operator's — a fifth brief-level exclusion, or an accepted permanent residue.
6. **`.claude/settings.local.json` holds three stale Bash allow-rules** naming kit scripts. Outside
   the write set; trivial for round (b).
7. **The stasis workflow's `AUDIT` const still runs `placement-audit.py`** because `--coverage` and
   `--stasis` have **no native rendering**. `--ledger`, `--focus` and `--headline` all moved. Round
   (b) must port those two or retire the two dimensions they feed.
8. **`seam_matrix_test.py` is red** — pre-existing, unrelated, disclosed above.
9. **No independent technical review.** The plan requires one for this station, plus a fresh-agent
   observation with context reset for the before/after reader comparison. Neither is self-certifiable
   and neither was done here.

## What this seat did NOT do

- Did not delete anything under `.claude/scripts/memory-kit/` or `.claude/memory-kit/`.
- Did not run the fresh-reader before/after question set (station six's other integration half).
- Did not touch `elohim/eprfs/**`, `.claude/settings.json`, `.claude/epr-meta/recipes.yaml`,
  `.claude/subject-routing.yaml`, `.claude/settings.local.json`, `.claude/scripts/converge/**`,
  `.claude/scripts/dev-dashboard/**` or `.claude/shifts/**`.
- Did not rewrite the `.md` prose surfaces (skills, agents, commands) that name still-live kit
  scripts — those tools exist, the instructions are accurate, and the sweep belongs with the
  deletion. The one exception was the `memory-kit` skill's step 9, which named a script this round
  deleted.
- Did not run cargo, did not claim a berth, did not commit, did not push.

---

# Station six (integration seat, round (b)) — the artifact is cleared

Round (a) repaired the ledger and left the deletion for this round. **`.claude/memory-kit/` and
`.claude/scripts/memory-kit/` no longer exist.**

## Headline

**The parity fold reads `52 passed · 1 failed · 0 skipped` of 53 rows** (round (a): 21/22/10;
native round (b): 22/21/10). Zero rows are `skipped` — every row now carries a replacement that
resolves, a test that exists, and a reference sweep that ran. The single failure is one dated
record, named in §7 and deliberately not edited.

```
$ epr flow report parity --inventory genesis/docs/superpowers/plans/memory-kit-replacement/parity-inventory-2026-09-10.md
parity  …@bafkreibxkuuau5rgri3ivclgoikpwjtajq76yaxa3jtsjyfwogoxt7uc7i  (53 rows: 52 passed · 1 failed · 0 skipped)
scan    5028 files, 45226705 bytes; roots .claude genesis elohim/sdk justfile; extensions py js mjs json yaml sh
EXIT=0
```

The scan itself is the second measurement worth reading: **5130 → 5028 files, 49.68 MB → 45.23 MB.**
102 fewer files and 4.45 MB less tree, in the roots the fold walks.

Native headline, all five slots, no fallback and no bridge:

```
$ epr flow report --headline
  memkit: retired — report tier removed 2026-09-11
  mempalace: ⚠ failed — 213 of 544 .md file(s) newer than .mempalace/.last-mine (+120s grace) — has reached the hard watermark 1
  cleanup: 98 pressure-points since the beginning within hard 120 ✅
  scope: ⚠ 3 to hold (local-conductor,owned-substrate)  →  epr flow hold --scope --apply
  memory-budget: ⚠ 22412 bytes is past the soft watermark 20000 (hard 24000)
EXIT=0
```

PATH binary for every number above: `/opt/rust/cargo/bin/epr`, sha256
`f9190e921c114776a4eee5a186b82aa2684c03561f2435531bf5f936e8998f59` — the digest the brief names.

## Gate evidence

Every exit status echoed on its own line. No cargo, no berth: the brief scopes this seat to the
non-cargo legs, and `elohim/eprfs/**` is the native seat's write set.

```
$ EPR_BIN="$(which epr)" python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
Ran 34 tests — OK (skipped=1)
HOOKS_EXIT=0

$ python3 .claude/scripts/epr-meta-pin.py --verify
19 pinned row(s) verified clean (19 total) · 15 pinned row(s) verified clean (15 total)
PIN_EXIT=0

$ node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify
88 packages: 73 package-first, 0 source-fidelity, 15 native — 1972 passed
PKG_EXIT=0

$ python3 .epr-meta/elohim/lenses/__tests__/lens_declaration_test.py
43 assertions passed
LENS_EXIT=0

$ python3 .epr-meta/elohim/lenses/memory/__tests__/memory_coherence_audit_test.py
10 assertions passed
COHERENCE_EXIT=0

$ cd genesis/a2o && EPR_BIN="$(which epr)" pnpm exec cucumber-js --profile ceremony
5 scenarios (5 passed) · 22 steps (22 passed)
CEREMONY_EXIT=0

$ cd genesis/a2o && EPR_BIN="$(which epr)" pnpm exec cucumber-js --profile collective-memory
4 scenarios (4 passed) · 17 steps (17 passed)
COLLECTIVE_EXIT=0

$ epr flow report --headline
HEADLINE_EXIT=0

$ epr flow report parity --inventory …/parity-inventory-2026-09-10.md
PARITY_EXIT=0

$ just codegen agents write
PASS: wrote package-derived projection fixtures / runtime projections
EXIT=0
```

The hooks suite was **RED when this round opened** — 7 failures, all in `drift_observation_test.py`,
all pinning the producer bridge the native seat emptied (`_HEADLINE_BRIDGE = ()`). That is disclosed
rather than glossed: the brief's verify-first step did not pass, and repairing it was this round's
own step (2) mandate. §5 says what those tests assert now. The count moved 38 → 34 because five
bridge tests and the kit-pressure chain test were deleted with their subjects and two were added.

`python3 -m unittest discover -s .claude/scripts/_lib/__tests__ -p '*_test.py'` → `LIB_EXIT=1`,
three loader errors. **Two are loader artefacts** (`capacity_ratification_test`,
`ci_harvest_controller_restart_test` — both exit 0 standalone; round (a) recorded the same pair).
**One is a genuine pre-existing red**, `seam_matrix_test`: *"every crate with a seam-registry.yaml is
routed to some column."* It concerns seam registries — `elohim/elohim-storage/seam-registry.yaml`
and `elohim/eprfs/epr-cli/seam-registry.yaml` are another seat's uncommitted work — and nothing this
round edited. A standalone sweep of all 15 `_lib` tests finds exactly that one red. **A FOURTH
loader error appeared mid-round and was MINE:** `managed_surfaces_test` went red when I re-pointed
the process-gospel match list, because the relocated `.epr-meta/elohim/lenses/CLAUDE.md` was not in
it. Fixed (§6); 51 assertions pass.

---

## 1. What was relocated

Authored and live evidence first, deletion second — the brief's order, and the right one: nothing
was removed before its contents had somewhere to be.

| From | To | Count | Why it moved rather than being deleted |
|---|---|---|---|
| `.claude/memory-kit/horizon-scans/` | `genesis/docs/analysis/horizon-scans/` | 3 | AUTHORED quarterly scans. Nothing regenerates a 2026-05-14 reading of an external landscape. |
| `.claude/memory-kit/balance-sheets/` | `.eprfs/status/balance/` | 24 | AUTHORED paired burden measurements. Each sheet is one half of a pair that only means something beside its twin. |
| `.claude/memory-kit/recall-executions/` | `.eprfs/status/recall/` | 1 (the 27 JSON receipts were already adopted) | The one non-JSON member, a trial-tree tarball cited by `collective-memory-integration/task-3-report.md:108`. |
| `.claude/memory-kit/context-coverage.yaml` | `.epr-meta/elohim/lenses/context-coverage.yaml` | 1 | The ONE hand-edited file in the report tier — a tuning surface, not a derivation. The native reader already looked here first (`stasis.rs:81`). |
| `.claude/scripts/memory-kit/{CLAUDE,LIFECYCLE}.md` | `.epr-meta/elohim/lenses/` | 2 | Authored process gospel. Rewritten in place (§3). |
| `.claude/scripts/memory-kit/.epr-meta` | `.epr-meta/elohim/lenses/.epr-meta` | 1 | The recall-algorithm compose-gate rule, with its `retire-when:` re-stated against the new directory. |
| 7 scripts with NO native replacement | `.epr-meta/elohim/lenses/{gospel,delivery,prior-art}/` | 7 | §2. |

**The `.gitignore` rung for `balance/`.** `.eprfs/status/*` is ignored; the ladder reopens only
`gap-items/` and `memory/`. A third rung was added beside them, with the reason in the file: a
balance sheet is paired measurement evidence, not a derivation. Verified with `git check-ignore`
(exit 1) and `git status` (24 files staged `A`). The terminal `/.eprfs/status/recall/` line stays
LAST, as its own comment requires.

### `recall-executions/` — the adoption, verified rather than assumed

The brief asked me to confirm station five's adoption before deleting. I compared every file byte
for byte rather than by name:

```
27 of 27 JSON continuations: byte-equal at .eprfs/status/recall/<session>/continuation.json
```

All 27 — `ceremony-corrected-acceptance-20260909` … `stale-edges-final-20260909` — are
`byte-equal`, not merely present. The native store holds three sessions the kit never had
(`mk-integ-6a-gospel`, `mk-parity-verify-6`, `mk-parity-verify-6b`), which are native-only sessions
created after the adoption. **The remaining non-JSON file was one:**
`collective-memory-observation-20260909.tar.gz`, a snapshot of an isolated observation trial tree.
It is a private record under the plan's privacy line (it contains a reader's continuation and
feedback), it is untracked, and `collective-memory-integration/task-3-report.md` cites it as
evidence. It moved into the same private terminal store rather than being deleted, and that report's
one pointer line was corrected. Directory removed.

### `gap-items/` — 249 files, and the one that was not yet adopted

The brief said the cache was already relocated (193 files) and the kit copy could be deleted. It was
193, and deleting on that basis would have dropped one live record. `epr flow project
--adopt-gap-items` reports:

```
scanned 249 · adopted 1 · alreadyPresent 193 · orphans 55 · unreadable 0
```

The one adoption is `plans__2026-09-10-doorway-federation-three-reds-to-green-plan.json` — a plan
authored AFTER station two's sweep. The 55 orphans are caches whose source document no longer
exists; the native verb reports them and refuses to adopt them, which is the right refusal. Store
is now 194. Kit copy deleted.

## 2. The seven lenses that had no native replacement

The brief's rule was "where a capability has NO native replacement, relocate rather than delete,
exactly as station four did for the memory analyzers." The fold's own `replacement` cell is the
test of that, and for all seven it opened `none` or `partial-with-a-named-gap`:

| Lens | New home | Declared as | Why not deleted |
|---|---|---|---|
| `claude-md-audit.py` | `lenses/gospel/` | `gospel-drift-findings@1` | Nothing native reads CLAUDE.md drift — `epr check`/`govern` gate WRITES, not doc drift. |
| `substrate-currency-audit.py` | `lenses/gospel/` | `substrate-currency-findings@1` | Nothing native scans gospel for PATH-EXISTS / process-status phrasing. |
| `locus-drift.py` | `lenses/gospel/` | `locus-drift-rollup@1` | `epr flow concerns` pages the cite edges; the per-locus roll-up is not ported. |
| `story-coverage-audit.py` | `lenses/delivery/` | `story-feature-coverage@1` | Nothing native projects story↔feature coverage. |
| `delivery-status-distribution.py` | `lenses/delivery/` | `delivery-status-gradient@1` | Nothing native projects the delivery-status gradient. |
| `spec-coherence-index.py` | `lenses/prior-art/` | `prior-art-index@1` | Nothing native indexes prior art. |
| `prep-brainstorm.py` | `lenses/prior-art/` | `brainstorm-preload@1` | `epr flow context` composes the flow plane, not the prior-art + scope preload. |

Each is a `family: *-lens` row with `procedure:` naming its real path, `unit:` naming its output
shape and `default-authority: observation` — the same shape station four used, so the report can
invoke them and fold their outputs. All seven were run live and their output verified in the new
home.

**Three of them needed more than a move, and those are the findings, not the mechanics:**

- **`spec-coherence-index.py` computed its repo root as `parents[3]`.** Correct at
  `.claude/scripts/memory-kit/`, off by one at `.epr-meta/elohim/lenses/prior-art/`. A `git mv`
  alone would have left it writing its index into `.epr-meta/` and reading a corpus rooted three
  levels too shallow — silently, because the walk just finds nothing. Fixed to `parents[4]` with
  the reason beside it.
- **`prep-brainstorm.py` ran two kit scripts as siblings** (`placement-audit.py --focus`,
  `focus-baseline.py --brief`, `placement-audit.py --ledger --json`). Station two ported all three
  readings, so the lens was rewired to `epr flow report placement --focus [--brief]` and
  `--ledger --json`, reading `pressure` off the native payload instead of re-deriving it from rows.
  The kit's inline comment said the pressure rule must exclude BLOCKED-BY-ENV; the native payload
  computes it with that same rule, and the comment now says so rather than restating the rule.
- **`locus-drift.py` skipped `/memory-kit/`** while walking the corpus. That filter became a
  no-op naming a deleted directory; it now skips the derived lens report tier, which is what the
  filter was actually for.

## 3. The dated report tier has ONE authority now

`.claude/memory-kit/<date>/` was resolved by **eleven** scripts, five of them lenses station four
had already relocated and which would have started writing into a deleted directory. The chokepoint
was `_lib.paths.reports_root`, so that is where the move landed:

```python
def reports_root(repo_root: Path) -> Path:      # .eprfs/status/lenses/
```

Everything under it is DERIVED — regenerable by re-running the lens that wrote it — so it is
untracked by the existing `.eprfs/status/*` rule and gets **no** re-include rung, deliberately
contrasting with `gap-items`/`memory`/`balance`, each of which holds something nothing can
regenerate. Seven scripts that had never imported `_lib` (`memory-review`, `path-update-scan`,
`path-update-apply`, `sprint-distill`, `plan-status`, `converge-apply`, `converge-scan`) gained the
five-line bootstrap rather than a hard-coded literal, because a second authority is how the tier
drifts the next time it moves.

`memory-coherence-audit.py` writes `cites-index.json` through the same helper, and
`memory-coherence-signal.py` now READS it through the same helper — reader and writer cannot drift.
The existing index was carried into the new home so the hook is not dormant until the next audit.

The relocated gospel was rewritten rather than moved verbatim: `.epr-meta/elohim/lenses/CLAUDE.md`
now leads with the native verb table, then the relocated-lens table, then keeps the old tool table
as a **`Tool (deleted) | Now`** crosswalk for readers arriving from a pre-2026-09-11 citation. The
hard-won gotchas, the pair-off boundary, the pollution-regulation section and the operating
principles are untouched — they were never about the scripts.

## 4. The hooks: one store, and the last accumulator

Six hooks wrote a fold AND a private JSON. Station one made that deliberate; the native derives
landed; round (b) removes the JSON. Per hook: the store-path function, the `_mutate` closure, the
`_store.locked_update` call, the now-unused `_lib.store` / `_lib.drift_score` imports, and the
`DEFAULT_THRESHOLD` / `RESCORE_EVERY_N_EDITS` constants that duplicated a DECLARED watermark.

**`sovereignty-guard-signal.py` was the one that could not simply drop its file**, and it is the
finding of this section. Its own PostToolUse message escalates at three landings, and `total` came
from the JSON tally. Dropping the write would have left `total = net_new` — the threshold could
never trip again, silently. Two things landed instead:

- `sovereignty-landings-ceiling@1` gained `derive: count-since-reset` and a
  `sovereignty-landings-reset@1` row, so the accumulation is a fold-plane derivation like the other
  five.
- `_observation.bound_count(bound_id, root=…)` — the inverse of `emit`: read a derived bound's
  `contributingFolds` back from `epr flow report --bound … --json`. Called ONLY on the rare branch
  where a landing actually fired, so the extra subprocess is paid per landing, never per edit. A
  miss returns `None` and the message reports this edit's landings while saying the total is
  unavailable, rather than printing a number nobody measured.

**One declared change of meaning, recorded on the row rather than hidden:** the kit SUMMED each
edit's `net_new` phrase count; `count-since-reset` counts one per landing EVENT. They differ only
when one edit lands two apex-sovereignty phrases at once. Counting events is the better reading for
a drift guard — three separate landings say more about corpus drift than one verbose paragraph —
and it is the derive the fold plane implements.

`_observation.py` lost `bridge_headline`, `parse_headline`, `_HEADLINE_BRIDGE`, `_scope_value` and
the fold memo (~90 lines): with every headline slot natively derived, bridging a value would DOUBLE
it. The two call sites in `load-project-context.py` and `delivery-gate.py` went with it.

**`memory-index-projection.py` is native-only, and the freshness guard survived as a REFUSAL.**
Round (a) kept the kit projector for one reason: the native index projects CONTRIBUTIONS, so
installing it right after an entry is written would delete that entry's own row. The guard now
declines to install instead of falling back. That is a preservation, not a loss, and the reason is
measured: `epr flow memory import .claude/memory` takes **7.9s** on this corpus against a 10s
PostToolUse budget with a 1.0s projection already spent, so contributing inside the hook is not
available; and the kit's directory scan only *looked* like a save — it wrote rows the contribution
plane does not carry, which the very next native run removed again. A row that appears and
disappears teaches nothing. The index is left untouched and the advisory names the import.

Probe route re-measured live: `{"route": "native", "reason": "native projection ready in 1.2s"}`.

## 5. The hook tests, inverted

The station-one contract was *"verb absent → the hook keeps writing its JSON."* That is now false by
design, so the tests assert its opposite:

- `test_verb_absent_writes_nothing_at_all` — no verb, no fold, **no fallback store**. The signal is
  lost for that edit, deliberately: a drift signal is worth a fold or it is worth nothing.
- `test_no_rewired_hook_writes_a_json_accumulator` — all six hooks × BOTH sides of the probe, with
  a helper that globs for *any* `*.json` anywhere under the fixture project rather than checking six
  names. Naming the six would pass a hook that invented a seventh, which is the regression that
  matters. It also asserts no hook recreates `.claude/memory-kit/`.
- `test_no_hook_path_runs_a_producer_bridge` — asserts the ABSENCE (`hasattr` is false, and neither
  consumer's source contains the call). An absence is exactly what gets silently undone.
- `test_bound_count_reads_the_accumulation_from_the_report` — including that an unreadable report
  returns `None`, never `0`: "could not measure" is not "measured zero."
- `GoldenReportCase` lost `BRIDGED_FOLDS`. `test_all_five_headline_slots_render_without_any_bridged_fold`
  appends NO observation and asserts five slots still render — that IS what retired the bridge.
  `test_a_retired_bound_says_retired_not_skipped` pins the distinction the native seat built.
  `test_a_bound_with_no_fold_is_skipped_never_zero` moved to `sovereignty-landings-ceiling@1`, a
  live bound this fixture has not observed, and asserts the payload carries no `observed` key at all.

## 6. Everything re-pointed

`measures.yaml`: the six `# STILL THE KIT:` rows the native seat annotated are re-pointed **in the
same act as their script's disposition**, which round (a)'s concern #2 named as the single most
likely thing to be forgotten. All six were RELOCATIONS, not retirements, so each `procedure:` names
the lens's real new path. Zero `# STILL THE KIT:` comments remain. `provenance:` was left alone
throughout — the registry declares it a citation of the constant a row replaces, and a citation of
a deleted file is honest history, not a dangling method pin.

| Surface | Change |
|---|---|
| `_lib/paths.py` | `reports_root` → `.eprfs/status/lenses/` (§3) |
| `_lib/managed_surfaces.py` | LIFECYCLE + the relocated gospel in the process-gospel match list; `claude-md-audit` / `spec-coherence-index` tool paths; the scope tool → `epr flow hold --scope --apply` |
| `_lib/epr_meta.py` | the brand-lint exemption → the lens report tier; `context-coverage.yaml` → its new home |
| `_lib/subject_routing.py`, `_lib/signal_measure.py` | docstrings that named the kit as the owner |
| `hooks/pickup-semantic-surfacing.py` | `mempalace-currency.py --status` → `epr flow report --bound mempalace-surfaces-changed-ceiling --json`, reading the outcome rather than a script's JSON |
| `hooks/deprecation-sentinel.py` | four Guard-Q comments quoting the deleted path; the guard's own path gate already covers `.epr-meta/` so its coverage survives the move |
| `scripts/delivery-scoreboard.py` | the lens path and the new output home |
| `scripts/converge/*`, `scripts/dev-dashboard/*` | the report tier + "generated by" lines that named the kit dir |
| `workflows/memory-stasis-loop.js` | `AUDIT` → `epr flow report placement`; a new `HEADLINE` const (the bounds headline is a SIBLING of the placement report, not a flag on it); the `memkit` dimension **removed entirely** from the schema, the sum, the log line, the dispatch table and both stasis predicates — a dimension that can never be true is noise |
| `subject-routing.yaml` | the focus-baseline sibling, the gospel_homes list, the script home, the reports home |
| `epr-meta/recipes.yaml` | two `gap-items` globs → `.eprfs/status/gap-items/` |
| `settings.local.json` | the three stale Bash allow-rules (round (a) concern #6) |
| `genesis/build-manifest.json` | the `memkit-retention.py` input → the two registry files the gate actually depends on |
| `justfile` | the `_gate-memory-ceremony` comment now names the INVERTED contract these tests pin |
| `.epr-meta/elohim/algorithms/recall-contract.json` | a source ROUTE and two `default_scope` rows still named kit paths — live contract values, not history. Re-pointed; pins re-verified clean; the ceremony profile re-run after the CID moved. |
| test fixtures | `subject_routing_test`, `brand_vocabulary_guard_test`, both `epr_meta_coverage` tests |

**`cluster_state_test.py` was a FOUR-PARSER AGREEMENT test** and three of the parties were deleted.
It imported `placement-audit.py`, `scope-reconcile.py` and `focus-baseline.py` and asserted each
read cluster-state the same way `_lib.cluster_state` does. `call_sites()` is now a shim over the
library, so **every trap stays pinned** — the column-0 comment, the block terminator, role-only
resources, duplicate keys in both orders, the decoy blocks, the real-file agreement, 65 assertions.
What is gone is named in the docstring rather than quietly dropped: four parsers agreeing was the
point when there were four; there is one Python parser now and the native one (`flow/scope.rs`) is
tested where it lives.

**`cleanup_pressure_rate_test.py` was deleted** — it tested `cleanup-pressure.py`'s scoring, and
the score is now `derive: distinct-subjects-since-reset`, asserted in `GoldenReportCase` against the
real binary.

**A new test earns the relocation: `.epr-meta/elohim/lenses/__tests__/lens_declaration_test.py`.**
A relocated lens is reachable two ways — by path, or through the `procedure:` on its registry row —
and a move breaks the second one *silently*, because a measure nobody can take reports `skipped`
rather than red. So it asserts the join in both directions (every declared path exists; every lens
on disk is declared), that every lens parses, that no lens resolves a path into the deleted kit
(code lines only — a `#` comment recording where the tier used to live is history worth keeping),
and that the kit tree stays deleted. 43 assertions.

## 7. The one row that did not pass, and why it was not forced

```
FAILED  .claude/scripts/memory-kit/cite-gen.py
  references failed :: 1 executable reference remains:
    .claude/shifts/2026-08-14T02-42-saga-leg2-drain-regressions-profiler-eyes.objective.json:25
```

That line is one entry in the `scope.paths` write-set of a shift that ran on 2026-08-14. It is a
dated statement about a tree that existed; nothing executes it after the shift closes, and editing
it to clear a gate would falsify a record. The native seat and round (a) reached the same
conclusion about this class, and I am not overturning it on my own authority.

**The structural fix is one line and it is the native seat's:** the fold already excludes
`genesis/data/timeline` and `genesis/a2o/reports` as dated-record roots. `.claude/shifts` is the
same class and belongs in that list. Until then the row is an accepted, named residue — the brief's
instruction was to name it rather than force it, and this is the naming.

## 8. Counts

**Deleted**

| | |
|---|---|
| `.claude/memory-kit/` | 174 files · 6.7 MB · 42 dated dirs · 15 state/lock JSONs · 9 tracked |
| `.claude/scripts/memory-kit/` | 10 scripts deleted + 7 relocated + 2 gospel docs relocated + 1 `.epr-meta` relocated + `__tests__` (2) + `__pycache__` |
| `gap-items/` (kit copy) | 249 files (194 adopted/present, 55 orphans reported by the native verb) |
| `recall-executions/` | 28 files (27 byte-verified adopted, 1 relocated) |
| `memory-kit` skill package | 1 package + 3 stored projections + 3 runtime skill dirs (`.claude`, `.codex`, `.agents`) |
| tests deleted with their subject | `cleanup_pressure_rate_test.py`; 5 bridge tests + 1 pressure-chain test inside `drift_observation_test.py` |

The ten scripts deleted: `cleanup-pressure`, `context-ratchet`, `decompose`, `focus-baseline`,
`memkit-retention`, `memory-index-projector`, `mempalace-currency`, `placement-audit`,
`scope-reconcile`, `state-machine-gen`.

**Moved** — 3 horizon scans · 24 balance sheets · 1 recall tarball · 1 tuning manifest ·
2 gospel docs · 1 compose-gate manifest · 7 lenses = **39 files**, all by `git mv` where tracked.

**Added** — 1 test (`lens_declaration_test.py`) · 8 registry rows (7 lens measures +
`sovereignty-landings-reset@1`) · 1 `.gitignore` rung · 1 helper (`_observation.bound_count`).

**Package count** 89 → 88; verifier checks 1998 → **1972 passed / 0 failed** (26 fewer: the retired
package and its three projections).

**Gospel** — `CLAUDE.md` / `AGENTS.md` changed by exactly one phrase this round: the `memkit`
discipline dropped from the memory-stasis-loop list, because the report tier it named is gone. Both
files were written through `.epr-meta/elohim/packages/agentdocs/elohim-root-gospel.json` and
`just codegen agents write`; neither was edited directly.

> **One incident, disclosed in full — and it is round (a)'s incident, repeated.** Mid-round I ran
> `git checkout` on the gospel package to undo a failed script's partial write, forgetting the file
> still carried round (a)'s UNCOMMITTED work (this sprint commits nothing). That reverted the body
> to HEAD's 40,993 chars and discarded round (a)'s Bounded-recall and trigger-section rewrites.
> Recovered exactly, by the same property round (a) used: `plant-eprfs-agentdoc` makes the package
> body a verbatim passthrough, and the 43,066-char projection was intact and byte-identical to the
> live `CLAUDE.md`. Restored from the projection, applied the one-phrase fold, reprojected, and
> verified `body == CLAUDE.md` at 43,057 chars. **Round (a) wrote down this exact lesson — "never
> `git checkout` a package that is dirty from someone else" — and I hit it anyway.** It is recorded
> twice now because a warning that only lives in a report is a warning that gets read after the
> fact; the durable fix is a guard, not another sentence.

## Concerns (why `DONE_WITH_CONCERNS`)

1. **One parity row is a permanent named residue** (§7). Clearing it needs a one-line `excluded:`
   addition on the native seat, or an operator ruling that dated shift records are exempt.
2. **`seam_matrix_test` is red** — pre-existing, unrelated, another seat's uncommitted seam
   registries, disclosed above and untouched.
3. **A drift signal with no binary is now LOST, not stored.** That is the deliberate trade of this
   round and it is worth stating as a cost, not just a design: an environment without `epr` on the
   PATH silently accumulates nothing. The fold plane is the only history.
4. **A newly-written memory entry's row does not reach `MEMORY.md` until someone runs
   `epr flow memory import`.** The guard preserves correctness (the entry's row is never deleted)
   but not immediacy. Closing it properly needs a sub-second single-file contribute verb — a native
   ask, named here rather than worked around.
5. **`sovereignty-landings-ceiling@1` counts events where the kit summed phrases** (§4). Declared
   on the row; a reader comparing across the cutover will see a slightly different number.
6. **The seven relocated lenses have one structural test, not seven behavioural ones.**
   `lens_declaration_test.py` pins the declaration↔file join and that each parses; it does not
   assert any lens's OUTPUT. All seven were run live this round and their output verified by hand,
   which is evidence in this report, not a gate.
7. **No independent technical review, and no fresh-reader before/after observation.** The plan
   requires both for this station. Neither is self-certifiable and neither was done here. The
   habit flip is explicitly gated on the fresh-reader comparison and remains ungated.

## What this seat did NOT do

- Did not run cargo, claim a berth, commit, or push.
- Did not touch `elohim/eprfs/**` or `.eprfs/status/flows.jsonl` beyond appending observations
  through the native verbs.
- Did not run the fresh-reader question set or arrange the independent review.
- Did not edit `.claude/shifts/**` (§7) or any `provenance:` citation.
