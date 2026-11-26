---
title: The placement read and the shared notes store — implementation design for step 4
id: placement-read-and-notes-store-design
status: draft
class: process-meta
date: 2026-11-23
context-tier: disclosed
steward: human:matthew
author: "Claude (session 6109194c, Fable 5.2 orchestrating); two designers and one adversarial reviewer, all read-only"
serves: [agents-arrive-briefed, recall-reaches-authority]
refines: agents-arrive-briefed-plan
tags: [arrival-brief, placement, notes-store, flow-store, guide-stars, capability-levels, recall, gates]
graduation-trigger: >-
  Landings L1 to L3 are on dev with the tests named here passing, the observable lines in
  section 3 reproduce on shem, and L2 and L3 reproduce on ethosengine from a fresh worktree.
cites: []
---

# The placement read and the shared notes store

This is the implementation design for step 4 of the plan *Agents arrive briefed*: the native
`placement` read (the arrival brief) and the two notes-store preconditions the plan puts before
it. Two designers drafted it and an adversarial reviewer challenged both drafts; this is the design
as the review corrected it. No code is written and nothing here is built. The designers' full test
lists and raw findings are in `genesis/local-dev/agents-arrive-briefed/step-4-design-raw-2026-11-23.json`.

In one paragraph: the brief is one recall purpose, `epr flow memory recall open --purpose
placement`, a read that pins nothing and stores only its own push state. Notes are read by subject
label through one reader and one closure rule, shared by `epr flow context`, `epr flow concerns`
and placement. The flow store is one per repository on a device: linked worktrees resolve it
through the repository's common git directory, and an explicit fold carries stranded worktree
records into it.

## 1. What was observed

Reproduced read-only on 2026-11-23 against dev `4d5c8b701` with epr `bafyreibmupfq…h6ea`.

**Three context reads.** On `genesis/agentic/bin/pool-preflight.sh` the GATE line reads
`ambiguous` with 7 names: `registers::gate_for_path` reads only manifests above the path, the
nearest (`genesis/build-manifest.json`) has no dir covering `genesis/agentic/bin`, so 7 root-scoped
projects tie, while `agentic-shift`, whose sources name the exact path, sits in
`genesis/orchestrator/build-manifest.json`. Its HABIT line names `agents-arrive-briefed` only because
`habits_for` falls back to every habit in the nearest `.epr-meta` (6) and prints the alphabetically
first. On `.claude/hooks/ram-guard.py`, GATE reads `no gate project covers this path` and HABIT
reads `acts-attributed-to-participants`, the root package's first. On
`elohim/eprfs/epr-cli/src/covenant.rs`, GATE reads `ambiguous: eprfs | eprfs-live-corpus` though both
declare dir `elohim/eprfs`, and `selectGateProjects` (`gate-runner.mjs` lines 69–79) runs both for
`just gate elohim/eprfs`; HABIT reads `addresses-are-declared-profiles`, first of 4.

**Other defects.** `epr flow context` refuses a directory target, and `habits_for` and
`gate_for_path` start at the parent, skipping a directory's own package and manifest.
`registers::habits_covering` is a substring test (`registers.rs:342`): called with `"."` it matches
68 of 68 habits, `app` 25 (through `happ`), `genesis/agentic` 5. (`context` does not reach the `.`
case today: it refuses `.`, and `habits_covering("")` returns empty, `registers.rs:332`.) `recall remember` refuses without a selected
stale-edge assertion, so no working record can be written. `lens::builtin_table`'s minimal
`density_bytes` is 1500 against the contract's 2000. The berth-note hook re-delivered five notes on
every tool call in the subagents (`repeat 5`); that is outside step 4.

**A correction drops out of context.** `concerns --corrections` lists 5 open corrections;
`context` on three of their subjects (`.claude/hooks/berth-note-inject.py`,
`genesis/agentic/pool-policy.json`, `.claude/epr-meta/measures.yaml`) prints `NOTES (0 shown, newest
first)`. `context_with` (`context.rs:156`) builds a `NoteSource` for the atom's current
`body_cid_of_file` (plus one per scoped commitment, `context.rs:213-233`), and
`read::notes_across` matches resource only; the correction on
`berth-note-inject.py` pinned `bafkreidjkelj…v2y` and the file now holds `bafkreies24r…nfa4`.
`concerns` matches the subject label in `classified_as[1]`, as does the participation read's
`Resolver::note_path`. Measure observations would flood a label match: `.` carries 7,272 and
`CLAUDE.md` 293, against at most 57 prose notes on any label.

**The store is per checkout.** From this worktree `concerns` reports 0 unresolved against 5 from
the main checkout. Of 114 worktree directories, 45 hold a `flows.jsonl`: 36,314 records, 5,625 unique
ones absent from main (4,887 measure observations, 351 intents, 244 commitments, 91 prose
observations, 19 verdicts, 12 rulings, 18 events, 2 processes, 1 spec); no open correction is
stranded. Counted by line, 5,733 are absent from main, of which 3,248 carry `dev-berth-events@1` and
82 `gate-verdict@1`, kinds that cross devices. 12 worktrees hold their own `actors.jsonl` and 2 a
feedback store; 8 `.claude/worktrees/agent-*` checkouts (3 with a store) sit outside `.worktrees/`;
git registers 80 worktrees. The device store holds 21,609 records.

**Costs.** `context` on one file: 2.09–2.26 s (240 MB). `concerns`: 0.54–0.57 s (58 MB), which
bounds loading the 17 MB sidecar. `covenant show`: 0.06 s. run-projection's 6 s budget was already
exceeded under load (`run-projection.py:127-130`).

## 2. Design

### 2.1 One notes reader, one closure index

`flow/closures.rs` (new) takes the closure derivation out of `concerns::corrections` (loops at
about lines 400–516: `CLOSURE_KINDS`, the `closes:` skip, the same-label check, the later-by-index
check, the issue strings) as `pub fn index(records: &[(Cid, FlowRecord)]) -> ClosureIndex` with
`is_closed(&Cid)`:

```rust
pub struct ClosureIndex {
    /// Every run:correction that carries no `closes:`, first occurrence per CID, in record order:
    /// cid, target (`classified_as[1]`), resource, occurred_at, reason, record index.
    pub corrections: Vec<CorrectionRecord>,
    /// Closures keyed by the correction CID each discharges (the map `Corrections.resolved` serializes).
    pub resolved: BTreeMap<String, Vec<Closure>>,
    /// Correction-scan issues first, then closure-scan issues, each in record order.
    pub issues: Vec<String>,
}
```

`is_closed` is `resolved.contains_key`. `Closure` moves to `closures.rs` and `concerns.rs`
re-exports it. `Correction` (with `closure_template`, `date` and `stale`) stays in `concerns.rs`,
built from `CorrectionRecord`, and `concerns` still adds `correction ledger is missing` itself, so
`concerns --corrections --json` stays byte-identical: `tests/flow_concerns_corrections.rs` checks
issue contents and the REVALIDATE exit, and `genesis/a2o/steps/devflow/run-plane.steps.ts:365`
reads `surfaces`, `reason` and `target` from the same JSON.

