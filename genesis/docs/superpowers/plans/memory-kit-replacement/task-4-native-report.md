---
id: memory-kit-replacement-task-4-native-report
title: Station four (native) — memory entries become contributions, MEMORY.md becomes a projection
status: DONE_WITH_CONCERNS
class: devflow
gap: plans__2026-09-10-memory-kit-replacement-finish#5
actor: agent:implementer@claude-opus-5
commits: []
cites:
  - "memory-kit-replacement-finish | The plan this station drains | sha256:dbf92bf02195bfaf | path: genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md"
  - "parity-inventory-2026-09-10 | The inventory rows this station converts from missing to native | sha256:48aa6d31bffeef22 | path: genesis/docs/superpowers/plans/memory-kit-replacement/parity-inventory-2026-09-10.md"
  - "private-thought-governed-fruit | The constitutional privacy line the import gate enforces | sha256:ea19f49a6700ad04 | path: genesis/docs/architecture/private-thought-governed-fruit.md"
---

# Station four (native) — memory entries become contributions, `MEMORY.md` becomes a projection

Two verbs landed: `epr flow memory import <dir>` and `epr flow memory project --index`. Both build
on the existing collective-memory path rather than forking it: import writes an ordinary
contribution request and calls `memory contribute` unchanged, so every refusal the collective
already makes for a hand-authored contribution it still makes for an imported one.

Status is `DONE_WITH_CONCERNS` for one reason, stated up front: **six live entries are refused for
malformed frontmatter, and three of them are rows the index carries today.** The live import was run
`--dry-run` only, as instructed. A live apply before that frontmatter is healed would silently drop
three rows from `MEMORY.md`. Details in §5; the fix is one line per entry and belongs to the
integration seat, which owns `.claude/memory/**`.

## 1. What shipped

| Artifact | Lines | What it is |
|---|---:|---|
| `elohim/eprfs/epr-cli/src/flow/memory/entries.rs` | 313 | Frontmatter reader + index renderer held at byte parity with `memory-index-projector.py` |
| `elohim/eprfs/epr-cli/src/flow/memory/import.rs` | 526 | `import` — the privacy gate, the frontmatter gate, and the batch contribute |
| `elohim/eprfs/epr-cli/src/flow/memory/index.rs` | 277 | `project --index` — the projection, the declared budget, the drift fold |
| `elohim/eprfs/epr-cli/src/flow/memory/mod.rs` | 458 | `Options`, `execute_with`, and the one-scan `ContributionActs` registry (§9) |
| `elohim/eprfs/epr-cli/tests/flow_memory_import.rs` | 644 | 12 tests, all on shipped output |
| `elohim/eprfs/epr-cli/tests/fixtures/memory-entries/` | 229 files | Frontmatter snapshot of the live corpus, taken 2026-09-10 |
| `elohim/eprfs/eprfs-agent/src/memory.rs` | +36 | The additive `Imported` provenance record |

`execute(root, op, input, session)` keeps its exact four-parameter shape and now delegates to
`execute_with(root, op, &Options)`; no existing caller or contract test changed.

## 2. `epr flow memory import <dir> [--dry-run] [--contributions <dir>] [--json]`

Per entry, in this order: privacy gate → read → frontmatter gate → build the contribution →
already-recorded check → write the request → `memory contribute`.

**Attribution.** Three identities meet on an imported entry and the substrate keeps them apart:

- `Contribution.author` — the **acting** participant, resolved from the registered session's actor
  claim, which is the same claim `contribute`'s note guard checks. Deriving it rather than accepting
  it as a flag means the two cannot disagree.
- `Imported.gitAuthor` — the **provenance** of the pinned bytes, `Name <email>`, from
  `git log -1 -- <entry>`. An untracked entry reads `(untracked: no commit carries these bytes)`
  rather than borrowing HEAD's author.
- the note's `steward:` slot — the git-signing human answerable for the tree, which
  `note_with_options_guard` already derives from HEAD. Import supplies nothing here, which is why it
  cannot be spoofed here.

