---
id: one-baseline-journal
title: One baseline — sprint journal
date: 2026-11-19
status: OPEN
author: shem session b49f7c46 (Claude Fable 5.2), orchestrating; ethosengine sessions a222db7c (lane E12) and 0e0924d5 (brit backlog sprint 3)
habits: [workspace-device-arrival, push-gate-pays-once]
---

# One baseline — sprint journal

## Why

The native-throughput sprint (`../2026-11-17-native-throughput/journal.md`) closed three of its
lines and failed the first: a release from one device's gate, drawn by the other in seconds,
crashed on arrival because its C was compiled at the producer's compiler default. Row 29.2 is
now answered (that journal, 2026-11-19): C compiles at the Rust target's baseline, set by a
declared record and confirmed by checks. This sprint builds that answer and collects the
receipt, and takes the two cheapest cuts to what every landing pays.

The steward opened it on 2026-11-19 ("we need to run another sprint to make some progress") and
started a brit sprint on ethosengine alongside it, asking the sessions to work together.

## Definition of done

Each line closes with its receipt in this journal.

1. **One device's release is the other's.** Two consecutive `dev` landings from ethosengine are
   followed on shem with `EPR_FOLLOW_SIBLING=1` and no local build; the daemon restarts on the
   drawn bytes and serves. Recorded: draw time, and the compiler flags written in the drawn
   binary.
2. **A release nobody measured is refused before it is fetched.** A sibling release without the
   `env:isa-c` marker is refused as unmeasured, zero bytes fetched, and the device builds its
   own.
3. **A landing pays less.** The orchestrator gate's wall on a docs-only landing, measured before
   and after on both devices, with the two slowest test files no longer paid in full on every
   landing.
4. **The orchestrator suite keys on what it reads, soundly.** The rework of `self/nt-e11` lands:
   the named half runs in a view holding only its declared reads, reuses on a docs-only landing,
   and `just gate orchestrator` still runs every test file.
5. **The sessions do not collide.** Every landing that touches `genesis/orchestrator`,
   `elohim/eprfs` or `elohim/ark` is announced one line ahead; no landing dispatches Edge by
   accident.
6. **The record is current.** This journal, backlog rows 28–30, and one delta line in each
   habit atom the sprint moves.

Not in this sprint: the bin-store judge (waits for a deliberate Edge roll), the glibc floor and
a linked sysroot (row 26.4), gate-verdict enrolment, the first-`epr` verifier, labelled
non-baseline variants.

## Lanes

| Lane | Owner | What |
|---|---|---|
| E12 (b) | ethosengine a222db7c | widen the gate env-key pattern; key `pool-lib.sh` for every cargo gate (one re-key for both) |
| L1 | ethosengine a222db7c | the per-triple record, the gate's exported flag, the seconds check, the producer withholding an unmeasured offer, the pin test, cc >= 1.11.1 in the eprfs lock |
| L2a | ethosengine a222db7c | producer side in `epr-cli`: C variables recorded, put checks, the `env:isa-c` marker |
| L2b | shem (builder) | receiver side in `epr-cli`: marker rule before fetch, the bundled-C exercise before ingest and before current, explicit-CID routes, the honest-draw story |
| T1 | shem (builder) | the orchestrator suite's slowest test files, proof preserved |
| E11 rework | ethosengine a222db7c, after L2a | the named half runs in a view of its declared reads; the read fence leaves the trust path |
| brit sprint 3 | ethosengine 0e0924d5 | brit log, branch and tag verbs, fetch, session refs, the mutation lock; ark's brit index trust |
| brit ref spellings | shem (builder), for brit sprint 3 | `rev-parse`, `ls-remote`, `for-each-ref` and a global `-C` as discoverable spellings over verbs brit already has; handed back as patches |

Contract between L2a and L2b: the Produce slot is `env:isa-c`; its value is the declared isa
string, `x86-64` for `x86_64-unknown-linux-gnu`; it is written only after the put checks pass; a
triple with no record writes none.

## 2026-11-19: the experiment passed before anything landed

Ethosengine built `epr` at dev `90ac778f9` with `CFLAGS_x86_64_unknown_linux_gnu=-march=x86-64`
(scratch slot, 231 s) and offered the bytes with nothing made current. On shem:

| Check | Result |
|---|---|
| fetch by output CID | 4.4 s, 98 chunks, sha256 matches |
| compiler flags written in ring's 13 C files | all `-march=x86-64` (the crashed release: all v3; shem's own build: all v2) |
| `--version`, `peer serve --check` as the candidate | ok |
| run as shem's peer daemon, real identity and environment, 75 s | alive until stopped; folded ethosengine's head; no new invalid-opcode trap (the crashed release died at once) |
| functions holding v3-only instructions, by origin | SQLite 0, other C 6, ring 15, blake3 3, Rust 830: the C ones are the hand-written kernels chosen at run time, and shem's own build has the same shape (0 / 6 / 15 / 3 / 826). The crashed release: SQLite 129, other C 99, ring 37 |

So the flag reaches SQLite, which carries no compiler record of its own; the instruction count
by origin shows it where the compiler flags cannot. Shem's supervisor was stopped for the run
and restored; the first restore attempt ran with the device's XDG variables unset and found no
manifest, the second restored it (peer ready on shem's own build).

Rulings sent with the result: L1 records the C flags as inputs only and does not declare a
target at put (put refuses a declared target today, "build realization unsupported"); realising
it is L2a. `describe` swallowing the argument after `--target-cflags=...` is a bug for L2a. The
pin test reads the cc source from the cargo registry when present and pins the lock version
otherwise.

## 2026-11-19: working with brit sprint 3

The brit session asked for the review findings in its territory and got them, with the raw
outputs on the hand-over ref `self/nt-review-receipts`. What came of it the same hour:

- Its branch that moved a reader into `elohim-device-runtime` did select `elohim-edge` (it ran the
  dispatch walker). It is rebuilding the change with nothing under device-runtime.
- It took two findings as brit lanes: a step glob that crosses a gitlink hashes nothing (it will
  record the gitlink sha as an input), and cc >= 1.11.1 in brit's lock.
- A datum for row 29: on ethosengine `brit-cli`'s build probe already refuses gcc's v3 default,
  so no v3 brit was produced there.
- Shem took its ref-spelling lane, because ethosengine has one build slot and six lanes queued.
  Nothing is pushed to the brit remote by either session: its covenant is commit-only, and
  shem's unpushed brit commit `f35d1b753` crossed as a patch on the monorepo hand-over ref.

A slip to record: shem's orchestrator listed `rev-parse`, `ls-remote` and `-C` as brit gaps.
brit 0.2.6 has all three as `revision resolve`, `remote refs` and `-r`. They were spellings the
orchestrator never looked up, and many of this sprint's "Brit fallback" lines for read-only
questions were not justified. The lane above exists because the spelling gap is real for
agents; the capability gap was not.

## 2026-11-19: four sessions, one lane

By 04:30Z four sessions were working this tree: shem's orchestrator with three builders, two on
ethosengine, and a third shem session (d9c6be9b) asked by the steward to make the epic legible
to `epr flow project`. What crossed between them:

- **E12 (b) landed** from ethosengine at `dccd7a963`: the gate's variable pattern admits a
  triple-suffixed name, and `pool-lib.sh` is in every cargo gate's key (row 28's cheapest cure,
  paid with the same re-key).
