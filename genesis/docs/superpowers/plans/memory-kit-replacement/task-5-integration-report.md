---
id: memory-kit-replacement-task-5-integration-report
status: DONE_WITH_CONCERNS
cites: []
gap: plans__2026-09-10-memory-kit-replacement-finish#6
actor: agent:implementer@claude-opus-5
session: mk-replace-integ-5
commits: []
---

# Station five (integration seat) — the packages, the stories and the gate name only native verbs

The native seat landed `epr flow memory recall` (sixteen operations) and
`epr flow concerns --corrections`. This seat made every surface that *names* the ceremony say the
native verb, moved the two a2o stories onto it, replaced the gate's Python recall leg with the two
native test binaries, and deleted the six Python scripts and their four test files. The kit
directory itself survives for station six.

## 1. Packages re-authored, projected, verified

| Package | Change | Version |
|---|---|---|
| `.epr-meta/elohim/packages/skills/memory-ceremony.json` | six passages re-authored (below) | 1.2.0 → 1.3.0 |
| `.epr-meta/elohim/packages/agents/librarian.json` | contract path + recall-packet paragraph | 1.0.0 → 1.1.0 |
| `.epr-meta/elohim/packages/agents/historian.json` | same two paragraphs | 1.0.0 → 1.1.0 |
| `.epr-meta/elohim/packages/agents/cartographer.json` | same two paragraphs | 1.0.0 → 1.1.0 |
| `.epr-meta/elohim/packages/agents/storyteller.json` | same two paragraphs | 1.1.0 → 1.2.0 |
| `.epr-meta/elohim/packages/skills/memory-kit.json` | **no change — verified none needed** | 1.1.0 |

`memory-kit.json` names no recall script, no `stale-record.py` and no contract path; it names the
ceremony by skill name only. Reported rather than silently skipped, because "re-author the
memory-kit package's recall references" was in the brief and the honest answer is that it has none.

### What changed inside the memory-ceremony package

- **Entry.** `python3 .claude/scripts/memory-kit/recall-ceremony.py open --session <id>` →
  `epr flow memory recall open --session <id> --need '<the question you carry>'`, followed by a new
  paragraph naming all sixteen operations and — keeping the privacy language the plan's
  constitutional section requires — stating that receipts and continuations are private records
  under `.eprfs/status/recall/<session>/`, mode `0600`, terminally ignored, never imported,
  projected, witnessed or targeted by feedback, with every path-shaped input refused if it reaches
  into that store.
- **Collective memory.** The Python's `collective` / `memory-project` / `memory-contribute` /
  `memory-feedback` / `memory-graduate` operations with `--memory-input` → the five standalone
  verbs `epr flow memory collective|contribute|project|feedback|graduate --input <request.json>`
  with `--session <registered-session>` on writes, and the sentence naming *why* they are not
  recall operations (one governed act, one address). Adds `project`'s own retention rule: save the
  exact output and pin it with `epr flow memory pin` before feedback names it, and a resumed
  investigation re-derives the projection from the same authored request rather than caching it.
- **Native helper.** Dropped `--epr-bin` / `EPR_BIN` / the `/tmp/eprfs-gate-target/debug/epr`
  locator — natively there is no second process to locate and no second stderr budget.
- **Contract path.** `.claude/scripts/memory-kit/recall-contract.json` →
  `.epr-meta/elohim/algorithms/recall-contract.json`, plus the sentence that its raw CID is the
  `method` pinned on every session and receipt.
- **Evidence packets.** The `recall-packet.py` paragraph → `open` a second session label on the
  same executor, then `search --search-scope … --name … --query …`, `source --path …`,
  `read --path … --lines START:END`. One executor now serves both, so the method pin is shared and
  only the session LABELS stay distinct.
- **Phase 0 and correction closure.** `stale-record.py` → `epr flow concerns --corrections
  [--since] [--json]`, with the identity rule spelled out (a later note carrying
  `closes:<exact CID>`; `--since` filters display only; a date or newer chronicle closes nothing).
  The closure recipe is now `epr flow note --on <surface> --kind observation --closes <CID>
  --reason '…'`, naming which kinds may close, why matching is on subject label rather than
  resource CID, and — per the native seat's concern 3 — that **chronicle frontmatter closures are
  no longer read**: the eight historical `stale_record_resolutions` entries were migrated to native
  notes and any new closure must be a note. The Phase 4 chronicle-frontmatter sentence was
  corrected in the same pass so the two halves of the doc agree.

