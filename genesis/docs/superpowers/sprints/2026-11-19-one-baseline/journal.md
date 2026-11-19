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