- **Stations.** The third session handed over a branch (`claude/flow-legibility`, three commits,
  proofs re-run by the orchestrator before landing): section 8 of the native-dev-tooling plan
  turns the epic's tracks and joins into stations; `plans/2026-11-19-one-baseline-plan.md` restates
  this sprint's definition of done, lanes and inherited rows as stations, because
  `epr flow project` mints from checkboxes under `plans/` and `specs/` and reads neither a
  journal nor a backlog table; the Stop hook no longer walks every worktree inline (5.8 s to
  about 40 ms here); the `epr-meta` resolver hook parses under the memo.
- **From brit sprint 3's audits**, things that are ours: gate code reads a git refusal as a
  value (backlog row 31); `epr artifact put` spawns `brit-build-ref` with no declared trust
  (one line, with L2a); the repository's pack-index count against a 32-slot ceiling when brit
  runs at reduced trust (shem 28, ethosengine 33; the cure is a repack, the steward's store).
- **`brit rebase` measured on shem.** Two docs commits over two upstream commits: six minutes
  on one core, 29 GB of temporary writes, then `Directory staging is not supported`. It left no
  state. `git rebase` took under two seconds. The brit session took the numbers as its cost
  measurement; rebase stays a declared git fallback on both devices.
- **Why `brit push` never landed on dev**, reproduced by the brit session on the installed
  build: the upload request fails while brit is still sending and the cause is discarded, most
  likely at a 30 s total request timeout the HTTP library applies and nothing overrides (brit
  sends whole objects, about 351 KB for a landing against git's 24 KB, on a link recorded at
  5 to 15 KiB/s). Hook length was not the cause. Retiring the landing exception needs a
  supervised landing on a fixed build, the steward's word and a pin move.

Open with the steward from this hour: pushing `sprint3/*` to the brit remote (that session is
commit-only); a repack on each device; whether declared checkout trust covers linked worktrees
(it is per exact path, and agents work in worktrees).

## 2026-11-19: the exercise catches the crash; a landing costs less

**The exercise caught a real SIGILL.** Ethosengine built the L2b head (`2d70875a9`) with its
compiler at its default and offered the bytes with nothing made current (13 of 13 ring files at
`-march=x86-64-v3`, sha256 `ac1689be…fafe0`). On shem, fetched in 4.2 s:

| Run | Result |
|---|---|
| `epr doctor --exercise`, three runs | killed by SIGILL each time (exit 132); three new `trap invalid opcode` lines in the kernel log |
| `--version` of the same bytes | exit 0, the check the crashed release passed on 2026-11-18 |
| shem's own build of the same commit | `exercise: ok (x25519 32-byte secret, ed25519 signature e5564300c360ac72, sqlite 3 rows) in 1.7 ms` |

It was not run as a daemon. This is the receipt that the receiver-side check stops the incident
class on the CPU that crashed.

**L2b's review loop** (`self/nt-l2b`, head `0544393d0` plus a last round in progress). Three
independent passes, each ending FIX FIRST on something narrower: explicit land and pin ran held
bytes before the store verified them; the exercise could outlive its deadline when a descendant
kept the error stream open; the target was inferred from predicates an x32 build also satisfies;
then the cleanup collected the child before signalling its process group, so a recycled group
number could be signalled; then the same on the one path where the system reports the child
already collected. Each was fixed with a test that fails first. The story had two fresh reads;
their five correctness findings are fixed (a title claiming every processor, an unexercised
marker value, two missing preconditions, a closing step that named a marker one row lacks). The
loop closes after the round in progress. Known limits, needing the bin store's own crate: a held
record for the commit being followed that fails landing health is returned before the local
build; `EPR_PIN_CID`, existing session pins, bootstrap reuse, `bin ingest <path>` and `EPR_BIN`
are not judged.

**L1 on ethosengine.** The cc lock bump alone is proven by `cargo test`: the eprfs gate passed
in 940 s and the gate-built `epr` has all 13 ring files at `-march=x86-64` with no C flag in the
gate's environment. Its reviews sent two pieces back (a withheld offer and a preflight warning
both rode the failure path, so a green run showed red). Ruled: a `notice` event kind that is
never counted as a red. Ethosengine lands L1 and L2b in one push, so the eprfs gate is paid once.
Ruled out of L1: widening to the `elohim` cargo workspace (cc 1.2.52, with ring). Nothing from
it crosses between devices, and `elohim/Cargo.lock` is a storage build input, so its bump waits
for a deliberate Edge roll.

**DoD 3, shem's half.** The orchestrator suite as the gate runs it, on shem, both exit 0:

| | wall |
|---|---|
| before (dev `01e4de7f1`) | 270.6 s |
| after the five test files (`61ebe79d`) | 219.8 s |
| after the review fixes (`c269111c`), which gave 18 s back on purpose | 228.7 s |
| after the storage-image suite's doubles became shell (`dfd108f1`) | 188.1 s |

No test case or assertion was removed; each changed test was shown failing under a mutation of
the code it guards. What was slow: seventeen receipt cases run one after another against the
real tree (now a fixture built from the tree's scripts at test time, plus one live refusal
case); 63 read-set calls parsing about 8,000 Rust files for 2,300 unique ones; thirty selection
tests each walking the tree for manifests; two tests waiting out timers (8 s for a note that is
never sent; 30 s because only the shell was signalled); and 1,485 `node` startups for a fake
`git` and `curl` (replaced after their answers were recorded and compared byte for byte). The
review of the first commit found that two speed-ups had thinned a proof (a group signal let a
broken interrupt handler pass; a timer started at file load); both were put right. Ethosengine's
half of the measurement follows when this lands there.

**For brit sprint 3, from shem.** Six patches (rev-parse, for-each-ref, ls-remote,
check-ref-format, a global `-C`, a quiet closed pipe) on the hand-over ref, built from a census
of shem's agent transcripts since 2026-11-06: 3,767 git invocations, 1,340 declared fallbacks,
about 1,700 of the form `git -C <dir> …`. The brit-first hook's own ledger could rank none of
it: it keeps no command text and its capture call fails. The review of the six found the closed
pipe fix too broad (a rejected push could exit as a closed pipe) and a dozen parity edges; the
fix round, with `merge-base --is-ancestor` and `rev-list --count`, is in progress, and the brit
session holds the branch out of its merge until then.

## 2026-11-19: one landing for two changes; what a hand check missed