### Agent packages

Both shared paragraphs, identical across all four agents:

- `` `.claude/scripts/memory-kit/recall-contract.json` `` → `` `.epr-meta/elohim/algorithms/recall-contract.json` ``.
- The `recall-packet.py --session … --need …` sentence → the native `open` / `search` / `source` /
  `read` ladder, with `--provider mempalace` named as the declared semantic widening,
  `adopt --from-session … --session …` as the explicit act when algorithm bytes change, and the
  private-receipt sentence carried into the agent surface too.

### Projection

```
just codegen agents write                                      EXIT=0
node …/package-projections.mjs verify   → 1998 passed, 0 failed  EXIT=0
```

Exactly 22 projection files changed, all five packages' own: `.claude|.codex|.agents` runtime
files plus their `.epr-meta/elohim/projections/*` fixtures. Verified by hashing all 483 projection
files before and after — no other package moved. (Twenty other agent packages were already dirty
in this shared tree when this seat started; their projections were already fresh and stayed
byte-identical.)

## 2. The two a2o stories run on the native verb

```
cd genesis/a2o && EPR_BIN=$(which epr) pnpm exec cucumber-js --profile ceremony
  5 scenarios (5 passed) · 22 steps (22 passed)                 EXIT=0
cd genesis/a2o && EPR_BIN=$(which epr) pnpm exec cucumber-js --profile collective-memory
  4 scenarios (4 passed) · 17 steps (17 passed)                 EXIT=0
```

Baseline before any edit, on the same binary: 5/22 and 4/17, both green. Same counts after. Both
re-run a third time after the contract edit in §4 — still 5/22 and 4/17.

`ceremony-reconciliation.steps.ts` needed only the driver swap: `python3 <entry> <op> … --epr-bin`
→ `<binary> flow memory recall <op> …`, plus argv[0] substitution in `follow()` (a linked action
names the verb `epr`; the run pins the built binary).

`collective-memory.steps.ts` needed more, because the five collective operations are deliberately
**not** among the sixteen. Its steps now drive `epr flow memory collective|project|feedback|
graduate` directly and use recall for the investigation half (`open`, `resume`, `measure`). Two
step-level consequences worth naming rather than hiding:

- **The projection is saved and pinned by the caller.** The Python ceremony saved the projection
  receipt into its private store and pinned it, handing back `projection_evidence`. Natively
  `project` returns `{operation, receiptCid, receipt, retained:false}` and its own payload says
  *"Ephemeral output, not saved by native command. Save exact output before consequential use or
  feedback; pin saved bytes with memory pin."* The step now does exactly that. This is the verb's
  declared contract, not a workaround.
- **Resumption re-derives rather than replays.** The Python's `resume` re-ran
  `epr flow memory project --input <retained request>` internally and republished it as
  `view.memory`. The step now makes that call explicitly. What is lost is the ceremony's retention
  of the request PIN — the Python refused with "projection request changed; explicitly select its
  new version" if the request file moved under it. Nothing native reproduces that guard. Recorded
  under Concerns.
- **A refusal is on stderr.** `invoke()` now surfaces a non-zero-status stderr body as
  `{refusal: …}` so the graduation story can assert on the refusal words. The Python printed its
  refusal view on stdout.

### Native defect found (exact key): the footprint lens path resolves against `--root`

`epr flow memory recall open` refuses in any tree that is not the elohim repository:

```
unresolved: ["invalid arguments: footprint lens failed: python3: can't open file
 '<root>/genesis/scripts/memory_balance.py': [Errno 2] No such file or directory"]
```

`recall.rs:1500` sets `lens = root.join(BALANCE_LENS_REL)` and `recall.rs:3435` makes `--lens`
`root.join(value)` too. The retired Python imported `memory_balance` as a MODULE, so its lens
SCRIPT always came from the executor's own repository while only the lens's SCOPE followed
`--root`. Natively the two are conflated, so every isolated fixture — and every consumer running
the executor against a tree other than this one — refuses at `open`.

The stories are unblocked by naming the repository's lens absolutely (`--lens <abs path>`; Rust's
`PathBuf::join` returns an absolute argument unchanged), with the divergence written into a comment
at the call site in both step files. That is a fixture configuration, not a papered-over assertion —
no assertion was weakened and no view key was reshaped. The **cure belongs to the native seat**:
resolve `BALANCE_LENS_REL` against the executor's own repository (or the contract's declared
`source_roots` owner), and keep `--root` as the sampling scope only.