`flow/read.rs`: `NoteSource` gains `label: Option<LabelMatch>` (`Exact(String)` or `Under(String)`).
`Under` uses the directory rule of `participate::Scoper::covers` (`participate.rs:556`), lifted to
`read.rs` as `pub(crate)` because `Scoper` is private to `participate.rs`; `.` and `""` both
normalize to the empty dir, which covers every label. Only the atom's own source carries a label;
the per-commitment sources keep `label: None`.

`notes_across_with(records, sources, limit, &NoteFilter { closed: Option<&ClosureIndex> })` matches
by address or by label on `classified_as[1]`. Before the sort and truncation it drops label-only
matches carrying a `measure:` slot (`note.rs:134`), and corrections the index marks closed whether
matched by address or by label (today a closed correction on the current bytes is shown; after L1
it is not). Closure notes, those carrying `closes:`, stay. A note matched both ways appears once
and keeps the address match's `via`. `notes_across` stays a wrapper passing `closed: None`, so the
ledger, `notes_on` and `view/readings.rs` are untouched.

`NoteView` gains `bytes: Option<NoteBytes>` and `written_against: Option<String>`, both skipped
when absent. `NoteBytes` serializes as `"current" | "differ" | "subject-absent"`. Both fields are
set only for a note on the target itself (no `via`): `written_against` is the note's full resource
CID; `bytes` is `subject-absent` when the target path is absent in this checkout and
`resolve_target` located it through the `labels.json` fallback (`context.rs:423-440`; otherwise a
missing path is refused, as today), else `current` when the resource equals `identity.cid` (every
address match), else `differ`.

`flow/context.rs` gathers NOTES this way with `LabelMatch::Exact(rel)` and one `ClosureIndex` per
snapshot; `--notes N` (default 5) stays:

```
  NOTES (<n> shown, newest first; by address and subject, closed corrections omitted)
    • <kind>[ [verdict]][ via <gap>] <full note CID> — <reason>
      <yyyy-mm-dd>[ · bytes: written against different bytes (<written_against>; this checkout holds <identity.cid>)]
```

The note CID is printed in full (`short_cid`, `mod.rs:2453`, would print 8…4 characters).
`<yyyy-mm-dd>` is the first ten characters of `occurred_at`, the HEAD commit date the note was
written against (`note.rs` `head_commit_provenance`). Every note prints the date line; the
`· bytes:` part prints only when `bytes` is set, reading `bytes: current`, the `differ` form above,
or `bytes: subject absent in this checkout`. The mark says "different", not "earlier": a note pins
only `occurred_at` and a resource CID. Placement calls the same reader with
`LabelMatch::Under(target)`.

### 2.2 Honest gate and habit resolvers

**Gate.** `gate_for_path` resolves in tiers. (1) The nearest manifest's longest covering `dir`;
several projects at one dir give the path form `just gate <dir>` with every member listed, as
`selectGateProjects` runs them. (2) Projects selected the way `gate-runner.mjs`
`projectsForChanges` selects them under `GATE_ORACLE=path` (`walkGraph` → `projectsFromStale`):

- Manifests come from `discoverManifests`'s skip set (`manifest-utils.mjs:18`: `.claude`, `.git`,
  `.superpowers`, `.worktrees`, `local-dev`, `node_modules`), not `git ls-files`, which fails after
  a restart. The narrower set `.git target node_modules .worktrees` finds 260 manifests instead of
  28 (194 under `.claude/worktrees`, 38 under `genesis/local-dev`) and took 4.0 s instead of 0.95 s
  on shem.
- A path matching `.ci-ignore` selects nothing on push (`ci-ignore.mjs` `filterChanged`, applied
  in `projectsForChanges`; `.ci-ignore` line 1 is `.claude/`). Patterns read as `parseCiIgnore`
  reads them: a trailing `/` is a prefix, a pattern containing `/` an exact path, any other a
  basename. Tier 2 is skipped for such a path and the view carries `ci_ignored: <pattern>`; tiers
  1 and 3 still name what to run by hand.
- A project's own `inputs.sources` match with governance paths included. Its borrowed steps (its
  `steps`, or every step of its manifest when it declares neither `steps` nor `inputs`,
  `graph-walker.mjs:143`) match their `inputs.sources` with paths containing a `.epr-meta` segment
  excluded (`gate-inputs.mjs:120,138`), plus their `buildProcess` files cut at `@`
  (`gate-inputs.mjs:146-149`), which name the path as literal sources.
- Matching is picomatch-default, as pre-push matches (`gate-inputs.mjs:136`): `*`, `?` and `**` do
  not match a dot-segment. In Rust that is `glob::Pattern::matches_with(MatchOptions {
  require_literal_separator: true, require_literal_leading_dot: true, case_sensitive: true })`; the
  glob crate's default `require_literal_leading_dot: false` would match dot-segments. A pattern the
  glob crate rejects is skipped and counted in the omission. CI's `ciGlobMatch` does match
  dotfiles, so this reproduces pre-push, not CI.
- A shared vector file, `genesis/orchestrator/testdata/gate-match-vectors.json` (new), holds
  `(pattern, path, matches)` rows read by both a `gate-runner` test and a `registers.rs` test.
- The walk and the `.ci-ignore` read are repository reads the gate-input audit must see: declare
  them with `// gate-reads:` lines, or the eprfs key falls back to the whole commit tree.

A source names the path when it matches and has a literal prefix before its first metacharacter (a
source with no metacharacter is all prefix), ranked by prefix length. If tier 1 gave the command,
every namer outside the tier-1 members is listed in `also_selected`, grouped by source, longest
prefix first, ties by name; otherwise the top rank gives the command and a tie is ambiguous.
Projects matched only by prefix-less globs such as `**/*.rs` are listed in `extension_wide`. (3)
Root-scoped projects of the nearest manifest; a tie is ambiguous.

`GateView` keeps `project`, `command`, `target_dir`, `rustflags`, `manifest` and `ambiguous`, and
gains `members`, `basis`, `also_selected: [{project, source}]`, `extension_wide`, `ci_ignored` and
`omission` (`pre-push also asks rakia affected; not consulted`, plus the count of skipped patterns).
Fields are snake_case like the existing ones (`registers.rs:391-403`). `ambiguous` is filled only
on a true tie. For a shared dir `project` is null, `command` is `just gate <dir>` and `members`
lists the projects. `basis` serializes as `{"kind": "dir" | "shared_dir" | "names_path" |
"root_scoped", "source": <pattern, for names_path only>}`.

Context renders one line, and keeps the `CARGO_TARGET_DIR` line for a manifest that declares
`targetDir` (real manifests declare none; their `run.cargo` holds `workspace`, `profile`,
`rustflags` and `env`). For `elohim/eprfs/epr-cli/src/flow/context.rs`:

```
  GATE — just gate elohim/eprfs (eprfs, eprfs-live-corpus) · also selected: memory-ceremony (names this file), cargo-bridge, jenkins-bridge, orchestrator, orchestrator-tree (elohim/eprfs/**) · extension-wide: address-profile-lint, rust-style
```

`elohim/eprfs/**` is a source of `gate-cargo-bridge` (`bridges/cargo/build-manifest.json`),
`gate-jenkins-bridge` (`bridges/jenkins/build-manifest.json`) and `lint-jenkinsfiles`
(`genesis/orchestrator/build-manifest.json`, borrowed by `orchestrator` and `orchestrator-tree`),
and `memory-ceremony` names `context.rs` itself; for `covenant.rs` the same line omits
`memory-ceremony`. A ci-ignored path appends `· ci-ignored (.ci-ignore: <pattern>); a push of this
path selects no gate`. The three paths of §1 resolve to `just gate agentic-shift` (extension-wide:
`address-profile-lint`), `just gate elohim/eprfs`, and none with `ci-ignored (.ci-ignore:
.claude/)` for `.claude/hooks/ram-guard.py`.

`.claude/**` is ci-ignored, so no source entry can select a gate for the hooks: `memory-ceremony`
declares no `steps`, its own sources already include `.claude/hooks/__tests__/*_test.py`, and a
push of `.claude/hooks/__tests__/bootstrap_projection_test.py` alone selects nothing. Its recipe
runs that directory (`python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'`)
only when something else selects it. For L3, run `EPR_BIN=<the built epr> python3 -m unittest
discover -s .claude/hooks/__tests__ -p bootstrap_projection_test.py` and the same with
`-p covenant_hooks_test.py` by hand, and cite both runs in the habit delta.

**Habit.** This is `context.rs::HabitView` (`context.rs:87`), not `walk.rs`'s (`walk.rs:777`). It
gains `basis` (`{"kind": "names_path"}` or `{"kind": "declared_nearby", "package":
"<dir>/.epr-meta"}`) and `chosen: bool`. A check or ref names the path only by whole-path token:
split on whitespace, backticks, quotes, parentheses and commas; strip a leading `./`, trailing `.`,
`,`, `:` and `;`, a `#…` anchor and a `:<digits>` suffix; the token then equals the target or
starts with `<target>/`. This replaces the substring test in `registers::habits_covering`, whose
only caller is `context.rs:464`. For `.` the `names_path` arm is skipped and the root package's
declaration arm states its basis. `choose_covering_habit` picks: one namer; several namers are
ambiguous; else one nearby declaration; several are ambiguous, naming the package, the ids and the
cure "name the path in one habit's refs:"; else none.

`ContextResult.habits` stays a `Vec<HabitView>` in today's order (namers, then the nearest
package's declarations) and carries the choice: at most one row has `chosen: true`; with none
chosen, the namers (or, when there are none, the declarations) are the ambiguous candidates; an
empty Vec is none. The choice lives in the rows rather than a new key because
`tests/flow_participate.rs:653` pins the exact top-level key set of `context --json`. ACTIONABLE's
served-habit fallback (`context/actionable.rs:78`, today `habits.first()`) takes the chosen row's
id and is null when the reading is ambiguous or none; a frontmatter `serves:` still comes first.

HABIT prints `HABIT — <id> [<status>[, active]] (names this path)` or `(declared in <package>)`;
`HABIT — ambiguous: <ids> name this path`; `HABIT — ambiguous: 6 declared in
genesis/agentic/.epr-meta, none names this path (name the path in one habit's refs:)`; or, with no
rows, `HABIT (no habit names this path or is declared nearby)`. Many paths will read ambiguous until
habit atoms name their paths; that is a data gap for the steward.

GAP: which existing ACTIONABLE tests rely on the `habits.first()` fallback has not been checked; run `tests/flow_edges.rs` and the `context/actionable.rs` unit tests and update any that do.

`registers::start_dir(root, rel)` (a directory starts at itself) and `HabitEntry.invariant` land in
L1 as groundwork first used in L3; §6 keeps directory targets out of `epr flow context`.
`habit_census::Declaration` (`habit_census.rs:109-117`) reads `invariant` itself before its
`#[serde(flatten)] habit: HabitEntry`, so the flattened field stays empty unless `parse_habit`
copies `declaration.invariant` into it.

### 2.3 One flow store per repository on a device

`git::main_checkout(root)` reads files only: `<root>/.git` names a `gitdir:` whose `commondir`
canonicalizes to a directory named `.git`; a submodule, bare repository, non-git directory or
unreadable step keeps `root`. It replaces the copies at `device/supervise.rs:2221`,
`session_stasis.rs:163` and `lsp.rs:190` (the last spawns git and resolves a submodule to its
parent).

`flow/plane.rs` (new) owns `FLOWS_REL`, `ACTORS_REL`, `store_root`, `open_flows`, `open_actors`,
`feedback_root` and `stranded`. All epr-cli flow-plane access goes through it: 51
`SidecarFlowStore::open(` sites in 29 files, about 34 raw joins, and the actor and feedback opens.
`--root` still names the checkout for tree reads; epr-rea is untouched (`store.rs:233-239`). Moves to
the device root: `flows.jsonl`, `feedback.jsonl`, `feedback/`, `feedback.lock`, `actors.jsonl`. Stays
per checkout: `gap-items/`, `memory/`, `balance/`, `participants/`, `labels.json`, `index/`. Decide
before landing, by reading `flow/memory/validation.rs:660-672` and `:938-941`, whether
`actors.jsonl` moves with `affiliations.jsonl` and `memory-writes.jsonl` or stays with pins resolved
through the device store's claim index.

The non-Rust readers change in the same landing; if `shift_state.py:286` and
`shift_reconcile.py:91` lag, every shift refuses with "no original pinned dispatch evidence". The
others are `habits-project.py:103`, `saga-status.py:80`, `gate-impact.mjs:94` (`deviceRoot`),
`.claude/hooks/memory-index-projection.py:245` and `genesis/a2o/scripts/local-view-browser.mjs:41`.
Python uses `findings_root` from `.claude/scripts/_lib/epr_meta.py`, not git, and skips an
unterminated final line. `gate-cost.mjs` `readCycleUnion` stays as the transition reader.

**The fold.** `epr flow store fold [--apply] [--json] [--root DIR]` is dry by default and does not
delete or rename worktree files. It lists worktrees from `<common>/worktrees/*/gitdir` (covering
`.claude/worktrees`) and refuses a checkout whose `.epr-meta/repository.yaml` agent differs. It
first appends missing actor-claim lines under the actor store's lock, then missing flow records
under one `transaction()`, deduplicated by CID, in each worktree's append order, worktrees by
ascending log mtime. It skips and counts records whose `measure:` names a kind on pool-policy's
`berth.channel.crosses` (read from the file), `feedback:repair` intents without a referent
(`store.rs:304-317`), unparseable logs and unregistered directories; worktree feedback stores are
reported, not merged. The dry run prints `crossing kinds not folded: dev-berth-events@1 3248 ·
gate-verdict@1 82 (they cross from the device store and were never served)`,
`closure-before-correction inversions: N` and `cycle rows that would enter the last-20 window: M per
project`; `--apply` refuses N > 0 without `--accept-inversions`. A write verb in a worktree with a
stranded log folds it first under the same filters; a read prints `REVALIDATE: this worktree's own
flow log holds <n> records the device store does not; any write here folds them, or run: epr flow
store fold --apply`.