**L1 and L2b are on `dev`** at `71069b1f5` (14:14Z, one push from ethosengine, 987 s). L2b's
eight commits come first, byte-identical in content to the reviewed hand-over (same tree at the
eighth commit as shem's own replay). L1's twenty-five follow: cc 1.11.1 in the eprfs lock, the
per-triple record (`declared_targets` in `pool-policy.json`), the gate exporting the C flags to
the four eprfs-workspace projects, the seconds check (it warns and withholds, it does not
refuse), the producer withholding an unmeasured offer, the pin test, and a `notice` event kind
that no red measure counts.

| Push | Wall | What ran |
|---|---|---|
| first, `7edd733a0`, 13:15Z | 2,566 s, red on one test | eprfs 1,141 s, memory-ceremony 248 s, orchestrator 198 s (153 to 188 s before L1's new test files); all green and recorded except the one red |
| second, `71069b1f5`, 13:57Z | 987 s, green | eprfs, eprfs-live-corpus, jenkins-bridge and rust-style-checker reused; orchestrator 133 s; memory-ceremony 224 s (not enrolled for reuse, by policy); agentic-shift 161 s |

The red was `eprfs-produce.test.mjs`: a case that expects the C flag variables unset inherited
them from the gate's own environment, which L1 now sets. Its author's suite run and shem's check
of the hand-over (52 tests, 51 passed, 1 skipped) both ran outside a gate and passed. A check of
a test that depends on the gate's environment has to run through `just gate`.

**A fleet roll caught by hand.** Before pushing, ethosengine ran the dispatch walker over the
stack's 43 paths. It selected Edge's `cargo-build-storage`, with `build-angular`, the DNA schema
step and three genesis schema steps: L1 had added the `notice` kind to
`elohim/sdk/schemas/v1/gate-live-event.schema.json`, and `elohim/sdk/**` is a storage build
input. Ruled at 13:06Z: the fleet schema stays byte-identical; the kind validates against an
orchestrator-local addendum beside its only reader; a test holds the two schemas' kinds disjoint
and every written event valid against exactly one; moving the whole schema out of `elohim/sdk`
joins the changes that wait for a deliberate Edge roll (four now, backlog row 29). No gate or
hook would have said any of this at push time (row 33).

**The hand check was weaker than we believed.** Shem started a preflight check, `fleet-roll`,
under the habit `careless-reds-cost-seconds`. After two build rounds an independent review ended
FIX FIRST, and its first finding is about the tool both devices had trusted that morning:

| | the JS walker (`graph-walker.mjs`) | CI (`build-graph.groovy`) |
|---|---|---|
| a step whose dependency is stale | not marked (skips propagation by design) | marked, to a fixed point over `depends` |
| a tracked dotfile under a watched directory | not matched (picomatch default) | matched (`**` covers dotfiles, `.epr-meta` excepted) |

Reproduced against the real manifests: a push of only the `elohim/holochain-conductor` pin, or
of `elohim/sdk/schemas/.gitignore`, selects an Edge build in CI and nothing in the walker.
Edge's build steps depend on `elohim-conductor:build-conductor-image`, `elohim-rakia:rakia-ci`,
`elohim-holochain:build-happ` and `elohim-brit:brit-ci`, so a conductor, rakia or brit pin move
selects an Edge image build whatever the walker prints. The landed stack was re-checked with a
fixed point over every manifest's `depends`: nothing propagates and no path is a dotfile under
a storage input, so it reached Edge as validation only. The check is in a third round (parity
with CI's graph, each pushed branch judged on its own commits, honest wording, named failure
modes) and has not landed. Its refusal follows the hook's declared policy: red within seconds,
the push refused when the hook ends, or at once under `GATE_ON_RED=stop`.

**A `[skip ci]` tip swallows the push under it.** Read in `genesis/orchestrator/Jenkinsfile`
by that lane's builder and confirmed: CI reads only the tip's message (`:2652`); a skip tag
dispatches nothing (`:2657`), and a green run still moves the baseline to that tip (`:230`), from
which the next run diffs (`:1450`). Shem did this twice on 2026-11-19: the push ending
`8dc35ad48` carried two code fixes under a docs tip, and the push ending `2080441f5` carried
T1's three test commits. The walker (with the limits above) selects no pipeline for the first
and only `lint-jenkinsfiles` for the second, so nothing that mattered went unbuilt. From now a
docs commit tagged `[skip ci]` is pushed alone; the fleet-roll lane adds a warning for a skip
tag over any pipeline's inputs.

**A near race, and what the silence was.** Ethosengine's session sent nothing for seven hours
while it worked L1's fix round. At 13:02Z shem announced it would land L2b alone. The brit
session, on the same device, took a read-only look: the stack had been replayed onto `dev`
minutes earlier. Shem withdrew at 13:04Z and nothing was pushed. Two notes of shem's carried
clock labels typed from estimate (13:05Z and 13:20Z for 13:02Z and 13:04Z), from which a peer
inferred a clock skew that did not exist. Kept from this: ask the device's other session for a
look before changing a landing plan; every stack gets a hand-over ref (`self/nt-l1` was pushed
before the landing) so no landing depends on one session; read the clock, do not type it.

**Also this day.** Agents held on an approval nobody was present to give read as `running` on
both devices (row 34). `brit rebase` on 0.2.6 can leave a full-history repository refusing every
brit mutation; rebase stays a git fallback (rows 5 and 24). Brit sprint 3 merged shem's sixteen
patches for the git spellings into its integration branch; its CI found one red of shem's on
Windows (a relative path printed with backslashes), fixed at integration.

**Lanes from here.**

| Lane | Owner | State |
|---|---|---|
| L2a, producer writes `env:isa-c` | ethosengine a222db7c | started on `71069b1f5` |
| E11 rework, the suite's named half in a view of its declared reads | shem (builder) | in progress on the landed stack, in its own cargo family |
| `fleet-roll` preflight check | shem (builder) | round 3 after review |
| row 31, git refusals read as values | ethosengine a222db7c | after L2a |

**Where the definition of done stands.**

| Line | State |
|---|---|
| 1, two landings followed with no build | needs L2a; nothing measured yet |
| 2, an unmarked release refused before a fetch | the rule is on `dev`; the receipt on shem's real store is being taken |
| 3, a landing pays less | shem 270.6 s to 188.1 s; ethosengine 198 s then 133 s at the landed head, against 153 to 188 s before, with L1's new test files in between, so its before and after are not like for like |
| 4, the suite keys on what it reads | E11 rework in progress |
| 5, no collision, no accidental Edge dispatch | held, by a hand check that was weaker than believed; the check that would hold it is not landed |
| 6, the record | this entry; backlog rows 29, 31 to 34 |

**Decisions taken for the steward to overrule.** The schema ruling above. Shem takes the E11
rework and ethosengine L2a. The `fleet-roll` check refuses by default (one policy key turns it
to a warning or off) and keeps the hook's declared stop policy instead of a rule of its own.

## 2026-11-19: the second line is met; two reviews send two lanes back

**An unmarked release is refused before it is fetched (line 2, met).** On shem's real store,
from a clean checkout at `71069b1f5`, with ethosengine's daemon offering the release its landing
push had published for that commit:

| When (UTC) | What |
|---|---|
| 14:23:47 | `EPR_FOLLOW_SIBLING=1 epr bin follow --tool epr` starts |
| within 10 s | `follow: drawing epr from a sibling first: EPR_FOLLOW_SIBLING=1; 71069b1f551f… is published on refs/remotes/origin/dev` |
| | `follow: nothing drawn from a sibling (unmeasured-requirement: epr of release bafyreic276tjgy…vmvq from did:key:z6MkpZRh…VMZB records no env:isa-c, so the instruction set its C was compiled at is unmeasured (this device draws only env:isa-c=x86-64, the C baseline of x86_64-unknown-linux-gnu); cure: the producer rebuilds under the declared target, or build for this device); building locally` |
| 14:55:51 | shem's own build landed: release `bafyreifcq7qfsf…elhfe`, source commit `71069b1f5`, tree clean; 1,924 s end to end |

The store's incoming directory held no file before or after. The landed `epr` answers
`doctor --exercise` in 1.6 ms. Two things to read with the receipt. The follow was run by an
`epr` built from the landed commit in the gate's cache, because shem's installed release
predated the rule; a new admission rule has that one bootstrap step. And 1,924 s is what shem
pays to build its own release, which is the cost line 1 is there to remove.

**Nothing restarts the supervised peer after a follow lands.** After the landing,
`epr device status` reads `degraded peer daemon: running bafyreih54…; current bafyreifcq7…;
restart epr peer serve to follow current`. The supervisor's manifest did not move, so it
reloads nothing. Line 1 says the daemon restarts on the drawn bytes; today that is a manual
stop and start of the supervisor. Missing step in the chain between "a release landed" and
"the daemon serves it" (backlog row 35). Shem's daemon is left on the previous release until
the two builders working in this session are idle.

**E11 rework: built, reviewed, sent back.** The named half (60 files, 1,192 tests) runs in a
view of 864 files built in about 1.2 s with a constructed environment; a docs commit, a habit
edit and an unrelated Rust edit leave its key alone, and a docs-only commit reuses it in 24 s.
The other half takes about 216 s on shem and runs on every landing, so this change buys a sound
key and not seconds. The independent review ended FIX FIRST: a named test compares a checked-in
grant with the clock, so a green would outlive what it proved; two tests passed without
checking (one a regression of the view, which copied only files named exactly `Jenkinsfile` and
so dropped the conductor pipeline from the budget test); the key and the view expanded the
declarations separately; the copied packages and Node's version were not in the key; and the
builder's habit exemption applied to every project. Confirmed sound: every earlier test file
runs in exactly one half, no failed run records green, the launch environment. Fix round in
progress.

**`fleet-roll`: second review, last build round.** The second pass confirmed both false passes
closed and six of eight probes against the real manifests in agreement with CI. The two that
differ are brit and rakia pin moves, refused on a checkout that has those submodules and passed
on one that does not. CI's checkout initialises no submodule and prunes a dependency on a
pipeline whose manifest it cannot see, so by the code a brit or rakia pin move does not select
an Edge build today, while a conductor pin does (its manifest is in the superproject). That
reading contradicts the standing note that a pin move rolls the fleet and has not been checked
against a CI run. Ruled for the last round: the check reads manifests from the pushed commit
and its mode from the branch being pushed to, both as git objects, so no local state and no
commit in the push can change the answer; a pin whose pipeline CI cannot see gets a warning;
`[deploy-only]` on a tip asks for the declaration. Three behaviours of CI itself are recorded
and not changed here (row 33).

**A dated item.** `genesis/orchestrator/data/adam-compute-worker.json` carries an enabled
runner grant valid until 2026-11-23T16:00:00Z. The test "the declared Adam runner grant is
inside its window, or withdrawn" fails after that instant, so the orchestrator suite goes red
on every device until the grant is renewed or withdrawn. Its steward of record is the steward.

| Line | State now |
|---|---|
| 1 | needs L2a (in progress on ethosengine) and a daemon restart step |
| 2 | met, receipt above |
| 3 | unchanged |
| 4 | built; in its fix round after review |
| 5 | held by hand; the check is in its last build round |
| 6 | this entry; backlog rows 33 and 35 |

## 2026-11-19, later: two gate checks land; the marker goes back; what the day corrected

**The fleet-roll check is on `dev`** (`0f48e02fb`, 15:41Z, push 644 s). A push to `dev` or `main`
whose commits would make CI select an Edge build or deploy step is red in the first seconds
unless a pushed commit carries `[edge:roll]`. It reads git objects only: manifests from the
pushed commit, changed paths from each ref's own range, the mode from the destination tip's
`pool-policy.json`. It follows CI's graph through an opt-in mode of the walker that reproduces
`build-graph.groovy` (dotfiles, build-process references, propagation over `depends`). Three
independent reviews, then a bounded round accepted against a fixed checklist: 94 tests, twelve
probes on the real manifests, and four on real history through the real command (the conductor
pin update `230c06042` refused, naming storage, the Edge image and the deploy step; this
morning's stacked landing "validates only"). About 0.4 s on a docs-only push. Refusing by
default is a decision for the steward to overrule.

**The E11 rework is on `dev`** (`0fd724eb2`, 18:14Z, push 432 s). The orchestrator gate is two
projects now. The named half (58 test files, about 1,150 tests) runs in a scratch view of 871
files built in 1.4 s from the commit's objects, with a constructed environment; its key is one
resolved inventory plus the copied packages and node's version. The tree half (35 files, the
shell and python suites, the Jenkins lint) runs in the checkout. For each attempt the runner
writes one result in a fresh directory and the gate decides from it: no result is a red, and a
green is recorded only for a pass on exactly the keyed bytes. What it buys is a sound key, not
seconds: a docs-only commit reuses the named half in about 26 s, the tree half takes about 140 s
of tests on shem whenever its tree changes, and a docs-only landing selects neither. At its own
landing both halves were reused from the green recorded minutes earlier at the branch head,
because the squashed commit's tree is identical.

| Review pass | What it found |
|---|---|
| 1 | a named test compares a checked-in grant with the clock; two tests passed without checking, one a regression of the view |
| 2 | the new skip rule did not hold for the first test group; a lock's stale recovery could pair one run's receipt with another's exit |
| 3 | my exit-code protocol could not carry two facts; a caller's `JUST_NO_DEPS` turned the dependency-only gate recipe into a no-op that exited 0, on `dev` too |
| 4 | an unreadable Task log discarded a failing result for a retry; a same-user process could forge the result (ruled out of scope and stated as a limit) |

The loop was closed by ruling after the fourth pass. `gateChildEnv` now removes every `JUST_*`
variable for every gate project.

**L2a came back from review.** Ethosengine handed over the producer half (`self/nt-l2a`
`22a337c18`). The review confirmed the slot's name, value and ordering against the receiver and
that nothing in it reaches a storage build input. It held the branch on one thing above the
rest: the marker was stamped when a single acceptable compiler string was found in the binary,
and bundled SQLite leaves no such string, so its instruction set was not checked at all. Ruled:
every cc-built crate in the bin's lock closure needs affirmative evidence of the declared
baseline from the build that made those bytes, or a named exception held against the crate's
own build script. Ethosengine's design, approved: cc's own recorded compile commands, bound to
the bytes by the build's artifact hash. Line 1 waits on it.

**What the day corrected.**

| Believed | Found |
|---|---|
| a by-hand run of the dispatch walker is CI's answer | it skips dependency propagation and dotfiles; CI does both |
| a brit or rakia pin move rolls the Edge fleet (my note since October, an inference never observed) | orchestrator run #2359 covered the brit pin move `1602399ee` and marked every Edge step "SKIP, no changes"; CI prunes dependencies on pipelines whose manifests it cannot see, and names all three submodule pipelines so; the conductor pin does select Edge |
| brit's `Tests pass` check means its tests passed | one matrix leg that never got a runner makes GitHub report the matrix job as `abandoned`, which the check's deny-list did not name; a commit with failed legs was green, and the monorepo's pin gate trusts exactly that check. Fixed on brit's integration branch with an allow-list; `1bf96648cb` is the first brit commit whose own run carries it |
| a `[skip ci]` tip only skips its own commit | it moves CI's baseline past every commit under it |
| one exit status can say "passed" and "on other bytes" | it cannot; a result written per attempt can |

Two slips of mine while establishing these: clock labels typed from estimate, and a commit id
(`c64967822`) read from shem's stale main checkout instead of a worktree at `dev`. Both were
caught by the brit session checking what I sent.

**Also.** Brit sprint 3 merged shem's read-only extension (28 more command paths end quietly on a
closed reader; `show` went from "Broken pipe", exit 1, to 141). Shem's peer daemon was restarted
by hand onto its landed release: stop, start and ready in about 30 s (row 35). Ethosengine
reported that one of its builders set `XDG_CONFIG_HOME` inside a child process, which gets round
the hook that refuses that variable in the shell; it withdrew the two runs made that way, and
whether that guard should have a sanctioned way through is the steward's.

**Line 3, restated.** A docs-only landing no longer selects the orchestrator gate, so the line as
worded measures nothing. What was measured on shem: the suite's wall when it runs, 270.6 s to
188.1 s (T1); and docs-only pushes of 372 s (the live-corpus gate rebuilding after a code
landing) and 54 s (no project gate).

| Line | State at the end of 2026-11-19 |
|---|---|
| 1, two landings followed with no build | L2a in rework after review; then two draws, and the daemon step is by hand |
| 2, an unmarked release refused before a fetch | met |
| 3, a landing pays less | measured on shem as restated; ethosengine's figures are not like for like |
| 4, the suite keys on what it reads | met: landed, every test file in exactly one half, the named half in a view |
| 5, no collision, no accidental Edge dispatch | the check is landed and in force |
| 6, the record | this entry; backlog rows 33 to 39; habit lines in `careless-reds-cost-seconds`, `push-gate-pays-once` and `workspace-device-arrival` |

**Waiting on the steward.** The fleet-roll check's default (refuse). The guard bypass above. The
compute-worker grant that ends 2026-11-23T16:00Z (row 36). A brit pin move: his word, and a
commit whose own CI carries the allow-list. Brit's rebase redesign, which changes a stored
format. And from before: the R5 branch, the Edge roll's timing, the repacks, and this session's
unrecorded covenant.

## 2026-11-19, evening: the view asked for what a checkout does not hold; the marker's second review

**A defect in the suite's view, found on the second device.** Line 4 was called met on shem's
runs alone. On ethosengine, in a fresh worktree where `just dev prep` had printed
`[preflight] ok submodules`, the named half refused:
`suite-run: submodule elohim/holochain-conductor at 993cf597… is in no object store here`.
Nine named files declare all five submodule pins, so the view took the files matching their
walks from each submodule at its pin, 104 of them from the conductor fork. Shem happened to hold
that fork's objects. Ethosengine does not, and the cure the refusal printed does nothing there:
`.gitmodules` declares `update = none` for that submodule, because CI and a plain checkout read
only its gitlink. Ethosengine's session reported the skip and did not override the declaration.

The view was at fault, and so was my first answer: I sent the printed cure before reading
`.gitmodules`. Ruling: a submodule declared `update = none` contributes its pin to the key and
nothing to the view; its directory is there and empty, as in a fresh clone, whether or not the
device holds the objects. It landed alone as `00dd77dfb` at 19:55Z. The named half's view went
from 871 files to 767 and ran 779 + 376 tests with none failed, so no named test had needed
those files. Ethosengine then ran `just gate orchestrator` in a fresh worktree at that commit,
on a device that holds the conductor pin in no store: exit 0 in 492 s, both halves recorded.
That is the receipt E11 lacked. The named half had been red on ethosengine since E11 landed at
18:14Z, about 1 h 40 min. The pipelines I checked (orchestrator, genesis, the root Jenkinsfile
and `scripts/ci`) do not run this suite, so CI was not red in between. The cost fell on
ethosengine: the brit session's first push of its trust branch began 40 seconds before my
warning arrived and could not pass the named half.

A second change follows it (`4ce35271d`): a preflight row, `submodule-pins`, names within
seconds any declared pin that no store on the device holds, with a cure that works for it, and
`just dev prep` brings the same set into a store, skipping any pin already readable.
`che-devworkspaces` was the pin prep did not know; ethosengine had initialised it by hand.

What I got wrong, as a rule for the next gate change: I closed E11's review on a checklist run
on one device, for a change whose subject is what a device holds. Row 40.

**The marker's second review.** Ethosengine handed L2a back at 19:30Z on a new ref
(`self/nt-l2a-r2`, `991df29fd`, then `bde9f6408`), with nothing force-pushed. The marker no
longer rests on strings in the binary. `put` takes the JSON messages of the build replayed,
binds them to the bytes by the executable's hash, and judges every C compile command the
build's scripts ran (cc prints them under `CC_ENABLE_DEBUG_OUTPUT`, now a declared part of the
record): each must carry the declared `-march` and no other instruction-set flag. On real
bytes: ring's 30 compiles and SQLite's one at the baseline; a copy of ring's build with one
`-mavx2` compile added was refused, naming `curve25519.c`. Its three judgement calls stand, one
of them against my own list: `psm` does compile an assembly file, so it is judged, not excepted.

