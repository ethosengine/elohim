---
id: bounded-recall-native-report
cites: []
status: DONE_WITH_CONCERNS
gap: plans__2026-09-11-bounded-recall-mastery-sprint#1
actor: agent:implementer@claude-opus-5
session: native-recall-ux-20260911
commits: []
---

# Bounded recall mastery — native seat report

Gaps covered: `plans__2026-09-11-bounded-recall-mastery-sprint#1`, `#2`, `#3`, `#5`.
Everything below is one seat's work inside `elohim/eprfs/**`,
`.epr-meta/elohim/algorithms/recall-contract.json`,
`genesis/a2o/features/devflow/ceremony-reconciliation.feature` and its steps file. Nothing was
committed or pushed. The dirty `elohim-storage` tree and the `.claude/**` / `.agents/**` /
`.codex/**` edits beside this work belong to other lanes and were not touched.

## Station #1 — the entry carries the agent's question and reaches the tooling layer

**The need becomes the intent.** `open --need '<question>'` with no `--intent` now records that
question as the session intent, so every later receipt, `finish` and measurement is accounted
against the question the agent actually carried. `--intent` still wins when given. The fallback is
gated on `--need` having been *typed* (`Args::need_explicit`), not on the field being non-empty —
`need` carries a standing default (`"Orient and choose the next justified reconciliation action"`),
and adopting that as an intent would have replaced one borrowed purpose with another.

**`.claude/` is a declared source root.** The contract's `source_roots` gained `.claude/` (which
subsumes the old `.claude/memory/` entry), so skills, hooks, commands, agents, `epr-meta`, scripts,
workflows and memory are readable *through* the entry and therefore metered. `version` moved
`4 → 5`, so the method CID every receipt pins moved with it.

**Two subtrees stay refused inside the widened root.** The private recall store
(`.eprfs/status/recall/`, unchanged) and `.claude/worktrees/` — a sibling checkout is another
repository's working tree, and its bytes and assertions are not this repository's. The refusal is
stated once in `contained()`, which every bounded read and every discovery traversal passes
through, and again on the argument-level gate in `refuse_private_import()`. It has its own remedy
class rather than borrowing the private-record one. `worktrees` was also added to the contract's
`discovery.exclude_directories`.

**One extra limit.** `limits.habit_register_bytes: 1048576` — the habit register is a 419 KB
generated projection read in full by station #5's first screen, and charging it against
`source_bytes` (20,000) would have made the first screen impossible or the packet budget a lie. It
is accounted on its own usage line, the way `native_raw_bytes` already is. Absent from a contract,
it defaults to 1 MB.

