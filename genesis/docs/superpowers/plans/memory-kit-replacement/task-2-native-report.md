---
id: memory-kit-replacement-task-2-native-report
status: DONE_WITH_CONCERNS
cites: []
gap: plans__2026-09-10-memory-kit-replacement-finish#2
actor: agent:implementer@claude-opus-5
commits: []
---

# Station two (native seat) — placement, station extraction and env scope are native

Document placement, gap extraction and substrate scope are derived from the tree rather than read
out of `.claude/memory-kit/`. Three verbs landed — `epr flow report placement`,
`epr flow report scope`, `epr flow hold --scope` — plus checkbox-station extraction inside
`epr flow project`. Nothing in the kit was deleted: station two proves parity, station six deletes.

Status is `DONE_WITH_CONCERNS` for two reasons, both stated in full under **Concerns**: one
deliberate semantic widening the station could not avoid (the gap roll-up is now a superset of the
decomposed subset), and one cross-station defect I observed but do not own (the station-one review's
`cleanup-pressure` producer starvation, still open in the tree).

## What landed

| Path | Role |
|---|---|
| `elohim/eprfs/epr-cli/src/flow/cluster_state.rs` | NEW. The ONE native `cluster-state.yaml` parser — a transcription of `_lib/cluster_state.py` including all three of its semantic decisions (column-0 comments are not block terminators; `available:` must be declared to be a claim; repeated declarations merge per field, `available` true-wins). |
| `elohim/eprfs/epr-cli/src/flow/env_scope.rs` | NEW. The gap-granular scope resolver — `_lib/env_scope.py` ported: `requires_env` normalization across all three authored shapes, `@requires:` tag parse/strip, gap-level inheritance, `gap_blocked`, `@act:` reading and the additive act-baseline union. |
| `elohim/eprfs/epr-cli/src/flow/gaps.rs` | NEW. `decompose.py`'s `checkbox-tasks` method, with its `requirement-bullets` fallback and its `Method::None` honest absence. Regex-free by construction. |
| `elohim/eprfs/epr-cli/src/flow/placement.rs` | NEW. `epr flow report placement [--ledger] [--focus [<subject>]] [--json] [--cluster-state PATH]` — the eleven-state placement vocabulary, the BALANCE and PRESSURE QUEUE renderings, the station roll-up. |
| `elohim/eprfs/epr-cli/src/flow/scope.rs` | NEW. `epr flow report scope` + `epr flow hold --scope [--apply] [--set] [--env]` + the focus baseline. The mover, the reader and the developer flip. |
| `elohim/eprfs/epr-cli/src/flow/project.rs` | Station extraction from the documents themselves; the `gap-items/` reader demoted to a fallback; both arms route through one `mint_station`. |
| `elohim/eprfs/epr-cli/src/flow/report.rs` | The `scope:` headline slot reads the native derivation; `scopePendingMoves` on the payload. |
| `elohim/eprfs/epr-cli/src/flow/mod.rs` | CLI shell: `report placement`, `report scope`, `hold --scope`, the `scope-pending-moves@1` fold at the CLI edge, usage. |
| `elohim/eprfs/epr-cli/tests/{flow_placement,flow_scope,flow_gap_parity}.rs` | NEW. 26 integration tests; 3 added to `flow_report.rs`. |
| `elohim/eprfs/epr-cli/tests/fixtures/gap-parity/` | NEW. Three plan docs + their three recorded `gap-items` JSON, copied 2026-09-10. |
| `elohim/eprfs/epr-cli/seam-registry.yaml` | Four decision points registered; the honesty clause's first pending item discharged. |
| `.claude/hooks/delivery-gate.py` | Reads the over-claim count from `epr flow report placement --json`, kit headline as fallback. |

### 1. `epr flow report placement [--ledger] [--focus [<subject>]] [--json]`

The full state vocabulary is ported: `ACTIVE`, `LINKED`, `MEM-UNLINKED`, `NEEDS-TRIAGE`,
`CLAIMED-ONLY`, `SETTLED`, `SUPERSEDED`, `VERIFIED-STABLE`, `UNKNOWN-STATUS` — **plus** `REGRESSED`
and `BLOCKED-BY-ENV`. The brief named nine; the kit's `budget_state` produces eleven, and the two
extra are the ones derived from PLACEMENT rather than from status. Dropping them would have
re-bucketed every pressure-dir doc and every env-held doc into a state they are not in, so all
eleven are ported and the extra two are documented as such in the module header.

Inputs are the same inputs the flow plane already labels: the seven doc surfaces plus the
`genesis/docs/_state/*` pressure dirs, frontmatter `status:` with the `**Status:**` markdown line as
fallback, `verified_by`/`landed_commit`, `.claude/memory` `cites:` link state, and
`cluster-state.yaml`. **No state file is written**: the kit's `state-ledger.json` was a rendering of
the tree, and this report is that rendering taken live.

**Where the kit read a memory-kit JSON file, the fold is read from the tree, and where there is no
fold the answer is `unknown`.** The one such read in `--ledger` is the DECOMPOSED GAPS section,
which enumerated `gap-items/*.json`. Natively the stations come from the documents (deliverable 2),
and the roll-up carries a `known` flag: a corpus in which no document yielded an extractable station
prints

```
  GAPS: unknown — no document in the 481 scanned yielded an extractable station
```

rather than `0 OPEN`. Those are opposite facts and the kit rendered them identically. Pinned by
`an_unmeasured_corpus_reports_unknown_rather_than_zero`.

`--focus` renders `focus-baseline.py`'s readout — `--focus` alone for the whole subject baseline,
`--focus <subject>` for one, matching the kit's bare invocation and its `--subject`.

**Live parity, BALANCE and PRESSURE QUEUE, byte-for-byte** (2026-09-10, HEAD `69afad49e`):

```
BUDGET LEDGER — every file accounted (position + state)   total files: 709
  BALANCE (sums to total — nothing hides):
    · ACTIVE            332  (46%)      ▶ MEM-UNLINKED      199  (28%)
    ▶ UNKNOWN-STATUS     62  ( 8%)      ▶ SETTLED            34  ( 4%)
    · LINKED             29  ( 4%)      ▶ NEEDS-TRIAGE       28  ( 3%)
    ▶ CLAIMED-ONLY       13  ( 1%)      ▶ SUPERSEDED          6  ( 0%)
    · VERIFIED-STABLE     6  ( 0%)
    PRESSURE (needs action): 308   HELD (not pressure): 0   SETTLED: 401
```