**How I attributed, and why no persona was minted.** The brief asked for `author` = the git author's
identity. The collective validates `author` with `parse_agent_ref`, which admits only
`agent:<role>@<model>`; a human git identity cannot pass it. Minting `agent:matthew@human` to get
past the parser would be the substrate asserting a claim nobody made, so I did not. The human is
recorded twice and honestly — as `gitAuthor` on the bytes they wrote, and as the note's `steward:` —
and `author` names who performed the contributing act. This is the plan's "git-author provenance"
satisfied without a fabricated persona. Similarly `Contribution.steward` stays
`repo:ethosengine/elohim`, because the collective refuses a contribution whose steward is not the
collective's; the git author's stewardship is the note's steward slot, which is where the flow plane
already puts it.

**Field mapping.** `concern` = the entry's `name:`. `claim` = the entry's `description:` (also the
index row's text). `scope` = `repository`. `sources[0]` = the entry's path pinned by **raw** BlobCid.
`imported` = `{display, file, scopePath, gitAuthor, entryType, indexed}`. The plan asked for
`display` and a path-valued `scope`; `scope` is a closed three-value enum in the existing contract,
so the entry's path is carried as `imported.scopePath` and the display name as `imported.display`.
That is the only contract change: one additive `Option<Imported>` field,
`skip_serializing_if = "Option::is_none"`, so every pre-existing contribution file and every
projection receipt that embeds one is byte-identical.

**Idempotence is by content, checked before the append.** The note leg already no-ops a
byte-identical append, but that alone would re-open the store once per entry and would drift the
moment HEAD moved (a note is dated by the tree it was written against). So import asks the flow
plane first — `ContributionActs` holds the attributed contribution acts, keyed by the *same*
`classified_as` reason string the writer emits, spelled in one place (`contribution_reason`), and the
whole plane is read once per command rather than once per entry (§9). A second run over a drained directory appends **zero** events and rewrites
**zero** files. Pinned by test.

**Reach.** `reach = min(policy(entry path), policy(contribution request path))` — never widened. The
declared values are the collective's to set and have since moved; see §9 for the current reading.

## 3. `epr flow memory project --index [--budget <id>] [--out <path>] [--json]`

Rows come from **contributions**, not from a directory scan: a `.json` that does not parse as a
contribution, carries no `imported` provenance, or has no attributed observation in the flow plane
does not get to add a row to what every session loads. Rendering is the transcribed oracle —
`- [title](file.md) — description`, sorted by lowercased file name, title truncated at 80
characters, description at 200, the projector's exact header.

Three properties are deliberate:

- **The cap is declared, never coded.** The kit carried `HARD_BUDGET_BYTES = 24_000` as a constant in
  three files. Here the watermark is resolved through the same `Bound` the native report reads, from
  `measures.yaml` (lens rows) and `policies.yaml` (`class: measure` ceiling rows). `--budget` names a
  bound by id or by the measure it consumes. With no `--budget`, the command falls back to
  `memory-index-bytes@1` for the unloaded-row cap and refuses nothing; an undeclared pin is refused
  naming the registry path.
- **`index_unloaded` is a fold, not a private JSON file.** It lands as a structured observation on
  `memory-index-drift@1` through the existing note verb, subject `.`, unit `rows`. The reason pins
  the unloaded **set** by content address, so the same drift asked twice is one record, a changed
  drift is a new one, and a drained index witnesses its zero exactly once. Station one's
  `cleanup-pressure` lens row notes `memory-index-drift@1` "has no fold producer yet" — this is that
  producer.
- **Writing is opt-in.** With no `--out`, nothing is written. The drift fold *is* still appended
  without `--out`, because it is the report leg the station asks for, not an index write; I read
  "write nothing when `--out` is absent" as governing the index file. Flagging the reading in case
  the integration seat wants it narrower.

Order of operations on a breach: compute → append the fold → *then* refuse. A breached budget is
exactly when the drift matters, and a report that refuses without recording what it saw teaches
nothing twice.

## 4. Tests — 12, all on shipped output

`elohim/eprfs/epr-cli/tests/flow_memory_import.rs`, against a synthetic repo carrying the **real**
`.epr-meta/collective.json` and the **real** `.claude/epr-meta/measures.yaml` via `include_str!`, so
a registry drift turns these red rather than leaving them green against a local copy.

| Test | What it pins |
|---|---|
| `importing_the_corpus_twice_appends_exactly_one_round_of_events` | 229 entries → 226 contributed / 3 refused; second run: 226 skipped, 0 appended, sidecar line count unchanged, request bytes unchanged |
| `an_imported_contribution_carries_provenance_without_minting_a_persona` | `author` is the agent claim, `steward` is the collective, `gitAuthor` is the human, source pinned by raw CID |
| `a_dry_run_plans_and_writes_nothing` | no events, no contributions directory |
| `a_refusal_reports_whether_it_would_cost_the_index_a_row` | `refusedIndexedToday` distinguishes two entries with the same defect |
| `a_private_store_is_refused_and_the_same_bytes_are_importable_elsewhere` | **mutation-checked** — identical bytes, two homes |
| `a_private_file_inside_an_allowed_directory_is_refused_by_name` | `session.continuation.md` refused, `session-continuation-design.md` contributed |
| `the_projected_index_is_byte_identical_to_the_python_projector` | oracle + `sha256 8ee2e07eac63f45594cf18b6ab5f996594c5c25b0e511fd20bba198a627a83dd`, 98 rows, 23,993 bytes |
| `without_out_the_projection_writes_no_index` | reports, writes nothing |
| `the_declared_budget_refuses_one_byte_over_the_watermark_and_not_at_it` | 24,000 writes; 24,001 refuses naming `memory-index-bytes-ceiling@1` and writes nothing |
| `an_undeclared_budget_pin_is_refused_naming_the_registry` | refusal names `measures.yaml` |
| `the_unloaded_fold_is_appended_once_per_distinct_set` | same set → 1 fold; changed set → 2 |
| `a_drained_index_witnesses_its_zero_exactly_once` | a witnessed 0 is a record, and it is one record |

**Mutation check, run and reverted.** With `private_reason` short-circuited to `None`, exactly the
two privacy tests went red — `a_private_store_is_refused_…` reporting the private note actually
contributed, and `a_private_file_inside_…` reporting the continuation file imported — and the other
ten stayed green. The gate is load-bearing, and the tests fail for the right reason. The probe was
removed; the file is clean.

The parity harness is non-vacuous both ways: it runs the Python oracle when present (it did) **and**
asserts the digest unconditionally, so on a tree where the kit has been cleared the bytes are still
pinned.

## 5. Live dry-run — the concern

```
epr flow memory import .claude/memory --session mk-replace-native-4 --dry-run --json
counts: entries 229 · contributed 223 · refused 6 · refusedIndexedToday 3 · eventsAppended 0
reach : source repository · request workspace · effective workspace
```

Nothing was written: `.eprfs/status/memory` does not exist and the sidecar is unchanged. The three
`memory-index-drift` strings already in the live `flows.jsonl` are other seats' prose notes citing
the measure by name, not folds.

Six refusals, all the same defect — no declared kind:

| Entry | In the index today |
|---|---|
| `feedback_human_loop_not_terminal_authority.md` | **yes** |
| `feedback_private_thought_governed_fruit.md` | **yes** |
| `feedback_sccache_failure_classes.md` | **yes** |
| `feedback-identity-sovereignty-ontology-guard.md` | no (`index: false`) |
| `project_prod_main_lag_vs_alpha_dev.md` | no |
| `project_reach_enum_drift_reconciliation.md` | no |

The cause is malformed YAML, not a missing declaration: each carries a `metadata:` block whose
mapping is interrupted by a **column-zero `id:`**, after which the indented `type:` is no longer
inside `metadata:`. The Python projector reads the same way — that is why those three entries render
in `MEMORY.md` under their `name:` rather than their nested `title:`.

I kept the parser strict rather than resuming a block a column-zero key has closed. Absorbing
malformed YAML would hide a real defect in the corpus and would make the reader disagree with the
oracle it is pinned against. The refusal names the exact shape, and `refusedIndexedToday` reports the
blast radius so a live apply cannot lose rows unnoticed.

**Precondition for the live apply (integration seat, who owns `.claude/memory/**`):** move the
column-zero `id:` above the `metadata:` block in those six entries, or indent it into the block.
Re-run the dry run; apply when `refusedIndexedToday` is 0. The corpus also drifted during this
station — the snapshot taken at its start had 3 refusals and the current tree has 6 — so re-read the
dry run rather than these numbers.

Note the fixture is a **snapshot**, deliberately: it pins 229 entries / 98 rows / 23,993 bytes as of
2026-09-10 so the parity digest is stable while the live corpus keeps moving.

## 6. Declarations changed, and one I could not avoid

`.epr-meta/collective.json` gained two source rules. The declaration carried none for
`.claude/memory`, and the brief authorised adding one and saying so:

```json
{"path": ".claude/memory", "maxReach": "repository"},
{"path": ".eprfs",         "maxReach": "workspace"}
```

`.claude/memory` is `repository` because those entries are git-tracked and read by every session in
every clone — that is their actual locality, not a widening. `.eprfs` is `workspace` because
`/.eprfs/status/*` is gitignored: the sidecar that records a contribution is workspace-local, so a
contribution recorded there cannot honestly claim repository reach. The `min` of the two is what
import declares, which is why the effective reach is `workspace`. **If the operator wants
repository-reach memory contributions, the contributions home must move to a git-tracked path with a
`repository` rule** — `--contributions <dir>` takes it, no code change needed. Flagging rather than
choosing: that is a reach decision, not an implementation one.

## 7. Gate evidence

Cargo berth claimed before every invocation and released after.

```
env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 \
  cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace
EXIT=0
  47 suites · 595 passed · 0 failed  (flow_memory_import: 12 passed)

env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target \
  cargo fmt --manifest-path elohim/eprfs/Cargo.toml --all -- --check
EXIT=0

env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 \
  cargo clippy --manifest-path elohim/eprfs/Cargo.toml --workspace --all-targets -- -D warnings
EXIT=0
```

An intermediate fmt run reported diffs in `flow/cites.rs` and `tests/flow_cites.rs` — the station-3b
seat's files, running concurrently. I formatted only my own files with `rustfmt` rather than
`cargo fmt`, and by the final gate that seat had formatted theirs; the workspace is clean and I
clobbered nothing. No commits, no pushes; only the paths listed in §1 plus
`.epr-meta/collective.json` and `epr-cli/Cargo.toml` (`sha2` added to `[dev-dependencies]`) were
touched. `flow/report.rs`, `flow/measures.rs` and `flow/cites.rs` were read but never written.

The workspace restarted mid-station (container restart, `/tmp` wiped). Source survived on disk; the
gate target was rebuilt from scratch and every number above is from the rebuilt tree.

## 8. Open, for the seats that follow

1. **The six malformed entries** (§5) — the live apply's precondition. Integration seat.
2. ~~**The contributions home and reach** (§6)~~ — **taken by the integration seat**, which added
   `{"path": ".eprfs/status/memory", "maxReach": "repository"}`. Effective reach is now `repository`.
   See §9.
3. **The fold without `--out`** (§3) — flag the reading if it should be narrower.
4. **Missing-description parity** — the projector renders a placeholder row for an entry with no
   `description:`; import refuses such an entry, so it can never reach the projection. The live
   corpus has none, so today's parity is exact; declared rather than discovered later.
5. **Independent technical review** — `epr-cli/.epr-meta`'s `reconciliation-fitness-review` rule and
   the plan both require a separately-appointed reviewer for this station. Not self-certifiable.
6. **Deprecation sentinel** — reading `.claude/memory/project_content_graph_native_rust_not_cozo_apollo.md`
   fired fingerprint `1a6760c5c52b` on prose quoting a *historical* Kuzu deprecation. A false
   positive; not dispatched, because the finding is a memory-entry quotation and the triage agent's
   write set overlaps other seats'. Recorded so it is not lost.

---

## 9. Follow-up — the projection was 113 seconds; it is now 1.1

The integration seat measured `project --index` at ~119 s on the live tree against a 10-second
PostToolUse budget, so the hook router kept taking the kit leg. This section records the fix and the
numbers.

**Cause.** `observation_recorded` re-opened, re-read and re-parsed the whole flows sidecar once per
contribution. On the live tree that is 229 contributions × 7,800 records — a quadratic scan hiding
behind a function that looked like a lookup.

**Fix.** `ContributionActs::open(root)` reads the plane **once** and builds a set of
`(resource body CID, provider, reason slot)` triples — exactly the tuple the old per-record predicate
matched, filtered to contribution acts by the `reason:Collective contribution ` prefix. `import` opens
it before its loop; `index::collect` opens it after its early return, so a repository with no
contributions never reads the plane at all.

**The guard against regression is the signature, not a stopwatch.** `holds(&self, …) -> bool` takes no
`&Path` and returns no `FlowResult`: it *cannot* touch the filesystem, so a per-contribution scan is
not expressible without deliberately re-opening the registry. A wall-clock assertion would have been
flaky on a shared box and — because the fixture's sidecar is two orders of magnitude smaller than the
live one — would not have caught this defect in the first place.

**Measured, live tree, 229 contributions / 7,803-record sidecar:**

| Command | Before | After |
|---|---:|---:|
| `epr flow memory project --index --json` | **113.73 s** | **1.19 s** · then 1.06 / 1.05 / 1.13 s over three consecutive runs |
| `epr flow memory import .claude/memory --dry-run --json` | — | 6.16 s (229 skipped; not hook-budgeted) |
| `cargo test --test flow_memory_import` | 96.9 s | 33.1 s |

Output is unchanged across the fix: 98 entries, 23,993 bytes, population
`{contributions: 229, optedOut: 131, unattributed: 0}`, `indexUnloaded: 0`. Target was under 2 s;
measured 1.05–1.19 s, comfortably inside the 10 s hook budget.

**Byte parity re-confirmed on the LIVE tree**, which is stronger evidence than the fixture alone:

```
epr flow memory project --index --out <scratch>
sha256  8ee2e07eac63f45594cf18b6ab5f996594c5c25b0e511fd20bba198a627a83dd
pin     8ee2e07eac63f45594cf18b6ab5f996594c5c25b0e511fd20bba198a627a83dd
diff <native projection> .claude/memory/MEMORY.md  →  identical
```

The native projection over **contributions** and the kit's projection over **frontmatter** produce
the same bytes on the live corpus. The scratch file was removed; no index was written.

**One test changed, and it is a better test.** `importing_the_corpus_twice…` asserted
`effective == "workspace"`, which pinned a *declaration* rather than this leg's logic; when the
integration seat raised `.eprfs/status/memory` to `repository` (§6, §8.2) it went red for the right
reason in the wrong place. It now asserts the **relation** — `effective == min(source, request)` —
so the collective may move its own source rules while a widening bug still fails. The declared reach
on the live tree is now `source: repository · request: repository · effective: repository`, and the
concern raised in §6 is closed by the owner who owned it.

**Gate, re-run with the cargo berth claimed and released:**

```
env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 \
  cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace
EXIT=0
  49 suites · 638 passed · 0 failed  (flow_memory_import: 12 passed)

env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target \
  cargo fmt --manifest-path elohim/eprfs/Cargo.toml --all -- --check
EXIT=0

env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 \
  cargo clippy --manifest-path elohim/eprfs/Cargo.toml --workspace --all-targets -- -D warnings
EXIT=0
```

Files touched in this follow-up: `flow/memory/{mod,import,index}.rs` and
`tests/flow_memory_import.rs`. `flow/memory/recall.rs` and `flow/concerns.rs` — station five's seat,
building concurrently — were not read or written. No commits, no pushes. The integration router was
told nothing; it re-measures and flips itself.