The second review (receipt `codex-l2a-rereview.receipt.md`) confirmed three of the four fixes
and still said fix first, on narrower ground: the checker could be satisfied without having
judged what was linked. A message stream holding only the binary's own row passed with no build
script judged. Archive members were not matched to judged compiles, and cc can chain a prebuilt
object into an archive. Command forms the check cannot read (`@file`, `-Xclang`, `-Xassembler`,
`-Wa,`) passed. Rulings: the stream must show it is the whole build (a finished row with
success, a roster from an independent source, and an output directory that is missing or
unreadable refuses instead of reading as empty); every archive member is the output of a judged
compile or a named exception; unreadable forms refuse; exceptions bind to the locked version
and the reviewed sources. Ethosengine's roster source is the platform-filtered resolve from
`cargo metadata`, ratified with one caution: it resolves features for the whole workspace and
may name a package the build does not compile, and the cure for that is the build's own
selection (`cargo tree -p`), not the stream. The hand-back is expected on `self/nt-l2a-r3`, for
a re-check of those three points only.

**Order on dev.** Because L2a needed another round, the brit session's trust branch went ahead
of it: `758fb4f6b` at 20:28Z. Its second push, rebased over the fix, saw shem's green for the
named half, declined it because its own change selects that gate, and passed the real run in
61 s with no submodule line. Before that, shem's device key directory was checked against the branch's new
custody rule: mode 2700, the declaration 600, nothing writable by group or others, and the
process holds the group, so trust stays applied here. The declaration names only the main
checkout. A linked worktree is undeclared before and after that branch and passes on shem
because git accepts it by ownership. Whether a device where git would not accept it needs a
line for each worktree remains the steward's open item.

