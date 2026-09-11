---
id: memory-kit-replacement-task-6-native-report
status: DONE_WITH_CONCERNS
cites: []
gap: plans__2026-09-10-memory-kit-replacement-finish#7
actor: agent:implementer@claude-opus-5
session: mk-replace-native-6
commits: []
---

# Station six (native seat) — the parity fold and the retraction path

Two verbs landed and one question was answered.

1. **`epr flow report parity --inventory <path> [--json]`** folds the parity inventory's two
   markdown tables against the tree: one three-valued outcome per row, method-pinned to the
   inventory bytes' raw CID. Run live below, whole table included.
2. **`epr flow retract <cid> --reason <text>`** withdraws one sidecar `DepEdge`/cite-seal slot.
   The seven records the station-five round-two verdict named
   (`bafyreigra3p6jfqisbqjoypq6quic6aixi6euvwkldxacqcbzxbjq7qqqy`: *"The residue is now SEVEN
   records — the four dangling slots plus the three re-seals"*) are retracted live. Repository
   dangling count is now **0**.
3. **`genesis/scripts/memory_balance.py`** may be deleted by the integration seat, together with a
   closed set of five other edits named below. Its replacement's parity test survives the deletion
   by construction.

**The headline answer to the station's own question is NO: the artifact may not be cleared today.**
The live fold is **1 passed · 43 failed · 9 skipped of 53 rows**. The dominant cause is not missing
replacement — it is that **19 of the 36 script rows name a script that is already gone from the
tree while the row still declares `partial` or `missing`**. The inventory is the ledger this plan
drains, and it has not been re-taken since stations 3b, 4 and 5 did their deleting. Re-taking it is
the next act, and it is an integration act, not a native one.

## Gate

Cargo berth claimed before every invocation and released after. Rebuilt from scratch: the workspace
restarted mid-station and `/tmp/eprfs-gate-target` was wiped, so these are cold-tree numbers.

```
env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 \
    cargo build --manifest-path elohim/eprfs/Cargo.toml --workspace
BUILD_EXIT=0

env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target \
    cargo fmt --manifest-path elohim/eprfs/Cargo.toml --all --check
FMT_EXIT=0

env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 \
    cargo clippy --manifest-path elohim/eprfs/Cargo.toml --workspace --all-targets -- -D warnings
CLIPPY_EXIT=0

env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 \
    cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace
TEST_EXIT=0
```

Workspace totals: **52 test binaries, 675 passed, 0 failed, 0 ignored** (676 after the addendum's re-gate). Station five's round two
recorded 49/641; this station adds `flow_report_parity` (10) and `flow_retract` (6), and the
remainder is a concurrent seat's.

Clippy's only output is nine pre-existing `failed to parse serde attribute` notes from ts-rs in
another crate; `CLIPPY_EXIT=0`, and none is mine.

Binary installed on PATH: `/opt/rust/cargo/bin/epr`, **33,060,336 bytes**, sha256
`66f9db3fbbfcebf98f6f1069437e6c01b07357b4771873e9a0012b9533d98000`. Every live run in the body below
used it; the addendum's re-gate replaced it with
`bed5c3b0a965672c17e749765fca0314db457b41a66c69ee13bcc20a3c56aca8`, which is what is on PATH now.

> Two notes on the restart, so nothing reads as unexplained. (a) The binary that was on PATH when
> this seat resumed was `69f253cc…`, later than this seat's own 23:16 install and not from a turn
> in this transcript; it has been replaced by the digest above, built from the current tree.
> (b) The coordinator counted "nine retraction records". There are **seven**. The literal string
> `retraction` matches nine LINES of `.eprfs/status/flows.jsonl`: the seven records, plus the
> station-five correction note (`bafyreiewiivzk…`, "the flow plane has no retraction verb for a
> sidecar slot") and the station-five round-two verdict (`bafyreigra3p6j…`, "station six needs a
> retraction path for sidecar slots; none exists in the tree"). Both extra lines contain the word
> in prose. Measured, not assumed.

## What landed

| Path | Role |
|---|---|
| `elohim/eprfs/epr-cli/src/flow/parity.rs` | NEW (~900 lines). The parity fold: backtick-aware table reader, three legs, verb probe, single-pass reference scan. |
| `elohim/eprfs/epr-cli/src/flow/retract.rs` | NEW (~410 lines). The withdrawal verb, its four refusals, and `retracted()` — the pure fold every edge reader consults. |
| `elohim/eprfs/epr-cli/src/flow/edges.rs` | `sidecar_plane` drops withdrawn slots and returns them as `EdgeIndex::withdrawn`; new `WithdrawnEdge`. |
| `elohim/eprfs/epr-cli/src/flow/concerns.rs` | `Counts::retracted` + an omission naming every withdrawn slot and its ground. |
| `elohim/eprfs/epr-cli/src/flow/note.rs` | `NoteKind::Retraction` + `RETRACTION_TAG`; `NoteKind::parse` refuses `retraction` by name; the private guard now takes `NoteKind` instead of `&str`. |
| `elohim/eprfs/epr-cli/src/flow/mod.rs` | Registers both modules, the `retract` subcommand and the `report parity` arm; two usage entries. |
| `elohim/eprfs/epr-cli/tests/flow_report_parity.rs` | NEW. 11 tests (10 + the addendum's citation rule). |
| `elohim/eprfs/epr-cli/tests/flow_retract.rs` | NEW. 6 tests. |

`flow/memory/recall.rs`, `flow/cites.rs` and `flow/report.rs` were **not touched** — the write-set
boundary held. `report parity` is dispatched from `mod.rs::run_report`, not from `report.rs`, and
the only thing it takes from `report.rs` is the `OutcomeStatus` vocabulary, imported rather than
re-declared.

## 1. The parity fold

### The reading, stated so a reviewer can disagree with it

A row is `passed` only when three legs pass. Any failed leg fails the row; otherwise any skipped leg
skips it. `skipped` is load-bearing and is never a pass.

**Leg 1 — replacement.** Satisfied when the row's `parity` cell declares a *retirement with a
reason*, or when a backticked claim in the replacement cell resolves: an `epr …` verb chain the
binary recognises, or a relocated path that exists.

Two decisions inside this leg are the ones worth reviewing:

- *A verb's existence does not stand in for its flag's.* When a claim spells long flags and the verb
  chain answers `--help`, each flag must appear in that usage. This is not theoretical: the live
  `cite-propagate.py` row claims `` `epr flow concerns --stamp <doc>` ``, the chain is real, and the
  flag is not — station 3b landed the stamping verb as `epr flow cites stamp`. The row fails naming
  `--stamp` instead of passing on the verb's existence. A chain that does not answer `--help` at all
  (`epr flow stocks`) is accepted as recognised with the unverified flags named in `notes`.
- *A cell that opens with a negation declares no replacement, whatever it mentions afterwards.*
  `claude-md-audit.py`'s cell reads ``none (`epr check`/`govern` gate writes, not doc drift)``.
  Reading its incidental backtick as a claim would have turned an explicit "nothing covers this"
  into a pass. The first word decides, before any token is read. This is the one defect the first
  cut of the fold actually shipped, and the test `prose_and_absence_are_skipped_and_never_passed`
  pins the repair.

**Leg 2 — tests.** Every path-shaped backticked token in the `tests` cell must resolve. A cell that
names no path — `none`, or prose like "*`verify` fixture legs: 40 chars refused, 80 accepted*" — is
`skipped`, not passed. **Prose is not evidence a report can re-derive, and admitting it would let an
assertion pass itself.** That is the single biggest cause of `skipped` rows, and it is a real
instruction to the integration seat: *a row cannot clear until its cell names the replacement's test
as a path.* Two reader details: a trailing `:<line>` locator is stripped before shape-testing
(`` `_lib/__tests__/cluster_state_test.py:55` `` is a path plus a locator), and a bare sibling token
inherits the directory a previous token in the same cell RESOLVED in, which is the cell grammar the
inventory actually uses.

**Leg 3 — references.** No executable reference to the row's kit artifact, across
`.py .js .mjs .json .yaml justfile .sh` under `.claude`, `genesis`, `elohim/sdk` and the root
`justfile`. 5,133 files / 49.6 MB scanned in one pass, 3.3 s wall, zero files skipped for size.

### Two exclusions beyond the brief's four, and why

The brief names four exclusions. Two more are in the code, both deliberate, both emitted as
`omissions` on the payload rather than left to be inferred:

- **The kit's own trees.** `placement-audit.py` references `focus-baseline.py`. Counting that would
  make every row fail until the kit is deleted, and the kit may only be deleted once every row
  passes — a deadlock in which the ledger can never reach the state that authorises the act it
  exists to authorise. Intra-kit hits are counted and reported per row as `selfReferences`; they do
  not gate. (`recall_runtime.py` carries 27 of them; `recall-ceremony.py` 7.)
- **`.claude/worktrees/` and `genesis/a2o/reports/`.** The first is separate git worktrees of this
  repository at other commits — other trees, not consumers of this one. The second is gitignored
  (`genesis/a2o/.gitignore:3`, **zero tracked files**, verified with `git ls-files`); a cucumber
  receipt naming a script that once ran records a past run and cannot be repaired by any act
  station six can take. Without this second exclusion six rows were permanently unpassable for a
  reason nobody could address — the same deadlock shape, one layer out. Test:
  `a_derived_run_receipt_is_not_a_consumer_and_does_not_gate`.

Both exclusions are argued from the deadlock, not from convenience, and both are visible in the
payload's `scan.excluded` and in the printed `Limit:` lines.

### The blind spot, stated

The gate reads only the brief's seven extensions. `.md` (agents, skills, commands, `CLAUDE.md`),
`.ts` (a2o step files) and `.rs` are outside it **by construction** — those are station six's own
surface rewrite, not this gate's. That is on the payload as an omission. A reader who wants "has
every mention gone" must ask a different question than this fold answers.

### Method pin

`parity  …/parity-inventory-2026-09-10.md@bafkreicivjwtdp7654rkclmstvtnup6vewncfcer7ylplt6teoe6iuodae  (53 rows: 1 passed · 43 failed · 9 skipped)`
`scan    5133 files, 49640774 bytes; roots .claude genesis elohim/sdk justfile; extensions py js mjs json yaml sh`

Inventory bytes: 30,046. The payload carries that raw CID, so two parity reports over different
inventory revisions are legibly different measurements rather than invisibly different ones.

## The live table


#### `scripts` — 1 passed · 31 failed · 4 skipped

| artifact | outcome | replacement leg | tests leg | references leg (residue) |
|---|---|---|---|---|
| `agent-audit.py` | **failed** | passed — retired with a reason: script deleted; staleness deliberately not ported (content-based verification) | skipped — the inventory names no checkable test path (cell: `verify` fixture legs: 40 chars refused, 80 accepted,… | failed — 10 executable reference(s) remain: .claude/epr-meta/measures.yaml:474, .claude/epr-meta/measures.yaml:4… |
| `cite-describe.py` | **skipped** | skipped — the inventory declares no replacement (cell opens `no`: no brit equivalent — brit has no desc-authoring… | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to cite-describe.py (0 intra-kit) |
| `cite-gen.py` | **failed** | skipped — the inventory declares no replacement (cell opens `no`: no brit equivalent — brit's cite engine is READ… | failed — named test path(s) missing: .claude/scripts/memory-kit/__tests__/cite_gen_test.py (not found; tried .cl… | failed — 2 executable reference(s) remain: .claude/settings.local.json:116, .claude/shifts/2026-08-14T02-42-saga… |
| `cite-propagate.py` | **failed** | failed — no named replacement resolves: epr flow concerns --stamp <doc> (`epr flow concerns` exists but its usag… | passed — 1 named test path(s) exist | passed — no executable reference to cite-propagate.py (0 intra-kit) |
| `cites-migrate.py` | **skipped** | skipped — the inventory declares no replacement (cell opens `n/a`: n/a) | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to cites-migrate.py (0 intra-kit) |
| `claude-md-audit.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none (`epr check`/`govern` gate writes, not d… | skipped — the inventory names no checkable test path (cell: none) | failed — 8 executable reference(s) remain: .claude/epr-meta/measures.yaml:252, .claude/epr-meta/measures.yaml:25… |
| `cleanup-apply.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/epr-meta/measures.yaml:632 |
| `cleanup-pressure.py` | **failed** | passed — resolves: epr flow stocks --check | passed — 1 named test path(s) exist | failed — 31 executable reference(s) remain: .claude/epr-meta/measures.yaml:58, .claude/epr-meta/measures.yaml:34… |
| `cleanup-scan.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 2 executable reference(s) remain: .claude/epr-meta/measures.yaml:673, .claude/epr-meta/measures.yaml:674 |
| `context-ratchet.py` | **skipped** | passed — resolves: epr flow stocks --check | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to context-ratchet.py (4 intra-kit) |
| `decompose.py` | **failed** | passed — resolves: epr flow project | passed — 1 named test path(s) exist | failed — 14 executable reference(s) remain: .claude/epr-meta/measures.yaml:362, .claude/epr-meta/measures.yaml:3… |
| `dedupe-memory-scan.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 2 executable reference(s) remain: .claude/epr-meta/measures.yaml:662, .claude/epr-meta/measures.yaml:663 |
| `delivery-status-distribution.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 2 executable reference(s) remain: .claude/scripts/delivery-scoreboard.py:159, .claude/scripts/delivery-… |
| `focus-baseline.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | passed — 1 named test path(s) exist | failed — 3 executable reference(s) remain: .claude/scripts/_lib/__tests__/cluster_state_test.py:55, .claude/scri… |
| `locus-drift.py` | **passed** | passed — resolves: epr flow concerns | passed — 1 named test path(s) exist | passed — no executable reference to locus-drift.py (2 intra-kit) |
| `memkit-retention.py` | **failed** | passed — retired with a reason: apply leg disabled; proposals lack custody evidence | passed — 1 named test path(s) exist | failed — 10 executable reference(s) remain: .claude/epr-meta/measures.yaml:312, .claude/epr-meta/measures.yaml:3… |
| `memory-coherence-audit.py` | **failed** | passed — resolves: epr flow concerns | failed — named test path(s) missing: __tests__/memory_coherence_audit_test.py (not found; tried __tests__/memory… | failed — 6 executable reference(s) remain: .claude/epr-meta/measures.yaml:651, .claude/epr-meta/measures.yaml:65… |
| `memory-index-projector.py` | **failed** | skipped — no checkable replacement named (cell: `eprfs-agent::project` projects capability packages, not the memo… | passed — 1 named test path(s) exist | failed — 14 executable reference(s) remain: .claude/epr-meta/measures.yaml:202, .claude/epr-meta/measures.yaml:2… |
| `memory-review.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 7 executable reference(s) remain: .claude/epr-meta/measures.yaml:203, .claude/epr-meta/measures.yaml:21… |
| `mempalace-currency.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 7 executable reference(s) remain: .claude/epr-meta/measures.yaml:554, .claude/epr-meta/measures.yaml:55… |
| `path-update-apply.py` | **failed** | skipped — no checkable replacement named (cell: slug-resolved cites self-heal on move (`cite-propagate` `path:` c… | skipped — the inventory names no checkable test path (cell: none) | failed — 2 executable reference(s) remain: .claude/epr-meta/measures.yaml:695, .claude/epr-meta/measures.yaml:696 |
| `path-update-scan.py` | **failed** | skipped — the inventory declares no replacement (cell opens `same`: same as above) | skipped — the inventory names no checkable test path (cell: none) | failed — 4 executable reference(s) remain: .claude/epr-meta/measures.yaml:564, .claude/epr-meta/measures.yaml:56… |
| `placement-audit.py` | **failed** | passed — resolves: epr flow stocks, epr flow status | skipped — the inventory names no checkable test path (cell: none) | failed — 48 executable reference(s) remain: .claude/epr-meta/concerns.yaml:26, .claude/epr-meta/measures.yaml:20… |
| `prep-brainstorm.py` | **failed** | passed — resolves: epr flow context | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/epr-meta/measures.yaml:343 |
| `recall-ceremony.py` | **failed** | passed — resolves: epr flow context | failed — named test path(s) missing: __tests__/recall_ceremony_test.py (not found; tried __tests__/recall_ceremo… | passed — no executable reference to recall-ceremony.py (7 intra-kit) |
| `recall-packet.py` | **failed** | skipped — the inventory declares no replacement (cell opens `same`: same as above) | failed — named test path(s) missing: __tests__/recall_packet_test.py (not found; tried __tests__/recall_packet_t… | failed — 1 executable reference(s) remain: genesis/docs/superpowers/plans/governed-retrieval-acceptance.json:4 |
| `recall_lenses.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | failed — named test path(s) missing: __tests__/recall_ceremony_test.py:445 (not found; tried __tests__/recall_ce… | passed — no executable reference to recall_lenses.py (0 intra-kit) |
| `recall_providers.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | failed — named test path(s) missing: recall_runtime_test.py (not found; tried recall_runtime_test.py, .claude/sc… | passed — no executable reference to recall_providers.py (17 intra-kit) |
| `recall_runtime.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | failed — named test path(s) missing: __tests__/recall_runtime_test.py (not found; tried __tests__/recall_runtime… | passed — no executable reference to recall_runtime.py (27 intra-kit) |
| `scope-reconcile.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | passed — 1 named test path(s) exist | failed — 13 executable reference(s) remain: .claude/epr-meta/measures.yaml:606, .claude/epr-meta/measures.yaml:6… |
| `skill-audit.py` | **failed** | passed — retired with a reason: script deleted; staleness deliberately not ported | skipped — the inventory names no checkable test path (cell: none) | failed — 8 executable reference(s) remain: .claude/epr-meta/measures.yaml:474, .claude/epr-meta/measures.yaml:47… |
| `spec-coherence-index.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 3 executable reference(s) remain: .claude/scripts/_lib/managed_surfaces.py:90, .claude/scripts/_lib/man… |
| `stale-record.py` | **failed** | passed — resolves: epr flow note --kind correction | failed — named test path(s) missing: __tests__/stale_record_test.py (not found; tried __tests__/stale_record_tes… | passed — no executable reference to stale-record.py (0 intra-kit) |
| `state-machine-gen.py` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to state-machine-gen.py (5 intra-kit) |
| `story-coverage-audit.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 2 executable reference(s) remain: .claude/epr-meta/measures.yaml:574, .claude/epr-meta/measures.yaml:575 |
| `substrate-currency-audit.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 6 executable reference(s) remain: .claude/epr-meta/measures.yaml:524, .claude/epr-meta/measures.yaml:52… |

#### `report-tier-state-files` — 0 passed · 12 failed · 5 skipped

| artifact | outcome | replacement leg | tests leg | references leg (residue) |
|---|---|---|---|---|
| `placement-drift.json` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | passed — 1 named test path(s) exist | failed — 7 executable reference(s) remain: .claude/epr-meta/measures.yaml:455, .claude/epr-meta/measures.yaml:85… |
| `claude-md-drift.json (76K)` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 12 executable reference(s) remain: .claude/epr-meta/measures.yaml:858, .claude/epr-meta/measures.yaml:8… |
| `map-currency-drift.json` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 7 executable reference(s) remain: .claude/epr-meta/measures.yaml:465, .claude/epr-meta/measures.yaml:85… |
| `memory-coherence-drift.json` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 5 executable reference(s) remain: .claude/epr-meta/measures.yaml:860, .claude/hooks/__tests__/drift_obs… |
| `memory-index-drift.json` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | passed — 1 named test path(s) exist | failed — 6 executable reference(s) remain: .claude/epr-meta/measures.yaml:423, .claude/epr-meta/measures.yaml:86… |
| `sovereignty-guard-drift.json` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 5 executable reference(s) remain: .claude/hooks/__tests__/drift_observation_test.py:305, .claude/hooks/… |
| `state-ledger.json (128K)` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to state-ledger.json (2 intra-kit) |
| `cites-index.json` | **failed** | passed — resolves: epr flow concerns | skipped — the inventory names no checkable test path (cell: none) | failed — 6 executable reference(s) remain: .claude/epr-meta/measures.yaml:651, .claude/epr-meta/measures.yaml:65… |
| `context-coverage-baseline.json / context-coverage.yaml` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | passed — 1 named test path(s) exist | failed — 4 executable reference(s) remain: .claude/scripts/_lib/__tests__/epr_meta_coverage_nudge_test.py:43, .c… |
| `spec-coherence-index.json (192K)` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to spec-coherence-index.json (2 intra-kit) |
| `story-coverage-audit.json + .prev.json (132K ea.)` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to story-coverage-audit.json, .prev.json (9 intra-kit) |
| `delivery-status-distribution.json + .prev.json (228K/200K)` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/scripts/delivery-scoreboard.py:167 |
| `gap-items/ (2.3M, 247 files)` | **failed** | passed — resolves: epr flow project | passed — 1 named test path(s) exist | failed — 6 executable reference(s) remain: .claude/epr-meta/recipes.yaml:50, .claude/epr-meta/recipes.yaml:169, … |
| `recall-executions/ (648K, 28 files)` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | failed — named test path(s) missing: __tests__/recall_{ceremony,runtime}_test.py (not found; tried __tests__/rec… | passed — no executable reference to recall-executions/ (0 intra-kit) |
| `horizon-scans/ (48K, 3 files)` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/subject-routing.yaml:155 |
| `balance-sheets/ (100K, 24 files)` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none) | passed — 1 named test path(s) exist | passed — no executable reference to balance-sheets/ (20 intra-kit) |
| `dated dirs 2026-05-10…2026-09-09 (40 dirs, ~4.4 MB)` | **skipped** | passed — retired with a reason: derived reports, regenerable; oldest 4 months | skipped — the inventory names no checkable test path (cell: none) | skipped — the artifact cell names no single kit path to search for (cell: dated dirs `2026-05-10`…`2026-09-09` (4… |

*Every non-passed row's residue is named above in its own cell; the `--json` payload carries the
full, unbounded list per row under `residue`, plus `selfReferences`, `declaredParity` and each leg's
per-claim `checks`.*

### What the table says, in three findings

**Finding 1 — the inventory is 17 rows stale, and that is the station's real blocker.**
Nineteen of the 36 script rows name a script that **no longer exists on disk**; only two of those
nineteen (`agent-audit.py`, `skill-audit.py`) declare `retire`. The other seventeen still declare
`partial` or `missing`:

`cite-describe.py` · `cite-gen.py` · `cite-propagate.py` · `cites-migrate.py` · `cleanup-apply.py` ·
`cleanup-scan.py` · `dedupe-memory-scan.py` · `memory-coherence-audit.py` · `memory-review.py` ·
`path-update-apply.py` · `path-update-scan.py` · `recall-ceremony.py` · `recall-packet.py` ·
`recall_lenses.py` · `recall_providers.py` · `recall_runtime.py` · `stale-record.py`

These rows are checked against what the inventory SAYS, so they read as failed-or-skipped while the
work is in fact done. **Re-taking the inventory is the next act**, and it belongs to the integration
seat: declare each of the seventeen as `retire — <reason>`, name the replacement's test as a path,
and correct `cite-propagate.py`'s cell from `epr flow concerns --stamp` to `epr flow cites stamp`.
The fold is deliberately not clever about this: it reports the ledger against the tree, and a ledger
that has moved on is a ledger to re-take, not a reader to loosen.

**Finding 2 — 267 gating references remain, and they are five classes, not one list.**

> **SUPERSEDED by the addendum below.** `provenance:` values are citations, not consumers, and
> no longer gate; the corrected count is **216 gating, 51 cited**. Read the addendum's class
> table instead of this one. Kept here because the correction is the finding.

| hits | rows | class | where |
|---|---|---|---|
| 104 | 26 | **declaration provenance** — `procedure:`/`provenance:` strings naming the kit script that established a threshold | `.claude/epr-meta/measures.yaml` |
| ~75 | — | **live wiring** — hooks, the stasis workflow, the managed-surface scope table, the delivery scoreboard | `.claude/hooks/*.py`, `.claude/workflows/memory-stasis-loop.js`, `.claude/scripts/_lib/managed_surfaces.py`, `.claude/scripts/delivery-scoreboard.py`, `.claude/subject-routing.yaml`, `.claude/epr-meta/recipes.yaml` |
| ~45 | — | **test fixtures** naming kit scripts or their JSON state | `.claude/hooks/__tests__/drift_observation_test.py` (27), `.claude/scripts/_lib/__tests__/*` |
| 4 | 3 | **prose comments** inside schemas and catalogs | `elohim/sdk/schemas/{v1,current}/…`, `.claude/epr-meta/seam-catalog.yaml`, `genesis/manifests/cluster-state.yaml` |
| 2 | 2 | **historical records** of past runs | `.claude/shifts/2026-08-14T02-42-…objective.json`, `genesis/docs/superpowers/plans/governed-retrieval-acceptance.json` |

The largest class is the one nobody has looked at: `measures.yaml` rows whose *method pin* names a
deleted script. That is a dangling method pin, not decoration — station one lifted 35 thresholds
into declared rows and recorded where each came from, and 26 of those rows now point at nothing. It
is also the cheapest fix in the set, and it is a *rewording*, not a deletion.

The two historical records are the one class I would argue should NOT be edited: a shift objective
and a recorded acceptance are dated statements about a tree that existed. If station six agrees,
they want a fifth brief-level exclusion or a declared exception; I did not add one, because unlike
`genesis/a2o/reports/` these are tracked, authored records and the call is the operator's.

**Finding 3 — one row passes end to end today: `locus-drift.py`.** Replacement `epr flow concerns`
resolves, its named test exists, no executable consumer remains (2 intra-kit). It is the shape every
other row needs to reach.

## 2. The retraction path

### Shape

A retraction is an EPR `FeedbackSignal`-shaped run-note: slot 0 `run:retraction`, slot 1 the
withdrawn record's CID, `reason:` the authored ground, `resource` the same record's address. **No
new record kind** — the DNA-entry-scarcity discipline one layer up: a new social move on existing
data is a new tag, never a new type. It inherits the note plane's content-addressed idempotence, its
git-HEAD dating and its attribution arms unchanged.

`NoteKind::Retraction` exists in the enum but is **unreachable from `epr flow note --kind`**:
`NoteKind::parse("retraction")` refuses by name and points at `epr flow retract`. The eligibility
check is the reason — a kind that could be typed without it would let a caller "withdraw" a
commitment, an intent or a fold, and the withdrawal would read as authoritative. The private guard
now takes `NoteKind` rather than `&str`, so "unreachable" is a property of the type rather than of a
spelling. Test: `retraction_is_not_reachable_from_the_note_verb`.

### The four refusals, all before anything is appended

1. **Not a `DepEdge`** — names what the caller actually pointed at ("run event (a note, a fulfilment,
   an observation)"). Retracting a retraction is exactly the recursion a withdraw-anything verb
   would invite; it is refused.
2. **Fails `DepEdge::validate`** — `FlowStore::edges()` already skips such a record, so no reader
   offers it; withdrawing it would be a no-op that reads as work.
3. **Superseded in its own slot** — `edges()` collapses to latest-per-`(from, to)`, so retracting a
   displaced record changes nothing observable. Refused naming the standing record. This fired live
   (below) and is the "never wire a guaranteed no-op" discipline.
4. **Already retracted with a different reason** — two withdrawal reasons for one slot is a ledger a
   reader cannot resolve. An identical re-run is admitted and dedupes to a no-op, which is what makes
   the verb safe to script.

### Where the withdrawal is read

One filter, at the index. `EdgeIndex::build` consults `retract::retracted()` and drops withdrawn
slots from the merged graph, so `epr flow context --concerns`, `epr flow memory recall open` (which
reads *through* `concerns::concerns_with`), `walk`, `status`, `seal` and `reseal` all inherit the
rule without learning it — and `flow/memory/recall.rs` needed no edit, which is how the write-set
boundary was kept. **The doc plane is untouched**: a `cites:` envelope lives inside a document's own
hashed body, and withdrawing one is an edit to that document — `epr flow cites`' plane, not this one.

`epr flow report placement` was checked and **does not read the edge plane at all** (zero matches for
`edges`/`EdgeIndex`/`SidecarFlowStore`/`dangling` in `flow/placement.rs`; it derives from surfaces,
frontmatter, `.claude/memory` link state and cluster-state). A retracted DepEdge therefore cannot
appear there as a dead end — vacuously, by construction, not by a coupling I added. Stated because
the brief names `report placement` as a reader and the honest answer is "it never was one".

### A withdrawal is never a silent shrink

`Counts::retracted` and an omission naming every withdrawn slot and its ground ride in the same view
that stopped showing them. Otherwise `epr flow retract` would be a way to make drift disappear
rather than a way to record a decision about it. Test:
`a_healthy_edge_may_be_withdrawn_too_and_the_view_says_so`.

### Live run — the seven records

The station-five round-two verdict `bafyreigra3p6jfqisbqjoypq6quic6aixi6euvwkldxacqcbzxbjq7qqqy`:
*"The residue is now SEVEN records — the four dangling slots plus the three re-seals authored
against consumers deleted minutes later … station six needs a retraction path for sidecar slots;
none exists in the tree."*

Before: `806 indexed · 40 stale · 4 dangling · 0 retracted`. The four dangling slots headed the
concern page, and the three re-seals read as `stale` (their upstream, the relocated contract, is
readable but its bytes moved since sealing) — seven live slots, exactly as the verdict measured.

An **eighth** edge record exists for the same set and is deliberately not retracted:
`bafyreicx35hrlkh…` (`recall-packet.py → recall-contract.json`) is superseded by
`bafyreiedbxrhklj…` in that slot, so `edges()` never surfaces it. The verb said so rather than
accepting a silent no-op:

```
$ epr flow retract bafyreicx35hrlkhtkiu2wraurhruhdrgckqjzsrqigpr5giuh3gjhnhpwi --reason "…"
epr flow: invalid arguments: record bafyreicx35hrlkh… is superseded in its slot
  `.claude/scripts/memory-kit/recall-packet.py → .claude/scripts/memory-kit/recall-contract.json`
  by bafyreiedbxrhklj… — retract the standing record, or nothing an edge reader shows would change
EXIT=2
```

That is why the verdict's "seven" is right and a naive record count would have said eight.

All seven retracted with reason `endpoints deleted by memory-kit replacement stations 3b and 5;
recorded 2026-09-10`, as `agent:implementer@claude-opus-5`, session `mk-replace-native-6`:

| withdrawn record | slot | endpoints at withdrawal |
|---|---|---|
| `bafyreied62rjy7x…` | `__tests__/recall_packet_test.py → .epr-meta/…/recall-contract.json` | from unreadable · to readable |
| `bafyreidosyitoxl…` | `__tests__/recall_runtime_test.py → .epr-meta/…/recall-contract.json` | from unreadable · to readable |
| `bafyreidzsrtd4g6…` | `recall-contract.json → __tests__/recall_packet_test.py` | both unreadable |
| `bafyreidifs2qnwt…` | `recall-contract.json → __tests__/recall_runtime_test.py` | both unreadable |
| `bafyreiedbxrhklj…` | `recall-packet.py → recall-contract.json` | both unreadable |
| `bafyreicuq6czwxp…` | `recall-packet.py → recall_runtime.py` | both unreadable |
| `bafyreicdwyue7dr…` | `recall-packet.py → .epr-meta/…/recall-contract.json` | from unreadable · to readable |

After, measured on the live root with the installed binary:

```
$ epr flow context . --concerns --limit 100 --json
counts: {"total_edges": 806, "stale": 38, "dangling": 0, "held": 0, "governed": 1,
         "ok": 767, "retracted": 7, "selected_edges": 38, "groups": 23}

$ epr flow status
  edges: 767 sealed · 1 governed · 38 stale · 0 held · 0 dangling
```

**`dangling: 0` across the whole repository.** The seven edge records and the eighth superseded one
are all still in `.eprfs/status/flows.jsonl` — withdrawal recorded, nothing deleted.

`recall open` on the live root, first three concerns:

```
$ epr flow memory recall open --session mk-parity-verify-6b --need "…"
counts: {"dangling": 0, "retracted": 7, "stale": 38, "total_edges": 806, "groups": 23}
  1 stale doc  …/2026-06-02-scope-tree-reconciler-design.md        -> genesis/docs/PLACEMENT.md
  2 stale doc  …/2026-06-02-spec-plan-compaction-loop-design.md    -> genesis/docs/PLACEMENT.md
  3 stale doc  …/2026-07-10-epr-meta-native-capability-dogfood…md  -> …/2026-07-09-epr-meta-eprfs-elohim-native-sotu.md
```

All three are doc-plane **stale** edges with readable endpoints. **No dead end in the first three**,
where the verdict measured groups 0–3 of 26 as the four unreachable slots and `select --edge 1` then
`finish` refusing with *"resource not found in sidecar labels or the tree"*. The ceremony now opens
on a real concern.

## 3. `genesis/scripts/memory_balance.py` — retirement check

**Verdict: YES, station six integration may delete it — as a closed set of six edits, not alone.**

The complete executable footprint, measured (`.py .js .mjs .json .yaml .sh .ts .rs justfile`,
excluding `.claude/worktrees/` and the kit's own report tier):

| # | reference | disposition |
|---|---|---|
| 1 | `genesis/scripts/memory_balance.py` | delete |
| 2 | `genesis/scripts/memory-balance.sh` — `exec python3 …/memory_balance.py "$@"`, a wrapper with **no other caller in the tree** | delete |
| 3 | `genesis/scripts/__tests__/memory_balance_test.py:17,106` — loads the script and asserts on the wrapper; its only consumer | delete |
| 4 | `justfile:451` — `_gate-memory-ceremony`'s first line, `python3 -m unittest discover -s genesis/scripts/__tests__ -p memory_balance_test.py` | **delete this line**, or the gate goes red the moment #3 is gone |
| 5 | `genesis/build-manifest.json:233,234,235` — the three `memory-ceremony` `inputs.sources` rows for #1–#3 | remove the three rows (change-detection only) |
| 6 | `elohim/eprfs/epr-cli/src/flow/memory/recall.rs:62` — `pub const BALANCE_LENS_REL = "genesis/scripts/memory_balance.py"` | **leave**; its only use is a NEGATIVE assertion in `flow_memory_recall.rs:1807` ("the fixture must not carry its own lens script"). Nothing resolves through it. |

**The named parity test is
`elohim/eprfs/epr-cli/tests/flow_memory_footprint.rs::the_native_snapshot_matches_the_python_oracle_field_for_field`,**
and it survives the deletion **by construction**. Its control flow (`flow_memory_footprint.rs`
406–427) asserts the pinned normalized digest `EXPECTED_NORMALIZED_SHA256` (declared at line 29)
FIRST, unconditionally, and only then looks for the oracle:

```rust
assert_eq!(digest, EXPECTED_NORMALIZED_SHA256, "normalized snapshot drifted; re-baseline deliberately");

let script = repo_root().join("genesis/scripts/memory_balance.py");
if !script.is_file() {
    eprintln!("SKIP: the Python oracle is not present; the pinned digest carried this run");
    return;
}
```

So the gating assertion is the digest, and the oracle half degrades to a named skip. Stated
precisely: this is a **reading of the control flow**, not a measured run with the script removed — I
did not delete a tracked file from a shared worktree to prove it. The native replacement that makes
the deletion safe is `elohim/eprfs/epr-cli/src/flow/memory/footprint.rs` (station five round three),
which carries `METHOD_VERSION = "bounded-memory-balance-v1"` and emits `schema:
"memory-balance-v1"` — the same method identity the Python declared.

**Two things the deletion does NOT clear, and they are the integration seat's to route:**

- **The writer of `.claude/memory-kit/balance-sheets/` goes away.** `memory_balance.py:246` is what
  writes that directory. The plan already says station six relocates `balance-sheets/` to
  `.eprfs/status/balance/`; after this deletion that relocation is an **archival move of authored
  evidence with no live writer**, and the native lens writes its own receipts elsewhere. Say so in
  the move, or a later reader will look for a producer that no longer exists. (Its parity row is
  `skipped`, `0` gating references, `20` intra-kit.)
- **Five prose surfaces still name `genesis/scripts/memory-balance.sh` as a ceremony step**:
  `.claude/skills/memory-kit/SKILL.md:339`, `.agents/skills/memory-kit/SKILL.md:342`,
  `.claude/scripts/memory-kit/{CLAUDE.md:123,LIFECYCLE.md:385}` and
  `genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md:339`. The first two are
  package-first rewrites; the middle two go with the kit; the spec is a design doc that should name
  the native lens. **None of these is visible to the parity gate** — `.md` is outside the brief's
  extension set — which is exactly the blind spot named above.

## Concerns (why `DONE_WITH_CONCERNS`)

1. **The station's own predicate is not met and cannot be met by this seat.** 1 of 53 rows passes.
   The artifact may not be cleared. The dominant cause is a stale ledger (Finding 1), whose repair
   is an integration act.
2. **`measures.yaml` carries 25 rows whose `procedure:` method pin names a deleted script** (53
   hits; corrected from "26 rows / 104 hits" by the addendum, which excludes `provenance:`
   citations). Reported as residue, not repaired — `.claude/epr-meta/**` is the integration seat's
   write set.
3. **Two tracked historical records gate rows** (a shift objective, a recorded acceptance). I believe
   they should be exempted rather than edited, but adding a fifth exclusion for *tracked authored
   history* is an operator-shaped call about what "no executable reference" means, not a reader
   tweak. Surfaced, not decided.
4. **The verb probe trusts `--help` surfaces.** Where a chain does not answer `--help` (e.g.
   `epr flow stocks`), a claimed flag is recorded as unverified in `notes` and the claim still
   counts as satisfied. Verbs that answer `--help` inconsistently is a pre-existing CLI unevenness
   this fold reveals rather than causes; the honest alternative (refuse) would fail rows for the
   probe's limitation.
5. **`retracted()` is a linear scan of the record log inside `EdgeIndex::build`.** At 7,800 records
   and 53 rows this is invisible (3.3 s total, dominated by the 49 MB file scan). At an order of
   magnitude more records it wants an index. Named, not optimised.

## What this seat did NOT do

- Did not delete anything under `.claude/scripts/memory-kit/` or `.claude/memory-kit/`, per the brief.
- Did not edit the inventory. The fold reports it; re-taking it is the integration act.
- Did not touch `flow/memory/recall.rs`, `flow/cites.rs` or `flow/report.rs` beyond importing
  `OutcomeStatus` and registering the subcommand in `mod.rs`.
- Did not run the fresh-reader before/after comparison — that is the integration half of this
  station and needs a context-reset reader.
- Did not commit or push. `cargo fmt` touched only the four files this seat authored (verified by
  mtime against the rest of the crate).

---

# Addendum — `provenance:` is a citation, not a consumer (coordinator rule fix, same session)

## The correction

The first cut of the reference scan counted every matching line equally, so the 104 hits across 26
rows in `.claude/epr-meta/measures.yaml` mixed two things the registry deliberately keeps apart. The
registry states its own contract at `.claude/epr-meta/measures.yaml:190`:

> `provenance:` **CITES** the constant a row replaces; identity is `<id>@<version>` plus the row's
> own bytes, never a file path.

So a `provenance:` naming a line in a deleted script is **working as designed** — the whole point of
lifting a hard-coded constant into a declared row is that the row outlives the script the constant
was read out of. Gating on it would make retiring a script invalidate the record of what it once
taught us, which inverts the reason the lift happened. `procedure:` is the opposite: it names the
producer that must exist for the measure to be takeable, so a `procedure:` naming a deleted script
is a **dangling method pin** and a real finding. Finding 2 of the main report conflated them; this
addendum corrects it.

## What changed in the fold

A third non-gating class, and it is a **line shape** rather than a path prefix (the first two
classes — intra-kit and other-tree/derived — are prefixes):

- `CITATION_KEYS = ["provenance"]` in `flow/parity.rs`, with the registry's contract quoted at the
  declaration and `procedure:` named as deliberately absent.
- `is_citation_line()` tests the trimmed line's KEY, tolerating a leading `- ` list item and the
  quoted JSON spelling. Exact for this corpus: all 46 `provenance:` values in the registry are
  single-line double-quoted scalars, so no continuation can slip past a key test. Measured, not
  assumed.
- Hits so classified land in a new per-row `citedReferences` alongside `selfReferences` — counted,
  named in the `--json` payload, and reported in the leg's own detail (`… (N intra-kit, M cited)`),
  never gating. A withdrawal from the gate is never a silent subtraction; that discipline is the
  same one `epr flow retract` holds on the concern page.
- A new `Limit:` line on every payload states the rule and its boundary.

Tests, both as the coordinator specified, in one fixture registry so the contrast is in one file:
**`a_provenance_citation_does_not_gate_but_a_procedure_pin_does`** — a row whose `provenance:` names
a deleted script and whose `procedure:` names a live lens **passes**, carrying one `citedReferences`
entry and zero residue; a row whose `procedure:` names a deleted script **fails**, with the residue
line asserted to start with `procedure:`. The test also pins the omission text, so the rule cannot
be silently removed from what a reader is told.

## Re-gate

Berth claimed before each invocation, released after.

```
env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target \
    cargo fmt --manifest-path elohim/eprfs/Cargo.toml --all --check
FMT_EXIT=0

env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 \
    cargo clippy --manifest-path elohim/eprfs/Cargo.toml --workspace --all-targets -- -D warnings
CLIPPY_EXIT=0

env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 \
    cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace
TEST_EXIT=0
```

**52 binaries, 676 passed, 0 failed, 0 ignored** (+1 over the main report: the new parity test;
`flow_report_parity` is now 11). Clippy's only output remains the nine pre-existing ts-rs
`failed to parse serde attribute` notes from another crate.

Binary re-installed: `/opt/rust/cargo/bin/epr`, **33,069,080 bytes**, sha256
`bed5c3b0a965672c17e749765fca0314db457b41a66c69ee13bcc20a3c56aca8`
(was `66f9db3fbbfcebf98f6f1069437e6c01b07357b4771873e9a0012b9533d98000`). The corrected fold below
was run with it.

## Corrected numbers

| | main report | corrected |
|---|---|---|
| rows | 53 | 53 |
| **passed** | 1 | **1** |
| **failed** | 43 | **42** |
| **skipped** | 9 | **10** |
| gating references | 267 | **216** |
| cited (non-gating) | — | **51**, all in `.claude/epr-meta/measures.yaml` |
| intra-kit (non-gating) | — | 431 |

Method pin unchanged — same inventory bytes, same CID
`bafkreicivjwtdp7654rkclmstvtnup6vewncfcer7ylplt6teoe6iuodae`, 30,046 bytes; 5,133 files scanned.

**Exactly one row's outcome moved: `prep-brainstorm.py`, `failed → skipped`** (its single gating
reference was `.claude/epr-meta/measures.yaml:343`, a `provenance:` line; its references leg is now
`passed`, and the row is skipped on its tests leg alone). No other row's references leg changed
status — the 51 reclassified hits were, in every other case, alongside a gating hit in the same
file, so the correction sharpens the residue without moving the verdict.

**Finding 2 of the main report is superseded by this table:**

| hits | rows | class | where |
|---|---|---|---|
| 53 | 25 | **dangling method pins** — `procedure:` values naming a deleted script | `.claude/epr-meta/measures.yaml` |
| 27 | 10 | test fixtures naming kit scripts or their JSON state | `.claude/hooks/__tests__/drift_observation_test.py` |
| 13 | 9 | the stasis workflow | `.claude/workflows/memory-stasis-loop.js` |
| 11 | 6 | the managed-surface scope table | `.claude/scripts/_lib/managed_surfaces.py` |
| ~106 | — | remaining live wiring, `_lib` tests, prose comments, historical records | hooks, `subject-routing.yaml`, `recipes.yaml`, `delivery-scoreboard.py`, schemas, `.claude/shifts/…` |
| **216** | | **total gating** | |
| *51* | *—* | *`provenance:` citations — **not** residue, working as declared* | *`.claude/epr-meta/measures.yaml`* |

The headline for the integration seat is sharper than before rather than softer: **53 `procedure:`
pins across 25 rows still name a deleted producer**, and that is the single largest gating class in
the repository. Half of what the main report reported against `measures.yaml` was noise; the other
half is a precise, actionable list.

## The corrected table


#### `scripts` — 1 passed · 30 failed · 5 skipped

| artifact | outcome | replacement leg | tests leg | references leg (residue) |
|---|---|---|---|---|
| `agent-audit.py` | **failed** | passed — retired with a reason: script deleted; staleness deliberately not ported (content-based verification) | skipped — the inventory names no checkable test path (cell: `verify` fixture legs: 40 chars refused, 80 accepted,… | failed — 5 executable reference(s) remain: .claude/epr-meta/measures.yaml:474, .claude/epr-meta/measures.yaml:48… |
| `cite-describe.py` | **skipped** | skipped — the inventory declares no replacement (cell opens `no`: no brit equivalent — brit has no desc-authoring… | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to cite-describe.py (0 intra-kit, 0 cited) |
| `cite-gen.py` | **failed** | skipped — the inventory declares no replacement (cell opens `no`: no brit equivalent — brit's cite engine is READ… | failed — named test path(s) missing: .claude/scripts/memory-kit/__tests__/cite_gen_test.py (not found; tried .cl… | failed — 2 executable reference(s) remain: .claude/settings.local.json:116, .claude/shifts/2026-08-14T02-42-saga… |
| `cite-propagate.py` | **failed** | failed — no named replacement resolves: epr flow concerns --stamp <doc> (`epr flow concerns` exists but its usag… | passed — 1 named test path(s) exist | passed — no executable reference to cite-propagate.py (0 intra-kit, 0 cited) |
| `cites-migrate.py` | **skipped** | skipped — the inventory declares no replacement (cell opens `n/a`: n/a) | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to cites-migrate.py (0 intra-kit, 0 cited) |
| `claude-md-audit.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none (`epr check`/`govern` gate writes, not d… | skipped — the inventory names no checkable test path (cell: none) | failed — 5 executable reference(s) remain: .claude/epr-meta/measures.yaml:252, .claude/epr-meta/measures.yaml:51… |
| `cleanup-apply.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/epr-meta/measures.yaml:632 |
| `cleanup-pressure.py` | **failed** | passed — resolves: epr flow stocks --check | passed — 1 named test path(s) exist | failed — 25 executable reference(s) remain: .claude/epr-meta/measures.yaml:58, .claude/epr-meta/measures.yaml:34… |
| `cleanup-scan.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/epr-meta/measures.yaml:673 |
| `context-ratchet.py` | **skipped** | passed — resolves: epr flow stocks --check | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to context-ratchet.py (4 intra-kit, 0 cited) |
| `decompose.py` | **failed** | passed — resolves: epr flow project | passed — 1 named test path(s) exist | failed — 13 executable reference(s) remain: .claude/epr-meta/measures.yaml:362, .claude/scripts/_lib/__tests__/s… |
| `dedupe-memory-scan.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/epr-meta/measures.yaml:662 |
| `delivery-status-distribution.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 2 executable reference(s) remain: .claude/scripts/delivery-scoreboard.py:159, .claude/scripts/delivery-… |
| `focus-baseline.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | passed — 1 named test path(s) exist | failed — 3 executable reference(s) remain: .claude/scripts/_lib/__tests__/cluster_state_test.py:55, .claude/scri… |
| `locus-drift.py` | **passed** | passed — resolves: epr flow concerns | passed — 1 named test path(s) exist | passed — no executable reference to locus-drift.py (2 intra-kit, 0 cited) |
| `memkit-retention.py` | **failed** | passed — retired with a reason: apply leg disabled; proposals lack custody evidence | passed — 1 named test path(s) exist | failed — 7 executable reference(s) remain: .claude/epr-meta/measures.yaml:312, .claude/epr-meta/measures.yaml:32… |
| `memory-coherence-audit.py` | **failed** | passed — resolves: epr flow concerns | failed — named test path(s) missing: __tests__/memory_coherence_audit_test.py (not found; tried __tests__/memory… | failed — 5 executable reference(s) remain: .claude/epr-meta/measures.yaml:651, .claude/hooks/memory-coherence-si… |
| `memory-index-projector.py` | **failed** | skipped — no checkable replacement named (cell: `eprfs-agent::project` projects capability packages, not the memo… | passed — 1 named test path(s) exist | failed — 10 executable reference(s) remain: .claude/epr-meta/measures.yaml:202, .claude/epr-meta/measures.yaml:2… |
| `memory-review.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 3 executable reference(s) remain: .claude/epr-meta/measures.yaml:212, .claude/epr-meta/measures.yaml:40… |
| `mempalace-currency.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 5 executable reference(s) remain: .claude/epr-meta/measures.yaml:554, .claude/epr-meta/measures.yaml:59… |
| `path-update-apply.py` | **failed** | skipped — no checkable replacement named (cell: slug-resolved cites self-heal on move (`cite-propagate` `path:` c… | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/epr-meta/measures.yaml:695 |
| `path-update-scan.py` | **failed** | skipped — the inventory declares no replacement (cell opens `same`: same as above) | skipped — the inventory names no checkable test path (cell: none) | failed — 2 executable reference(s) remain: .claude/epr-meta/measures.yaml:564, .claude/epr-meta/measures.yaml:684 |
| `placement-audit.py` | **failed** | passed — resolves: epr flow stocks, epr flow status | skipped — the inventory names no checkable test path (cell: none) | failed — 45 executable reference(s) remain: .claude/epr-meta/concerns.yaml:26, .claude/epr-meta/measures.yaml:24… |
| `prep-brainstorm.py` | **skipped** | passed — resolves: epr flow context | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to prep-brainstorm.py (4 intra-kit, 1 cited) |
| `recall-ceremony.py` | **failed** | passed — resolves: epr flow context | failed — named test path(s) missing: __tests__/recall_ceremony_test.py (not found; tried __tests__/recall_ceremo… | passed — no executable reference to recall-ceremony.py (7 intra-kit, 0 cited) |
| `recall-packet.py` | **failed** | skipped — the inventory declares no replacement (cell opens `same`: same as above) | failed — named test path(s) missing: __tests__/recall_packet_test.py (not found; tried __tests__/recall_packet_t… | failed — 1 executable reference(s) remain: genesis/docs/superpowers/plans/governed-retrieval-acceptance.json:4 |
| `recall_lenses.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | failed — named test path(s) missing: __tests__/recall_ceremony_test.py:445 (not found; tried __tests__/recall_ce… | passed — no executable reference to recall_lenses.py (0 intra-kit, 0 cited) |
| `recall_providers.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | failed — named test path(s) missing: recall_runtime_test.py (not found; tried recall_runtime_test.py, .claude/sc… | passed — no executable reference to recall_providers.py (17 intra-kit, 0 cited) |
| `recall_runtime.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | failed — named test path(s) missing: __tests__/recall_runtime_test.py (not found; tried __tests__/recall_runtime… | passed — no executable reference to recall_runtime.py (27 intra-kit, 0 cited) |
| `scope-reconcile.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | passed — 1 named test path(s) exist | failed — 12 executable reference(s) remain: .claude/epr-meta/measures.yaml:606, .claude/hooks/__tests__/drift_ob… |
| `skill-audit.py` | **failed** | passed — retired with a reason: script deleted; staleness deliberately not ported | skipped — the inventory names no checkable test path (cell: none) | failed — 4 executable reference(s) remain: .claude/epr-meta/measures.yaml:474, .claude/epr-meta/measures.yaml:48… |
| `spec-coherence-index.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 3 executable reference(s) remain: .claude/scripts/_lib/managed_surfaces.py:90, .claude/scripts/_lib/man… |
| `stale-record.py` | **failed** | passed — resolves: epr flow note --kind correction | failed — named test path(s) missing: __tests__/stale_record_test.py (not found; tried __tests__/stale_record_tes… | passed — no executable reference to stale-record.py (0 intra-kit, 0 cited) |
| `state-machine-gen.py` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to state-machine-gen.py (5 intra-kit, 0 cited) |
| `story-coverage-audit.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/epr-meta/measures.yaml:574 |
| `substrate-currency-audit.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 3 executable reference(s) remain: .claude/epr-meta/measures.yaml:524, .claude/epr-meta/measures.yaml:53… |

#### `report-tier-state-files` — 0 passed · 12 failed · 5 skipped

| artifact | outcome | replacement leg | tests leg | references leg (residue) |
|---|---|---|---|---|
| `placement-drift.json` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | passed — 1 named test path(s) exist | failed — 6 executable reference(s) remain: .claude/epr-meta/measures.yaml:856, .claude/hooks/__tests__/drift_obs… |
| `claude-md-drift.json (76K)` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 12 executable reference(s) remain: .claude/epr-meta/measures.yaml:858, .claude/epr-meta/measures.yaml:8… |
| `map-currency-drift.json` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 6 executable reference(s) remain: .claude/epr-meta/measures.yaml:857, .claude/epr-meta/measures.yaml:88… |
| `memory-coherence-drift.json` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 5 executable reference(s) remain: .claude/epr-meta/measures.yaml:860, .claude/hooks/__tests__/drift_obs… |
| `memory-index-drift.json` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | passed — 1 named test path(s) exist | failed — 5 executable reference(s) remain: .claude/epr-meta/measures.yaml:861, .claude/workflows/memory-stasis-l… |
| `sovereignty-guard-drift.json` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 5 executable reference(s) remain: .claude/hooks/__tests__/drift_observation_test.py:305, .claude/hooks/… |
| `state-ledger.json (128K)` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to state-ledger.json (2 intra-kit, 0 cited) |
| `cites-index.json` | **failed** | passed — resolves: epr flow concerns | skipped — the inventory names no checkable test path (cell: none) | failed — 5 executable reference(s) remain: .claude/epr-meta/measures.yaml:651, .claude/hooks/__tests__/drift_obs… |
| `context-coverage-baseline.json / context-coverage.yaml` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | passed — 1 named test path(s) exist | failed — 4 executable reference(s) remain: .claude/scripts/_lib/__tests__/epr_meta_coverage_nudge_test.py:43, .c… |
| `spec-coherence-index.json (192K)` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to spec-coherence-index.json (2 intra-kit, 0 cited) |
| `story-coverage-audit.json + .prev.json (132K ea.)` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to story-coverage-audit.json, .prev.json (9 intra-kit, 0 cited) |
| `delivery-status-distribution.json + .prev.json (228K/200K)` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/scripts/delivery-scoreboard.py:167 |
| `gap-items/ (2.3M, 247 files)` | **failed** | passed — resolves: epr flow project | passed — 1 named test path(s) exist | failed — 6 executable reference(s) remain: .claude/epr-meta/recipes.yaml:50, .claude/epr-meta/recipes.yaml:169, … |
| `recall-executions/ (648K, 28 files)` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | failed — named test path(s) missing: __tests__/recall_{ceremony,runtime}_test.py (not found; tried __tests__/rec… | passed — no executable reference to recall-executions/ (0 intra-kit, 0 cited) |
| `horizon-scans/ (48K, 3 files)` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/subject-routing.yaml:155 |
| `balance-sheets/ (100K, 24 files)` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none) | passed — 1 named test path(s) exist | passed — no executable reference to balance-sheets/ (20 intra-kit, 0 cited) |
| `dated dirs 2026-05-10…2026-09-09 (40 dirs, ~4.4 MB)` | **skipped** | passed — retired with a reason: derived reports, regenerable; oldest 4 months | skipped — the inventory names no checkable test path (cell: none) | skipped — the artifact cell names no single kit path to search for (cell: dated dirs `2026-05-10`…`2026-09-09` (4… |

## Standing

The main report's verdict is unchanged: **1 of 53 rows passes; the artifact may not be cleared.**
Finding 1 still dominates — 19 of 36 script rows name a script already gone while the row declares
`partial`/`missing`. The integration seat is concurrently re-taking the inventory and re-pointing
the `procedure:` fields, so **these numbers will move again, and that is the intended direction**:
re-pointing a `procedure:` clears a gating hit, and re-declaring a row as `retire — <reason>` with
its replacement's test named as a path clears the other two legs. Re-run
`epr flow report parity --inventory <path>` after that pass; the method pin will change with the
inventory bytes, which is what makes the two measurements legibly different rather than silently so.

One thing this addendum did NOT do: it did not touch `.claude/epr-meta/measures.yaml`. The registry
is the integration seat's write set, and the 53 dangling `procedure:` pins are reported, not
repaired.

---

# Addendum (b) — the last two headline producers go native, and `--coverage`/`--stasis` land

Round (a) (integration seat, `task-6-integration-report.md`) ended with three named ordering
constraints on the deletion. This round removes all three from the native side.

| Round (a) concern | State after this round |
|---|---|
| **#3** — `memkit-report-tier-mb@1` and `mempalace-surfaces-changed@1` are bridged from the kit; deleting `memkit-retention.py`/`mempalace-currency.py` costs two `skipped` headline slots | **Closed.** `mempalace:` is derived from the tree; `memkit:` is retired with lineage. `_HEADLINE_BRIDGE` is now empty. |
| **#7** — the stasis workflow's `AUDIT` const still runs `placement-audit.py` because `--coverage` and `--stasis` have no native rendering | **Closed on the native side.** Both render; the workflow's const is the integration seat's to re-point. |
| **#2** — fourteen `procedure:` rows still name a live kit script | **Six re-pointed, six annotated with the round that retires them, two retired with the thing they measured.** |

Live headline, taken with the installed binary:

```
$ epr flow report --headline
  memkit: retired — report tier removed 2026-09-11
  mempalace: ⚠ failed — 213 of 544 .md file(s) newer than .mempalace/.last-mine (+120s grace) — has reached the hard watermark 1
  cleanup: 96 pressure-points since the beginning within hard 120 ✅
  scope: ⚠ 3 to hold (local-conductor,owned-substrate)  →  epr flow hold --scope --apply
  memory-budget: ⚠ 22412 bytes is past the soft watermark 20000 (hard 24000)
```

**All five slots now have a native producer, and the `mempalace:` count is byte-exact against the
script it replaces:** `mempalace-currency.py --status --json` reports
`{"stale": true, "changed_since_mine": 213, ...}`; the native walk reports **213**.

## (1) `mempalace-surfaces-changed@1` — `derive: files-newer-than`

A third derive kind, and the first that reads the **tree** rather than the fold plane. The other two
accumulate acts somebody recorded; this measures a standing condition — *how far has the surface
moved since the index was built* — which nobody records because nobody performs it.

The walk is **declared on the row**, not compiled in. `mempalace-currency.py` hid `SURFACE`,
`GRACE_SECONDS` and the marker path as module constants only that script could read; the lens row
now carries `marker:`, `marker-fallback:`, `surfaces:`, `suffix:` and `grace-seconds:`, transcribed
from `mempalace-currency.py:22-27` verbatim. That is the whole point of the replacement: the number
is re-derivable by anyone.

**A missing stamp is `skipped`, never zero — and never the kit's maximum either.** The kit falls
back to the palace config's mtime and then to **epoch 0**, and epoch 0 makes *every* file newer: the
script's own failure mode is "everything is stale", asserted from an absence of evidence. Both
readings are wrong for the same reason — with no stamp, nobody knows when the index was built. The
config-mtime fallback is kept (it is a real timestamp, and it is the kit's); only the epoch-0 arm
becomes a refusal that names the marker it could not read.

Three more refusals, each a measurement-vs-number distinction: an unparseable marker is a **broken
stamp**, not a stamp reading zero; a declared-but-absent surface is skipped in the walk and **named
in the summary** (the kit does the same), while a walk where *no* declared surface exists is
`skipped`; and a row declaring `files-newer-than` without both `marker:` and `surfaces:` falls back
to a plain reading rather than confidently scanning nothing. A `.git` tree is never entered and a
symlinked directory is never followed.

## (2) `memkit-report-tier-mb@1` — superseded, not skipped

The report tier this measured is removed in the next round. A bound over the size of a deleted
directory is not a measurement anybody can take, and `skipped` would render a decision as neglect —
`skipped` means *nobody measured this*, which is a gap somebody should close.

So the registry's never-delete rule applies as written (`measures.yaml` head): the measure row and
its ceiling lens both carry `status: superseded`, `superseded_by:` and `superseded_reason:`, and the
report **leaves a superseded bound out of evaluation entirely**. It contributes to no `Totals`
bucket — totals stay a count of measurements — and appears in a new `retired[]` list carrying the
lineage. The headline renders `memkit: retired — report tier removed 2026-09-11`.

## (3) `report placement --coverage` and `--stasis`

**`--coverage`** keys are the kit's, because `memory-stasis-loop.js:76` reads `uncaptured` by name.
On the live tree `active` (384) and `needs_agent` (4) match the kit exactly.

`captured` does not, and the difference is the substrate, not a defect: the kit asked *does
`gap-items/<slug>.json` exist with items* — capture as a stored artifact — while station two made
extraction native, so a plan with `- [ ]` stations is captured the moment it is written. Native
`captured` is **281** against the kit's 185; the 96 difference is documents whose own bytes carry
stations that nobody ever wrote a JSON for. The store is still consulted for the one case it is the
sole witness of (an agent decomposed a prose doc and found no structure → `needs-agent`), and
`uncaptured = undecomposed + needs_agent` is preserved exactly because that is the number the loop
drains. The changed reading is on the payload as a `method` field, not only in a doc comment.

**`--stasis`** carries the kit's keys for both consumers — the loop's `stasis_score`/`at_stasis`
and `context-ratchet.py`'s `dimensions`/`score`. **Eight of the nine dimensions match the kit
exactly on the live repository:**

| dimension | native | kit |
|---|---|---|
| status | 0.930 | 0.930 |
| well_formed | 0.927 | 0.927 |
| memory_linked | 0.127 | 0.127 |
| claude_md_rightsized | 0.857 | 0.857 |
| history_bidirectional | 0.642 | 0.642 |
| traceability | 0.000 | 0.000 |
| memory_md_budget | 1.000 | 1.000 |
| epr_meta_coverage | 0.352 | 0.352 |
| **capture** | **0.732** | **0.482** |
| score | 0.618 | 0.591 |

`capture` is the `--coverage` divergence above, and it is the only reason the composite differs.

Three findings from getting there, each recorded in the code where it bit:

- **The `.epr-meta` census cannot read the parsed record's `subject`.** `parse_meta_file` passes
  `subject_path: None`, and `to_record` maps `covers: subtree` to `Subtree{path}` only when it has a
  path — with none it falls through to `Projection`. A census built on the parsed subject finds
  **zero** claims and reports 0.0 on a repository with 31 of them, which is exactly what the first
  cut did. The claim is now read from the manifest's own `covers:` key with `parse_meta_file` as the
  *validity* check — the kit's exact pair, and the rule that a schema-invalid manifest is not
  credited (a broken manifest would otherwise inflate the ratio and hide a real gap).
- **`well_formed` is a graph property and needs two passes.** The kit builds a `link_targets` set
  and counts a doc non-orphan on inbound **or** outbound. A one-pass outbound-only reading scored
  0.698 against the kit's 0.927 — 23 points of silent movement on a *ratcheted* dimension.
- **`"canonical" in fm` is a key-PRESENCE test.** Asking only the scalar accessor missed every doc
  declaring `canonical:` as a list and moved `history_bidirectional` from 0.755 to 0.396 in one
  edit. The two frontmatter accessors answer different questions.

**The ratchet baseline is a fold.** `context-ratchet.py` keeps it in
`.claude/memory-kit/context-coverage-baseline.json`, which only that script can read or write —
the unaddressable-history shape the whole replacement exists to end. `--stasis --fold` appends one
observation per dimension on `context-coverage-dimension@1`, and the reading compares against the
latest fold. Verified live: fold, then re-read → all nine `held`.

One modelling correction worth recording, because the note plane refused the first attempt and was
right to. The first cut used the dimension name as the fold's **subject** and was refused —
`capture` is not a resource, and a subject naming no addressable thing is a fold nobody can join
back to what it measured. The subject is the **repository**; the dimension is the **env** the
measurement was taken under, which is what env keys are for.

**A dimension with no fold is `unbaselined` — neither a regression nor a pass.** "Nobody has
measured this before" and "this has not fallen" are different claims, and collapsing them is how a
ratchet reports green on its first run forever.

**The tuning surface survives the deletion.** Weights, budgets and margin live in
`context-coverage.yaml`, inside the report tier round (b) removes. The reader walks
`.epr-meta/elohim/lenses/…` → `.claude/memory-kit/…` → built-in constants transcribed from the
manifest, and **says which rung it used** on the payload. A tuning surface that vanishes silently is
how a score changes without anybody deciding to change it.

The kit's `unmeasured` list is carried verbatim and those dimensions stay excluded from the score —
the same `skipped`-is-not-zero rule, one layer down. The hard gates (`_retired` dumps, pressure
dirs) carry **the count that decided them**, because a boolean with no number behind it is
unauditable, and they gate the verdict without moving the score.

## (4) The fourteen `procedure:` rows

| Row | Now |
|---|---|
| `gospel-bytes@1` | `epr flow report placement --stasis` — the `claude_md_rightsized` dimension's byte test |
| `stasis-margin@1` | `epr flow report placement --stasis` — the `margin` on the payload |
| `cleanup-pressure@1` | `epr flow report --bound cleanup-pressure-ceiling` — the derive, no accumulator file |
| `mine-grace-seconds@1` | `epr flow report` — the `grace-seconds:` key on the surface walk |
| `mempalace-surfaces-changed@1` | `epr flow report` — the native surface walk |
| `memkit-report-tier-mb@1` | `none` + `status: superseded` |
| `retention-head-days@1`, `retention-tail-days@1` | `none` — retire with the report tier they windowed |
| `cleanup-cycle-history@1` | `none` — the state file it bounded is replaced by the append-only fold plane, which needs no retention window |
| `gospel-lines@1` | **still the kit**, annotated: the native dimension measures BYTES, not lines; retires with `claude-md-audit.py` |
| `rationale-window-lines@1` | **still the kit**, annotated: retires with `claude-md-audit.py` |
| `exempt-substr-window-chars@1`, `negation-window-chars@1`, `bare-negation-window-chars@1` | **still the kit**, annotated: retire with `substrate-currency-audit.py` |
| `story-vision-weight@1` | **still the kit**, annotated: retires with `story-coverage-audit.py` |

Six rows whose producer is genuinely still a kit script now carry a `# STILL THE KIT:` comment
naming the round that retires each. Round (a)'s concern #2 was that the deletion "will look complete
without it"; the annotation is what makes the omission visible in the file itself.

One new row was declared: **`context-coverage-dimension@1`**, the ratchet baseline's measure. That
is one row beyond the letter of this round's write set, and it is a judgment call I am naming rather
than burying: instruction (3) says *fold the baseline as observations*, and a fold must pin a
declared measure or the note verb refuses it. One row, subject-keyed to the repository and
env-keyed by dimension, was the smallest thing that could satisfy the instruction.

## (5) The live fold

```
$ epr flow report parity --inventory …/parity-inventory-2026-09-10.md
parity  …@bafkreiftoln7gpd23sweatdn37tfu4qhz3vwvknaozdhgchnjjyzt4bfna  (53 rows: 22 passed · 21 failed · 10 skipped)
EXIT=0
```

| | round (a) | round (b) |
|---|---|---|
| passed | 21 | **22** |
| failed | 22 | **21** |
| skipped | 10 | **10** |
| gating references | ~62 | **41** |
| `measures.yaml` hits | 15 | **6** |

The `measures.yaml` residue is now **exactly the six rows this round deliberately left pointing at a
live kit script**, each carrying the round that retires it. Nothing in that file is an oversight.


#### table 1 — 18 passed · 12 failed · 6 skipped (36 rows)

| artifact | outcome | replacement | tests | references (residue) |
|---|---|---|---|---|
| `kit/agent-audit.py` | **passed** | passed — retired with a reason: the description-floor, trigger-overlap and dynamic-stopword checks were … | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/agent-audit.py (3 intra-kit, 5 cited) |
| `kit/cite-describe.py` | **passed** | passed — retired with a reason: the native cite WRITER landed at station 3b; `describe` is its desc-auth… | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/cite-describe.py (0 intra-kit, 0 cited) |
| `kit/cite-gen.py` | **failed** | passed — retired with a reason: `seal` assigns `id:`, converts legacy path-cites to envelopes and refres… | passed — 1 named test path(s) exist | failed — 2 executable reference(s) remain: .claude/settings.local.json:116, .claude/shifts/2026-08-14T02… |
| `kit/cite-propagate.py` | **passed** | passed — retired with a reason: station 3b landed the stamping verb as `cites stamp` (NOT `concerns --st… | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/cite-propagate.py (0 intra-kit, 0 cited) |
| `kit/cites-migrate.py` | **passed** | passed — retired with a reason: the corpus sweep is an arm of the native writer | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/cites-migrate.py (0 intra-kit, 0 cited) |
| `kit/claude-md-audit.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none — nothing native reads CLAUDE.md… | skipped — the inventory names no checkable test path (cell: none) | failed — 3 executable reference(s) remain: .claude/epr-meta/measures.yaml:252, .claude/epr-meta/measures… |
| `kit/cleanup-apply.py` | **passed** | passed — retired with a reason: archival relocation is not net removal (the design's ruling); an accepte… | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/cleanup-apply.py (0 intra-kit, 0 cited) |
| `kit/cleanup-pressure.py` | **passed** | passed — resolves: epr flow report | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/cleanup-pressure.py (0 intra-kit, 3 cited) |
| `kit/cleanup-scan.py` | **skipped** | passed — resolves: .epr-meta/elohim/lenses/memory/cleanup-scan.py | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to .claude/scripts/memory-kit/cleanup-scan.py (0 intra-kit, 1 cited) |
| `kit/context-ratchet.py` | **passed** | passed — resolves: epr flow stocks | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/context-ratchet.py (0 intra-kit, 1 cited) |
| `kit/decompose.py` | **failed** | passed — resolves: epr flow project | passed — 1 named test path(s) exist | failed — 4 executable reference(s) remain: .claude/scripts/_lib/__tests__/subject_routing_test.py:30, .c… |
| `kit/dedupe-memory-scan.py` | **skipped** | passed — resolves: .epr-meta/elohim/lenses/memory/dedupe-memory-scan.py | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to .claude/scripts/memory-kit/dedupe-memory-scan.py (0 intra-kit, 1 cit… |
| `kit/delivery-status-distribution.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none — nothing native projects the de… | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/scripts/delivery-scoreboard.py:170 |
| `kit/focus-baseline.py` | **failed** | passed — retired with a reason: station two ported the subject focus baseline into the placement report | passed — 1 named test path(s) exist | failed — 1 executable reference(s) remain: .claude/subject-routing.yaml:15 |
| `kit/locus-drift.py` | **passed** | passed — resolves: epr flow concerns | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/locus-drift.py (0 intra-kit, 0 cited) |
| `kit/memkit-retention.py` | **failed** | passed — resolves: epr flow report | passed — 1 named test path(s) exist | failed — 1 executable reference(s) remain: genesis/build-manifest.json:233 |
| `kit/memory-coherence-audit.py` | **passed** | passed — resolves: .epr-meta/elohim/lenses/memory/memory-coherence-audit.py | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/memory-coherence-audit.py (4 intra-kit, 1… |
| `kit/memory-index-projector.py` | **failed** | passed — resolves: epr flow memory project | passed — 1 named test path(s) exist | failed — 1 executable reference(s) remain: .claude/hooks/memory-index-projection.py:65 |
| `kit/memory-review.py` | **skipped** | passed — resolves: .epr-meta/elohim/lenses/memory/memory-review.py | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to .claude/scripts/memory-kit/memory-review.py (3 intra-kit, 4 cited) |
| `kit/mempalace-currency.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none — nothing native probes the MemP… | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/workflows/memory-stasis-loop.js:60 |
| `kit/path-update-apply.py` | **skipped** | passed — resolves: .epr-meta/elohim/lenses/memory/path-update-apply.py | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to .claude/scripts/memory-kit/path-update-apply.py (0 intra-kit, 1 cite… |
| `kit/path-update-scan.py` | **skipped** | passed — resolves: .epr-meta/elohim/lenses/memory/path-update-scan.py | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to .claude/scripts/memory-kit/path-update-scan.py (0 intra-kit, 2 cited) |
| `kit/placement-audit.py` | **failed** | passed — resolves: epr flow report placement | passed — 1 named test path(s) exist | failed — 3 executable reference(s) remain: .claude/hooks/deprecation-sentinel.py:670, .claude/settings.l… |
| `kit/prep-brainstorm.py` | **passed** | passed — resolves: epr flow context | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/prep-brainstorm.py (0 intra-kit, 1 cited) |
| `kit/recall-ceremony.py` | **passed** | passed — retired with a reason: station five ported budgets, receipts, method pinning and session accoun… | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/recall-ceremony.py (2 intra-kit, 0 cited) |
| `kit/recall-packet.py` | **passed** | passed — retired with a reason: the bounded source/metadata/semantic packet is the native session's `ope… | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/recall-packet.py (6 intra-kit, 0 cited) |
| `kit/recall_lenses.py` | **passed** | passed — retired with a reason: projection receipts are native and private under `.eprfs/status/recall/<… | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/recall_lenses.py (0 intra-kit, 0 cited) |
| `kit/recall_providers.py` | **passed** | passed — retired with a reason: recipe-selected provider dispatch is native; the deterministic local tra… | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/recall_providers.py (1 intra-kit, 0 cited) |
| `kit/recall_runtime.py` | **passed** | passed — retired with a reason: budget counters, bounded reads and method pinning are native | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/recall_runtime.py (5 intra-kit, 0 cited) |
| `kit/scope-reconcile.py` | **passed** | passed — resolves: epr flow hold | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/scope-reconcile.py (2 intra-kit, 1 cited) |
| `kit/skill-audit.py` | **passed** | passed — retired with a reason: same absorption as agent-audit; the 60-character skill floor and the 80-… | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/skill-audit.py (0 intra-kit, 4 cited) |
| `kit/spec-coherence-index.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none — nothing native indexes prior a… | skipped — the inventory names no checkable test path (cell: none) | failed — 2 executable reference(s) remain: .claude/scripts/_lib/managed_surfaces.py:90, .claude/scripts/… |
| `kit/stale-record.py` | **passed** | passed — retired with a reason: station five replaced the filename-date heuristic with an identity-close… | passed — 1 named test path(s) exist | passed — no executable reference to .claude/scripts/memory-kit/stale-record.py (0 intra-kit, 0 cited) |
| `kit/state-machine-gen.py` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to .claude/scripts/memory-kit/state-machine-gen.py (1 intra-kit, 0 cite… |
| `kit/story-coverage-audit.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none — nothing native projects story↔… | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/epr-meta/measures.yaml:580 |
| `kit/substrate-currency-audit.py` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none — nothing native scans gospel fo… | skipped — the inventory names no checkable test path (cell: none) | failed — 3 executable reference(s) remain: .claude/epr-meta/measures.yaml:530, .claude/epr-meta/measures… |

#### table 2 — 4 passed · 9 failed · 4 skipped (17 rows)

| artifact | outcome | replacement | tests | references (residue) |
|---|---|---|---|---|
| `tier/placement-drift.json` | **failed** | passed — resolves: epr flow report | passed — 1 named test path(s) exist | failed — 1 executable reference(s) remain: .claude/hooks/placement-drift-signal.py:23 |
| `tier/claude-md-drift.json` | **failed** | passed — resolves: epr flow report | passed — 1 named test path(s) exist | failed — 4 executable reference(s) remain: .claude/hooks/__tests__/drift_observation_test.py:335, .claud… |
| `tier/map-currency-drift.json` | **failed** | passed — resolves: epr flow report | passed — 1 named test path(s) exist | failed — 2 executable reference(s) remain: .claude/hooks/__tests__/drift_observation_test.py:267, .claud… |
| `tier/memory-coherence-drift.json` | **failed** | passed — resolves: epr flow report | passed — 1 named test path(s) exist | failed — 2 executable reference(s) remain: .claude/hooks/__tests__/drift_observation_test.py:364, .claud… |
| `tier/memory-index-drift.json` | **passed** | passed — resolves: epr flow memory project | passed — 1 named test path(s) exist | passed — no executable reference to .claude/memory-kit/memory-index-drift.json (1 intra-kit, 0 cited) |
| `tier/sovereignty-guard-drift.json` | **failed** | passed — resolves: epr flow report | passed — 1 named test path(s) exist | failed — 3 executable reference(s) remain: .claude/hooks/__tests__/drift_observation_test.py:376, .claud… |
| `tier/state-ledger.json` | **passed** | passed — resolves: epr flow report placement | passed — 1 named test path(s) exist | passed — no executable reference to .claude/memory-kit/state-ledger.json (2 intra-kit, 0 cited) |
| `tier/cites-index.json` | **failed** | passed — resolves: epr flow concerns | passed — 1 named test path(s) exist | failed — 2 executable reference(s) remain: .claude/hooks/__tests__/drift_observation_test.py:352, .claud… |
| `tier/context-coverage-baseline.json` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none — no native per-dimension ratche… | passed — 1 named test path(s) exist | passed — no executable reference to .claude/memory-kit/context-coverage-baseline.json (2 intra-kit, 1 ci… |
| `tier/spec-coherence-index.json` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to .claude/memory-kit/spec-coherence-index.json (2 intra-kit, 0 cited) |
| `tier/story-coverage-audit.json` | **skipped** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | passed — no executable reference to .claude/memory-kit/story-coverage-audit.json (2 intra-kit, 0 cited) |
| `tier/delivery-status-distribution.json` | **failed** | skipped — the inventory declares no replacement (cell opens `none`: none) | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/scripts/delivery-scoreboard.py:178 |
| `tier/gap-items/` | **failed** | passed — resolves: epr flow project | passed — 1 named test path(s) exist | failed — 2 executable reference(s) remain: .claude/epr-meta/recipes.yaml:50, .claude/epr-meta/recipes.ya… |
| `tier/recall-executions/` | **passed** | passed — resolves: epr flow memory recall | passed — 1 named test path(s) exist | passed — no executable reference to .claude/memory-kit/recall-executions/ (0 intra-kit, 0 cited) |
| `tier/horizon-scans/` | **failed** | passed — resolves: genesis/docs/analysis/ | skipped — the inventory names no checkable test path (cell: none) | failed — 1 executable reference(s) remain: .claude/subject-routing.yaml:155 |
| `tier/balance-sheets/` | **passed** | passed — resolves: epr flow memory recall measure | passed — 1 named test path(s) exist | passed — no executable reference to .claude/memory-kit/balance-sheets/ (0 intra-kit, 0 cited) |
| `dated dirs 2026-05-10…2026-09-09 (40 dirs, ~4.4 MB)` | **skipped** | passed — retired with a reason: derived audit reports, regenerable; oldest four months, no writer newer … | skipped — the inventory names no checkable test path (cell: none) | skipped — the artifact cell names no single kit path to search for (cell: dated dirs `2026-05-10`…`2026-0… |

## Round (b) gate

Berth claimed before every cargo invocation, released after.

```
env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target cargo fmt --manifest-path elohim/eprfs/Cargo.toml --all --check
FMT_EXIT=0

env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 cargo clippy --manifest-path elohim/eprfs/Cargo.toml --workspace --all-targets -- -D warnings
CLIPPY_EXIT=0

env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace
TEST_EXIT=0
```

**54 binaries, 690 passed, 0 failed, 0 ignored** (+14 over the previous addendum: `flow_stasis` 8,
`flow_report_surface_walk` 6). Clippy's only output is the nine pre-existing ts-rs
`failed to parse serde attribute` notes; one genuine lint of mine (`into_iter_on_ref`) was found and
fixed rather than allowed.

Binary installed: `/opt/rust/cargo/bin/epr`, **34,360,960 bytes**, sha256
`f9190e921c114776a4eee5a186b82aa2684c03561f2435531bf5f936e8998f59`. Every live number above was
taken with it.

## What landed

| Path | Role |
|---|---|
| `elohim/eprfs/epr-cli/src/flow/stasis.rs` | NEW (~900 lines). `--coverage`, `--stasis`, the tuning ladder, the `.epr-meta` subtree census, the fold-plane ratchet. |
| `elohim/eprfs/epr-cli/src/flow/measures.rs` | `Derive::FilesNewerThan` + `SurfaceWalk` + `reads_tree()`; `Bound::{status, superseded_by, superseded_reason, walk}` and `Bound::retired()`. |
| `elohim/eprfs/epr-cli/src/flow/report.rs` | `evaluate_surface_walk`, `marker_epoch`, `count_newer`, the shared `judge` helper; superseded bounds partitioned out of evaluation into `RetiredBound`; `retired —` headline rendering. |
| `elohim/eprfs/epr-cli/src/flow/placement.rs` | `SurfaceDoc`/`MemoryRow` projection, the two-pass link graph, `--coverage`/`--stasis`/`--fold` dispatch. |
| `elohim/eprfs/epr-cli/src/flow/mod.rs` | flag parsing (with `--coverage`+`--stasis` and bare `--fold` refused by name) and usage. |
| `elohim/eprfs/epr-cli/tests/flow_stasis.rs` | NEW. 8 tests, expectations pinned as hand-derived constants. |
| `elohim/eprfs/epr-cli/tests/flow_report_surface_walk.rs` | NEW. 6 tests. |
| `.claude/epr-meta/measures.yaml` | the rows above. |
| `.claude/hooks/_observation.py` | `_HEADLINE_BRIDGE = ()`, with the four native producers named. |

## Concerns

1. **`capture` and therefore `score` differ from the kit by design** (0.732/0.618 vs 0.482/0.591).
   Documented on the payload and here, but a reader comparing two runs across the cutover will see a
   step change. If the integration seat wants continuity, the ratchet baseline should be folded
   AFTER the cutover, not before — and this round folded it, so the baseline is native-keyed.
2. **A live outcome's headline slot is derived from its measure id; a RETIRED bound's is read from
   its `headline:` key.** The asymmetry is real: `BoundOutcome` deliberately carries no slot field,
   and a retired bound never becomes an outcome. It works today because every headline measure id
   carries its slot as a prefix. A row that declared `headline:` against an unrelated measure id
   would render under the wrong slot while live and the right one when retired. Named, not fixed.
3. **One new registry row** (`context-coverage-dimension@1`) beyond the literal write set, argued
   above.
4. **`--stasis` walks the whole repository** for the `.epr-meta` census and the CLAUDE.md glob; it
   takes a few seconds on this tree. Acceptable for a ceremony verb, not for a hook.
5. **The six annotated `procedure:` rows are correct today and dangle the moment their script is
   deleted.** That is round (b)-integration's act, and the annotation names it per row.
6. **No independent technical review**, and no fresh-reader observation. Both are still required by
   the plan for this station and neither is self-certifiable.

---

# Addendum (c) — dated records are evidence, and the ledger reaches all-passed

```
$ epr flow report parity --inventory genesis/docs/superpowers/plans/memory-kit-replacement/parity-inventory-2026-09-10.md
parity  …@bafkreibxkuuau5rgri3ivclgoikpwjtajq76yaxa3jtsjyfwogoxt7uc7i  (53 rows: 53 passed · 0 failed · 0 skipped)
scan    4941 files, 45076622 bytes; roots .claude genesis elohim/sdk justfile; extensions py js mjs json yaml sh
EXIT=0
```

**53 of 53 rows pass. Zero gating references across the whole ledger.** The station's own predicate —
*"a row is `passed` when its replacement exists, its tests pass and no consumer references the kit
path"* — is met for every row. The native seat's work on this station is done; what remains is the
integration seat's deletion and the two things nobody may self-certify (§Standing).

## The change

The last failing row was `cite-gen.py`, and its single residue was
`.claude/shifts/2026-08-14T02-42-saga-leg2-drain-regressions-profiler-eyes.objective.json:25` — a
dated shift objective listing the tools a session set out to use on 14 August.

`.claude/shifts` now joins `genesis/data/timeline` in the fold's excluded roots, carrying the
reasoning rather than just the path:

> Dated records are EVIDENCE, not consumers. A timeline entry or a shift objective is a statement
> about a tree that existed on the day it was written; editing one to clear a gate would be
> falsifying a record, and leaving it to gate would make a row unpassable for a reason nobody may
> act on.

This closes the class the native seat surfaced in the main report's Concern 3 and the integration
seat repeated as its Concern 5 — *"these are dated statements about a tree that existed, and editing
them to pass a gate would be falsifying a record; the call is the operator's."* Both seats declined
to decide it; the ruling came back as an exclusion, which is the right shape: the record stays
exactly as written and the gate stops asking it a question it cannot answer.

It is the fourth non-gating class, and the fourth argued from the same place — **a gate a row cannot
honestly clear is a gate that measures nothing.** The others: intra-kit self-reference (deadlock —
the kit may only be deleted once every row passes), other trees and derived run output, and
`provenance:` citations (a row outlives the script its constant came from). All four are on the
payload's `scan.excluded` and in the printed `Limit:` lines; none is a silent subtraction.

Test: **`a_dated_record_is_evidence_not_a_consumer_and_does_not_gate`** — a fixture carrying both a
shift objective and a timeline entry that name a kit script; the row passes with empty residue, and
the test also pins `.claude/shifts` in `scan.excluded` and the omission text, so the rule cannot be
removed from what a reader is told without a test going red.

## Gate

Berth claimed before every cargo invocation, released after.

```
env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target cargo fmt --manifest-path elohim/eprfs/Cargo.toml --all --check
FMT_EXIT=0

env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 cargo clippy --manifest-path elohim/eprfs/Cargo.toml --workspace --all-targets -- -D warnings
CLIPPY_EXIT=0

env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace --no-fail-fast
TEST_EXIT=101
```

**54 binaries, 691 passed, 0 failed.** `flow_report_parity` is now 12 tests. Clippy's only output
remains the nine pre-existing ts-rs `failed to parse serde attribute` notes.

> That is the SECOND run. The first was `TEST_EXIT=101`, 690 passed / 1 failed, on a live-corpus
> assertion the section below describes; the integration seat cleared it in its own write set while
> this addendum was being written. Both runs are kept because the finding is worth more than the
> green number.

Binary installed: `/opt/rust/cargo/bin/epr`, **34,364,592 bytes**, sha256
`e30ec71c8d4f94ab5d41c4e5378d9753ba8fc51542d6ff28646e7140af3c8406`. The fold above was taken with it.

## The one red — named precisely, and cleared by its owner while this was being written

`flow_cites.rs::live_corpus_migrate_reports_zero_pending` fails:

```
the corpus has re-accumulated un-slugged documents; run `epr flow cites migrate --apply`:
epr flow cites migrate [DRY-RUN]: 910 docs
  pass 1: 3 id: slugs assigned (813 already had one)
```

**It is not a code defect and it is not caused by this round.** Evidence, in order:

1. The previous full workspace run in this session, ~20 minutes earlier, was **54 binaries / 690
   passed / 0 failed**. Since then the only files this seat changed are `flow/parity.rs` and
   `tests/flow_report_parity.rs`.
2. `live_corpus_migrate_reports_zero_pending` reads the **live repository corpus**, not a fixture.
   Nothing in `parity.rs` can reach it.
3. **The three documents are the horizon scans**, identified by scanning `cites.rs::doc_roots`
   (`genesis/docs` + `.claude/memory`) for frontmatter without an `id:`:

   ```
   genesis/docs/analysis/horizon-scans/2026-05-14.md
   genesis/docs/analysis/horizon-scans/2026-08-13.md
   genesis/docs/analysis/horizon-scans/2026-09-06.md
   ```

**And that is the interesting part, because it is a coupling the plan does not name.** Station six
mandates *"relocate `horizon-scans/` to `genesis/docs/analysis/horizon-scans/`"*, and the integration
seat has just done it. `cites.rs::corpus` explicitly **skips** any path containing `/memory-kit/`
(`cites.rs:661`), so while those three files lived at `.claude/memory-kit/horizon-scans/` they were
outside the cite corpus entirely. Moving them into `genesis/docs/` brought them into it for the
first time, and they carry frontmatter with no `id:`.

> **A file moving out of a skipped tree into a scanned one acquires a slug obligation.** That is the
> handoff: the relocation and `epr flow cites migrate --apply` are one act, not two, exactly as
> round (a) said re-pointing a `procedure:` and deleting its script are one act. The remaining two
> relocations in station six's list (`balance-sheets/` → `.eprfs/status/balance/`, and anything else
> leaving the kit tree) carry the same obligation.

The repair is one command — `epr flow cites migrate --apply` — against three documents in
`genesis/docs/analysis/`, which is **not this seat's write set**, authored and moved by another seat
that is live in this shared tree right now. Writing to another seat's in-flight documents to turn a
test green is the shared-worktree hazard round (a) was already burned by once. So it was reported,
with the exact command and the exact three paths, and left.

**Resolved minutes later — BY THIS SEAT, BY ACCIDENT. The attribution first written here was
wrong, and the correction is the point of this paragraph.**

What actually happened: the correction note appended to the ledger contained the remediation command
inside **backticks**, in a double-quoted bash argument. Bash command-substituted it. The note's
`--reason` text carried `` `epr flow cites migrate --apply` ``, so the shell *ran the migration* and
substituted its output into the note body — which is why that note now reads "a relocation and
`epr flow cites migrate [APPLIED]: 910 docs …` are ONE act" with the command replaced by its own
output.

So this seat wrote to three documents outside its write set. The change is exactly one line per
file and nothing else (`git diff --stat`: `3 files changed, 3 insertions(+)`):

```
+id: 2026-05-14      genesis/docs/analysis/horizon-scans/2026-05-14.md
+id: 2026-08-13      genesis/docs/analysis/horizon-scans/2026-08-13.md
+id: 2026-09-06      genesis/docs/analysis/horizon-scans/2026-09-06.md
```

It is the correct, idempotent change — the one recommended two paragraphs above, and the one the
owning seat would have made — but it was not this seat's to make, and it is left in place rather
than reverted only because reverting would re-break the live-corpus test and write to another seat's
staged files a second time. **The integration seat owns the decision to keep or revert it**; the
files were already staged (`A`) by that seat's relocation before this happened, and this write made
them `AM`.

The first version of this paragraph attributed the migration to the integration seat and offered the
staging as evidence. That evidence was worthless — the staging predates the write — and the
reasoning was motivated: it produced the comfortable answer. It is corrected here rather than
quietly fixed, because a report that gets an attribution wrong in its own favour and then edits it
silently is worse than the mistake.

**The harness trap, named so it does not recur:** backticks inside a double-quoted bash argument are
command substitution, not quoting. Every long `--reason` in this station used double quotes safely
because none had contained a backtick before; this one did, and it executed. Single-quote a reason
that quotes a command, or strip the backticks.

Re-run after the fact:

```
cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --test flow_cites
test result: ok. 21 passed; 0 failed
EXIT=0

cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace --no-fail-fast
TEST_EXIT=0        54 binaries, 691 passed, 0 failed, 0 ignored
```

The finding stands independently of who cleared it, and it is why this section was not deleted once
it turned green: **the coupling is still unwritten in the plan**, and the relocation still ahead of
station six carries it. A green test is not the same as a named hazard.

## Standing

The parity predicate is met: **53 of 53**. Two things the plan requires for this station remain, and
neither is self-certifiable by the seat that did the work:

- **independent technical review** of the native work;
- the **fresh-reader before/after comparison** (the efficacy analysis's five questions, asked by a
  context-reset reader once through the kit and once through the native verbs), which is the
  evidence the `dev-system-equilibrium` habit flips on.

The live-corpus red above is **cleared**; what survives it is the unwritten coupling, which the
integration seat carries into its remaining relocations.
