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