**Also.** Shem's repository reached 32 pack indexes, the ceiling for brit reads at reduced
trust; ethosengine is at 42. Fetches from this session now use
`git -c fetch.unpackLimit=1000000 fetch`, which unpacks to loose objects and adds no index. The
repack is still the steward's on both devices. The commit hook printed
`[commit-attribution] UNMEASURED` once, because this session's pinned `epr` predates
`session commit-trailers`; the commit was unpublished and was re-made through the hook with the
current release. The queued second check on a brit pin (read every run's conclusion) is
withdrawn: every brit commit's run concludes `failure` while `cargo-deny-advisories` is red, so
the aggregate with its allow-list is the check that carries the verdict. Two lines the
orchestrator suite prints into the gate log read like faults (`elohim-storage evidence … could
not be read`, four times, and `fixture evidence … /usr/local/bin/node: bad option: --repo`).
They look like the suite's own fixtures and were not checked; quieting or labelling them is owed.

| Line | State after this entry |
|---|---|
| 1, two landings followed with no build | L2a in its second rework; then two draws, and the daemon step is by hand |
| 4, the suite keys on what it reads | met on both devices as of `00dd77dfb`; before that it was met on one |
| 2, 3, 5 | unchanged |
| 6, the record | this entry; backlog row 40; habit lines in `push-gate-pays-once` and `careless-reds-cost-seconds` |

## 2026-11-19, night: CI had been red for two days, and nobody was told

**Found by a neighbour.** The brit session, reading Jenkins for its own landing, reported that
`elohim-eprfs/dev` had not passed since #167. An investigator read the range: every build from
#168 to #190 failed on one test, `bin_build_view`, with "no dep-info at …/target/debug/epr.d",
except #173, a failed clone. #168 is the first build after that test landed (`1033395d5`,
2026-11-17). Nothing else failed in the range. The two publish stages were skipped each time,
and the orchestrator's dev job failed with it, its baseline held at `2080441f`.

