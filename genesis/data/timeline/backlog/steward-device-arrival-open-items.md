

### Row 64, delta (2026-11-21): the hook counts builds

Item 1 is done. Rule 4 of `.claude/hooks/cargo-disk-guard.py` now counts builds through
`group_builds`. A seat is held by a cargo driver or gate-runner with no driver below it, by a
driver that is itself compiling, and by a compiler whose cargo is gone. A process whose ancestry
cannot be walked to the top counts on its own, and a failure of the grouping counts every
process, so an error refuses work. The refusal reads `CARGO SEATS FULL: N of M builds running`,
names each build and what it runs under, and prints a wait over those `/proc` directories.
Against this device's live process table during a six-job build, the old rule read seven
processes against a limit of two; the new rule read one build of two.

Four review rounds by Codex astra (receipts `astra-seats-review*.receipt.md` in the sprint's
campaign directory on shem) each ended "fix first", and each fix is in: two cargos compiling
under one parent counted as one seat, side by side and then nested; a new test that would have
sent a real signal to whatever held pid 100; a wait hint that waited for every build to end; a
cargo started with an option before its subcommand, or with a subcommand the hook does not know,
was never listed, so two of them shared the gate-runner's seat; and the `d` alias and four
compiler names (`rustdoc`, `clippy-driver`, `cc1plus`, `clang`) were missing from the lists. The
last fix was not read again.

What the count still rests on: a list of cargo subcommands and a list of compiler names. Two
cargos of an unlisted subcommand, each running only an unlisted compiler, under one gate-runner,
would still share its seat. The memory and I/O rules do not depend on either list.

Found on the way and still open:

- Neither `test_cargo_disk_guard_yield.py` nor the new `test_cargo_disk_guard_seats.py` runs in
  any gate: the root justfile runs one other file from `.claude/hooks/tests`. Wiring them in is
  a justfile edit, which re-keys every gate that runs from it (row 59), so it waits for that row.
- A gate-runner holds a seat through its non-cargo steps, and a gate with no cargo holds one for
  its whole run. That is the old behaviour and the safe direction.
- `genesis/docs/superpowers/specs/2026-10-26-shared-toolchain-servers-design.md` quotes the old
  refusal text in three places.
- The hook reads a proposed command with a separate parser (`segment_head_and_verb`), which was
  not read in this pass. Whether it gates `cargo --color=always build` is unmeasured.

Items 2 and 3 are unchanged: the push loop, then the measured landing.