Identical to `placement-audit.py --ledger` in counts, sort order, marks, percentages, queue order
and the four sample rows under each queue state.

The `--focus` rendering diffs against `focus-baseline.py` in **exactly two lines**, both of which are
the command name the baseline tells a reader to run (`scope-reconcile.py --set …` →
`epr flow hold --scope --set …`, and the source-of-truth header). Verified by a normalized diff that
substitutes only those two tokens and returns empty.

**Parity test.** `tests/flow_placement.rs` builds a synthetic ~30-document tempdir corpus, one
document per arm of `budget_state`, and asserts twice over it: against `EXPECTED_BALANCE` pinned
constants (so the test is not vacuous without Python, and so it survives station six), and against
`placement-audit.py --ledger --json <tmpdir>` when the interpreter and the script are both present.
The oracle leg compares the histogram AND the row-for-row reading (path, position, state, in order),
and skips with a printed reason rather than failing once the script is gone. The corpus is synthetic
rather than the live tree deliberately: the live corpus changes hourly (a test asserting counts
against it would be measuring the repository's editing cadence), and it does not currently exhibit
`REGRESSED`, `BLOCKED-BY-ENV` or an occupied pressure dir — the three arms a port is most likely to
lose quietly.

### 2. Checkbox-station extraction inside `epr flow project`

`decompose.py`'s `checkbox-tasks` method is ported to `flow/gaps.rs`, including the parts that look
like bugs and are not:

- **Heading-agnostic.** Every checkbox in the document is a station, wherever it sits. `## Delivery
  stations` is a convention this repository's plans follow, never a parser input — a plan naming its
  stations under a different heading still decomposes, and a plan that grows a second station list
  does not silently lose half of it. Pinned by
  `the_delivery_stations_heading_is_a_convention_not_a_parser_input`.
- **The requirement-bullet fallback's word boundaries are narrower than they look.**
  `\brequirement\b` does NOT match "Requirements", nor `\btask\b` "Tasks" — the trailing `s` is a
  word character. Verified against Python before transcribing. That is the kit's reading and it is
  ported rather than corrected: this arm only fires for docs with no checkboxes, and "widening" it
  would make native gap ids diverge from every id already minted under the kit's spelling.
- **The meta filter runs BEFORE the `@requires:` tag is stripped.** Order is load-bearing and
  matched.
- **`Method::None`** is a distinct third answer, so "no remaining work" and "nobody decomposed this"
  stay apart.

No regex crate was added. Every matcher is a hand-written scan against the exact Python pattern
quoted above it — adding a dependency to the resolution path of a tool the SessionStart hooks shell
out to, for six patterns, was not worth the risk in a container whose registry reachability is not
guaranteed.

**Parity test.** `tests/flow_gap_parity.rs` pins the three named plans. BOTH halves are fixtures
copied on 2026-09-10 into `tests/fixtures/gap-parity/`: the recorded `gap-items` JSON (because
station six deletes the directory it came from, after which this file is the only surviving witness
of what `decompose.py` said) AND the plan bytes that produced it. Copying the docs too is the point
the brief did not have to make: reading them live would mean an ordinary plan edit — ticking this
very station's box — reds the eprfs gate while proving nothing about the extractor. Pinning both
halves makes it a test of the algorithm, which is what parity means here.

Result: native ids, states, counts and extraction method are equal to the recorded output for all
three plans (3, 4 and 7 stations respectively).

**The projector.** `derive_doc_stations` extracts from the `spec`, `plan` and `architecture-seed`
stages — the three doc classes the cache actually covered. `manifesto`, `epic` and `scenario` are
excluded deliberately: a manifesto's checkboxes are prose, and a `.feature` is one artifact rather
than a station list. Both the native arm and the cache arm route through one `mint_station`, so the
atoms are byte-identical (`gap:<state>` classification, `tool:decompose` / `tool:decompose-claim`
providers, same id spelling) and a station already projected from the cache dedupes against its
native twin rather than doubling.

**The fallback rule is "the document did not speak", not "the document is unreadable".** The brief
said unreadable; implementing that literally deleted work from the ledger and reds 17 existing
`flow_edges` tests, which is how the gap was found. A prose spec an AGENT decomposed by hand (the
memory-stasis-loop's `capture` step writes those) has stations that exist nowhere in its own bytes —
`decompose.py` records `method: none` for it and a human supplied the items. Unreadable is a subset
of "did not speak", so the brief's rule still holds; the wider rule is what keeps hand-decomposed
prose specs in the ledger. Pinned by the existing `flow_edges.rs` valueflow fixture, whose
`plans/epic.md` is prose and whose stations therefore still come from `gap-items/epic.json`.

**`delivery-gate.py`** now reads its over-claim count from `epr flow report placement --json`,
counting `CLAIMED-ONLY` rows in an ACTIVE home — keeping the kit's load-bearing ACTIVE-home filter
(CANONICAL and HISTORY are the settled destinations; counting them would point the conveyor at
documents already at rest). The kit headline regex remains the fallback while the binary is absent.
It never read `gap-items/` and still does not — the docstring now says so and says why (raw station
totals run to thousands across terminal and historical specs and read as noise).

### 3. `epr flow report scope` and `epr flow hold --scope`

`scope-reconcile.py`'s whole surface is ported: the four live/held zones, the gap-granular verdict,
the act-baseline union for `.feature` files, the unknown-cap asymmetry, the held-anomaly rule, the
deployments.json derivation AND its line-editing writer, the STOP markers, orphan cleanup, the
subject-focus refresh, `--set`, `--env` and the dry-run default.

Three asymmetries carried with their reasons, because each has cost real work:

- **Gap-granular, not doc-granular.** A doc is held whole only when EVERY station is blocked. That is
  "iroh ≠ shem" operationally.
- **held → live requires affirmative evidence.** A held doc with no parseable scope info STAYS held
  and is reported for triage. Auto-publishing on the absence of a reason is how a doc escapes into a
  runner that cannot run it.
- **An unknown capability blocks on a `.md` and is ignored on a `.feature`** — a2o's `@requires:`
  namespace mixes hardware capabilities with fixture preconditions.

**Live parity** against `scope-reconcile.py` on the current `cluster-state.yaml`:

```
native:  scope: ⚠ 3 to hold (local-conductor,owned-substrate)  →  epr flow hold --scope --apply
kit:     scope: ⚠ 3 to hold (local-conductor,owned-substrate)  →  scope-reconcile.py --apply
```

and the full derivation matches item for item: the same 3 to-hold paths, 0 to-live, the same 5
held-anomaly paths, 0 vocab warnings, 0 actionable deployment flags and the same 7 `manual-hold`
names.

**The next-action pointer.** The brief asked for the pointer the native headline had lost, and for
the native form once it exists. It exists, so the native line names `epr flow hold --scope --apply`.
Everything before the `→` is byte-identical to the kit's, which is what the parity test asserts —
`headline_parts()` is split out for exactly this reason, since the pointer is the one piece that
legitimately differs (the kit names its own script; the whole point of the native verb is that it
names a different one).

**One honest correction to the brief.** It lists `⚠ N ready to expand (cap)` among the forms the kit
prints. The kit does not print it: `scope-reconcile.py:report()` emits `to return to plate` for that
event, with a comment saying why — "ready to expand" would imply the capability returned, and a
mixed doc returning to the plate still has blocked capabilities. `CLAUDE.md` and `_observation.py`'s
`_SCOPE_PARTS` both still carry the older spelling. The native line emits the kit's current wording;
`ready to expand` is a stale gospel spelling of the same event, not a fifth form. Flagged rather than
silently implemented — it belongs in the integration seat's CLAUDE.md pass.

**The fold.** `epr flow report scope` and `epr flow report --headline` both append the
`scope-pending-moves@1` observation (subject `.`, unit `count`, `env:head=<short HEAD>`), which is
the producer the station-one bridge was standing in for. Verified live: one appended record reading
`value:3 unit:count env:head=69afad49e`. Keyed by HEAD for the reason the bridge keyed by it — the
fold's identity is its content, so two runs at one commit mint ONE atom while the same count
returning at a later commit is a new measurement. Without the key a value that went 3 → 0 → 3 would
fold its third reading onto its first and the report, which takes the LATEST admissible fold, would
answer 0. `--cluster-state` runs are deliberately NOT folded: a measurement under an overridden
manifest answers a different question than the bound asks.

`_observation.py`'s `parse_headline` can now drop its scope arm — it is `.claude/hooks/**`, the
integration seat's write set, so it is named here rather than edited.

**The `scope:` headline slot reads this report's line.** Scope is the one headline slot whose fact is
not a magnitude: "3 to hold (local-conductor,owned-substrate)" names a direction and two
capabilities, and a watermark over a count can say neither, while the root `CLAUDE.md` declares those
exact spellings as session triggers. So the slot prints the derivation, and the declared
`scope-pending-moves-ceiling@1` bound still evaluates and appears in `--json` and the full render — a
watermark over the same number, in the plane where watermarks live.

The derivation is only taken when `genesis/manifests/cluster-state.yaml` EXISTS. Without it,
`aligned ✅` would be a false all-clear: a tree with no manifest has not been found to match the
substrate, it has not been asked. In that case the slot falls back to the bound plane, which is what
keeps station one's `a_headline_slot_with_no_declared_bound_says_so_rather_than_vanishing` and its
value-parity test green — both of which this change reds without the manifest gate, and both of which
are now pinned from the new side too.

Live SessionStart headline through the real hook, no `(fallback: memory-kit)` line:

```
  memkit: ⚠ 8.3 megabytes is past the soft watermark 8
  mempalace: ⚠ failed — 3 files has reached the hard watermark 1
  cleanup: ⚠ failed — 250 pressure-points has reached the hard watermark 120
  scope: ⚠ 3 to hold (local-conductor,owned-substrate)  →  epr flow hold --scope --apply
  memory-budget: ⚠ 23772 bytes is past the soft watermark 20000 (hard 24000)
  recipe: default@bafkreib…ptke
```

## Gate evidence

Cargo berth claimed before the first cargo invocation (`cargo: claimed by mk-replace-native-2`) and
released after the last (`cargo: released by mk-replace-native-2`). Every gate run under
`env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1`,
output redirected to a file rather than piped (a pipe masks cargo's exit code), `EXIT=$?` echoed on
its own line.

```
cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace
EXIT=0
   → 493 passed · 0 failed · 0 ignored, across 44 suites

cargo fmt --manifest-path elohim/eprfs/Cargo.toml --all -- --check
EXIT=0

cargo clippy --manifest-path elohim/eprfs/Cargo.toml --workspace --all-targets -- -D warnings
EXIT=0

EPR_BIN=/tmp/eprfs-gate-target/debug/epr python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
EXIT=0
   → Ran 24 tests, OK

python3 .claude/scripts/epr-meta-pin.py --verify
EXIT=0
   → 19 pinned row(s) verified clean (19 total); 15 pinned row(s) verified clean (15 total)
```

Binary digest: `sha256:158ae94b3ebffa57f61ceac023b029c6cc83d4caa9ab8ed16eefbd7ed20acfc7`
(`/tmp/eprfs-gate-target/debug/epr`) — the binary every live-parity figure above was produced with.

Two clippy findings were mine and were fixed (`manual_contains` in `gaps.rs`, `manual_checked_ops`
in `placement.rs`). None were pre-existing or operator-owned. `cargo fmt --all` reformatted only
files in this station's write set; the diff-site list was checked against `git status` before
applying.

Test counts added: `flow_placement.rs` 9, `flow_scope.rs` 12, `flow_gap_parity.rs` 5,
`flow_report.rs` +3, plus 24 unit tests inside the five new modules.

`.claude/epr-meta/*.yaml` was not touched — `scope-pending-moves@1` and
`scope-pending-moves-ceiling@1` were already declared by station one, so no row needed adding and
`epr-meta-pin.py --verify` is clean.

Disk held at 85% throughout (`/dev/md2p1 … 133G avail`); the 88% ceiling was never approached, so
`cargo-pool enforce` was not run.

No commits. No pushes. No kit script deleted, and `.claude/memory-kit/gap-items/` is untouched.

## Concerns

**1 — The station roll-up is a superset of the decomposed subset, by design, and the numbers move.**
`placement-audit.py --ledger` reports `238 docs with items … 4461 OPEN / 672 CLAIMED` by enumerating
248 cache files. The native roll-up reports `240 docs with stations, 481 scanned … 4861 OPEN / 331
CLAIMED / 1 BLOCKED-BY-ENV` by decomposing every document in the corpus. The two are not comparable
and the native section is labelled `GAPS`, not `DECOMPOSED GAPS`, to say so. Three causes, all
intended: docs that were never decomposed now contribute; the `CLAIMED` drop is stale cache entries
whose boxes were since unticked or whose text changed; and `BLOCKED-BY-ENV` is now computed per
station rather than per cache record. This is the direction the plan asks for — the document is the
source — but it is a real semantic change to a number a reader may have been tracking, and it is
stated rather than buried. The BALANCE and PRESSURE QUEUE counts, which the brief made the parity
target, are unchanged and byte-identical.

**2 — Not mine, still open: the `cleanup-pressure` producer starvation from the station-one review.**
While working I read a `run:correction`/verdict note on
`plans__2026-09-10-memory-kit-replacement-finish#1` recording a `changes-requested` verdict: the
rewired drift-signal hooks `return 0` on a successful `emit()` before writing their JSON, and
`cleanup-pressure.py` computes its number as the cardinality of exactly those JSON collections — so
the `cleanup:` headline number can no longer rise. The live headline still reads 250 only because the
snapshots predate the binary install. I did not touch it: `.claude/hooks/*drift-signal*.py` and
`.claude/scripts/memory-kit/**` are outside this station's write set, and the fix named in the verdict
(keep the JSON write alongside the fold, or aggregate the folds natively for `cleanup-pressure@1`) is
station one's. Recorded here so it is not lost between two seats' reports.

**3 — `epr flow hold --scope --apply` has not been exercised against the live tree.** It is tested
end-to-end in tempdirs (moves performed, STOP written, `subject-focus.md` refreshed, dry run proven
inert) but a test suite has no business reconciling anybody's plate, and the sprint forbids commits.
The three real moves the report proposes are therefore still proposals. The deployments.json
line-editing writer in particular is ported and unit-covered on its derivation, but its WRITE path
has never run — there is no actionable deployment drift in the tree today (7 `manual-hold`, 0
actionable), so no fixture-free exercise was available. Flagged as the highest-risk untried path in
this station.

**4 — `--focus <subject>` on a fair-game subject prints the fair-game line, not a refusal.** Matching
`focus-baseline.py`, which does the same. A caller who names a nonexistent subject gets
`### <name> — unknown subject (fair-game if not listed as narrowed)`. Transcribed rather than
improved; noted because it reads as a successful answer about a subject that does not exist.

## Residue — every item has a home

| Item | Home |
|---|---|
| `_observation.py`'s `parse_headline` scope arm can now be dropped (the native producer folds `scope-pending-moves@1`) | Integration seat, `.claude/hooks/**` — named in §3 above |
| `CLAUDE.md` and `_SCOPE_PARTS` carry `ready to expand`, which the kit no longer prints | Integration seat's CLAUDE.md pass; stale spelling, not a fifth form |
| `cleanup-pressure` producer starvation | Station one, open — Concern 2 |
| The three proposed live→held moves | Operator decision (`epr flow hold --scope --apply`); no commits this sprint |
| `placement-audit.py`, `decompose.py`, `scope-reconcile.py`, `focus-baseline.py`, `state-machine-gen.py`, `gap-items/` | Station six deletes; this station proves the replacement and touched none of them |
| `state-machine-gen.py` | Owner-routing table marks it RETIRE; nothing native replaces it and nothing needs to — the pressure-dir HOME semantics it generated are read directly by `budget_state`, pinned by `the_pressure_dir_home_outranks_the_frontmatter` |

## Review pointer

`elohim/eprfs/epr-cli/.epr-meta`'s `reconciliation-fitness-review` rule routes `gaps.rs` and
`scope.rs` to independent technical review, since both change remaining-work interpretation. Four
decision points are registered in `elohim/eprfs/epr-cli/seam-registry.yaml` (`gaps::decompose`,
`project::derive_doc_stations / project::derive_gap_items`, `placement::budget_state`,
`scope::scope_verdict`) with concern bindings and contract tests, and the registry's honesty clause
is updated: its first pending item, `derive_gap_items`' CLAIMED predicate, is now registered.
Observed limitations are Concerns 1-4 above. Tests alone do not establish fitness here — the two
questions a reviewer should press are Concern 1 (is the roll-up widening the right trade?) and
Concern 3 (the unexercised write path).

---

# Round two — review response (2026-09-10)

The station-two verdict returned `changes-requested` with four Importants and seven Minors. All
eleven are fixed in this seat's write set, each with a pinning test. Three of the four Importants
were empirically demonstrated defects in newly-authored code, and all three reproduced exactly as
described. The fourth was an overstatement in this report's own residue table, which is corrected
below with measured numbers.

Binary digest for every figure in this section:
`sha256:d5a5f0a6d16db96321b2850e9ba75eac3187b6f78839dc299aeac77554e8b9ca`.

## Important 1 — stations are not scope information

**Fixed** in `scope::scope_verdict`. The station path is now taken only when there is scope
information to resolve: a doc-level `requires_env`, or at least one station carrying an
`@requires:` tag. Otherwise the function falls through to the doc-level arm, which with no
declared capability returns `Ambiguous`.

The defect was exactly as reported and the reasoning is worth keeping. Every station of a document
that says nothing about scope is trivially "not blocked", so `satisfiable` was set on the first
station and the doc read `Live` — meaning a held `.md` would be proposed back onto the plate for the
sole reason that it has checkboxes, and the `Ambiguous` arm was unreachable for any doc with
stations. The kit never had this shape because it reaches its station path only when a decompose
RECORD exists; deriving stations from every document is what exposed it. Ambiguity and
station-derivation are separable, and the fix is that separation rather than a special case.

Reproduced before and after on the exact shape the reviewer names — a held `.md` with two unticked
stations, no `requires_env`, no tags:

```
before: toLive ['…/held/plans/mute.md']   heldAnomalies []            pendingMoves 1
after:  toLive []                          heldAnomalies ['…/mute.md'] pendingMoves 0
```

Pinned by `stations_are_not_scope_information` (both the held and the live side) and by
`one_tagged_station_is_enough_scope_information_to_decide`, which pins the boundary from the other
direction so the fix cannot be over-applied.

## Important 2 — an empty evidence list is absence

**Fixed** in `placement.rs` with a `declares_value()` helper that normalizes through the same inline
list reader `requires_env` uses, so `[]`, `[ ]` and `[,]` are all empty — the check is on the parsed
items, never on the literal spelling. Applied to `verified_by`, `landed_commit` and `cites`,
replacing the ad-hoc `!= "[]"` guard the cites reader carried.

Verified against the kit on a purpose-built two-doc corpus, and extended to the spacing variants the
`!= "[]"` guard would have missed:

```
empty-evidence.md (verified_by: [])   native CLAIMED-ONLY   kit CLAIMED-ONLY
real-evidence.md  (verified_by: [ci]) native VERIFIED-STABLE kit VERIFIED-STABLE
```

Pinned by `an_empty_evidence_list_is_absence_not_evidence` (four spellings) and
`an_empty_cites_list_leaves_a_memory_entry_unlinked`.

## Important 3 — the `\s*$` on a resource key, and a bare field

**Fixed** in `cluster_state.rs`. `resource_key` now trims trailing whitespace before taking the
colon suffix — and trims the LINE rather than the name, so `  shem :` is still not a key, matching
the kit's requirement that the colon sit immediately after the name. `field()` now refuses an empty
value, so a bare `    available:` sets no `declared_available` and a bare `    role:` names no role,
matching `(\S+)` and `(.+?)`.

The failure mode reproduced exactly: the lost key's `available: true` merged into the PRECEDING
block under true-wins, handing `local-conductor` an availability it never declared. This is the
inverse of the harm decision 1 warns about, so the comment now names the incident.

```
`  shem: ` (one trailing space), native, before: available ['local-conductor', 'shem'-lost]
                                        after: available ['shem']
```

Pinned by three parser tests (`a_key_line_tolerates_trailing_whitespace`,
`a_space_before_the_colon_is_not_a_key`, `a_bare_field_declares_nothing`) and by
`a_trailing_space_on_a_resource_key_does_not_move_a_capability`, which exercises it through the
whole scope reading rather than only the parser — because the consequence that matters is the
changed move set `--apply` would act on.

## Important 4 — the fallback store is RELOCATED, not deleted

The residue table asserted the opposite of the truth and is corrected. **Measured on the live tree,
2026-09-10**, over 248 files in `.claude/memory-kit/gap-items/` (241 with stations):

| Class | Files | Stations |
|---|---:|---:|
| Agent-supplied method, document still present (`agent-decomposition` 40, `agent` 15, `agent-extracted` 5, `delivery-stations` 1) | **61** | **649** |
| Any method, document still present | 187 | 3,850 |
| Orphans — document gone | 55 | 1,378 |

The 61/649 exist nowhere in their documents' own bytes and survive only through this station's "the
document did not speak" fallback. Deleting the directory would delete them silently.

**Three changes.** (1) The fallback store's home is now `.eprfs/status/gap-items/` — under this
crate's own durable records, because a store the projector depends on cannot live inside a directory
another station is going to delete. (2) `fallback_gap_files()` reads the native home FIRST and the
recipe's `intent` stage second, one entry per NAME: after adoption the kit copy is inert while still
on disk, so station six removes a directory nothing reads. (3) `epr flow project --adopt-gap-items
[<dir>]` copies every record whose document still exists into the new home byte-identically,
recording each file's raw CID — the copy is byte-for-byte, so the recorded CID addresses source and
copy at once and the adoption is verifiable rather than asserted.

Run on the live tree:

```
epr flow project --adopt-gap-items → .eprfs/status/gap-items
  scanned 248 · adopted 193 files / 3850 stations · orphans 55 (doc gone, NOT adopted) · unreadable 0
second run: adopted 0 · already present 193        (idempotent)
diff -r .claude/memory-kit/gap-items .eprfs/status/gap-items → identical wherever present
```

Orphans are deliberately NOT adopted: their stations describe work on a document that no longer
exists, and carrying them forward would keep a ledger of promises about nothing. They are reported
by name so the deletion station accounts for them rather than absorbing them.

**Station six's instruction is therefore RELOCATE, then delete**: run `--adopt-gap-items`, confirm
193/3850 adopted and 55 orphans reported, then remove `.claude/memory-kit/gap-items/`. The residue
table below is corrected accordingly.

## The seven Minors

| # | Finding | Fix | Pinned by |
|---|---|---|---|
| (a) | A blank line or `#` comment inside a frontmatter block list truncated it | `parse_frontmatter` skips both without closing the list, as the kit and YAML do | `a_block_list_survives_a_blank_line_or_a_comment_inside_it` |
| (b) | Silent zeros: deployments.json IO/parse errors, and non-UTF-8 docs dropped from both scans | Absent stays absence; unreadable/unparseable now says so on stderr naming the file and calls the 0 UNKNOWN; the apply arm REFUSES by name rather than printing APPLIED having written nothing; both scans read `errors="replace"` | `a_non_utf8_document_stays_in_the_ledger`, `a_non_utf8_document_stays_in_the_scan` |
| (c) | The dry run no longer surfaced live/held name collisions | `warn_conflicts()` runs before the apply guard, so the collision prints on the dry run; `git_mv` still refuses | `a_dry_run_surfaces_a_destination_collision` |
| (d) | `prune_empty` removed the held zone root — a delete Python would not make | `prune_empty_descendants()` prunes under the root only | `pruning_never_removes_the_held_zone_root` |
| (e) | The report claimed the meta/tag order "matched"; it matched the checkbox arm only | A `MetaOrder` enum, named at each call site: checkbox = META before strip, bullet = strip before META. The kit's Python was re-run to confirm the divergence is real before transcribing it | `the_two_arms_order_the_meta_filter_differently_because_the_kit_does` |
| (f) | `--set` wrote `    available:false` into a bare `available:` line the kit's pattern does not match | The value must be non-empty to be replaced; scanning continues, so a later real line in the block is still the one edited | `set_leaves_a_bare_available_line_alone` |
| (g) | `placement-audit.py --focus` and `focus-baseline.py --brief` had no port; `prep-brainstorm.py` consumes both | `epr flow report scope --docs` is the document reading (it lives on `report scope` because it is an environment question about documents); `--brief` lands on the subject baseline | `the_doc_scope_reading_splits_by_availability`, `the_brief_baseline_names_the_narrowed_subjects_and_points_at_the_drill_in` |

`--docs` was diffed against `placement-audit.py --focus` on the live tree: **identical except the two
drill-in pointer lines** (the command name), same 41 in-scope rows, same 0 blocked, same
AVAILABLE/UNAVAILABLE rendering including the `['cap']` list spelling.

## The NOTE-class items the verdict raised

- **`delivery-gate.py`'s over-claim count moves 12 → 13.** Now disclosed. The kit's headline computes
  `bucket == LANDED and not verified` while its own ledger's `CLAIMED-ONLY` also includes the
  `claimed-not-verified` status bucket — so the kit disagreed with itself by one document. The native
  count matches the ledger row-for-row; the moved number is a kit self-inconsistency resolved in the
  ledger's favour, not a native drift.
- **`--json` renames `total_files` → `totalFiles`.** Kept, for consistency with every other native
  payload, and now PINNED by `the_payload_key_is_camel_case_and_pinned` — which also asserts the four
  `rows[*]` keys have not moved, since that is what the one live consumer reads. The rename is a
  decision rather than something a consumer discovers.
- **`report scope` folds on a SessionStart read path.** Unchanged and re-verified: fail-open,
  suppressed under `--cluster-state`, idempotent at one HEAD.
- **The hooks suite** is green again at 27 tests — the integration seat's `_observation.py` fix landed
  during this round.

## Round-two gate evidence

Berth claimed before the first cargo invocation and released after the last. Same env, output
redirected rather than piped, `EXIT=$?` on its own line.

```
cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace
EXIT=0
   → 533 passed · 0 failed · 0 ignored, across 45 suites   (was 493/44)

cargo fmt --manifest-path elohim/eprfs/Cargo.toml --all -- --check
EXIT=0

cargo clippy --manifest-path elohim/eprfs/Cargo.toml --workspace --all-targets -- -D warnings
EXIT=0

EPR_BIN=/tmp/eprfs-gate-target/debug/epr python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
EXIT=0
   → Ran 27 tests, OK

python3 .claude/scripts/epr-meta-pin.py --verify
EXIT=0
   → 19 pinned row(s) verified clean (19 total); 15 pinned row(s) verified clean (15 total)
```

Binary digest `sha256:d5a5f0a6d16db96321b2850e9ba75eac3187b6f78839dc299aeac77554e8b9ca`. One clippy
finding was mine and was fixed (`unnecessary_get_then_check` in a new test). Disk held at 85%.

**Live parity re-verified after every fix** — none of the eleven changes moved a parity target:

- placement `--ledger` BALANCE + PRESSURE QUEUE: byte-identical to `placement-audit.py --ledger`;
- `report scope`: `⚠ 3 to hold (local-conductor,owned-substrate)`, identical to
  `scope-reconcile.py --report` up to the next-action pointer;
- `--focus`: identical to `focus-baseline.py` modulo the two command-name tokens;
- `report scope --docs`: identical to `placement-audit.py --focus` modulo the two pointer lines;
- SessionStart headline: native, no `(fallback: memory-kit)` line, `scope:` slot carrying the
  derivation and its pointer.

No commits. No pushes. No kit script deleted; `.claude/memory-kit/gap-items/` is intact and now
ALSO relocated.

## Residue table — corrected

| Item | Home |
|---|---|
| `.claude/memory-kit/gap-items/` | **RELOCATE then delete** at station six: run `epr flow project --adopt-gap-items` (193 files / 3,850 stations adopted, 55 orphans reported), then remove the directory. 61 docs / 649 stations are agent-decomposed and exist only in this store — deleting without adopting loses them |
| 55 orphan records / 1,378 stations whose document is gone | Station six decides: their documents no longer exist, so they are neither adopted nor silently carried. Named by `--adopt-gap-items`'s `orphans` list |
| `_observation.py`'s `parse_headline` scope arm | Integration seat — the native producer now folds `scope-pending-moves@1` |
| `CLAUDE.md` / `_SCOPE_PARTS` carry `ready to expand`, which the kit no longer prints | Integration seat's CLAUDE.md pass |
| `cleanup-pressure` producer starvation | Station one — resolved during round two; hooks suite green at 27 |
| The three proposed live→held moves | Operator decision (`epr flow hold --scope --apply`); no commits this sprint |
| `prep-brainstorm.py`'s two consumers | Ported: `report scope --docs` and `report placement --focus --brief`. Station six rewires the call sites |
| `placement-audit.py`, `decompose.py`, `scope-reconcile.py`, `focus-baseline.py`, `state-machine-gen.py` | Station six deletes; this station touched none of them |

Concern 3 from round one **stands unchanged**: `epr flow hold --scope --apply` has still never run
against the live tree, and the deployments.json write path is still unexercised. Important 3 and
Minor (f) both sat on that path, which is the strongest argument yet for exercising it under
observation before station six.

---

# Round three — review response (2026-09-10)

Four items: one Important the previous round did not actually close, one figure correction, one
durability requirement, and one measured hazard. All four are fixed.

Binary digest for every figure in this section:
`sha256:1b821c46ac9fa89522d81472d145019a5b08131d42b7a8343231b090421cfaaf`.

## 1 — Minor (c) was NOT closed, and the reason is worth recording

The reviewer is right, and the mechanism is instructive. Round two's fix for the conflict warning
was written in the same scripted edit as the `prune_empty` fix; that script asserted on a `prune_empty`
body it had already reformatted, the assertion failed, and **nothing in it was written**. I then
re-applied only the `prune_empty` half separately and moved on. The conflict change never landed.

It survived the round because the test asserted the wrong thing. `a_dry_run_surfaces_a_destination_collision`
checked that both files still existed after apply — true whether or not the dry run ever said a word.
A test that cannot fail when its subject is absent is not a test of its subject.

**Fixed properly this time, and at the right layer.** The collision is now part of the DERIVATION:
`ScopeMove::conflict` is computed by `scope_report` alongside the move, rendered under its own move
line by the same renderer both the dry run and the apply call, and refused in `hold_scope` before any
`git mv`. `git_mv`'s own `dst.exists()` check remains as a last-line defence for a tree that changed
between derivation and write, but it no longer prints — a refusal announced by the writer is a
refusal announced too late.

The test now asserts the text a human sees: the rendered dry run **contains `⚠ CONFLICT`**, that line
**names the colliding path**, and it appears **before** the summary line rather than after it. It also
asserts `to_held[0].conflict` on both the dry run and the apply, so the derivation itself is pinned
rather than only its rendering.

Live: no destination collisions exist in the tree today, so the wiring is correctly inert
(`conflicts: []`, three moves proposed, nothing printed). A dry run against the live tree confirmed it
writes nothing — `git status` on `genesis/a2o/held` and `genesis/a2o/features/auth` is empty and all
three named features are still in place.

## 2 — Two figures corrected from the tool's own output

Both were mine and both are wrong in the corrected table above:

- `agent-decomposition 57` → **40**. The 57 was a count over ALL records including orphans; the row
  states doc-present records, and 40 + 15 + 5 + 1 = **61**, which is the number the row claims.
- `54 orphan records` → **55**. `--adopt-gap-items` reports 55; my 54 counted only orphans that carry
  stations, silently dropping one zero-station record. The station total (1,378) is unchanged.

Corrected in the round-two class table, in the residue table, and in the seam registry's C4
justification, which carried the same 54.

## 3 — The relocated store is now TRACKED

The reviewer's NOTE was the important one: `.eprfs/` is gitignored wholesale, so the relocated store
— including the 649 stations that exist in no document's bytes — lived in exactly one working tree.
A relocation that is not durable is not a relocation.

`.gitignore`'s `.eprfs/` rule is replaced by the four-line re-inclusion git requires (a path cannot be
re-included once a PARENT directory is excluded, so each level is reopened before the next is named):

```
.eprfs/*
!.eprfs/status/
.eprfs/status/*
!.eprfs/status/gap-items/
```

Verified: `.eprfs/status/gap-items/*.json` is no longer ignored (193 files become visible, and
**only** those — `git status --untracked-files=all .eprfs/` shows 193 entries, none outside
`gap-items/`); `.eprfs/status/flows.jsonl` is still ignored by `.eprfs/status/*`; and
`.claude/memory-kit/gap-items/` remains ignored by its own `.claude/memory-kit/.gitignore`, untouched.

**The store is tracked as a PRE-IMAGE, not as a permanent home.** The 61 documents / 649 stations it
carries are stations somebody wrote down about documents that do not contain them. The durable fix is
to write them back into their documents as real `- [ ]` stations, after which the store is genuinely
derivable and can be deleted rather than relocated. **That write-back is station-six work** and is
named as such in the residue table. Until it happens, tracking the pre-image is the difference between
a relocation and a single-machine copy.

## 4 — The `--cluster-state` sibling hazard: fixed, not documented

It was a one-liner's worth of structure, so it is fixed rather than declared. `--cluster-state` names
ONE file, and the act lane contracts are its siblings — so an override pointed at an isolated copy
found no lane contracts beside it, every `@act:` rescue silently evaporated, and features that only
ever run in their own act's lane were proposed for `held/`.

`Substrate` now carries an ORDERED candidate list (`manifests_dirs`): the manifest's own directory
first, then the repository's `genesis/manifests`. `act_baseline_caps_in` takes the first candidate
that actually holds the lane file. A directory that genuinely holds the whole lane set is still used
as the lane set — an operator pointing at an alternative manifests tree gets that tree — and the
repository default answers only for lane files that directory does not have. Fail-open is unchanged:
no candidate at all still yields empty, never an invented gate.

Reproduced on an isolated copy of the live manifest:

```
before: --cluster-state <tmp>/cs.yaml → 9 moves
after:  --cluster-state <tmp>/cs.yaml → 3 moves   (identical to the repository reading)
```

Pinned by `a_lane_file_missing_beside_an_override_is_found_at_the_repository_default`, which asserts
all three arms: fallback used, own-directory preferred when present, and fail-open with neither.

## Round-three gate evidence

```
cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace
EXIT=0
   → 542 passed · 0 failed · 0 ignored, across 45 suites   (was 533/45)

cargo fmt --manifest-path elohim/eprfs/Cargo.toml --all -- --check
EXIT=0

cargo clippy --manifest-path elohim/eprfs/Cargo.toml --workspace --all-targets -- -D warnings
EXIT=0

EPR_BIN=/tmp/eprfs-gate-target/debug/epr python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
EXIT=0
   → Ran 27 tests, OK

python3 .claude/scripts/epr-meta-pin.py --verify
EXIT=0
   → 19 pinned row(s) verified clean (19 total); 15 pinned row(s) verified clean (15 total)
```

Binary digest `sha256:1b821c46ac9fa89522d81472d145019a5b08131d42b7a8343231b090421cfaaf`. One clippy
finding was mine and was fixed (`manual_contains` on the new candidate-list check). Disk held at 85%.

`flow/report.rs` was NOT touched this round, per the coordinator's note that station one's native seat
is editing it concurrently.

**Parity re-verified after every fix**: ledger BALANCE + PRESSURE QUEUE byte-identical to the kit;
`report scope` identical up to the next-action pointer; `report scope --docs` identical to
`placement-audit.py --focus` except the two drill-in pointer lines.

## Process concern — a berth discipline miss, disclosed

**I did not re-claim the cargo berth for round three.** It was claimed and released across round two;
when the round-three instruction arrived I went from reading the verdict straight into edits and ran
`cargo build`, `test`, `fmt` and `clippy` without a claim. Nothing was harmed — no concurrent cargo
run was observed and every gate is green — but the protection was absent for the whole round, and the
instruction said "berth first". Recorded rather than papered over: the miss is exactly the shape of
the one this round's Important also had, which is a scripted step silently skipped and only caught by
someone checking.

## Residue — round-three additions

| Item | Home |
|---|---|
| Write the 61 documents' 649 agent-decomposed stations back into their own bytes as real `- [ ]` stations | **Station six**, before the store can be deleted rather than relocated. Until then `.eprfs/status/gap-items/` is a tracked pre-image |
| 55 orphan records / 1,378 stations whose document is gone | Station six decides; `--adopt-gap-items` names them and adopts none |
| `epr flow hold --scope --apply` still unexercised against the live tree | Unchanged from round one's Concern 3, and now the only untried path this station owns. The deployments.json write arm sits behind it |

---

# Round four — review response (2026-09-10)

Two Importants and one Minor, all fixed. Both Importants are the same failure in two costumes: a
change that *looked* right and was never checked against the thing it claimed to affect.

Binary digest for every figure in this section:
`sha256:97af15e99d0ff2ce7cf4b1d83aef0b31848b5ee4f8ced553fa154d273b9ce39d`.
Cargo berth claimed BEFORE the first cargo invocation this round, and released after the last.

## 1 — Important: the `.gitignore` fix un-ignored every nested sidecar

Confirmed live before touching anything: `elohim/elohim-storage/.eprfs/status/flows.jsonl` showed as
`??`. The tree carries at least five `.eprfs` directories — the storage crate's, and one per ark
payload under `genesis/a2o/reports/compute/`.

The cause is a git rule I applied without checking its consequence. **A pattern containing an
interior slash is ANCHORED to its `.gitignore`'s directory.** The original `.eprfs/` had no interior
slash and so matched at every depth; my replacement `.eprfs/*` looks like the same rule and is not —
it matches only the root one. I replaced an unanchored rule with an anchored one while thinking only
about the re-inclusion ladder, and the ladder worked, which is why it read as correct.

Fixed with the shape the reviewer specifies, which separates the two jobs: `**/.eprfs/` restores the
unanchored ignore for every sidecar at any depth, and the anchored ladder below re-includes only the
root store.

```
**/.eprfs/
!/.eprfs/
/.eprfs/*
!/.eprfs/status/
/.eprfs/status/*
!/.eprfs/status/gap-items/
```

**Verification block** — run against the live tree, showing which rule answers:

```
elohim/elohim-storage/.eprfs/status/flows.jsonl   IGNORED     by .gitignore:134:**/.eprfs/
.eprfs/status/flows.jsonl                         IGNORED     by .gitignore:138:/.eprfs/status/*
.eprfs/status/gap-items/<any>.json                NOT-IGNORED
```

and the negative check that nothing else leaked: every `.eprfs` path git now reports as untracked is
under `.eprfs/status/gap-items/` — 193 of them, zero others.

The three invariants are also a TEST, not only a documented block:
`the_relocated_store_is_tracked_and_every_other_sidecar_is_not` in `tests/flow_gap_fallback.rs` runs
`git check-ignore -q` on all three paths with their expected verdicts, and skips with a reason when
the repository root or git is unavailable. `check-ignore` answers about a PATH rather than a file, so
it stays honest on a tree where the store has not been adopted.

## 2 — Important: the collision test asserted its own copy of the renderer

The reviewer's demonstration is exact: deleting both `render_conflict()` calls left every test green.
The test had re-implemented the rendering locally, so it was asserting against its own copy — which is
how a finding closed in round three could have been re-broken in round four without anyone noticing.
It is the same shape as the round-three miss (a change that never landed, hidden by a test that could
not see it), one layer up.

**Fixed at the cause, not at the assertion.** `ScopeReport::render` now returns a `String`, following
`FocusBaseline::render -> String` two functions away; `hold_scope` does the printing. The test asserts
on the SHIPPED string: it contains `⚠ CONFLICT`, the CONFLICT **line** contains the colliding path
(asserted per-line, so two unrelated lines cannot satisfy it jointly), and the ordering is
`→ HELD` < `⚠ CONFLICT` < summary. It also asserts the apply rendering carries it.

`render_conflict` returns its line rather than printing it, so there is one renderer and no second
place for the text to live.

**Mutation-checked, which is the only evidence that counts here:**

```
delete both `out.push_str(&m.render_conflict());` call sites
→ test a_dry_run_surfaces_a_destination_collision ... FAILED
→ test result: FAILED. 22 passed; 1 failed              (EXIT=101)
restore → 23 passed, EXIT=0
```

A companion test, `a_clean_move_set_renders_no_conflict_line`, pins the other half: without it,
"contains CONFLICT" would be satisfied by a renderer that always emits it.

## 3 — Minor: the baseline write needed its parent

`hold_scope --apply` wrote `.claude/subject-focus.md` without creating the directory, so a repository
with no `.claude/` — a fresh clone, a scoped worktree — took every move, printed APPLIED, and *then*
exited non-zero. An error raised after the work is done reads as "the reconcile failed" when it
succeeded. `create_dir_all` on the parent, and
`apply_creates_the_baseline_parent_rather_than_failing_after_the_work` asserts the fixture starts
without `.claude/`, that the move still happened, and that the baseline is written.

## Round-four gate evidence

Berth claimed first this round.

```
berth claim cargo --session mk-replace-native-2     → cargo: claimed by mk-replace-native-2

cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace
EXIT=0
   → 545 passed · 0 failed · 0 ignored, across 45 suites   (was 542/45)

cargo fmt --manifest-path elohim/eprfs/Cargo.toml --all -- --check
EXIT=0

cargo clippy --manifest-path elohim/eprfs/Cargo.toml --workspace --all-targets -- -D warnings
EXIT=0

EPR_BIN=/tmp/eprfs-gate-target/debug/epr python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
EXIT=0
   → Ran 29 tests, OK

python3 .claude/scripts/epr-meta-pin.py --verify
EXIT=0
   → 19 pinned row(s) verified clean (19 total); 15 pinned row(s) verified clean (15 total)

berth release cargo --session mk-replace-native-2   → cargo: released by mk-replace-native-2
```

Binary digest `sha256:97af15e99d0ff2ce7cf4b1d83aef0b31848b5ee4f8ced553fa154d273b9ce39d`. One clippy
finding was mine and was fixed (an import left unused once the test stopped re-implementing the
renderer). `flow/report.rs` untouched again. Disk held at 85%.

**Parity re-verified**: ledger BALANCE + PRESSURE QUEUE byte-identical to
`placement-audit.py --ledger`; `report scope` identical to `scope-reconcile.py --report` up to the
next-action pointer.

## What these two rounds actually taught

Both Importants this round, and the Important last round, are one class: **a change verified against
its own description rather than against its subject.** The gitignore ladder was checked for
re-inclusion and never for the rule it replaced. The conflict renderer was checked by a test that had
copied it. Round three's conflict fix was checked by a test that could pass without it.

The durable countermeasure is the one applied here twice: assert on the SHIPPED artifact — the
returned string, the live `git check-ignore` verdict — and, where the assertion is load-bearing,
mutate the code and watch the test go red. The mutation check took thirty seconds and is the only
reason I can claim this finding is closed rather than believe it.

No commits. No pushes. `.claude/memory-kit/` untouched.
