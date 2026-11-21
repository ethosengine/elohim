

## Row 53, delta (2026-11-21): measured, and the reason for one thread is the I/O guard

The eprfs workspace tests (2,698 tests, the live-corpus legs skipped) on shem, one build, by
`RUST_TEST_THREADS`. Per-run logs and a 20 s sampler of the guards are in the sprint's campaign
directory on shem (`threads/`).

| Threads | Runs | Clean | Wall-clock | I/O guard pauses | Peak writes | Peak host I/O pressure |
|---|---|---|---|---|---|---|
| 1 | the gate on landing 2 | | about 1,620 s | | | |
| 4 | 5 | 5 | 604 to 656 s | 3 | 61 to 72 MB/s | 14.8% |
| 3 | 10 | 8 | 696 to 752 s | 4 | 58 to 71 MB/s | 15.0% |
| 2 | 2 | 1 | 923 s; 977 s | 0 | 59 to 63 MB/s | 8.8% |

Nothing was changed. The bar was ten clean runs at the chosen count beside a running build, and
three threads gave eight. The two failures there were both in runs the I/O guard had frozen for
about 36 s: a case with a 15 s subprocess timeout expired, and a berth call inside another case
failed. The failure at two threads was a different defect, a timing assumption in a test, since
fixed (row 66). So the row's question has an answer: more threads compress this workspace's
writes past the guard's first line (10% full avg10), the guard freezes the run, and a frozen run
fails whenever a shorter timeout is in flight. The cure is the write volume (row 65), after which
the count is measured again.

## Row 60, delta (2026-11-21): threads are safer here than the row said

The row names one unit test in `eprfs-local` that changes the working directory and calls it a
race under threads. It is not: `relative_roots_remain_bound_when_process_cwd_changes` re-executes
itself in a child process and changes the directory there
(`elohim/eprfs/eprfs-local/src/capture/tests.rs`, the `EPRFS_CAPTURE_CWD_TEST_CHILD` guard). In
the whole workspace's test code six lines change process-wide state: three in that child, and
three in `eprfs-peer/tests/relay_compat.rs`, a binary with one test. Running the pipeline's own
nextest command on the device is still the way to make the two recipes one.

## Row 65 — the slow eprfs test file makes its own stripped copy of `epr` for every story

`elohim/eprfs/epr-cli/tests/device_supervise.rs` builds each story's release by running
`strip --strip-all` over the 160 MB debug `epr` (66 MB out, about 4.8 s) and then ingesting it,
once per story, 29 stories. At one thread those writes are spread over twelve minutes. At four
threads they are compressed into about three: the test run wrote 61 to 72 MB/s and pushed the
host's I/O pressure to 11 to 15% against the I/O guard's first line of 10%, and the guard paused
the run for 36 to 41 s in three runs of five (2026-11-21; one of those with no other build
running). At three threads the guard paused four runs of ten, and two of those failed because a
case with a 15 s timeout was in flight while the run was frozen; at two threads the pressure
stayed under 9% in two runs.

Owed: one stripped copy per test process, shared by the stories that never modify their release
and copied only by the ones that do. That removes most of the strip time (about 139 s of a
one-thread run) and most of the write burst, after which four threads can be measured again.
Not done here because some stories append to their release to make a second one, and telling
those apart is a reading of all 29.

## Row 66 — two session-envelope cases asserted a refusal that held only until a holder noticed

Found by load on 2026-11-21: one eprfs test run in eleven failed, the one that overlapped a
second build's five thousand tests (load 15). `ResumeEnvelopeTest.test_resume_refusals_write_nothing`,
subtest `namespace`, expects `epr session open` to refuse across a pid-namespace mismatch. The
case removed the predecessor's fixture `/proc` entry first. The predecessor's holder polls for
that once a second, exits and releases its lock, and a released holder is, by design, proof of
death across a foreign namespace (`device-runtime` `envelope.rs`, `holder_released`, with its own
test). So the refusal held only until the holder noticed, and a loaded device ran the command
after that. The product is as designed; the case was not (diagnosis: Codex astra, receipt
`astra-namespace-flake.receipt.md` in the sprint's campaign directory).

Fixed on shem's branch `wallclock/eprfs-threads` (53ea31d7b), not yet on dev: in
`genesis/agentic/session_envelope_test.py` the `namespace` subtest and the sibling
`test_gc_preserves_unknown_and_foreign_namespace_but_collects_a_previous_boot` keep the
predecessor's entry until their assertion has run. Three clean runs at load 13.7 to 17.9
afterwards. It lands with the next eprfs change, so the eprfs gate is paid once. This matters beyond the thread count: with two cargo seats in use, a timing
assumption like this one is how a second build turns a green gate red.