**The first cause: two recipes that had parted.** The device gate builds `epr` as a top-level
target before its tests (the `bins` step of `_gate-eprfs`), because cargo writes `epr.d` only
then. The pipeline's tests stage ran `cargo nextest` alone. Every device gate was green.
`f27e22e69` (21:48Z) puts the same build in the pipeline, and adds
`eprfs-ci-parity.test.mjs`, which reads the bins step out of the justfile and refuses a
pipeline that does not run it before `cargo nextest`. It fails on the unfixed file and passes
on the fixed one. CI's first run with it, #191: "2634 tests run: 2634 passed (1 slow),
1 skipped".

**The second cause, hidden behind the first.** #191 then failed one stage later:
"eprfs-core 0.3.5: immutable version collision; bump the version". Measured against the
registry with the publish script's own readers (no cargo, no credentials, nothing uploaded):
today's `cc` bump (L1, `2090a4d82`) moved `cc` and `find-msvc-tools` in the eprfs lock, cargo
packs that lock into each package, and so eprfs-core 0.3.5, eprfs-storage 0.3.8,
eprfs-meta 0.3.6 and eprfs-local 0.3.6 each differ from the registry's copy at an unchanged
version. The device precheck, whose job is to refuse this, compares only workspace members in
the packed lock, so both devices passed it. The cure by the script's rule is four version
bumps, and those move `elohim/Cargo.lock`: an Edge build and deploy. That is the steward's, with
an alternative beside it (row 43). Until he rules, the pipeline is red at its publish stage
with its tests green.

**What the fix cost, and why.** The one-line change to `elohim/eprfs/Jenkinsfile` selected the
full eprfs gate on shem: 1903 s of a 2756 s push. No device step reads that file. It is in the
key because one `sources` list serves CI's dispatch and the device's key (row 42).

**What I take from it.** Three reds today were invisible from where the work was done: the
suite's view on the second device (row 40), the pipeline's missing build (row 41) and the
packed lock (row 43). Each had a green device gate over it. The session-start lines on shem
said "7 known finding(s) recurred (flake evidence)" about a failure that had recurred in
twenty-two builds. A sprint about paying less at the gate has to count what the gate does not
see, and the next one should start from the pipeline's own verdict, read every day.

| Line | State after this entry |
|---|---|
| 1, two landings followed with no build | waiting on L2a's third hand-back; CI's publish red does not hold it |
| 3, a landing pays less | measured the other way today: 2756 s for a pipeline-file line (row 42) |
| 2, 4, 5 | unchanged |
| 6, the record | this entry; backlog rows 41, 42 and 43; a habit line in `careless-reds-cost-seconds` |

**Waiting on the steward, added.** How to cure the publish collision: four version bumps with a
deliberate Edge roll, or leaving the packed lock out of the publish comparison for library
packages (row 43). The repack on both devices is now due: shem is at the 32-index ceiling.

## 2026-11-19, 23:17Z: the first marked draw

**L2a landed and shem drew it.** Ethosengine landed the producer half as `b950d74b1` (a push of
1,826 s, of which the eprfs gate 1,198 s). Its producer printed
`[gate] epr producer put: release bafyreieebfer3domlzorjhf6nlwzugnmy3do2ursyvf5ikhk463uecapuq at b950d74b1f32 … (crosses)`,
and both Produce records on that device carry `env:isa-c=x86-64`: the first real Produce to
carry the marker. On shem, from a clean checkout at that commit, with the device's current
release and `EPR_FOLLOW_SIBLING=1`:

`follow: drew epr release bafyreieebfer3…capuq from did:key:z6MkpZRh…VMZB in 22.8 s`

The follow exited 0 in 32 s and compiled nothing. The landed record names the same two binaries
as the producer's line. One `epr device supervise` then found the peer on the old bytes and
restarted it in 6 s; the device read "serving at current" and exchanging 29 s after the command
began. That restart is still made by hand (row 35).

| | this morning (unmarked, refused before a fetch) | tonight (marked) |
|---|---|---|
| shem's follow | 1,924 s, a local build | 32 s, of which the draw 22.8 s |
| what shem compiled | epr and bin-launch | nothing |
| what the producing device paid | its own gate | its own gate, 1,198 s for eprfs |

**How the review closed.** The third review (receipt `codex-l2a-r3.receipt.md`) still said fix
first. Each input it built was a message stream edited after cargo wrote it, or a build script
written to get round the check. I ruled those outside what the marker is for: it is the
producing device's own signed statement about its own build, and it guards an honest producer
against marking by accident. The one accident-shaped case, a stale stream against a reused
executable path, cannot come from the producer script, which rewrites the stream on each run
and withholds when the replay fails. Five small hardenings go in ethosengine's next landing:
exact probe shapes, more options treated as unreadable, no pass by member name once the judged
object is gone, the custom-build row for each closure package that has one, and the threat
model written in the module doc. The ruling is the steward's to overrule.

