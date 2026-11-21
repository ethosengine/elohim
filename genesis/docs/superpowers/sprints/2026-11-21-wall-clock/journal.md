

### 16:15Z: the steering record is on dev

`f1e3540fd..21a8060cd` at 16:15:48Z, 282 s: orchestrator-tree 208 s, orchestrator reused (9 s),
eprfs-live-corpus 65 s. Notes published, the main checkout fast-forwarded.

### 16:20Z to 17:10Z: the hook counts builds (row 64, item 1)

A Sonnet agent wrote the first version from a brief: group the heavy processes by their highest
heavy ancestor and compare the number of groups with the limit. Codex astra read it four times,
and each round ended "fix first".

- Round 1, FIX FIRST: two cargos running side by side under one gate-runner, or under one
  `cargo test`, shared a highest ancestor and read as one seat, which admits a third build. And
  a new test left `os.kill` unmocked, so the hook under test would have signalled the real pid
  100. No such pid existed on this device when the tests ran.
- Round 2, FIX FIRST: the reworked count (a build is a driver with no driver below it) still
  hid a cargo that is compiling while its child cargo compiles, and a ring of drivers longer
  than the walk's bound could read as no build at all.
- Round 3: a cargo started with an option before its subcommand was never listed as a build, so
  two of them under one gate-runner shared its seat. The listing now reads past options, and any
  cargo with a live compiler under it holds a seat whatever its subcommand.
- Round 4: the `d` alias and four compiler names were missing from the lists. Added. This fix
  was not read again; what the count still rests on is in row 64's delta.

The count that came out of it: a seat is held by a driver with no driver below it, by a driver
that is the nearest driver above a live compiler, and by a compiler with no driver above it; a
walk cut short counts its process alone; a failed grouping counts every process. 42 tests in the
two hook test files and 29 in the two agentic files pass. Run against the live process table
during the thread measurement's build, the old rule read seven processes and the new rule one
build.

The error fallback in the first version returned at most one build, which admits work when the
grouping fails. It now returns every process, which refuses.

Outside the signed write set, under the steward's approval of the plan:
`.claude/hooks/cargo-disk-guard.py`, `.claude/hooks/tests/`, `genesis/agentic/bin/berth`.