**Coordinator item (integration seat's finding).**
`elohim/eprfs/epr-cli/src/flow/memory/entries.rs` `HEADER` — the generated `MEMORY.md` header
named `memory-index-projector.py --apply`, a deleted kit script, so every re-projection reproduced
an instruction nobody could follow in the one file a session reads first. It now names
`epr flow memory project --index --budget memory-index-bytes@1 --out .claude/memory/MEMORY.md`.
That changes projected bytes, so `flow_memory_import.rs` was re-baselined deliberately:
`EXPECTED_INDEX_BYTES` 23,993 → 24,056 and `EXPECTED_INDEX_SHA256`
`8ee2e07e…` → `841513f5…`, with the reason recorded on the constant. The row rule, the row count
(98) and every rendered row are unchanged; only the header line moved. **The live
`.claude/memory/MEMORY.md` still carries the old header** — re-projecting it is a `.claude/**`
write, outside this seat's set. The integration seat should run the command the new header names.

Tests: `the_agents_own_question_becomes_the_session_intent`,
`the_tooling_layer_is_readable_and_a_sibling_checkout_is_not`,
`the_live_contract_declares_the_tooling_layer_and_receipts_pin_its_bytes` (asserts the live
artifact declares `.claude/`, `version >= 5` and `habit_register_bytes`, and that a session pins
the contract's raw CID), all in `tests/flow_memory_recall.rs`.

## Station #2 — legible refusals and a compact human view

**A human refusal is two lines.** `refused: <message>` and `next: <remedy>`, exit 2 preserved, no
JSON, no re-printed orientation. `--json` keeps the exact envelope it had. The mode is resolved
before `parse_args` runs (`argv.contains("--json")`), because a refusal the parser itself produces
has to be rendered too. Three refusals that previously escaped the envelope entirely (unknown
operation, missing `--session`) now go through it; a bare `recall` with no operation prints usage
on stdout at exit 0, matching the help rule below.

**The human view is a screen, not a JSON dump.** `render()` gives each accounting structure exactly
one line: `Continuation:`, `Accounting:`, `Frontier:`, `Measurement:`, `Usage:`, `Unresolved:`, plus
the concerns block's `Measured scope:` / `Page:` and omissions as clipped bullets. Byte magnitudes
are grouped in threes. **`Linked choices:` is untouched** — it is the progressive-discovery payload
and the habit's own guard names shrinking it as the way to fake this green.

Tests: `a_human_refusal_is_two_lines_and_the_json_envelope_is_unchanged`,
`the_human_view_is_compact_and_the_json_view_is_whole` (human `open` under 6 KB on the station-five
fixture; `--json` still carries `cumulative.totals` and `continuation.counts`). All 38 pre-existing
assertions in that suite stayed green unchanged.

## Station #3 — help is a question

`epr --help` / `-h` / `help`, `epr flow --help`, `epr flow memory --help` and
`epr flow memory recall --help` all print usage on **stdout** and exit **0**; a bare
`epr flow memory` prints its eight-operation list the same way. `epr flow` with no subcommand stays
an argument error, as its own comment argues. New suite `tests/help_is_a_question.rs`, one test per
dispatcher (four), each asserting exit 0, empty stderr and a `usage:` line.

## Station #5 — the first screen at its door, and the `recall` headline slot

**Focused `open`.** When the caller typed a `--need` and either the scope is a directory other than
`.` or a question term names one, the view renders a `first_screen` BEFORE the concern groups:

- the habits from `genesis/manifests/habits.yaml` whose id (weighted ×3) or invariant words the
  question's terms touch, each with its `status` and the first *paragraph* of its evidence ledger —
  the newest DELTA. The register is read by a bounded line scan that reassembles YAML folded
  scalars, so the delta is a sentence rather than the first 100 columns of one.
- up to `limits.search_results` candidate sources from the existing traversal, now available as
  `discover_scored(…, terms, …)`: `discover()` is a thin wrapper passing no terms, so `search` and
  `source --tag` keep today's deterministic window byte-for-byte. With terms it collects a wider
  window, ranks by declared-metadata term overlap (`title`, `description`, path, tags), truncates to
  the declared result count, names the provider and reports the ranking rule as *candidate
  discovery, not authority*. Terms that name the area itself are dropped, because they match every
  file under it and rank nothing.
- one `Outline <path>` linked choice per candidate — a command, never an excerpt.

A whole-scope `open` (no typed `--need`, or terms that touch no habit and no directory) gets no
first screen and keeps the grouped stale-edge convergence view, now compact.

**The headline slot.** `HEADLINE_ORDER[0]` is `recall` (position kept); `recall-unmetered-bytes*`
derives into it. The other three journey middot deliberately do NOT derive into the slot — with a
bare `starts_with("recall")` the line printed whichever recall row the registry listed first
(`recall-metered-bytes-ceiling`), which is a different measurement. `memkit` stays in the slot
*vocabulary* (its retired bound still declares it, and `flow_report_surface_walk` asserts it by
name) but is out of the printed order. A skipped recall slot prints
`recall: skipped — no journey fold` rather than `skipped (no fold for recall-unmetered-bytes@1)`.
`epr flow mod.rs`'s help text now says `recall:` where it said `memkit:`.

Tests: `a_focused_question_gets_its_habit_its_delta_and_ranked_sources` (habit id, status and
newest-delta-only; candidates inside `source_roots`, ranked, `habit_register_bytes` and
`scanned_files` charged, `Outline …` actions emitted; and a whole-scope open with no area match
carrying `first_screen: null`); `the_recall_slot_names_a_missing_journey_rather_than_a_missing_row`
and the amended order/derivation unit tests in `report.rs` and `tests/flow_report.rs`.

## Station a2o scenario

One scenario, tagged `@concern:recall-reaches-authority` on the scenario (the feature keeps
`@concern:dev-system-equilibrium`): *A fresh agent's own question is the investigation's purpose and
the tooling is in reach*. It went through **three** blind-reader rounds per `genesis/a2o/.epr-meta`.

- Round 1 (REVISE): "packet budget" undefined; four assertions bundled; "receipted excerpt" opaque;
  "declared source scope" never declared by a Given. → The sibling-checkout refusal moved out of the
  story (the Rust suite proves it), the scope became a Given, the narrative gained a paragraph.
- Round 2 (REVISE, two blockers): the byte-counting mechanism lived only in prose, and the
  "not the algorithm's generic one" contrast had nothing observable behind it. → Both became Givens.
  The generic purpose is now **observed**, not quoted: the step opens a throwaway session with no
  question and reads back the purpose the algorithm states on its own, so the final Then asserts a
  distinction against something the story actually shows.
- Round 3 (REVISE): the Givens defined vocabulary instead of establishing preconditions; "entry" was
  overloaded between the two concerns; and the title's second promise, "the tooling is in reach", was
  not causally proven — nothing showed the scope was the *mechanism* rather than reads simply
  succeeding. → Every Given became observable: the generic purpose is the purpose the algorithm
  states for an investigation nobody brought a question to; the packet limit is asserted wider than
  the passage and far narrower than the repository (a limit that admitted the whole tree would make
  the closing assertion true of a journey that read everything); the question is quoted in the When.
  A contrast Then was added — the same verb, the same session, one path inside the scope and one
  outside it, the second refused — which is what makes the scope Given load-bearing. The narrative
  now names the RECALL ENTRY once, as the one command every scenario above already reaches its
  evidence through, so "entry" stops colliding with "instruction entry".
- Round 4 (REVISE, no blockers, three MAJOR, all fixable in the one scenario): the quoted question
  used domain nouns the document never grounds ("palace index", "kit scripts"), so a reader could
  not tell whether its content was load-bearing; the algorithm's default purpose was asserted
  *against* but never shown, making the "not X" Then unfalsifiable from the reader's seat; and
  "RECALL ENTRY" was narrative-only vocabulary the steps never used. → The question and the fixture
  skill were reworded into terms the scenario itself grounds ("the command that rebuilds a stale
  index"); the default became "the standing description the recall algorithm states as its purpose
  when nobody brings a question", with a shape the step now CHECKS (it carries none of the
  question's distinctive words, so "naming no question" is an assertion rather than a claim); and
  the When opens "the recall entry" by name.
- Round 5 (REVISE, no blockers, three MAJOR): a Then step embedded an undeclared ACTION (the
  out-of-scope read appeared only inside an assertion, so a step author could not tell what
  triggered the refusal); the title promised two properties while the body proved four; and
  "recall entry" / "recall algorithm" were narrative-only vocabulary in executable steps. → The
  out-of-scope read became its own `When … attempts the same read against a path outside the
  declared scope`, with the Then asserting only its outcome; the title now names all four
  properties; and a Given makes the entry answer for itself — it invokes `recall --help` and checks
  that the entry's own usage names `open`, `read` and `finish`, so "the one command" is a fact the
  story verifies rather than a term the narrative asserts.
- **Round 6: READY.** "The scenario's audience, value, and causal proof are recoverable from the text
  alone. The three title promises each have a corresponding Then clause." One non-blocking MAJOR
  remains, recorded below.

Final shape (13 steps, all passing):

```gherkin
  @concern:recall-reaches-authority
  Scenario: A fresh agent's question becomes the purpose, its tooling is in reach, and what it read is receipted and bounded
    Given a ceremony whose declared source scope covers the tooling directory
    And the recall entry, the one command that opens an investigation, reads a bounded passage and reports a purpose and a byte count
    And a skill file in that directory naming the command that rebuilds a stale index
    And the standing description that entry states as its purpose when nobody brings a question
    And a declared packet limit wider than that skill passage and far narrower than the repository
    When a fresh agent with no prior ceremony context opens the recall entry carrying the question "Which command rebuilds the stale index now the old scripts are gone?"
    And it reads the passage of that skill file which names the command
    And it attempts the same read against a path outside the declared scope
    Then the investigation's stated purpose is that question, not the standing description
    And the skill passage is preserved as a receipt recording the exact bytes the agent read
    And the out-of-scope read was refused, so the declared scope is what put the skill in reach
    And the investigation's completion report names that same question, and the counted bytes stay under the packet limit
```

Six rounds, each with a FRESH reader. Blockers went **2 → 2 → 1 → 0 → 0 → 0**, verdicts
REVISE ×5 then **READY**. Every round's revision is in the file; none was waved off.

The one MAJOR round 6 raised and did not block on, left for the orchestrator to judge: the Given
`the recall entry, the one command that opens an investigation, reads a bounded passage and reports
a purpose and a byte count` reads as a *definition* of the entry's capabilities, and those are the
same capabilities the Then clauses prove — "Given it does X … Then it did X" has the shape of
circularity even though the Then proves those capabilities behaved CORRECTLY for this question. It
was added in round 5 precisely to answer the previous reader's complaint that "recall entry" was
narrative-only vocabulary, and its step is not decorative (it invokes `recall --help` and checks
that the entry's own usage names `open`, `read` and `finish`). Splitting the definition from the
state would satisfy both readers; it is one line of judgement, not a technical gap.

Two classes of blind-reader finding were DEFERRED rather than fixed, and the orchestrator should
decide them:

1. **A contrasting scenario for the harm.** Two readers asked for a second scenario showing what a
   dropped question *costs* (`When a fresh agent opens without naming a question / Then the purpose
   is the algorithm's own and the handover names no recipient`). The brief said ONE scenario and the
   habit's check names one journey, so the harm is shown as a Given (the generic purpose is now
   observed, not quoted) rather than as its own scenario. A second scenario would be the stronger
   story; it is a scope call, not a technical one.
2. **One Then step still carries its causal clause** — `the out-of-scope read was refused, so the
   declared scope is what put the skill in reach`. Round 4 wanted the clause split out; round 3 had
   demanded exactly this causal link be moved INTO the Gherkin, because without it the scope Given
   was not load-bearing. Round 5's structural half (the read is now a declared When) was taken; the
   trailing "so …" is kept on purpose — it is what makes the title's second promise provable to a
   reader of the steps alone.
3. **Pre-existing surrounding material.** "ceremony", "concern view" and "receipt" want one-line
   groundings in the feature's ORIGINAL narrative; scenario 2's immutability constraint lives in
   prose rather than in a Then; and the Feature preamble is now ~40 lines of prose that a reader must
   carry before the first scenario. All of these are edits to the `dev-system-equilibrium` scenarios
   and their narrative, which this seat did not own.

Lint: `pnpm exec eslint` and `prettier --check` are clean on the steps file (two `sonarjs/no-duplicate-string`
errors my additions pushed over threshold were fixed by naming `contractFile` and `questionFlag`).
`cucumber-js --profile collective-memory` — the memory-ceremony gate's other cucumber leg — is also
green (4 scenarios, 17 steps). The ceremony profile ends at **6 scenarios / 34 steps, all passing**.

## Files changed

```
.epr-meta/elohim/algorithms/recall-contract.json        version 4→5, .claude/ root, habit_register_bytes, 2 method lines, worktrees exclusion
elohim/eprfs/epr-cli/src/flow/memory/recall.rs          need→intent, FOREIGN_TREES refusal, human refusals, render(), discover_scored(), habit register, first screen
elohim/eprfs/epr-cli/src/flow/memory/mod.rs             usage() + help/bare-operation answer on stdout
elohim/eprfs/epr-cli/src/flow/memory/entries.rs         MEMORY.md HEADER names the native projection verb
elohim/eprfs/epr-cli/src/flow/mod.rs                    headline help text memkit: → recall:
elohim/eprfs/epr-cli/src/flow/report.rs                 HEADLINE_ORDER slot 0 recall, derivation, skipped wording, unit tests
elohim/eprfs/epr-cli/src/main.rs                        epr --help answers on stdout, exit 0
elohim/eprfs/epr-cli/tests/flow_memory_recall.rs        +5 tests
elohim/eprfs/epr-cli/tests/help_is_a_question.rs        NEW, 4 tests
elohim/eprfs/epr-cli/tests/flow_report.rs               recall measure+bound in the fixture registry, order test, +1 test
elohim/eprfs/epr-cli/tests/flow_memory_import.rs        re-baselined index bytes + digest (header change)
genesis/a2o/features/devflow/ceremony-reconciliation.feature   +1 scenario, +1 narrative paragraph
genesis/a2o/steps/devflow/ceremony-reconciliation.steps.ts     +10 steps, 2 extracted literals
```

## Gate

Every command run from `/projects/elohim` with
`env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1`
and `EXIT=$?` echoed on its own line (never read from piped output).

```
cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli \
  --test flow_memory_recall --test flow_concerns_corrections --test flow_memory_footprint
EXIT=0
  flow_concerns_corrections   9 passed; 0 failed
  flow_memory_footprint      10 passed; 0 failed
  flow_memory_recall         41 passed; 0 failed   (36 pre-existing + 5 new)

cargo test … --test help_is_a_question --test flow_report --test flow_report_surface_walk --test flow_memory_import
EXIT=0
  flow_memory_import         12 passed; 0 failed
  flow_report                96 passed; 0 failed
  flow_report_surface_walk    6 passed; 0 failed
  help_is_a_question          4 passed; 0 failed

cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --lib
EXIT=0
  186 passed; 0 failed

cargo fmt --all --manifest-path elohim/eprfs/Cargo.toml -- --check
EXIT=0

cargo clippy --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --all-targets -- -D warnings
EXIT=0

cargo build --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli
EXIT=0

cd genesis/a2o && EPR_BIN=/tmp/eprfs-gate-target/debug/epr pnpm exec cucumber-js --profile ceremony
EXIT=0
  6 scenarios (6 passed) · 34 steps (34 passed)

cd genesis/a2o && EPR_BIN=/tmp/eprfs-gate-target/debug/epr pnpm exec cucumber-js --profile collective-memory
EXIT=0
  4 scenarios (4 passed) · 17 steps (17 passed)

cd elohim/eprfs && cargo test --workspace          # the `_gate-eprfs` recipe's third leg
EXIT=101
  9 suites ok; ONE failure — flow_cites::live_corpus_migrate_reports_zero_pending (see below)
```

One suite in the crate is RED and it is not this work: `flow_cites::live_corpus_migrate_reports_zero_pending`
reports `pass 1: 1 id: slugs assigned (820 already had one)`, meaning exactly one document in the
cites corpus (`genesis/docs` + `.claude/memory`, per `cites.rs::doc_roots`) carries `cites:` or an
`id:`-bearing frontmatter but no slug. It was already failing before this seat's first edit (the
first full-crate run of the session reported the same `911 docs · 1 slug assigned`), and the only
`.md` this seat wrote — this report — carries an `id:`. Fixed by `epr flow cites migrate --apply`
on whichever lane added it.

## Test-drive — before and after

Command (repository root, no `--json`):

```
epr flow memory recall open --session <s> \
  --need 'How does the memory ceremony re-mine the MemPalace index after the kit was retired?'
```

BEFORE (pre-sprint binary): **14,061 bytes**, and the first line was

```
Intent: Find current authority and diagnostic wisdom for a fresh native reconciliation task without loading the full memory head.
```

— the recipe's purpose. The question was recorded nowhere. `Continuation:`, `Cumulative:`,
`Frontier:`, `Measurement:`, `Usage:` and the concerns `Page:` / `Omissions:` were seven
`to_string_pretty` blocks totalling ~5 KB of inline JSON between the reader and the Linked choices.

AFTER: **8,611 bytes** (−39%). First 40 lines:

```
Intent: How does the memory ceremony re-mine the MemPalace index after the kit was retired?
Scope: .
Worthwhile finish: A reviewed, evidence-backed outcome addressing the intent, with per-edge reconciliation and an explicit unresolved frontier. Responsible stopping is valid.
Guiding context: Sacred Attention: correct decisions and preserved uncertainty before byte savings — bafyreig7os3vfj4z63vgy7lpfgcnzzr2z26vrvdtheapoegvgzhnnhxkr4
Guiding context: Governed views select assertions and share evidence without creating another queue — bafyreieicbex3dkkgounj4latxya32ezpl6622lpij327mzndwptfqkf7q

Concerns:
Measured scope: 45 stale · 0 dangling · 23 group(s) · 809 edge(s) indexed
Page: 12 shown from offset 0 · 33 omitted
Shared source: genesis/docs/PLACEMENT.md
  1. genesis/docs/superpowers/plans/2026-06-02-subject-routed-decomposition-plan.md [stale, doc] — current subject-class home contract; the originally proposed section is present, without proving gate implementation
  2. genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md [stale, doc] — the contract defining the three doc homes this loop tends toward stasis
  3. genesis/docs/superpowers/specs/2026-06-01-verification-result-index-design.md [stale, doc] — the contract whose four verification states this index records rather than forks
  4. genesis/docs/superpowers/specs/2026-06-02-scope-tree-reconciler-design.md [stale, doc] — the contract this proposes held/ doctrine and requires_env capability vocabulary for
  5. genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md [stale, doc] — the contract whose three doc homes these content-addressed links survive moves between
  6. genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md [stale, doc] — the contract whose retired-language and doc homes this loop proposes six edits to
  7. genesis/docs/superpowers/specs/2026-06-02-subject-routed-decomposition-design.md [stale, doc] — current subject-class home contract; the originally proposed section is present, without proving gate implementation
  8. genesis/docs/superpowers/specs/2026-06-25-doc-lifecycle-as-epr-development-substrate-design.md [stale, doc] — Genesis Docs Placement Contract
  9. genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md [stale, doc] — Genesis Docs Placement Contract
Shared source: genesis/docs/analysis/2026-07-09-epr-meta-eprfs-elohim-native-sotu.md
  10. genesis/docs/superpowers/plans/2026-07-10-epr-meta-native-capability-dogfood-and-graph-plan.md [stale, doc] — EPR Meta / EPRFS / Elohim-Native Capability SOTU
Shared source: genesis/docs/architecture/private-thought-governed-fruit.md
  11. genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md [stale, doc] — private-thought-governed-fruit
Shared source: genesis/docs/content/elohim-protocol/architecture/2026-08-25-doorway-auth-posture-declared-stage.md
  12. genesis/docs/content/elohim-protocol/architecture/2026-08-25-doorway-auth-refusal-runbook.md [stale, doc] — the canon half of this pair — the rule this runbook operationalizes; read it for WHY a refusal is shaped this way, come back here for what to do about one
Selection: Incident edges with native stale or dangling verdict; group by shared upstream source. Held/governed/ok counted but excluded.
Omissions:
  · Only registry-declared sealed doc citations and native sidecar edges are indexed; unsealed references and undeclared document families are outside this view.
  · Counts cover incident edges in the declared scope, not all repository assertions. Paging bounds output, not graph scan or source bytes.
  · Each page recomputes the current graph; offsets are not snapshot identities. Restart paging after changing source or native records.
  · No semantic ranking or substantive repair/conflict judgment is inferred. Open each consumer's native context for assertion, review and acceptance records; it…
  · 7 sidecar slot(s) withdrawn by retraction and excluded from this view: .claude/scripts/memory-kit/recall-packet.py →…

Continuation: 0 finding(s) · 0 evidence · 0 question(s) · next: choose a concern

Accounting: attempt 1 · 0 source bytes · 12,597 native bytes · 1.9s · unmetered 0

Frontier: 0 unresolved question(s)

Measurement: baseline 3,542,321 B over 301 file(s) sampled; receipt 64,811 B
```

The FOCUSED door, same binary, `--scope genesis/docs/superpowers/specs --need 'Does
recall-reaches-authority hold and what is the compose-gate design?'` — rendered before the concern
groups:

```
Area: genesis/docs/superpowers/specs
Habit: recall-reaches-authority [red] — RED WRITTEN 2026-09-11 (born red; the orchestrator's own bootstrapping into this sprint is the first journey sample — 14 probe rounds and ~100 KB of unmetered direct reads before the concern's shape was nameable, 1 mistaken assertion…
Candidate sources (local, bounded filesystem traversal ranked by declared-metadata term overlap; candidate discovery, not authority):
  1. genesis/docs/superpowers/specs/2026-05-01-atproto-lexicon-projection-doorway-design.md — AT Protocol Lexicon Projection at Doorway
  2. genesis/docs/superpowers/specs/2026-06-01-verification-result-index-design.md — Verification-Result Index — the system→state store that closes the stasis loop
  3. genesis/docs/superpowers/specs/2026-06-15-node-resource-tunables-and-exhaustion-shape-design.md — 2026-06-15-node-resource-tunables-and-exhaustion-shape-design.md
  · metadata read failed: YAMLError
```

Refusals, human mode:

```
$ epr flow memory recall read --session s --path .claude/worktrees/sprint-x/notes.md --lines 1:5
refused: invalid arguments: `.claude/worktrees/sprint-x/notes.md` is another checkout's working tree, not this repository's declared source. Name the path inside this tree that carries the same concern.
next: `.claude/` is in scope so the tooling layer is reachable, but a sibling checkout is not this repository's source. Name the path inside this tree.
EXIT=2

$ epr flow memory recall read --session s --path /etc/hosts --lines 1:5
refused: invalid arguments: outside declared source scope
next: Name a path inside the contract's declared source_roots; `epr flow memory recall recipe` lists them.
EXIT=2
```

And the headline's first line, live:

```
$ epr flow report --headline
  recall: ⚠ failed — 100000 bytes has reached the hard watermark 20000
  mempalace: ⚠ failed — …
```

## For the orchestrator — `just gate memory-ceremony` is RED on three hook assertions this seat cannot edit

`just gate memory-ceremony` runs `.claude/hooks/__tests__` and the live index parity check, both
outside this seat's write set. I ran that leg directly
(`EPR_BIN=/tmp/eprfs-gate-target/debug/epr CARGO_TARGET_DIR=/tmp/eprfs-gate-target python3 -m unittest
discover -s .claude/hooks/__tests__ -p '*_test.py'` → `Ran 34 tests … FAILED (failures=3, skipped=1)`).
These are the exact three, read from the live files, not from the brief's line numbers:

1. **`drift_observation_test.py:543-546`**,
   `GoldenReportCase.test_all_five_headline_slots_render_without_any_bridged_fold`. The literal
   tuple `("memkit", "mempalace", "cleanup", "scope", "memory-budget")` becomes
   `("recall", "mempalace", "cleanup", "scope", "memory-budget")`. Order and position are unchanged;
   only slot 0's label moves. The docstring's sentence "and `memkit` is a RETIRED bound" should say
   that `recall` renders the last journey fold, or `skipped — no journey fold` when none exists.
   Observed failure: ``AssertionError: False is not true : no `memkit:` line in: recall: skipped — no journey fold …``

2. **`drift_observation_test.py:548-554`**,
   `GoldenReportCase.test_a_retired_bound_says_retired_not_skipped`. It calls
   `self.line("memkit")`, which now finds nothing, because `memkit` left `HEADLINE_ORDER` and the
   headline no longer prints a line for it. The retired bound is STILL retired and `report.rs` still
   resolves its label (`slot_prefix("memkit") == "memkit"`), so the claim is still true — it has to
   be read from the `--json` retired list (`payload.retired()` / the `retired` array) rather than
   from the headline text. The equivalent-in-spirit headline assertion is that slot 0 reads
   `recall: skipped — no journey fold` when no journey has been folded.

3. **`memory_index_projection_test.py:267`**,
   `GoldenParityCase.test_the_native_projection_equals_the_index_on_disk`. This one is caused by the
   coordinator's `entries.rs` `HEADER` fix and is **fixed by re-projecting, not by editing the
   test**: the live `.claude/memory/MEMORY.md` still carries the deleted-script instruction, so
   re-projection renders different bytes than the tree holds. Run
   `epr flow memory project --index --budget memory-index-bytes@1 --out .claude/memory/MEMORY.md`
   (a `.claude/**` write) and the assertion becomes true again.

The Rust half of that gate recipe (`cargo build`, then
`--test flow_memory_recall --test flow_concerns_corrections --test flow_memory_footprint`) and both
cucumber legs are green — see the gate block above.

**Pre-existing sessions need `adopt`.** The contract version bump `4 → 5` changes the method CID
every session and receipt pins, so any session opened under version 4 will refuse with
`algorithm bytes changed`. The remedy line already says it:
`epr flow memory recall adopt --from-session <old> --session <new>`. The receipts are intact;
adoption carries both the method and executor digests forward with their prior accounting.

**Re-project `MEMORY.md`.** The header template moved; the live file still carries the deleted-script
instruction until `epr flow memory project --index --budget memory-index-bytes@1 --out
.claude/memory/MEMORY.md` is run. That write is `.claude/**`.

## Left undone, and why

1. **`discover` aborts its whole traversal on the first unparseable frontmatter** — **RESOLVED in
   Round 2 below** (coordinator ruled skip-and-name; implemented, tested, contract bumped 5 → 6).
   The original finding is kept verbatim because Round 2's rationale is only legible against it.

   *(original)*
   (`frontier.push("metadata read failed: YAMLError"); break 'outer`). That is a deliberate honesty
   invariant — a window that could not read every candidate is not a trustworthy window — but in the
   live corpus it caps the focused screen's ranked sources badly. `genesis/docs/superpowers/specs/`
   alone holds **7** documents whose frontmatter neither `serde_yaml` nor PyYAML parses
   (`2026-06-26-che-keyless-governed-peer-client-design.md`,
   `2026-08-10-evidence-ladder-push-left-design.md`,
   `2026-07-31-care-aggregation-adoption-policy-floor-design.md`,
   `2026-06-20-weave-epic-arc-design.md`,
   `2026-07-15-frame-witness-primitive-architecture-design.md`,
   `2026-08-11-measure-dynamics-confidence-ontology-design.md`,
   `2026-06-14-elohim-substrate-passes/VISION-RECURSION-ai-covenant-recursion-2026-06-14.md`), most
   reading as an unquoted `key: value: value`. Either the corpus is repaired, or `discover` learns
   to SKIP-and-name a bad row instead of aborting — that is a change to the contract's stated
   honesty rule and belongs to whoever owns the contract, not to this seat. **The ranked-source leg
   of station #5 is real but under-powered until this is decided.** The fixture test proves the
   ranking; the live corpus is where it is blunted.

2. **The whole-scope test-drive shows no first screen**, correctly: "memory ceremony re-mine
   MemPalace index" touches no habit id and names no directory, so it is a convergence question and
   gets the grouped view. A reader wanting the focused door must name `--scope` or a path. Whether
   habit *invariant* matching should be looser (today a habit needs three term hits in its invariant,
   or one in its id) is a tuning call I made conservatively — a loose matcher that shows an unrelated
   habit on every open is worse than one that shows none.

3. **No `recall measure --phase close` fold was written** for this session, so the habit's third
   check is still unproven by this seat. The four recall-journey middot need a *fresh reader's*
   honest account, and per the habit's own guard clause (3) "a fresh-reader observation taken by the
   agent that made the change is not a fresh reader" — recording my own numbers would have been the
   exact regression the guard names.

4. **`retrieval` (the `search` operation's payload) is still a JSON block in human mode.** It is the
   answer of that operation, the way `Linked choices` is of `open`, and compacting it was not asked
   for. Named here so it is a decision rather than an oversight.

---

# Round 2 — skip-and-name in `discover` (coordinator ruling, 2026-09-11)

Frontier item 1 above was ruled: an unparseable candidate is **skipped, counted and named**, never a
reason to abandon the traversal. Implemented as specified.

## What changed

**`discover_scored` in `recall.rs`.** The three frontmatter *parse* failures — invalid UTF-8, a
boundary the metadata budget could not prove, and a YAML error — each pushed a frontier line and
`break 'outer`. They now push the candidate's repository-relative path onto an `unreadable` list and
`continue`. The traversal runs to its ordinary budget.

The account is made in two places, because a reader who scans only the frontier and a reader who
reads the whole view must both learn of it:

- **omissions** — one line, the wording the ruling specified:
  `N candidate(s) unreadable (frontmatter did not parse) and were skipped: <path>, <path>…`, the
  list capped at the first 8 paths then `+M more`. Sorted, so the same corpus names the same paths
  in the same order.
- **frontier** — `unreadable candidates: N`. And when a read actually hit its cap (`data.len() >=
  budget`), a second line: `some candidate frontmatter exceeded the metadata budget; raise
  limits.metadata_bytes to read it`. That distinction survived the change deliberately: a document
  the budget truncated and a document that is malformed have different remedies, and collapsing
  them into one line would have lost the one the reader can act on.
- The result also carries `unreadable_candidates: N` so a `--json` caller reads a number rather
  than parsing prose, and `omissions: []` when nothing was skipped — the account appears only when
  there is something to account for.

**The first screen surfaces it.** `first_screen` carries the discovery's `omissions` alongside its
`unresolved`, and the human renderer prints both as bullets under the candidate list, so a short
ranked list that is short *because of malformed documents* says so rather than reading as a thin
corpus.

**Unchanged on purpose, and named here so it is a decision:** the two I/O aborts (`read_dir`
failing on a directory, and `File::open`/read failing on a candidate) still stop the traversal. The
ruling's wording is "a candidate whose frontmatter fails to parse", and a filesystem that will not
answer is a different class from a document that will not parse — it says the traversal itself is
unreliable, not that one document is. Likewise the two pre-existing SILENT skips (frontmatter that
parses to something other than a mapping; a `tags:` field that is not a list of strings) are
untouched: those are documents that parsed and did not qualify, which an existing assertion pins as
non-membership rather than non-readability.

**Contract `version` 5 → 6.** `discovery.unreadable_candidates` states the rule (skip, count, name;
never silently drop; never abandon the traversal) and records why the abort it replaces satisfied
the same honesty rule while making the window useless. The method CID moved again — **a session
opened under version 5, including the one this report's first test-drive used, now needs
`adopt`** exactly as the 4 → 5 bump required.

## Tests

`tests/flow_memory_recall.rs`, two new plus one amended (43 in the suite, was 41):

- `an_unparseable_candidate_is_skipped_counted_and_named_not_a_full_stop` — the bad document sits
  BETWEEN two good ones in sort order, so an abort loses one of them and a silent skip loses the
  account. Asserts both good paths return, `unreadable_candidates == 1`, the omission names
  `docs/2-bad.md`, and the frontier carries `unreadable candidates: 1`. Then an all-bad scope:
  zero candidates, non-empty omissions, non-empty frontier. Then a clean scope: zero, empty, empty
  — the account appears only when something was skipped.
- `the_first_screen_names_the_candidates_it_could_not_read` — a focused `open` whose ranked list is
  shortened by a malformed sibling carries the omission naming it, and still returns the readable
  candidate.
- `only_a_complete_frontmatter_boundary_establishes_membership` amended: it asserted the old abort
  line. It now asserts the skip account AND that the budget-cut remedy line is still emitted, so
  the distinction the ruling could have erased stays pinned.

## Gate (round 2)

```
cargo test -p elohim-epr-cli --test flow_memory_recall --test flow_concerns_corrections --test flow_memory_footprint
EXIT=0
  flow_concerns_corrections   9 passed; 0 failed
  flow_memory_footprint      10 passed; 0 failed
  flow_memory_recall         43 passed; 0 failed

cargo test -p elohim-epr-cli --test help_is_a_question --test flow_report --test flow_report_surface_walk --test flow_memory_import
EXIT=0
  12 / 96 / 6 / 4 passed; 0 failed

cargo test -p elohim-epr-cli --lib                             EXIT=0   (186 passed)
cargo fmt --all -- --check                                     EXIT=0
cargo clippy -p elohim-epr-cli --all-targets -- -D warnings    EXIT=0
cargo build -p elohim-epr-cli                                  EXIT=0
cucumber-js --profile ceremony                                 EXIT=0   (6 scenarios, 34 steps)
cucumber-js --profile collective-memory                        EXIT=0   (4 scenarios, 17 steps)
cd elohim/eprfs && cargo test --workspace                      EXIT=0   (55 suites ok, 0 failed)
```

The whole eprfs workspace is now green: `flow_cites::live_corpus_migrate_reports_zero_pending`,
red in round 1, has cleared — `epr flow cites migrate` reports `0 id: slugs assigned (821 already
had one)`, so the lane that owned the un-slugged document fixed it.

## Test-drive (round 2) — the focused door on the tooling layer

```
epr flow memory recall open --session round2-focus \
  --need 'How does the memory ceremony re-mine the MemPalace index after the kit was retired?' \
  --scope .claude/skills
```

3,800 bytes, EXIT=0. The first screen:

```
Intent: How does the memory ceremony re-mine the MemPalace index after the kit was retired?
Scope: .claude/skills
Worthwhile finish: A reviewed, evidence-backed outcome addressing the intent, with per-edge reconciliation and an explicit unresolved frontier. Responsible stopping is valid.
Guiding context: Sacred Attention: correct decisions and preserved uncertainty before byte savings — bafyreig7os3vfj4z63vgy7lpfgcnzzr2z26vrvdtheapoegvgzhnnhxkr4
Guiding context: Governed views select assertions and share evidence without creating another queue — bafyreieicbex3dkkgounj4latxya32ezpl6622lpij327mzndwptfqkf7q

Area: .claude/skills
Habits: none in the register match this question's terms
Candidate sources (local, bounded filesystem traversal ranked by declared-metadata term overlap; candidate discovery, not authority):
  1. .claude/skills/mem-horizon-scan/SKILL.md — SKILL.md
  2. .claude/skills/memory-ceremony/SKILL.md — SKILL.md
  3. .claude/skills/delivery-stasis/SKILL.md — SKILL.md
  · 1 candidate(s) unreadable (frontmatter did not parse) and were skipped: .claude/skills/converge/SKILL.md
  · discovery budget exhausted; narrow the scope or continue with a named question
  · unreadable candidates: 1

Concerns:
Measured scope: 0 stale · 0 dangling · 0 group(s) · 0 edge(s) indexed
Page: 0 shown from offset 0 · 0 omitted
```

The agent asked about re-mining the MemPalace index and the second-ranked source is the
memory-ceremony skill — reached through the entry, metered, with a `read` command attached. Before
this round the same call returned no candidates at all.

The direct before/after on the scope round 1 recorded as blunted, same command both times
(`--scope genesis/docs/superpowers/specs --need 'Does recall-reaches-authority hold and what is the
compose-gate design?'`):

```
BEFORE   Candidate sources: none in scope matched this question's terms
           · metadata read failed: YAMLError

AFTER    Candidate sources (local, bounded filesystem traversal ranked by declared-metadata term overlap; candidate discovery, not authority):
           1. genesis/docs/superpowers/specs/2026-05-01-atproto-lexicon-projection-doorway-design.md — AT Protocol Lexicon Projection at Doorway
           2. genesis/docs/superpowers/specs/2026-05-10-converge-skill-design.md — 2026-05-10-converge-skill-design.md
           3. genesis/docs/superpowers/specs/2026-06-01-coherence-substrate-design.md — 2026-06-01-coherence-substrate-design.md
           · 1 candidate(s) unreadable (frontmatter did not parse) and were skipped: genesis/docs/superpowers/specs/2026-06-26-che-keyless-governed-peer-client-design.md
           · result window reached; remaining corpus not inspected
           · unreadable candidates: 1
```

The traversal now ends because it FILLED its result window, not because one document stopped it —
and the document it could not read is named, which is the thing the abort was protecting.

## Files changed in round 2

```
.epr-meta/elohim/algorithms/recall-contract.json        version 5→6, discovery.unreadable_candidates rule
elohim/eprfs/epr-cli/src/flow/memory/recall.rs          discover_scored skip-and-name; first_screen + render carry omissions
elohim/eprfs/epr-cli/tests/flow_memory_recall.rs        +2 tests, 1 amended
```

Nothing under `.claude/**` or `genesis/docs/superpowers/specs/**` was touched: the seven malformed
documents are the ceremony door's corpus work, and `.claude/skills/converge/SKILL.md` — which this
round's own test-drive names as unreadable — belongs to the integration lane. They are now visible
to whoever owns them instead of silently capping every search.

---

# Round 3 — the cut character, and the body the read already covered

Both defects were inside the write set and both are fixed. The second fix works as specified and,
**measured on the reader's own question, is not sufficient** — the reason is a number, and it is
below.

## Defect (1) — a cut character is the reader's fault, not the document's

Confirmed exactly as reported. `.claude/skills/converge/SKILL.md` carries 708 bytes of valid
frontmatter in a 19,886-byte file; byte 8190 begins an em dash (`\xe2\x80\x94`); the
`limits.metadata_bytes`=8192 read ended inside it; `String::from_utf8` failed; discovery filed the
whole document under "frontmatter did not parse". A bounded read ends mid-character routinely — that
is what a byte budget does — so the failure was structural, not incidental.

**Fix.** After the bounded read the tail is trimmed back to the last complete UTF-8 boundary
(`Utf8Error::valid_up_to`) and only then parsed. The frontmatter itself is still exact bytes: the
trim can only ever remove tail, and if it lands *before* the closing delimiter the header cannot be
proven and the row is reported as an unproven boundary — which is true — rather than as malformed.
A file whose very first byte is invalid is still named unreadable.

Verified live: `search --search-scope .claude/skills/converge` now returns
`.claude/skills/converge/SKILL.md` with `unreadable_candidates: 0` and `omissions: []`.

Test: `a_character_cut_by_the_metadata_budget_does_not_condemn_the_document` builds a document whose
em dash starts two bytes before the cap, asserts the candidate is returned, asserts
`metadata_fingerprint == sha256(exact header bytes)`, and asserts that a term *past* the window is
still invisible — the bound is still a bound.

### The seven `genesis/docs/superpowers/specs/` files, re-checked under the fix

They are boundary-clean at 8192 as you said, and **six of the seven still fail, all for the same
single cause**: a `cites:` list entry is an unquoted YAML scalar containing `: ` (or a leading `@`),
so the parser reads it as a nested mapping. Not edited — quoted verbatim, path, line, column and
the parser's own words:

| file | line:col | parser error | the offending text |
|---|---|---|---|
| `2026-06-26-che-keyless-governed-peer-client-design.md` | 17:187 | `mapping values are not allowed here` | `- rea-compute-commitment-primitive \| … \| path: genesis/docs/architecture/rea-compute-commitment-primitive.md` |
| `2026-08-10-evidence-ladder-push-left-design.md` | 14:75 | `found character '@' that cannot start any token` | `- scope-tree-reconciler-design \| Supplies the tier-position vocabulary: @requires: tags + cluster-state.yaml …` |
| `2026-07-31-care-aggregation-adoption-policy-floor-design.md` | 10:259 | `mapping values are not allowed here` | `- observer-protocol \| … \| path: genesis/docs/content/elohim-protocol/observer-protocol.md` |
| `2026-06-20-weave-epic-arc-design.md` | 15:164 | `mapping values are not allowed here` | `- dht-is-a-notary-not-a-byte-store \| the binding constraint: capacity/rollup aggregation is gossip+projection …` |
| `2026-07-15-frame-witness-primitive-architecture-design.md` | 20:207 | `mapping values are not allowed here` | `- elohim-seam-map-concern-routing \| the routing: SDK-seam (envelope type + frame data …) \| path: …` |
| `2026-08-11-measure-dynamics-confidence-ontology-design.md` | 13:281 | `mapping values are not allowed here` | `- middot-measure-primitive-design \| Extends: measure family kind/rate vocabulary generalizes …` |
| `2026-06-14-elohim-substrate-passes/VISION-RECURSION-…-2026-06-14.md` | 12:73 | `expected <block end>, but found '<scalar>'` | `- confession.md (the two AI sections: "The elohim are already fallen" :93-101; …)` |

Every one is a `cites:` entry that would parse if quoted. They are named in the view's omissions
line now instead of capping the search, which is what this sprint owed them; the repair is the
ceremony door's corpus work.

## Defect (2) — the body was read and never looked at

Confirmed and fixed as specified. Query terms are now matched against the declared frontmatter AND
the body bytes the **same bounded read already covered** — no second read, no wider budget. Each
candidate reports where its terms were found (`match: ["title","body"]`, plus `declared_hits` and
`term_hits`), the human view prints `[matched in body, description]` after each row, and the
receipt, omissions and "candidate, not authority" wording are untouched.

Two interpretation decisions, named so they are decisions:

- **"Frontmatter above body" is scored per term**, not as a lexicographic override: a declared hit
  is worth 2, a body-only hit 1, summed. Your test — "a body-only hit ranks below a frontmatter hit
  *for the same term*" — holds exactly. A strict override would have ranked one hit on a word every
  document shares above five hits on the words that actually name the concern, which is the failure
  this round exists to fix.
- **The result window no longer closes early when terms are given.** It used to stop at
  `search_results × 4`; once body text became searchable almost every document matched *something*,
  so that made the ranking a ranking of whatever the traversal reached first. Every row the scan
  budget allows is now collected, ranked, then truncated to `search_results`. The traversal is
  bounded by `scan_bytes`/`scan_files`/`scan_entries`/`scan_seconds` exactly as before — only the
  result window moved, and the frontier still names the budget that stopped the walk.

Contract `version` 6 → 7; `discovery.bounded_prefix_match` and
`discovery.boundary_cut_is_not_a_parse_failure` state both rules. **Sessions on v6 now need
`adopt`** — the third method move this sprint.

Tests: `a_term_carried_only_by_the_body_is_found_and_ranked_under_a_declared_hit` (both documents
returned; declared first with `match: ["title"]`, body second with `match: ["body"]`,
`declared_hits: 0`, `term_hits: 1`; the plain `--query` path — the one the fresh reader actually
used — reaches the body too; and with `scan_files: 1` the traversal still stops and still names
`budget exhausted`). One existing test was amended with its reason recorded:
`provider_substitution_and_refusal_preserve_intent` used `ok()` for `search --query purpose`, which
now matches all three fixture bodies (`# Purpose`), fills the three-result window and exits 2 on an
honest frontier line — it uses `view()` and asserts the three candidates.

## The measured verdict on the reader's journey: still short, and here is the number

```
epr flow memory recall open --session r3-drive \
  --need 'what commands re-mine the MemPalace index and when may the marker be stamped' \
  --scope .claude/skills
```

3,803 bytes, EXIT=0. First 25 lines:

```
Intent: what commands re-mine the MemPalace index and when may the marker be stamped
Scope: .claude/skills
Worthwhile finish: A reviewed, evidence-backed outcome addressing the intent, with per-edge reconciliation and an explicit unresolved frontier. Responsible stopping is valid.
Guiding context: Sacred Attention: correct decisions and preserved uncertainty before byte savings — bafyreig7os3vfj4z63vgy7lpfgcnzzr2z26vrvdtheapoegvgzhnnhxkr4
Guiding context: Governed views select assertions and share evidence without creating another queue — bafyreieicbex3dkkgounj4latxya32ezpl6622lpij327mzndwptfqkf7q

Area: .claude/skills
Habits: none in the register match this question's terms
Candidate sources (local, bounded filesystem traversal ranked by term overlap over declared metadata and the bounded body prefix already read, declared hits first; candidate discovery, not authority):
  1. .claude/skills/plant-eprfs-hook/SKILL.md — SKILL.md [matched in body, description]
  2. .claude/skills/plant-eprfs-command/SKILL.md — SKILL.md [matched in body, description]
  3. .claude/skills/agentic-developer/SKILL.md — SKILL.md [matched in body, description]
  · discovery budget exhausted; narrow the scope or continue with a named question

Concerns:
Measured scope: 0 stale · 0 dangling · 0 group(s) · 0 edge(s) indexed
Page: 0 shown from offset 0 · 0 omitted
Selection: Incident edges with native stale or dangling verdict; group by shared upstream source. Held/governed/ok counted but excluded.
Omissions:
  · Only registry-declared sealed doc citations and native sidecar edges are indexed; unsealed references and undeclared document families are outside this view.
  · Counts cover incident edges in the declared scope, not all repository assertions. Paging bounds output, not graph scan or source bytes.
  · Each page recomputes the current graph; offsets are not snapshot identities. Restart paging after changing source or native records.
  · No semantic ranking or substantive repair/conflict judgment is inferred. Open each consumer's native context for assertion, review and acceptance records; it…

Continuation: 0 finding(s) · 0 evidence · 0 question(s) · next: choose a concern
```

**The memory-ceremony skill is still not on that list, and no ranking change can put it there.**
Measured, not inferred:

| fact | value |
|---|---|
| `.claude/skills/memory-ceremony/SKILL.md` size | 24,273 bytes |
| first occurrence of `re-mine` | byte 22,903 |
| first `mempalace --palace …` command | byte 22,922 |
| first `.last-mine` (the marker) | byte 23,267 |
| per-file read window (`limits.metadata_bytes`) | 8,192 bytes |
| reader terms present in that 8,192-byte prefix | `commands` ×1, `index` ×8 — and **none** of `re-mine`, `mempalace`, `marker`, `stamped` |
| scan over `.claude/skills` | 41 of 44 files, `scan_bytes` 262,144 of 262,144 — budget exhausted |

The answer is in the last 6% of the file. Searching "the bytes already read" cannot see it, because
those bytes do not contain the question's words. Scoped straight at the file, it scores
`declared_hits: 0, term_hits: 2, match: ["body"]` — it is *found*, ranked and labelled, but on
`commands`/`index`, the two words shared by forty other skills.

The mechanism itself is verified working where the evidence is inside the window: over
`.claude/skills/memory-ceremony`, the queries `warranted`, `assertion` and `next agent` all return
the file, and none of those words appears in its frontmatter. Before this round, all three returned
nothing.

### What would actually close the reader's journey — your call, not mine

*(Resolved in Round 4: the coordinator declared option 1, and option 2 proved necessary alongside
it — the widened window alone still left the answering document ranked fifth.)*

You said "within the existing budgets", so I did not move one. Three options, in the order I would
rank them:

1. **A declared body-scan budget.** `metadata_bytes` is named for metadata and 8 KB is right for
   it; a separate `body_scan_bytes` (and a matching rise in `scan_bytes`) would let discovery read
   the whole of a document of ordinary size. `.claude/skills` is 44 files totalling well under
   1 MB — the same order as the `habit_register_bytes: 1048576` this sprint already added, and
   sub-second. This is the only option that makes the entry answer the question the reader asked.
2. **Rarity-weighted ranking (IDF over the scanned window).** Cheap, no extra reads, and it would
   stop `commands`/`index` from outranking `mempalace`/`re-mine`. It improves every ranked screen —
   but it cannot help *this* question, because the rare words are not in the window at all. Worth
   doing for its own sake; not a fix for the measured defect.
3. **Accept the bound and say so louder.** When every returned candidate is `body`-only on common
   terms, the frontier could say "no candidate matched the distinctive terms of this question
   within the per-file window" instead of only "budget exhausted" — turning a thin answer into a
   named limit, which is what the reader needed to know before concluding the question was
   unanswerable. This is cheap and I can do it on a word from you.

## Gate (round 3)

```
cargo test -p elohim-epr-cli --test flow_memory_recall --test flow_concerns_corrections --test flow_memory_footprint
EXIT=0
  flow_concerns_corrections   9 passed; 0 failed
  flow_memory_footprint      10 passed; 0 failed
  flow_memory_recall         45 passed; 0 failed

cargo test -p elohim-epr-cli --test help_is_a_question --test flow_report --test flow_report_surface_walk --test flow_memory_import
EXIT=0
  12 / 96 / 6 / 4 passed; 0 failed

cargo test -p elohim-epr-cli --lib                             EXIT=0   (186 passed)
cargo fmt --all -- --check                                     EXIT=0
cargo clippy -p elohim-epr-cli --all-targets -- -D warnings    EXIT=0
cargo build -p elohim-epr-cli                                  EXIT=0
cucumber-js --profile ceremony                                 EXIT=0   (6 scenarios, 34 steps)
cucumber-js --profile collective-memory                        EXIT=0   (4 scenarios, 17 steps)
```

## Files changed in round 3

```
.epr-meta/elohim/algorithms/recall-contract.json   version 6→7; discovery.bounded_prefix_match + .boundary_cut_is_not_a_parse_failure
elohim/eprfs/epr-cli/src/flow/memory/recall.rs     UTF-8 boundary trim; body-prefix matching with match kinds; per-term scoring; result window opened when terms are given; human row shows where it matched
elohim/eprfs/epr-cli/tests/flow_memory_recall.rs   +2 tests, 1 amended (45 in the suite)
```

Nothing under `.claude/**` or `genesis/docs/superpowers/specs/**` was touched.

---

# Round 4 — the budget is declared, and the reader's question is answerable

The window was the binding constraint and it is now a declared number rather than a workaround.
**The memory-ceremony skill appears ranked with `[matched in body]`**, and following the choices the
screen hands over reaches the exact passage.

## The declared budget

`recall-contract.json` `limits`, version **7 → 8**:

| limit | was | is | what it bounds |
|---|---|---|---|
| `body_scan_bytes` | — | 65,536 | per-file window term discovery may read FOR MATCHING |
| `scan_bytes` | 262,144 | 2,097,152 | the traversal as a whole |
| `scan_seconds` | 3 | 5 | the traversal as a whole |
| `metadata_bytes` | 8,192 | 8,192 | unchanged — the frontmatter read, still byte-exact |

Three properties hold and are stated in `discovery.body_scan_window`:

- **A wider body window never widens what establishes membership.** The header is still parsed from
  `metadata_bytes` of exact bytes; `metadata_fingerprint` is unchanged. `discover` reads a wider
  slice and then looks at only the first 8 KB of it to decide whether the document is a candidate
  at all.
- **The window is spent only when there is something to match.** A pure tag filter (`source --tag`
  with no query) still reads `metadata_bytes` per file, so tag discovery keeps its file coverage
  instead of trading 256 files for 32.
- **Discovery bytes are scan bytes, never evidence.** Verified on the live run below: the ranked
  screen charged `scan_bytes` and `scanned_files`, `source_bytes: 0`; the `read` that quoted the
  answer charged `source_bytes: 844`. An excerpt receipt is still the only thing that quotes a
  document.

A candidate larger than the window is scanned to it and **counted**: the omissions line says
`N candidate(s) scanned to the body-scan window only`, and `partially_scanned_candidates` carries
the number for `--json`. A term past the window is invisible, which is not evidence of absence.

## One more change, because the budget alone did not do it

With the window widened, the whole of `.claude/skills` traversed in one call (56 files, 696 KB,
1.6 s, no budget exhausted) — and the memory-ceremony skill still came **fifth**. The reason was the
ranking, measured:

| term | documents carrying it (of 44) |
|---|---|
| `when` | 43 |
| `commands` | 15 |
| `index` | 14 |
| `marker` / `stamped` | 4 |
| `mempalace` / `re-mine` | 3 |

Counting matched terms equally made `when` — carried by 43 of 44 — worth the same as `mempalace`,
carried by 3, and a declared `when` in a description worth twice a body hit on `mempalace`. So the
ranking is now **occurrence- and rarity-weighted**, over the window this call actually read:

```
score = Σ over matched terms of  (1 + ln occurrences) × max(ln(N / documents-carrying-it), 0.01) × (declared ? 2 : 1)
```

- **Rarity** is computed from the scanned window itself, so it needs no corpus statistics kept
  anywhere and cannot go stale.
- **Occurrences** are sublinear — the tenth mention says less than the second.
- **Declared × 2 survives intact**: a declared hit still outranks a body hit *on the same term*,
  which is the invariant round 3 set and its test still pins.
- The floor (`0.01`) keeps a word every candidate shares contributing a little rather than nothing,
  so a reader's ordinary words are not silently discarded.

Each candidate now reports `matched_terms: { term: { found_in, occurrences } }`, so the ranking's
evidence is in the view rather than in this document.

## The reader's journey, end to end

```
epr flow memory recall open --session r4-final \
  --need 'what commands re-mine the MemPalace index and when may the marker be stamped' \
  --scope .claude/skills
```

3,802 bytes, EXIT=0. First 25 lines:

```
Intent: what commands re-mine the MemPalace index and when may the marker be stamped
Scope: .claude/skills
Worthwhile finish: A reviewed, evidence-backed outcome addressing the intent, with per-edge reconciliation and an explicit unresolved frontier. Responsible stopping is valid.
Guiding context: Sacred Attention: correct decisions and preserved uncertainty before byte savings — bafyreig7os3vfj4z63vgy7lpfgcnzzr2z26vrvdtheapoegvgzhnnhxkr4
Guiding context: Governed views select assertions and share evidence without creating another queue — bafyreieicbex3dkkgounj4latxya32ezpl6622lpij327mzndwptfqkf7q

Area: .claude/skills
Habits: none in the register match this question's terms
Candidate sources (local, bounded filesystem traversal ranked by term overlap over declared metadata and the bounded body window already read, each term weighted by how often it occurs and how rare it is in that window, a declared hit worth twice a body hit; candidate discovery, not authority):
  1. .claude/skills/agentic-developer/SKILL.md — SKILL.md [matched in body, description]
  2. .claude/skills/memory-ceremony/SKILL.md — SKILL.md [matched in body]
  3. .claude/skills/plant-eprfs-agentdoc/SKILL.md — SKILL.md [matched in body, description]

Concerns:
Measured scope: 0 stale · 0 dangling · 0 group(s) · 0 edge(s) indexed
Page: 0 shown from offset 0 · 0 omitted
Selection: Incident edges with native stale or dangling verdict; group by shared upstream source. Held/governed/ok counted but excluded.
Omissions:
  · Only registry-declared sealed doc citations and native sidecar edges are indexed; unsealed references and undeclared document families are outside this view.
  · Counts cover incident edges in the declared scope, not all repository assertions. Paging bounds output, not graph scan or source bytes.
  · Each page recomputes the current graph; offsets are not snapshot identities. Restart paging after changing source or native records.
  · No semantic ranking or substantive repair/conflict judgment is inferred. Open each consumer's native context for assertion, review and acceptance records; it…

Continuation: 0 finding(s) · 0 evidence · 0 question(s) · next: choose a concern
```

`agentic-developer` ranking first is not noise and was not suppressed: it carries `mempalace` 11
times and `re-mine` 4 times against memory-ceremony's 8 and 1. Both are honest candidates for these
words, and both arrive as commands rather than as claims.

Following the screen's own choices from there, with nothing typed that the view did not offer:

```
epr flow memory recall source --session r4-final --path .claude/skills/memory-ceremony/SKILL.md
  → 10 "Inspect <heading>" choices

epr flow memory recall read --session r4-final \
  --path .claude/skills/memory-ceremony/SKILL.md --lines 205:205
  → "When verified canonical memory surfaces change, re-mine natively: `mempalace --palace
     .mempalace/palace sync --root . --apply`, then `mempalace --palace .mempalace/palace mine
     <surface>` … then stamp `date +%s.%N > .mempalace/.last-mine` ONLY if no step was
     lock-blocked …"
  usage: source_bytes 844, source_files 1
  receipt_keys: [".claude/skills/memory-ceremony/SKILL.md:205:205"]
```

Both halves of the question — which commands, and when the marker may be stamped — in one receipted
844-byte excerpt. That is the journey the fresh reader could not complete in 12 operations.

## Tests

`tests/flow_memory_recall.rs`, two new plus one amended (47 in the suite, was 45):

- `the_body_scan_window_holds_a_document_and_is_still_a_window` — a term at byte ~20,000 is found
  and labelled `["body"]`; the same term past 65,536 is NOT found, `partially_scanned_candidates`
  is 1 and the omissions line reads `1 candidate(s) scanned to the body-scan window only`; and with
  `scan_bytes: 4096` the traversal still stops under its cap and still names `budget exhausted`.
- `a_rare_term_outranks_a_word_every_candidate_shares` — eight documents share `common`, one
  carries `pelican` three times; the rare one ranks first, and its `matched_terms` reports
  `pelican: { found_in: body, occurrences: 3 }` so the ranking's evidence is inspectable.
- `a_character_cut_by_the_metadata_budget_does_not_condemn_the_document` amended: its "a term past
  the window is invisible" assertion assumed an 8 KB window and now belongs to the test above; it
  keeps the byte-exact-header assertions, which are what it exists for.

## Gate (round 4)

```
cargo test -p elohim-epr-cli --test flow_memory_recall --test flow_concerns_corrections --test flow_memory_footprint
EXIT=0
  flow_concerns_corrections   9 passed; 0 failed
  flow_memory_footprint      10 passed; 0 failed
  flow_memory_recall         47 passed; 0 failed

cargo test -p elohim-epr-cli --test help_is_a_question --test flow_report --test flow_report_surface_walk --test flow_memory_import
EXIT=0
  12 / 96 / 6 / 4 passed; 0 failed

cargo test -p elohim-epr-cli --lib                             EXIT=0   (186 passed)
cargo fmt --all -- --check                                     EXIT=0
cargo clippy -p elohim-epr-cli --all-targets -- -D warnings    EXIT=0
cargo build -p elohim-epr-cli                                  EXIT=0
cucumber-js --profile ceremony                                 EXIT=0   (6 scenarios, 34 steps)
cucumber-js --profile collective-memory                        EXIT=0   (4 scenarios, 17 steps)
```

## Files changed in round 4

```
.epr-meta/elohim/algorithms/recall-contract.json   version 7→8; body_scan_bytes 65536; scan_bytes 2097152; scan_seconds 5; discovery.body_scan_window
elohim/eprfs/epr-cli/src/flow/memory/recall.rs     body window read + metadata-window membership split; partial-scan count and omission; occurrence/rarity ranking; matched_terms evidence
elohim/eprfs/epr-cli/tests/flow_memory_recall.rs   +2 tests, 1 amended (47 in the suite)
```

**The contract version has now moved 4 → 8 across this sprint.** Any session opened under an earlier
version refuses with `algorithm bytes changed` and continues with
`epr flow memory recall adopt --from-session <old> --session <new>`; the receipts are intact and
adoption carries both pins forward with their prior accounting. Worth one line wherever running
sessions are told about this sprint.

---

# Round 5 — the passage, not the document; and a focused journey can close

Fresh reader 2 reached the answering skill and still did not get the answer, then could not end its
session. Both are fixed, and the second one required the contract to stop saying something that is
no longer true.

## (1) Passage location is part of discovery

**The defect.** `source --path` handed back six headings in document order. The answer was under
"Phase 4 — verify the experience, reconcile and retain learning" — a heading that names nothing
about re-mining an index — so the reader read the heading list, learned nothing, and moved on.
Naming a document whose headings answer nothing is half an answer; the reader pays the other half
in guesses.

**The fix.** When the session carries a need, an outline annotates each section with the need-term
occurrences inside it and offers a read range **bounded to `limits.source_bytes`**, top-hit sections
first. A section larger than one excerpt is offered as its FIRST window and says so, rather than
handing over a range `read` would refuse. The focused `open` screen's candidates carry the section
their terms land in beside `[matched in body]`, and their linked choices read THAT range instead of
the file head. Every byte of this is charged to the scan counters; only what `read` returns is
`source_bytes`.

Live, on the reader's own question:

```
$ epr flow memory recall source --session r5 --path .claude/skills/memory-ceremony/SKILL.md \
    --need 'what commands re-mine the MemPalace index and when may the marker be stamped'

Outline: .claude/skills/memory-ceremony/SKILL.md (207 lines)
  9:12 Memory ceremony — preserve purpose through reconciliation
  13:43 Shared memory and deterministic lenses
  44:83 The deterministic lenses, after the kit  [hits: index ×7]
  84:110 Enter the journey  [hits: commands ×1, index ×1, when ×1]
  111:132 Recipe, evidence and continuity  [hits: when ×2, mempalace ×1]
  133:159 Phase 0 — witnessed corrections first
  160:170 Phase 1 — triage and orient without losing the selected concern  [hits: index ×3]
  171:183 Phase 2 — four judgments, shared investigation  [hits: index ×4, when ×1]
  184:191 Phase 3 — concrete decisions and authorized action  [hits: commands ×1]
  192:207 Phase 4 — verify the experience, reconcile and retain learning  [hits: mempalace ×7, when ×2, index ×1, re-mine ×1]

Linked choices:
Read Phase 4 — verify the experience, reconcile and retain learning — [hits: mempalace ×7, when ×2, index ×1, re-mine ×1]
  epr flow memory recall read --session r5 --path .claude/skills/memory-ceremony/SKILL.md --lines 192:207
Read The deterministic lenses, after the kit — [hits: index ×7]
  epr flow memory recall read --session r5 --path .claude/skills/memory-ceremony/SKILL.md --lines 44:83
…
```

The answer is line 205. The first choice offered covers 192:207 and returns it: `source_bytes 4029`,
receipt `.claude/skills/memory-ceremony/SKILL.md:192:207`, content containing
`mempalace --palace …`. The reader no longer has to guess which "Phase" holds its answer — the view
says `mempalace ×7` beside the one that does.

The focused screen now locates each candidate before it is opened at all:

```
  1. .claude/skills/agentic-developer/SKILL.md — SKILL.md [matched in body, description]
       § Close (363:388) [hits: mempalace ×4, re-mine ×4, index ×3, when ×2]
  2. .claude/skills/memory-ceremony/SKILL.md — SKILL.md [matched in body]
       § Phase 4 — verify the experience, reconcile and retain learning (192:207) [hits: mempalace ×7, when ×2, index ×1, re-mine ×1]
  3. .claude/skills/plant-eprfs-agentdoc/SKILL.md — SKILL.md [matched in body, description]
       § The FLIP that composes with the cite writer (17:24) [hits: index ×1, marker ×1]
```

and its linked choice is the passage, not the file:

```
Read .claude/skills/memory-ceremony/SKILL.md — Phase 4 — verify the experience, reconcile and retain learning (192:207)
  epr flow memory recall read --session r5-final --path .claude/skills/memory-ceremony/SKILL.md --lines 192:207
```

**Two operations from the first screen to the receipted answer**, both of them commands the view
handed over.

## (2) A focused journey can finish, and every refusal names its input

**`finish` on a focused journey.** It is accepted when the session holds at least one evidence
receipt even with no concern selected. It records outcome and question, lists the receipts it stands
on, and says outright that nothing was reconciled:

```
reconciled_edges 0
receipts ['.claude/skills/memory-ceremony/SKILL.md:192:207']
reconciliation_scope: "No concern edge was selected or reconciled: this journey answered a
                       question from inspected passages. The receipts above are what it stands on."
```

No `reconciliation` block is emitted and no edge standing is claimed — a focused finish must not
look like a reconciliation that happened to find nothing.

**The contract moved 8 → 9, because its own text had become false.** `ceremony.finish` declared
"with per-edge reconciliation", which is no longer true of every finish. It now reads: *"…standing
on either per-edge reconciliation (the ceremony door) or the inspected passages alone (the focused
door, which reconciles no edge and says so)."* A new `discovery.passage_location` states the
outline rule. This is the one text change of the round; I did not bump for the executor changes
alone.

**Refusals name the input.** Both classes the reader hit:

```
$ epr flow memory recall remember --session r5 --foo bar
refused: invalid arguments: unknown flag --foo; remember takes --finding --question --next-action --evidence path:START:END --classification
next: The message names the flags this operation accepts. Re-run with one of them, or `epr flow memory recall --help` for the whole surface.
EXIT=2

$ epr flow memory recall finish --session r5b --outcome o --question q     # nothing read yet
refused: invalid arguments: finish needs --outcome and --question (both given) and either a selected concern or at least one inspected passage; this session has 0 receipts — read a passage first: recall read --path <p> --lines START:END
next: Read the passage the answer rests on first, then finish: recall read --path <path> --lines START:END, then recall finish --outcome <what you found> --question <what is still open>.
EXIT=2
```

`accepted_flags(operation)` is a single named function, so a wrong guess costs one refusal that
teaches the surface rather than an unbounded number that do not. `remedy_for` gained the two
matching remedies.

## Tests

`tests/flow_memory_recall.rs`, four new (51 in the suite, was 47):

- `the_outline_says_which_section_the_question_lands_in` — six sections, the term only in the
  fourth, under a heading that does not name it: only section 4 is marked, `hits.pelican == 2`, the
  first offered choice is a `read` whose range actually covers the line carrying the term (asserted
  by slicing the fixture back out of the file, not by trusting the range).
- `an_oversized_section_is_offered_as_a_bounded_window_and_named` — one section past
  `limits.source_bytes`: `window_complete: false`, the offered range is non-empty and stops short of
  the section end, the omission names the per-excerpt budget, and the offered range is one `read`
  accepts (asserted by running it).
- `a_focused_journey_finishes_on_its_receipts_and_says_it_reconciled_nothing` — with nothing read,
  the refusal names the missing input and counts `0 receipts`; after one `read`, `finish` succeeds
  with `reconciled_edges: 0`, the receipt listed, and no `reconciliation` block.
- `a_refusal_about_an_input_names_the_flags_that_operation_accepts` — `--foo` names all four
  `remember` flags and points at `--help`; a missing `--next-action` names it too.

A `begin_focused` helper was added beside `begin`, because the focused door (a question, no concern
selection) is a different shape from the ceremony door and several assertions are about it.

## Gate (round 5)

```
cargo test -p elohim-epr-cli --test flow_memory_recall --test flow_concerns_corrections --test flow_memory_footprint
EXIT=0
  flow_concerns_corrections   9 passed; 0 failed
  flow_memory_footprint      10 passed; 0 failed
  flow_memory_recall         51 passed; 0 failed

cargo test -p elohim-epr-cli --test help_is_a_question --test flow_report --test flow_report_surface_walk --test flow_memory_import
EXIT=0
  12 / 96 / 6 / 4 passed; 0 failed

cargo test -p elohim-epr-cli --lib                             EXIT=0   (186 passed)
cargo fmt --all -- --check                                     EXIT=0
cargo clippy -p elohim-epr-cli --all-targets -- -D warnings    EXIT=0
cargo build -p elohim-epr-cli                                  EXIT=0
cucumber-js --profile ceremony                                 EXIT=0   (6 scenarios, 34 steps)
cucumber-js --profile collective-memory                        EXIT=0   (4 scenarios, 17 steps)
```

## Files changed in round 5

```
.epr-meta/elohim/algorithms/recall-contract.json   version 8→9; ceremony.finish no longer claims per-edge reconciliation always; discovery.passage_location
elohim/eprfs/epr-cli/src/flow/memory/recall.rs     outline_with_terms + best_section; hit-ordered bounded read choices; candidate §section; focused finish; accepted_flags; two remedies; render_outline/render_hits
elohim/eprfs/epr-cli/tests/flow_memory_recall.rs   +4 tests, +begin_focused (51 in the suite)
```

**The contract has moved 4 → 9 across this sprint.** Every move is listed in the round it happened
in; a session opened under any earlier version refuses with `algorithm bytes changed` and continues
with `epr flow memory recall adopt --from-session <old> --session <new>`.