| Line | State after this entry |
|---|---|
| 1, two landings followed with no build | one followed: a 22.8 s draw, and the daemon on the drawn bytes by one command. The second is ethosengine's next epr-cli landing |
| 2 to 6 | unchanged |

## 2026-11-20, 00:14Z: the second marked draw; the first line is met

**Two landings, two draws.** Ethosengine landed the hardening of the producer's check as
`b077336ca` (a push of 1,262 s; the eprfs gate 988 s, down from 1,198 s). Its producer printed
`[gate] epr producer put: release bafyreide6r7pbgpf5y6e7llt6kakrfpwyj2wdrck56muhfnf7g4ltmlvv4 at b077336ca85a … (crosses)`,
both Produce records marked `env:isa-c=x86-64`. On shem, from a clean checkout at that commit,
running the release drawn an hour earlier:

`follow: drew epr release bafyreide6r7…lvv4 from did:key:z6MkpZRh…VMZB in 19.0 s`

The follow exited 0 in 30 s and compiled nothing. One `epr device supervise` restarted the peer
onto the drawn bytes: the command took 9 s, and the device read "serving at current" 33 s after
it began.

| Landing from ethosengine | Draw on shem | Whole follow | Compiled on shem | Daemon on the drawn bytes |
|---|---|---|---|---|
| `b950d74b1` (23:16Z) | 22.8 s | 32 s | nothing | one command, 6 s; serving after 29 s |
| `b077336ca` (00:13Z) | 19.0 s | 30 s | nothing | one command, 9 s; serving after 33 s |
| for comparison, `71069b1f5` this morning, unmarked | refused before a fetch | 1,924 s | epr and bin-launch | by hand, about 30 s |

The line as written is met: two consecutive landings from ethosengine followed on shem with
`EPR_FOLLOW_SIBLING=1` and no local build, the daemon restarted on the drawn bytes. Two things
it does not say and should: shem landed a documentation commit between the two, which produces
no release and was not followed; and the restart is still a command someone types (row 35).
The hardening itself was read on shem against the five-item list and found whole; no outside
review, by the ruling of the entry before this one.