Side observation from the same cause: the ceremony profile went 7s → 35s, because the lens is now
a spawned `python3` per `open`/`measure` instead of an in-process import. Correct, not a defect.

**No view-shape divergence was found in the sixteen operations.** Every key the two step files
assert on — `orientation.intent`, `orientation.provider`, `node.edge.verdict`, `node.findings[].
evidence_state`, `continuation.{next_action,evidence,unresolved_questions,repeated_reads}`,
`evidence.sources[]`, `receipt_keys`, `retrieval.{candidates,authority,declaration.ranking}`,
`evidence_check.revalidation_required`, `prepared_action.argv`, `outcome.{report,standing}`,
`measurement.{baseline,evidence.sha256,comparison,meaning}`, `actions[].argv` — is present with
the same shape and the same words.

## 3. The gate names the native tests

`justfile` `_gate-memory-ceremony`, the Python recall leg replaced:

```diff
-    EPR_BIN="$CARGO_TARGET_DIR/debug/epr" python3 -m unittest discover -s .claude/scripts/memory-kit/__tests__ -p 'recall*_test.py'
+    cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --test flow_memory_recall --test flow_concerns_corrections
```

`genesis/build-manifest.json` `gate.projects.memory-ceremony.inputs.sources`:

```diff
-      ".claude/scripts/memory-kit/recall*.py",
       ".epr-meta/elohim/algorithms/recall-contract.json",
-      ".claude/scripts/memory-kit/__tests__/recall*_test.py",
+      "elohim/eprfs/epr-cli/tests/flow_memory_recall.rs",
+      "elohim/eprfs/epr-cli/tests/flow_concerns_corrections.rs",
```

Detection proven (not just parsed):

```
node genesis/orchestrator/gate-runner.mjs --target memory-ceremony --print   EXIT=0
printf 'elohim/eprfs/epr-cli/tests/flow_memory_recall.rs\n…' | gate-runner --changed-file-list --print
  → memory-ceremony  reasons: ["source: elohim/eprfs/epr-cli/tests/flow_memory_recall.rs", …]  EXIT=0
```

**The cargo leg was not run by this seat** — the berth is held by other seats, per the dispatch.
Every other leg of the recipe was:

```
python3 -m unittest discover -s genesis/scripts/__tests__ -p memory_balance_test.py
  Ran 9 tests · OK                                                          EXIT=0
EPR_BIN=$(which epr) python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
  Ran 38 tests · OK                                                         EXIT=0
python3 .epr-meta/elohim/lenses/memory/__tests__/memory_coherence_audit_test.py
  10 assertions passed                                                      EXIT=0
cucumber ceremony 5/22 · cucumber collective-memory 4/17                    EXIT=0
```

The installed `/opt/rust/cargo/bin/epr` used for every run is byte-identical to
`/tmp/eprfs-gate-target/debug/epr` (sha256 `a2f4a4d5…feef151`), i.e. the binary the native seat
gated.

## 4. Deletions and re-pointed references

Deleted (six scripts + four test files):

```
.claude/scripts/memory-kit/recall-ceremony.py          (untracked)
.claude/scripts/memory-kit/recall-packet.py            (untracked)
.claude/scripts/memory-kit/recall_runtime.py           (untracked)
.claude/scripts/memory-kit/recall_lenses.py            (untracked)
.claude/scripts/memory-kit/recall_providers.py         (untracked)
.claude/scripts/memory-kit/stale-record.py             (tracked → ` D`, unstaged)
.claude/scripts/memory-kit/__tests__/recall_ceremony_test.py   (untracked)
.claude/scripts/memory-kit/__tests__/recall_packet_test.py     (untracked)
.claude/scripts/memory-kit/__tests__/recall_runtime_test.py    (untracked)
.claude/scripts/memory-kit/__tests__/stale_record_test.py      (untracked)
```

plus both `__pycache__` trees. Only `stale-record.py` was ever committed; the other nine were
untracked working-tree files, so `git rm` refuses them and plain `rm` was used. Nothing was staged —
the index is untouched, as the sprint requires.

Executable references re-pointed:

| Surface | From | To |
|---|---|---|
| `genesis/a2o/steps/devflow/ceremony-reconciliation.steps.ts` | `python3 …/recall-ceremony.py <op> --epr-bin` | `epr flow memory recall <op>` (+ `--lens`, + argv[0] pin) |
| `genesis/a2o/steps/devflow/collective-memory.steps.ts` | same, incl. `collective` / `memory-*` ops | `epr flow memory recall` + the five collective verbs |
| `justfile` `_gate-memory-ceremony` | `unittest … 'recall*_test.py'` | `cargo test … --test flow_memory_recall --test flow_concerns_corrections` |
| `genesis/build-manifest.json` | `recall*.py`, `__tests__/recall*_test.py` | the two `.rs` test files |
| `.epr-meta/elohim/algorithms/recall-contract.json` | `governance.executor`, `ceremony.entry`, `ceremony.dependencies[recall_lenses.py]`, the `recall-packet.py --native --epr-bin` guidance line | `recall.rs`, `epr flow memory recall <operation> --session <id>`, entry removed, in-process wording |
| `.claude/scripts/memory-kit/.epr-meta` | `why`/`retire-when`/`Run` naming the Python executor and its tests | native executor + the two cargo test binaries |
| memory-ceremony skill package + 4 agent packages | see §1 | native verbs |

`genesis/a2o/cucumber.mjs` needed no change (both profiles already name only the step files).
No hook, command, workflow or `settings.json` entry referenced any of the six scripts — verified by
repo-wide grep before deleting.

**The contract's CID moved, on purpose.** Repointing the algorithm artifact away from its deleted
executor changes its bytes, and its raw CID is the method pin:

```
before  sha256 85a3edb48559367ade9fd7913aa6489ce5558d7af99087b59e35de65438e69db  11,117 bytes
        bafkreiefupw3jbkzgz5n5h6xse5kmse44vky26xzscd3lhrv3zsuhdtj3m
after   sha256 cf0e406ebfff52cb77d6f5e2c36a3e63083f912355b49ab700a66e22752a981a  11,143 bytes
        bafkreigpbzag5p77klfxpvxv4lbwuptdba7zci2vwsnloafgnyrhkkuydi
```

An existing session is refused with the contract-change words and must `adopt` into a new label —
which is the designed behaviour, and the 27 relocated legacy continuations were already reported
`resumable: false`. Nothing in the tree pins the old CID literally (grepped `.rs`/`.py`/`.json`/
`.ts`); `flow_memory_recall.rs` reads the LIVE contract and derives its CID, and `Contract::validate`
only checks `artifact_type`, `composition` and `limits`, none of which were touched. `epr flow
memory recall open` and `recipe` were re-run against the edited artifact and load clean, and both
cucumber profiles (which copy the live contract into their fixtures) re-ran green afterwards.

### Verification of no remaining executable reference

Grep over `.py .js .mjs .json .yaml .yml .ts .sh .toml justfile`, excluding `gap-items/`, reports
and the timeline chronicle, leaves exactly six files, none of them executable references:

| File | What it is |
|---|---|
| `.epr-meta/elohim/packages/agentdocs/elohim-root-gospel.json` | gospel body — **station six** |
| `.eprfs/status/gap-items/plans__2026-09-10-memory-kit-replacement-finish.json` | the plan's own gap text (excluded by the brief) |
| `.eprfs/status/memory/contributions/feedback_stale_record_feeds_memory_ceremony.json` | an imported memory contribution (station four's plane, and named `stale_record` only in its slug) |
| `genesis/docs/superpowers/plans/governed-retrieval-acceptance.json` | a historical acceptance record; naming the executor exercised at the time is what makes it evidence |
| `.epr-meta/elohim/packages/skills/memory-ceremony.json` | two occurrences of the chronicle FIELD `stale_record_resolutions`, both deliberate |
| `.claude/memory-kit/recall-executions/*.json` | 28 legacy receipts pinning the Python digest map — **station six** relocates/removes |

### Prose mentions left for station six's sweep

- **`CLAUDE.md` §Bounded recall, line 23** — `python3 .claude/scripts/memory-kit/recall-packet.py
  --session <ceremony-id> --need "Locate current authority" --case native-reconciliation`. Now a
  dead command. Its native replacement is
  `epr flow memory recall open --session <ceremony-id> --need "Locate current authority"`, then
  `search --search-scope <dir> --name <glob> --query <term>` / `source --path <p>` /
  `read --path <p> --lines START:END`. The same section's
  `recall-contract.json` sentence should say `.epr-meta/elohim/algorithms/recall-contract.json`.
  **Not edited — station six owns gospel.**
- **`AGENTS.md` line 108** — the identical line, and its two projection fixtures
  (`.epr-meta/elohim/projections/{claude,codex}/agentdocs/elohim-root-gospel/`), all fed by
  `.epr-meta/elohim/packages/agentdocs/elohim-root-gospel.json`. Fix the package, reproject.
- **`.claude/memory/feedback_stale_record_feeds_memory_ceremony.md:19`** — names
  `stale-record.py [--since <date>]`. Out of this seat's write set by the dispatch; the entry's
  lesson survives, its command does not.
- **Docs and plans** (`genesis/docs/analysis/2026-09-09-memory-ceremony-efficacy.md`,
  `genesis/docs/superpowers/plans/collective-memory-integration/{reader-entry,task-2-brief}.md`,
  `2026-09-10-agent-provenance-and-collective-affiliations.md`) — historical narrative; leave.
- **Chronicles** (`2026-09-09-reconciliation-recall-rails.md`,
  `2026-09-09-substrate-currency-evidence-recall.md`) — the record of what ran; leave.
- **`elohim/eprfs/epr-cli/tests/fixtures/gap-parity/*`** — copies of plan text used as gap-id
  fixtures; leave.

## 5. Closing checks

```
python3 .claude/scripts/epr-meta-pin.py --verify
  19 pinned row(s) verified clean (19 total)
  15 pinned row(s) verified clean (15 total)                                EXIT=0
EPR_BIN=$(which epr) python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
  Ran 38 tests · OK                                                         EXIT=0
node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify
  89 packages: 74 package-first, 0 source-fidelity, 15 native
  elohim-agent package checks passed: 1998 passed                           EXIT=0
```

Gate evidence line: `just gate memory-ceremony` — every leg run except the cargo leg (berth held by
other seats, per the dispatch); the legs this seat ran are quoted above, each with `EXIT=0`.

No commits, no pushes, no staged changes.

## Concerns

1. **The lens-path defect is real and unfixed.** `epr flow memory recall open` refuses outside this
   repository (§2). The stories are configured around it; every other consumer is not. It is a
   two-line change in `recall.rs` and it belongs to the native seat.

2. **`--tag` has no native flag.** `recall-packet.py` filtered discovery by frontmatter category
   (`--tag <category>`). `discover()` in `recall.rs` still takes a `tags` parameter and still
   filters on it, but `retrieve()` passes `&[]` and the CLI parser exposes no `--tag`, so the
   capability is present and unreachable. The agent packages therefore name `--name` and `--query`
   only. Small, but it is a parity gap the 60-case map does not surface (the Python's tag cases
   live in `test_discovery_exact_tags_and_groups_describe_only_returned_window`, whose Rust
   counterpart exercises tags through the library rather than the CLI).

3. **The retained-request pin is gone from the collective story.** The Python ceremony held the
   projection request's pin and refused a resumed `memory-project` whose request file had changed
   ("projection request changed; explicitly select its new version"). With the collective verbs
   standing alone there is no session that holds that pin, so a resumed reader re-derives from
   whatever the request file now says. The step file re-derives explicitly; nothing checks that the
   request is the same request. Whether that guard should return — as a `--expect <cid>` on
   `epr flow memory project`, or not at all because the receipt CID already changes visibly — is a
   design call the native seat or station six should make, not one this seat should invent.

4. **I edited the algorithm artifact, moving its method CID.** §4 states the before/after digests
   and why. It is the one act in this station with a durable consequence beyond the tree: every
   pre-existing recall session in this workspace must now `adopt` rather than `resume`. A reviewer
   who thinks the contract should have kept its bytes until station six can revert those four
   edits; the cost of doing so is an algorithm artifact whose `governance.executor` and
   `ceremony.entry` name files that no longer exist.

5. **The gate's cargo leg is unproven by this seat.** The two test binaries exist and the native
   seat reports them green (32 + 9), but this seat could not claim the berth. First integrated
   `just gate memory-ceremony` run should be treated as the leg's first real execution.

## What this station did NOT do

- Did not run cargo, and did not touch `elohim/eprfs/**` or `.eprfs/status/**`.
- Did not edit `CLAUDE.md`, `AGENTS.md`, the gospel package, or `.claude/memory/**`.
- Did not remove `.claude/memory-kit/recall-executions/`, `balance-sheets/` or the kit directory.
- Did not commit, push, or stage anything.