**Shift sandbox.** Codex steps run `codex exec -c sandbox_mode="workspace-write"` in the worktree
(`shift_runtime.py:79-81`). Before landing, run one step that records a note; if refused, add
`-c 'sandbox_workspace_write.writable_roots=["<main>/.eprfs/status"]'` to `shift_runtime.command`
(after checking the key) and make the store-open error name that cure. Do not fall back to the
worktree store.

### 2.4 The placement read

**Surface.** `epr flow memory recall open --purpose placement --target <path> [--need <prompt> |
--need-stdin] [--as <seat>] [--reader agent:<role>@<model>] [--reshow] [--json]`. The memory-ceremony
skill says "one governed act never has two addresses", so `epr flow context` gets no `--placement` flag and shares the resolvers. The module is
`recall/brief.rs`, since `flow::placement` already names `epr flow report placement`. `mod.rs`
accepts the purpose (lines 1285–1291) and the new flags; `refusal.rs` `accepted_flags("open")` lists
them; `journey.rs` (1770 of 1800 lines) does not grow.

**Order.** In `run()`, after `Contract::load` and before `Execution::open`: (1) a seat in
`contract.placement.isolated_seats` prints `brief: none composed — seat <s> is isolated
(recall-contract.json#/placement/isolated_seats)` and exits 0; (2) compare the stat key in
`.eprfs/status/recall/<session>/placement.json` (device `flows.jsonl` size and mtime, covenant ref
oid, mtimes of the target's `.epr-meta` files and build manifest, contract CID, target) and, if
equal without `--reshow`, exit 0 silently without loading records; (3) read the continuation
through `Execution::peek`, a shared-lock read that charges no budget and needs no method or executor
pin, printing `brief: recall session pinned to older method/executor — placement unaffected` if the
pins differ; (4) resolve the target (file or directory), load records once via `plane::open_flows`,
and call the section resolvers directly, not `context_with`.

**Parts.**
- *Goal*: the claimed commitment (L5; until then `unmeasured: commitment-claimed goals are not read
  yet`); else `placement.task[<seat>|self]`, recorded from that seat's own first `--need`; else the
  honoured `declaresPurpose`; else `untasked`. *Rung*: the covering habit's `checks[0]` at
  `<atom>#checks/0`, or `unmeasured: <reason>`.
- *Guide-stars*: system, each `contract.ceremony.guiding_context[i]` at
  `recall-contract.json#/ceremony/guiding_context/<i>`; directory, from the nearest
  `eprfs_meta::resolve_path` record, its `purpose`, its rules, the habit `invariant`, and rulings on
  or under the target whose provider is the steward of record (checked as `covenant::steward_standing`,
  `covenant.rs:497`); commitment, steward rulings on its CID, else `unmeasured: no commitment records
  a reason`. Every other ruling and every correction reason renders in a quoted block labelled
  `UNTRUSTED EVIDENCE — <provider> (quoted; not a steward ruling)`, in the layout of
  `.claude/skills/stasis-keeper-brief/SKILL.md:35-44` (below).
- *Authority*: one line, re-resolved each compose from `covenant::show` with `facet_session` from
  `session_envelope::covenant_scope`. Each clause renders only what it records: `principal
  human:matthew (steward of record)`; `purpose: <clause text>; write-set: <paths>; outside the
  write-set every rule applies in full`; `rule <id> varied within <paths>`; an unknown kind, its quote.
  Otherwise `dispatched under the conductor's covenant <session>` (with `--as`), `none held: no
  covenant is granted to this session yet`, or `unknown: the covenant could not be read (<error>)`.
  No `halt` line: no halt record exists.
- *Where to look*: the gate line; open corrections on or under the target, fenced, with bytes marks;
  the working record at `continuation.json#working-record`, or `unmeasured: no finding, open question
  or next action is recorded for this task yet`.
- *Course*, only for scaffold `locate-and-hand-one-command`: `run <rung check>`; `work in <subject>,
  as the ruling on it says: <reason>` for exactly one validated steward ruling; `the authority line
  above governs anything outside <writeSet>`, or nothing until backlog row 79 is ruled.

**State and push.** The header lists each `(address, cid)` and a digest. `placement.json`, per
generation and under its own lock, holds the stat key, the per-seat task and
`shown[facet_session][seat|"self"]`. A first read, `--reshow` or `--as` prints everything; later
reads print only when a value differs, marking lines and part heading `[changed]`. The spec's block
keeps `station: null`, `nearestNeighbours: []` with omissions, `confidence` (`high` / `low` /
`unknown`) and `recall`; it is capped at `limits.native_projection_bytes` (12000) with truncation
addresses. The floor is `render_floor_line(view, lens, floor, extra_omissions)` (`render.rs:232`),
given `view["execution_method"]["method"]` and the truncation count.

**Level, not model.** `lens.rs` maps the reader's model (`--reader`, else the actor claim, else the
envelope's model) to C1–C4 through `capabilityBindings` in `genesis/agentic/agent-runtimes.json`
(`EPR_AGENT_RUNTIMES` overrides; `// gate-reads:` comment; device-runtime's loader is not reused),
by equal id or a contiguous run of `-` tokens; several matches take the lowest with `effort
unobserved`; an unbound model reads `unbound` (minimal), with provenance `<model> → minimal (unrecognised
model: no runtime in agent-runtimes.json binds it)`; with no reader at all, `defaults.level` holds.
Provenance reads `claude-haiku-4-5 → C1 → minimal`. The plan's "bind names → levels" is realised as
model → C in `agent-runtimes.json` plus C → level in `lens_table`, as the contract commit says.

**Intent.** `initialize` seeds `intent_basis: pending` (`pending | covenant:<clause cid> |
prompt:<cid>`); a legacy "Session context" intent reads as pending. Adoption happens on the native
`epr covenant record` path under the lifecycle guard; placement only reads.

**Contract commit.** `lens_table` drops `stated`, gains `capability` (`C1` minimal, `C2` simple,
`C3` standard, `C4` detail) and `unbound: minimal`; a `placement` section holds `isolated_seats:
["blind-reader","stasis-keeper","appreciation"]`, `selection`, `omissions`, `layering`; the false
"symlinked" text is corrected; `recipe` and every `in_scope_of` in `recall-questions.json` re-pin in
the same commit (`mod.rs:421` refuses a mismatch).

**Render.** `run-projection.py` on `--event prompt` calls `--purpose placement --target <T>
--need-stdin --session <P1 session> --root <root>`, T being the covenant write-set root or `.`, and
prints stdout verbatim; the 6-line cap goes, the timeout stays, a failure prints `brief: skipped —
<reason>`. `stasis-keeper` joins `SKIP_AGENT_TYPES` (`covenant-subagent-stamp.py:41`);
`appreciation` gets `.claude/agents/appreciation.md`, or `isolated_by: dispatch-declaration` in the
contract with an omission.

**Example** (garden fixture, level `detail`, after the covenant is granted; the ruling is the
steward's; header source list omitted):

````
BRIEF — level detail · seat none · target garden   [placement is evidence, never authority]
  recipe bafkr4ia…q1 · lens bafkr4ib…x2 · selection placement-v1 · omissions 3 · receipts .eprfs/status/recall/s-garden/
  sources 9 · digest bafkr4ic…d7 · changed: authority
GOAL
  make the totals in the garden totals file match their records — Matthew's opening prompt · recall:s-garden#task @ bafkreig…t6
  rung: the covering habit's check passes — a2o @concern:garden-records-stay-true · garden/.epr-meta/garden-records-stay-true.habit.md#checks/0 @ bafkreie…h4
GUIDE-STARS
  system: the record is the authority, not a remembered copy · .epr-meta/elohim/algorithms/recall-contract.json#/ceremony/guiding_context/0 @ bafkr4ia…q1
  directory: keep the household's seed records true to what was planted · garden/.epr-meta/manifest.md#purpose @ bafkreid…m3
  directory: garden-records-stay-true — every total equals its records' sum · garden/.epr-meta/garden-records-stay-true.habit.md#invariant @ bafkreie…h4
  directory: ruling on garden/totals.md — totals are derived from records, never typed, as a typed total drifted twice · note bafyreih…r7
  commitment: unmeasured: no commitment records a reason
AUTHORITY [changed]
  principal human:matthew (steward of record); purpose: <clause text>; write-set: garden/**; outside the write-set every rule applies in full · covenant:s-garden @ bafkr4ij…a9
WHERE TO LOOK
  gate: just gate garden · garden/build-manifest.json#/gate/projects/garden @ bafkreif…b5
  open correction on garden/import.sh · note bafyreii…c8
    UNTRUSTED EVIDENCE — <provider> (quoted; not a steward ruling)
    ```text
    > import with the empty-date fixture; runs without it hid the rows they skipped
    ```
  working record: unmeasured: no finding, open question or next action is recorded for this task yet
````

Before the covenant the authority line reads `none held: no covenant is granted to this session
yet`. At `minimal`, a COURSE OF ACTION part follows: `1. run the scenario tagged
garden-records-stay-true`, `2. work in garden/totals.md, as the ruling on it says: totals are
derived from records, never typed`, `3. the authority line above governs anything outside
garden/**`. An unbound model's heading reads `BRIEF — level minimal (fixture-unknown is an
unrecognised model) · seat implementer · target garden`. A prompt with no changed source prints
nothing.

The quoted block follows `.claude/skills/stasis-keeper-brief/SKILL.md:35-44`: the label line
`UNTRUSTED EVIDENCE — <source> (quoted; not a steward ruling)`, then a `text` fence longer than
any backtick run in the content, with every content line inside it prefixed `> `. The skill's own
outer four-backtick wrapper is display only and is not rendered.

## 3. Landings

| | Contains | Observably true after | Gates selected |
|---|---|---|---|
| L1 | `closures.rs`; `notes_across_with` with context gather and render; gate tiers with `also_selected`/`extension_wide`/`ci_ignored`; the habit chooser by path token; `start_dir` and `HabitEntry.invariant` as groundwork; the "When you are corrected" paragraph of the root-gospel package (`.epr-meta/elohim/packages/agentdocs/elohim-root-gospel.json`), re-projected to `CLAUDE.md` and `AGENTS.md` as the `elohim-package-authoring` skill directs, because L1 makes its sentence that an edited file's correction "no longer shows in `epr flow context`" false. No contract change. | `epr flow context .claude/hooks/berth-note-inject.py --root /projects/elohim` (the correction lives in the main checkout's store until L2) shows `bafyreibpxb2u…riceq` marked `written against different bytes`; one gate line with what a push selects; one habit or a reason | cargo-bridge, jenkins-bridge, eprfs, eprfs-live-corpus, orchestrator, orchestrator-tree, address-profile-lint, memory-ceremony (through `concerns.rs`, `context.rs`, `flow/mod.rs`), rust-style (path oracle; gate sets below) |
| L2 | `main_checkout`, `plane.rs`, the sweep and scan guard, the non-Rust readers, the fold (crossing skip, actor claims, inversion counts), the Codex writable-root check. One landing. | a worktree note is read from main and survives the worktree's removal | the L1 nine plus agentic-shift, genesis-a2o and genesis (gate sets below) |
| L3 | §2.4 whole, the runtimes fixture, the one contract commit, `run-projection.py` with its tests run by hand (`.claude/**` is ci-ignored), `stasis-keeper` skipped. Opens through `plane.rs`. | scenario 1 (two recordable clauses), subagent target via `--as`, isolated compose-none, level scenario, on one device | the L1 nine; dialogue-stasis if `genesis/agentic/agent-runtimes.json` changes (gate sets below) |
| L4 | `recall remember` without an edge; SubagentStart head (payload verified first; PreToolUse(Task) rewrite as alternative); compact/resume `--reshow`; re-render after `epr covenant record` with `intent_basis` adoption | | |
| L5 | commitment rungs and done ladder; contract and grant authority for shift steps and runners; `unknown — local to <device>`; appreciation | | |
| L6 | docs: drop the router's "record corrections from the main checkout" line, re-project, habit delta | | |

**Gate sets per landing.** Computed on 2026-11-23 with `GATE_ORACLE=path GATE_INPUTS=off node
genesis/orchestrator/gate-runner.mjs --changed-file-list --names` over the files this document
names for each landing. The default `rakia` oracle may add one hop of consumers and was not
consulted.
- L1 (`flow/closures.rs`, `concerns.rs`, `read.rs`, `context.rs`, `registers.rs`, `flow/mod.rs`,
  `habit_census.rs`, `context/actionable.rs`, `tests/flow_edges.rs`, the vector file,
  `gate-runner.test.mjs`, the gospel package and its two projections): cargo-bridge,
  jenkins-bridge, eprfs, eprfs-live-corpus, orchestrator, orchestrator-tree, address-profile-lint,
  memory-ceremony, rust-style. memory-ceremony is selected only through `concerns.rs`, `context.rs`
  and `flow/mod.rs`; `read.rs`, `registers.rs` and `closures.rs` are not in its sources, and the
  gospel files alone select nothing.
- L2 (`git.rs`, `flow/plane.rs`, `flow/memory/recall/mod.rs`, `lsp.rs`, `shift_state.py`,
  `shift_reconcile.py`, `shift_runtime.py`, `habits-project.py`, `saga-status.py`,
  `gate-impact.mjs`, `memory-index-projection.py`, `local-view-browser.mjs`): the L1 nine plus
  agentic-shift, genesis-a2o and genesis.
- L3 (`recall/brief.rs`, `recall/lens.rs`, `recall/mod.rs`, `tests/common/mod.rs`,
  `recall-contract.json`, `recall-questions.json`, `run-projection.py`,
  `covenant-subagent-stamp.py`): the L1 nine; dialogue-stasis joins if
  `genesis/agentic/agent-runtimes.json` changes (it is among that gate's sources). The two hooks
  alone select nothing.

L4 to L6 cannot be computed yet: this document names no files for them.

GAP: the files L4–L6 touch, what becomes observable after each, and the gates they select.

L2 onward needs a fresh-worktree gate run on ethosengine before it is called met. The fold on shem,
then ethosengine, is an act under the covenant recorded as a flow note: dry run, review, `--apply`
once, re-run to show 0 new.

**First slice: L1.** It is the precondition named first and visible on one device at once; it
removes the duplicate reader before anything depends on it; it moves no contract, pin, session state
or store location, so it refuses no session and cannot strand or publish a record; and every
`context.rs` and `concerns.rs` change rides one memory-ceremony run. L3 then builds on measured
resolvers.

## 4. Tests and fixtures

New test files: `tests/flow_store_shared.rs` (L2, a real repository with `git worktree add`) and
`tests/flow_memory_recall_placement.rs` (L3, garden fixture built in the file, brit stub lifted from
`tests/covenant.rs` ~154 to `tests/support/covenant_stub.rs`). Unit tests land in `closures.rs`,
`read.rs`, `registers.rs`, `context.rs`, `git.rs`, `plane.rs` and `lens.rs`. L1's are listed below;
the L2 and L3 names are in the raw design file, which is gitignored (`**/local-dev/`) and present
only on shem.

L1 new tests:
- `closures.rs`: `a_closure_for_a_different_subject_closes_nothing`.
- `tests/flow_concerns_corrections.rs`: `corrections_from_records_equals_corrections` (parity on
  the existing fixture).
- `read.rs`: `a_label_arm_finds_a_note_pinned_to_other_bytes`,
  `a_measure_observation_is_admitted_by_address_only`,
  `a_closed_correction_is_dropped_before_the_window`,
  `under_matches_a_file_in_the_directory_but_not_a_sibling_prefix`,
  `one_note_matched_by_both_arms_appears_once`.
- `context.rs`: `choose_covering_habit_prefers_a_habit_that_names_the_path`,
  `several_namers_are_ambiguous`, `a_lone_declared_habit_covers_with_its_basis_named`,
  `several_declared_habits_are_ambiguous_with_the_package_and_cure`,
  `no_candidate_is_none_with_a_reason`, `a_short_directory_never_matches_inside_another_word`,
  `the_root_target_reads_the_root_package_not_every_habit`.
- `registers.rs`: `projects_sharing_one_dir_resolve_to_the_path_form_command_naming_every_member`,
  `a_project_whose_sources_name_the_path_beats_a_root_scoped_tie`,
  `a_glob_without_a_literal_prefix_never_names_a_path`, `tied_source_namers_are_ambiguous_never_picked`,
  `a_ci_ignored_path_selects_no_push_gate`, the vector test over
  `genesis/orchestrator/testdata/gate-match-vectors.json` (with its `gate-runner` twin), and, for
  the `start_dir` groundwork, `a_directory_target_reads_its_own_manifest`.
- `tests/flow_edges.rs`, beside `a_ruling_and_a_verdict_are_readable_in_context_newest_first` on
  `valueflow_fixture()`: `a_correction_stays_in_context_after_its_subject_is_edited` (bytes
  `differ`, `written_against` the old body CID, `identity.cid` the new one),
  `a_note_on_unchanged_bytes_reads_current`, `a_closed_correction_leaves_context_and_concerns_together`,
  `measure_folds_do_not_crowd_the_subject_window`, `the_subject_window_stays_bounded`,
  `context_lists_every_open_correction_concerns_lists`.

Tests the review added for later landings: `no_flow_plane_path_is_opened_outside_plane`, `fold_never_appends_a_crossing_kind`,
`a_folded_record_never_enters_the_channel_tree` (via `CrossingAllowList::admits`),
`fold_carries_the_claims_its_notes_pin`, `shift_runtime_test.py::codex_step_can_write_the_device_store`,
`a_successor_after_an_executor_change_still_gets_its_brief`,
`an_unchanged_stat_key_returns_without_loading_the_store` (under 150 ms),
`a_ruling_from_a_non_steward_is_quoted_untrusted_and_never_a_course_step`, and a hook test that
`SKIP_AGENT_TYPES` covers each isolated seat with an agent type. The draft's earlier-bytes test is
replaced by one asserting `NoteBytes::Differ`; the granted-covenant test asserts two clauses and
names the third as owed.

Existing tests and fixtures to update or rerun:
- L1, kept green unmodified: `tests/flow_concerns_corrections.rs` with byte-identical `--json` (it checks issue contents and order and the REVALIDATE exit); `genesis/a2o/steps/devflow/run-plane.steps.ts:365`, which reads `surfaces`, `reason` and `target` from that JSON; `tests/flow_participate.rs:653` `plain_context_is_unchanged_and_participate_refuses_its_windows`, which pins the exact top-level key set of `context --json` (L1 adds no top-level key); `tests/flow_edges.rs:1352-1381` `context_names_the_covering_habit_and_the_owning_gate_for_a_path`, which asserts `habits` holds `dev-system-equilibrium` with `source` `.epr-meta/dev-system-equilibrium.habit.md`, `gate.project`, `gate.command`, an empty `gate.ambiguous`, `gate.target_dir` and `gate.rustflags`, and no gate for `briefs/task-1-brief.md`; `a_ruling_and_a_verdict_are_readable_in_context_newest_first`, `the_notes_window_shows_exactly_n_taken_from_the_newest_end` and `a_note_resolves_a_gap_id_to_its_commitment_and_rolls_up_onto_the_plan` in the same file.
- L1, updated: the `registers.rs` unit test `covering_habits_are_the_ones_whose_checks_or_refs_name_the_path` calls `habits_covering` directly and follows its new rule or name; `the_gate_is_the_nearest_manifest_entry_whose_dir_covers_the_path` gains `members == [eprfs]` and `basis` `dir`; `NoteSource {` literals at `context.rs:213`, `:220`, `read.rs:200` gain `label: None`. Rerun the context consumers `flow_context_sections.rs`, `flow_acceptance.rs`, `flow_feedback_source.rs`, `flow_report_parity.rs`. No test, hook test or a2o step asserts the text of the NOTES, HABIT or GATE lines.
- L2: `shift_test.py`, `shift_reconcile_test.py`, `habits_project_ruling_test.py`, `gate-impact.test.mjs` gain linked-worktree cases; rerun `peer.rs` `peer_leases_reads_the_channel_store_the_daemon_serves_not_the_callers`, `supervise.rs` (~2277), `lsp.rs`, `gate-cost.test.mjs`, `berth_test.py` `WorktreeIntentTest`.
- L3, runtimes fixture: `tests/common::repo()` gains `genesis/agentic/agent-runtimes.json` binding claude-code C1 haiku, C2 sonnet, C3 opus, C4 fable, so the 16 `claim_actor` tests in `tests/flow_memory_recall_lens.rs` keep their levels; exact provenance strings move to the `→ C1 →` form.
- L3: the `lens_table` pin in `tests/common/mod.rs` (~256–275) moves to `capability`/`unbound`, minimal `density_bytes` 2000 there and in `builtin_table` together; three `lens.rs` unit tests rewritten.
- L3 contract re-pin: `recipe` and every `in_scope_of` in `recall-questions.json`, keeping `q-top-red` valid; `GOLDEN_FOCUSED` and `GOLDEN_WHOLE` re-baselined with a dated row in `tests/fixtures/recall-golden/README.md`; `GOLDEN_REFUSAL` should not move; run `tests/flow_memory_recall.rs` (~1519).
- L3: `session_envelope.rs` `a_first_open_does_not_wait_on_its_own_lifecycle_guard` asserts `intent_basis == "pending"` within <3 s charged, <4 s wall; `bootstrap_projection_test.py` (~194–226) asserts purpose `placement`.

## 5. Rulings from the review

1. **One notes reader** (`notes_across_with`, `ClosureIndex`). The draft added a second, with a mark ("earlier") no record supports.
2. **Authority says only what a clause records.** The draft printed "must ask first" and "may never", which no `covenant.rs` KINDS entry holds, and its course step decided row 79. Scenario 1's third clause goes to the steward: a clause kind, or a story amendment through the blind-reader loop.
3. **Only steward rulings are guide-stars; other note text is fenced.** The draft let any agent's ruling steer every later prompt.
4. **The fold skips crossing kinds.** The draft would have sent 3,248 berth events and 82 pre-G1 verdicts to ethosengine.
5. **Actor claims fold with the notes that pin them**, and the sweep covers the two missed readers and `.claude/worktrees`. The draft folded flows only.
6. **A sandboxed step writes the device store or fails naming the cure.** The draft made stranding a silent failure.
7. **Placement pins nothing.** The draft's `Execution::open` path killed the brief after a contract or executor change.
8. **The per-prompt path reads a stat key** (hooks rule `per-turn-injector-carries-its-budget`). The draft loaded the store, brit and git every prompt.
9. **The gate line reports what a change selects.** The draft hid memory-ceremony, extension-wide gates and the ungated hooks.
10. **Habit matching is by whole-path token; `.` reads the root package.** The draft's substring test read `ambiguous: 68`.
11. **Isolation is checked before open; tasks are per seat.** The draft wrote the conductor's continuation first and let a subagent's need become the parent's task.
12. **The lens change ships a runtimes fixture.** The draft listed two of sixteen affected tests and changed the plan's binding silently.
13. **A read does not rewrite intent; the field is `intent_basis`.** The draft adopted on read and reused `intent_source`, which means the recipe's plan path.
14. **The fold prints its order effects first and refuses inversions by default.** The draft left them for `--apply` to discover.

Minor corrections are applied above: the floor-line signature, fixture locations, module name, tree
walk, literal separator, prompt on stdin, and "navigation choices print one unresolved line" in place
of "sessions refuse once".

## 6. Left out, and unverified

Left out:
- The incident verb `epr flow placement --incident`; a slice-12 coordinator reuses this provider.
- `nearestNeighbours` and `station`: emitted empty with named omissions.
- Directory targets for `epr flow context`. `registers::start_dir` and `HabitEntry.invariant` still
  land in L1 (§2.2), as groundwork first used by placement in L3.
- Re-recording the task on a later prompt; a claimed commitment overrides it in L5.
- Rewording the system stars (the steward's content).
- Exact CI gate selection: `rakia affected` is not consulted.
- Commitment and intent scoping by stale file CID; a backlog row beside rows 76–81.
- The "matches the committed bytes" enrichment, to stay subprocess-free.
- Merging worktree feedback stores (generation-based, not mergeable by append).
- Moving `recall/`, `stasis/`, `lenses/`, `index/`, `affiliations.jsonl`, `memory-writes.jsonl`, `memory-import.log.jsonl`, `identity-reserve/`.
- Unioning separate clones; submodule stores stay separate.
- Deleting worktree logs after the fold; simplifying `readCycleUnion` before a release cycle.
- Cross-device notes: `berth.channel.crosses` is default-deny; graduation stays the transport.
- The berth-note subagent repeats: a correction from a session with write standing.

Unverified:
- SubagentStart payload fields beyond `session_id`, `hook_event_name`, `agent_id`, `agent_type`.
- Whether `load-project-context.py` can read SessionStart's `source`.
- A read accessor for the envelope's recorded model (`recorded_model` may be needed).
- Placement latency of about 0.6–0.9 s (extrapolated).
- That `GOLDEN_REFUSAL` does not move.
- Token binding: opus-5-5 → C3, fable-5-2 → C4, gpt-6-astra → C3 and C4, gpt-6-sol unbound (sol readers drop to minimal).
- Stasis tooling's tolerance of a later per-binding field.
- `flock(2)` exclusion on the `/projects` PVC.
- Memory validation and `memory/attribution.rs` with `actors.jsonl` moved alone.
- What `index/` (185 MB), `lenses/` and `berth-note/` hold.
- That every actor and feedback writer uses the opens the guard scans (15 references, 13 opens counted).
- `canonical_body` normalization (a whitespace edit may read "different").
- Shift contracts pinning an older `EPR_BIN`.
- The fold's effect on gate-cost recency and the "later than" check.
- `stocks --stock commitments` inflation from branch-worktree `epr flow project`.
- Gap-id label collisions across branches.
- That `context_cid` is the only pinned consumer of context JSON.
- That a successor's bin-store pin can differ after a restart.
- That the Codex workspace-write sandbox is enforced here.
- Whether pre-push runs gates serially or on both cargo seats.

## 7. Costs and risks

No version bump and no Edge roll, provided the work does not reuse device-runtime's loader, add a
workspace dependency or edit eprfs-core: `epr-cli` and `device-runtime` are `publish = false`, and
the Edge manifest watches `elohim/eprfs/Cargo.toml`, `eprfs-core/**`, `eprfs-serve/**`,
`device-runtime/**`, `eprfs-ts/**` and `elohim/Cargo.lock`, not epr-cli. Run the fleet-roll and
doc-cut preflights by hand on each head.

| Gate | Expected | p90 | Note |
|---|---|---|---|
| eprfs | 36–56 min | 57–99 min | 3 local failures in 20; sibling reuse possible |
| memory-ceremony | 26–54 min | 110–140 min | not reuse-enabled; `concerns.rs`, `context.rs`, `recall/*.rs`, contract |
| orchestrator | ~7 min | 8 min | |
| agentic-shift | ~5 min | — | L2 |
| cargo-bridge · orchestrator-tree · dialogue-stasis | ~4 · ~4 · ~3 min | 8 · — · — | dialogue-stasis only if `agent-runtimes.json` changes |
| jenkins-bridge · eprfs-live-corpus · rust-style · address-profile-lint | ~2 · ~2 · 1–2 · ~0 min | 5 · 4 · 3 · — | |
| genesis-a2o · genesis | not measured here | — | L2 only (`local-view-browser.mjs`) |

An L1- or L3-class push costs about 1.5 h typical and 3.5–4 h at p90 if run serially; L2 pays
memory-ceremony again. Which gates each landing selects is listed under §3, computed with
`GATE_ORACLE=path`; the default `rakia` oracle may add one hop and was not consulted.

**Build and test.** Build and test from the worktree with `just gate eprfs` and `just gate
memory-ceremony`; the gate runner resolves the cargo slot, `RUSTFLAGS` (both projects declare
`rustflags: ""`) and the toolchain that `pool-policy.json`'s `"*"` entry pins (1.100.0). Real gate
manifests declare no `targetDir`, so the GATE line prints no `CARGO_TARGET_DIR`. For a focused run
use `CARGO_TARGET_DIR=<slot printed by bash genesis/agentic/bin/cargo-pool key> RUSTFLAGS="" cargo
test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --test flow_edges` from the
worktree; the `cargo-disk-guard` PreToolUse hook denies native cargo without `CARGO_TARGET_DIR`
and names that command. Use this worktree's own slot, not another worktree's. `memory-ceremony`
runs `flow_concerns_corrections.rs` and the other recall test binaries but not `flow_edges.rs`;
`eprfs` runs the workspace tests (the whole workspace under the default `GATE_NARROW=shadow`),
`flow_edges.rs` among them.

**One-time refusals.** The contract commit moves the method CID: every live recall session is
refused once ("algorithm bytes changed"; `adopt` is the remedy), and so does the executor digest;
announce both over berth. `recall-reaches-authority` gets a delta: its de-duplication restarts at the
new CID. Goldens move by construction. After L1, `context_cid` moves for every path target: it
hashes the whole native context JSON (`context_sections.rs:97`), and L1 adds gate and habit fields
and the bytes mark on every note on the target. Every earlier navigation choice prints one
unresolved line.

**What the fold changes.** The store grows about 26%: measure `context` wall and RSS before and
after, record both in the habit delta, and do not call L2 met above 3 s. One shared inode means `epr
flow project` blocks other worktrees' writers while it holds its lock. Folded records land after
main's, so a correction folded after its closure stays open with a `REVALIDATE` issue (counted, and
refused by default), and gate-cost `costOf` (last 10 or 20 rows by append order) may read old rows as
recent for one push. Old binaries keep writing worktree logs until replaced; the write-time fold
catches them, and nothing is deleted.

## 8. Proposed amendment from the SP-287 survey (2026-11-26; not ruled)

Proposed by flight F4 of plan `2026-11-26-research-disciplines-exercised`, from the survey
`genesis/research/apollo-sp287-what-made-apollo-a-success-cross-pollination-2026-11-26.md`
(§2.2, rules C1 to C6; §2.3; study items S8 and S9). Nothing in sections 1 to 7 is changed by
this section. It states what would change in §2.4 and in the landing rows if the steward rules
for it; until he does, the landings stand as written. The author of this section did not write
the design and has not built against it.

**P1. The subject is an address, of which a path is one case.** §2.4's surface takes
`--target <path>`. The steward's direction of 2026-11-26 is that a brief composes "from the
position in the world", and that the position may be a node, an agent or any EPR an agent is
invoked from. Proposed: `--target` takes a path, a content address the device's store knows, or
an `epr:` id. For a subject that is not a path, a part that cannot be resolved prints
`unmeasured: <reason>` as every other missing part does. Today `epr flow context <cid>` accepts
only a CID that a local record already mints or names, and for a bare CID it skips the seals,
habit, gate and governance sections with a scope note (`flow/context.rs`, `resolve_target`); the
placement read should say unmeasured line by line where that read skips sections.

**P2. A further part, kept under every size limit: the stop.** §2.4 has four parts (goal and rung, guide-stars,
authority, where to look) and, for the scaffold `locate-and-hand-one-command`, a fifth, *Course*. Apollo's crew carried enough to end safely with no ground contact.
Proposed: a part named *Stop*, composed from the dispatch or commitment: the conditions on which
the seat stops and reports, and the state it must leave. It prints
`unmeasured: no stop condition is recorded for this task` when none is, and that line is itself
the finding. Under a size limit every other part is cut before it.

**P3. Rank against the declared ceiling, and say what was cut.** The habit declares a
session-open ceiling (95,000 bytes soft, 100,000 hard) and the design shapes output by level
(`lens_table`). Proposed: within a level, lines are composed in a stated order (stop; goal and
rung; course, where the scaffold has one; authority; guide-stars; where to look) against a share of that ceiling, and a brief that
drops lines ends with one line saying how many and where they can be read. A line is one of
three kinds: delivered, reachable by an address the brief prints, or withheld; the kind is
visible.

**P4. Arrival conditions are selected by the act, inside this read.** `epr device arrive` reads a
list of conditions and a session is shown every one of them at start, whatever it came to do.
Proposed: each condition declares which kinds of act depend on it (read, edit, run a gate,
land, run the mesh, roll the fleet); the placement read selects the conditions the seat's act
needs and prints a count of the rest with the command that lists them. The declaration is a
field on `Check` in `elohim/eprfs/epr-cli/src/device/arrival.rs`. No hook filters conditions:
"native composes; hooks only render" holds. A condition nobody's act selects still needs a seat
that repairs it between tasks; this section names that need and does not design the seat.

**P5. Study, no row: a control room is this read taken from a seat.** We decline a central
control room and keep the room: any EPR should be able to compose one for the agent or human
who sits there. `--as <seat>` on an address already composes a brief for a seat at a position.
What the seat may invoke from there, and what it shows of other seats at the same position, is
not designed here.

**P6. Study, no row: a judging seat's brief carries where the other voices come from.** Agents
working together are councils in small. A seat that judges should be handed, with the artifact,
the provenance of the seats that produced it: model, lab, device, and what each read. The
verdict notes of sprint `2026-11-26-research-disciplines` record this by hand; the gate
evidence's `independence` field is the same idea for machines.

**What would change in §3 if this is ruled.**

| Row | Change |
|---|---|
| L3 | `--target` accepts an address (P1); the *Stop* part (P2); composition order, the share of the ceiling and the closing "cut" line (P3). Scenario 1 gains a subject that is not a path and a task with no recorded stop condition |
| L4 | The SubagentStart and compaction renders carry the *Stop* part unchanged; arrival conditions are rendered as selected by the act (P4), which needs the `Check` field first |
| L5 | Commitment rungs supply the *Stop* part's conditions where a commitment records them |
| L1, L2, L6 | No change |

**Not proposed.** No second path to the brief: no `epr device status --for <act>` and no
filtering in `device-arrive.sh`. A due date on `claim` is not part of this section; it waits
for a reader (backlog `agentic-harness-borrows-backlog` row 16).