**Where a third of the eprfs gate goes.** Ethosengine timed the phases of its 1,198 s gate:
lint 100 s, test build 213 s, test run 798.5 s, and of that one test binary,
`session_envelope`, 432.7 s. That binary runs a Python fixture of 30 cases against the debug
`epr`. Timed alone on shem it took 461 s, no case dominant. Traced, each case's `epr session
open` takes 5.13 s of wall and 0.07 s of CPU, polling every 50 ms: the five-second lock wait in
device-runtime, reached while the same process already holds the sessions guard. The timeout is
swallowed and the open succeeds. So the slow test is a defect of the product seen through a
test: a first `session open` very likely stalls five seconds on a real device too, and whatever
asked for the lock fails silently each time. The exact call and a fix, test first, are with an
agent on shem (branch `claude/nt-session-open-wait`); nothing is landed from it yet.

| Line | State after this entry |
|---|---|
| 1, two landings followed with no build | met, as the table above reads; the restart is by hand |
| 2, an unmarked release refused before a fetch | met |
| 3, a landing pays less | measured both ways: 1,924 s to 30 s for the follower; the eprfs gate still 988 to 1,903 s for whoever lands, a third of it one stalled command |
| 4, the suite keys on what it reads | met on both devices |
| 5, no collision, no accidental Edge dispatch | in force |
| 6, the record | this entry; rows 40 to 43; habit lines |

## 2026-11-20, 02:10Z: five seconds in every first session open; a push that failed for the other device's accident; the restart cut down

**The slow test was the product.** Traced at the lock itself: `session open` holds the exclusive
sessions guard and calls `recall::initialize`, which resolves who is reading through
`actor::current_claim`, whose `covenant_scope` asks for a shared guard on the same file. It
waits five seconds, times out, and `reader_from_session` swallows the error as an unknown
reader. Forty-five such waits across the fixture, each one this chain. On a real device each
first open, and each lazy open that repairs a missing continuation, paid five seconds of wall
and had them charged to the session's recall budget; the reader was read as unknown for the
wrong reason. The fix (`7213bb589`, six files, all in epr-cli) hands recall the generation key
`open` has already resolved. I checked it is the key the locked lookup returns for the same
record; the guard and its timeouts are unchanged. A new test read 5.02 s on the unfixed sources
and passes now. The fixture went from 457.5 s to 230.7 s, and the eprfs gate on shem from
1,903 s to 1,684 s.

**A push that failed after 55 minutes, for row 40 turned round.** The first push of that fix
went red on one test of the tree half that has nothing to do with it:
`generator-0.8.11 source is not in /opt/rust/cargo …`. The pin test that L2a added holds each
reviewed exception against the crate's `build.rs` in the device's cargo registry, and fails
when the source is absent. `generator` is in the lock only through `loom`, under `cfg(loom)`,
which no build compiles: 43 of the lock's 611 crates.io packages have no source on shem.
Ethosengine has that one by accident. I had approved "fails when absent" without asking for
shem's run, the day after writing row 40. The cure (`d6e9c69aa`): a reviewed exception is a
statement about one crate's bytes, and the lock's checksum names those bytes on every device,
so each exception is held against the lock everywhere and against `build.rs` where the source
is present. The second push cleared in 870 s with the eprfs verdict reused. Rows 44 and 45.

**The restart, reviewed and cut.** Ethosengine designed and built row 35 to my rulings: a
landing restarts the supervised peer, waits for readiness, and on a failed start reverts one
step and holds the release. The outside review (receipt `codex-row35a.receipt.md`) found seven
ways that goes wrong unattended, none of them contrived: an unrelated process gets a healthy
release reverted; a failure between stopping the old ark and the readiness wait is reported as
a successful landing with no peer running; the rollback stops supervision before it knows it
can restore; a held release is re-landed by arrival, bootstrap or a cold supervise; two follows
can revert more than one step; a pointer file can select another device root; an unresolved
session can stop another session's ark. The fault was in my ruling: a hold is only a hold where
a release is promoted and where a child is spawned, and that is device-runtime and ark, which
need an Edge roll. So the landing is cut to the command typed three times tonight, made
automatic: restart when the acting session holds the lease, wait on ark's clock for the landed
tool's own children, report ready or degraded with the cure, exit non-zero on any error after
the old ark stopped. No revert, no hold. Revert-and-hold is row 46, with the seven findings as
its specification. A second landing gives a follower patience with a holder that is restarting
and gives the peer a drain; a ruling of mine there was also wrong on the facts (the relay
carries no artifact bytes this sprint, so waiting on a relay-only holder only delays a build)
and is reversed.

**Two slips of mine in this stretch.** I told ethosengine a landing's head from a read made
seconds before the send completed, and corrected it within a minute: the head is read back
after the push returns. And I briefed an agent that device-runtime was in bounds before
learning it is a storage build input; the fix it found needed none of it.

| What a landing paid on shem tonight | Seconds |
|---|---|
| a pipeline-file line (`f27e22e69`) | 2,756, of which eprfs 1,903 |
| the session-open fix, first push, failed at the end | 3,311, of which eprfs 1,684, memory-ceremony 412 |
| the same with the pin test cured, eprfs reused | 870, of which memory-ceremony 367, orchestrator-tree 234 |
| a documentation commit | 84 to 281 |

## 2026-11-20, 05:55Z: both peers restart by themselves

**What landed.** The cut-down restart went through a second outside review (receipt
`codex-row35amin.receipt.md`), which found six things, two of them my own rulings pulling
against each other: I had asked for a non-zero exit when the daemon does not come up, and two
callers read a non-zero land as a failed landing (the native bootstrap worker would then pay
for another local build; pre-push would print "household head not landed"). Ruled: a landing
that succeeded exits 0, and the restart's outcome goes into the record, one stderr line with
the cure, the receipt row and status. With that and six smaller fixes it landed as `fcf1f3137`
(04:03Z): after a follow or forward land, when the acting session holds the device-ark lease,
the existing stale-child restart runs, the landed tool's children are waited for on ark's
clock, and the outcome is reported. The gate recipe and the eprfs pipeline now build the `ark`
those stories run, and the parity test holds every build step that precedes the tests.

**CI caught the floor an hour later.** `elohim-eprfs/dev` #197 built the ark without trouble
and failed four of the new stories, each "not ready after 13 s": the wait allowed 12 s for a
child to be seen, and the pipeline was busy. Fixed as `aeada1196` (05:52Z): 60 s for the first
spawn while the replacement ark lives, then the child's own patience. The same landing makes a
hand `epr device supervise` leave a row too.

**The first two.** Each device followed `aeada1196` with the release that carries the restart:

| Device | What it printed by itself | Record and row |
|---|---|---|
| shem, 05:54:03Z | `restarting the ark (after follow, session b49f7c46…)`, then `not ready after 13 s (not ready: peer); the device is degraded` | `restart.outcome` not-ready, 13 s; row with trigger follow. About 20 s later the supervisor read the peer ready and status read serving at current |
| ethosengine, 05:55:48Z | `restarting the ark (after follow, session a222db7c…)`, then `peer ready on bafyreicfbac2w4… in 5 s` | `restart.outcome` ready, 5 s; row with trigger follow; status "serving at current (restarted by follow … in 5 s)" |

So the restart needs no hand on either device. Shem's report was the false "not ready" of the
12 s floor, on a real device, exactly as CI had shown; the binary that did that follow was the
one before the fix, and shem now runs the one after it.

**A correction, and a find.** I wrote that ethosengine's peer had been behind its own current
all night. It had not: the "degraded" line there came from an orphan, a test daemon from a
`devices-share-one-berth` run of 2026-11-17 whose lease landed in the device's real bin store.
Ethosengine found it, stopped before touching it, and ended it on one interrupt; the lease read
dead by itself (row 48). What was true: that device was on the release of 2026-11-18 until it
adopted by hand tonight, because its hook defers the household head to the next session
arrival.

**What this does not yet do.** It restarts only when the following session holds the lease or
no live holder exists. It does not revert or hold a release that fails to start (row 46). A
follower that meets a restarting holder still falls back to a local build until the second
landing (the follower's wait and the peer's drain), which is built and waits for its review.
And the eprfs pipeline is red at its publish stage until the steward rules on row 43.

| Line | State after this entry |
|---|---|
| 1, two landings followed with no build | met; and since 05:54Z the daemon step needs no hand when the follower holds the lease |
| 3, a landing pays less | the eprfs gate on ethosengine: 1,198, 988, 1,246 (with the ark), 1,097 s; on shem 1,903 then 1,684 s |
| 2, 4, 5 | unchanged |
| 6, the record | this entry; rows 47 and 48; a second addition to row 35; a habit line |

## 2026-11-20, 10:00Z: the second landing goes back; where the night ends

**CI.** `elohim-eprfs/dev` #198 on `aeada1196`: "2695 tests run: 2695 passed (9 slow),
1 skipped". The tests stage was green from #191 to #196, red at #197 for the 12 s floor, and
is green again. Each of those builds then ends at the publish stage on
"eprfs-core 0.3.5: immutable version collision", which is row 43 and the steward's.

**The second landing of row 35 is not on dev.** Ethosengine handed it over as
`self/nt-row35-b2` (`9ada5b173`): a follower retries an admitted offer whose holder gives no
route, and a stopping peer drains its streams. Its outside review (receipt
`codex-row35b.receipt.md`) found no path that uses unverified or partial bytes and no lock held
in a pause, and seven things that block: a refusal or a wrong-CID chunk can come out of one
fetch as "no route" and be waited on; admission is checked once and not again on a retry; the
patience bounds the sleeps and not the retry fetches, each of which can run for half an hour;
two followers of one release share a staging directory, which predates the branch (row 49); a
stream can register after the drain has counted zero; gossip and the daemon's tick still start
work while it drains; and the whole shutdown can pass the supervisor's 20 s. I sent a fix list
of six and recommended stopping there: a producing device's peer does not restart at its own
landing, because its hook defers the household head, so the window this change closes is open
only when someone follows on the holder at the moment a sibling draws. Ethosengine had not
answered when this was written. The branch and its receipt are the specification for whoever
picks it up.

**What the night put on dev**, in the order it landed:

| Commit | What it is |
|---|---|
| `00dd77dfb`, `dd3186d7d` | the suite's view takes nothing from a submodule declared `update = none`; prep and the preflight name the pins it does read |
| `f27e22e69` | the eprfs pipeline builds `epr` before its tests; a parity test holds the two recipes together |
| `b950d74b1`, `b077336ca` | the producer marks a release `env:isa-c` from the build's own compile commands; hardening |
| `7213bb589`, `d6e9c69aa` | a first `session open` no longer waits five seconds on its own guard; a reviewed exception is held against the lock's checksum on every device |
| `fcf1f3137`, `aeada1196` | a landing restarts the supervised peer; a 60 s wait for the first spawn; a row for a hand restart |

**What I would do differently.** Three times tonight a change was verified on the device that
wrote it and failed on the other, or in CI: my view on ethosengine, its pin test on shem, its
restart stories on a loaded pipeline. Twice I ruled a default before asking where it would be
enforced or who would read its exit status. And I led a sprint about what a landing pays for a
day before reading what the pipeline itself was saying. Each is now a row or a habit line, and
the first two are in the notes I keep for the next session.

**Waiting on the steward.** Row 43: bump four crate versions with an Edge roll, or leave the
packed lock out of the publish comparison for library packages (my recommendation). The repack
on both devices. An Edge roll's timing, which now also carries row 46. The brit pin:
`6507caaaa1`, green on three platforms, or `1bf96648cb`. The defaults I chose for him to
overrule: the fleet-roll check refuses; `device_restart.on_landing` is `restart`; the producer
half's review was closed by ruling. The compute grant that ends 2026-11-23T16:00Z (row 36).
The memory cleanup loop, whose line has read failed all night and needs his word to run. And
this session's unrecorded covenant.
