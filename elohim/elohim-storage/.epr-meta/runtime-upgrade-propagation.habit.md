---
epr-habit-version: 1
id: runtime-upgrade-propagation
invariant: >
  A runtime release (coordinator bundle first; config, binary, hApp bundle behind it) reaches
  every peer by ELECTION on its release channel — staged, soaked by a canary, promoted on
  attested evidence, converged, and revertible by re-election — with conductors never restarted
  or re-keyed for the coordinator class and mixed-version peers still talking throughout.
status: green
active: false
checks:
  - "a2o @concern:runtime-upgrade-propagation (genesis/a2o/features/delivery/runtime-upgrade-propagation.feature — @wip: the ten scenarios are written and blind-reader-revised; step definitions must compose the three drivers below before this habit can be observed by a runner)"
  - "manual chain (mesh): genesis/a2o/scripts/epr-release-package.ts (T1) → release-ceremony.ts channel create / publish / promote / revert <manifest> (T2) → GET /admin/adoption on every peer (T3/T4) → release-attestation-probe.ts EXIT=0 (T5)"
refs:
  - "spec: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md (§5 verify floors, §10 receipt chain)"
  - "arc: genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md (cycle-time table)"
  - "receipt atom: genesis/data/timeline/backlog/task-runtime-upgrade-a2o-receipt.md"
  - "controller: elohim/elohim-storage/src/services/release_adoption/{watch,verify,state,apply}.rs"
retire-when: >
  when three consecutive coordinator releases reach the alpha fleet through their release
  channel with no Jenkins roll and no operator hand on any pod, each with its cycle-time row
  in the arc doc — the CI roll is then no longer a delivery path for the class and the
  register describes a product, not a practice.
---
DELTA 2026-09-02 00:43Z (local mesh receipt, r2 channel; transcript
`genesis/a2o/reports/release-ceremony/2026-09-01/transcript.md`): the full §10 chain
PASSED by hand-composed drivers — publish→3/3 staged ≤19 s; canary (james, mode
`canary`) hot-swapped ~12 s after his first sweep, conductor PID unchanged, wasm hash
flipped; soak attestation after 30 s read 1/1 by both observers; promote→3/3 applied
in 75 s; revert via `revert <channel> <manifest>`→3/3 restored in 31 s; attestation
probe qualifying 2 / builder-excluded 1 / mismatched 0. Five typed refusals became
five controller fixes (bd5d3984b 547c28d62 851ab2fae 2b02dd86f + driver revert) and
two zome/build atoms (update_content star chain; hc-rna cdylib link). Status GREEN on this measured pass (the hand-composed driver chain is the runnable check;
the a2o feature's steps composing the same drivers is the next station).
Preconditions learned: doorway A up (bootstrap/signal home) before any election measure;
candidate = coordinator-only bytes with byte-identical integrity (COORD_BUILD_MARKER, or a
wasm custom section while the DNA workspace build is broken).
