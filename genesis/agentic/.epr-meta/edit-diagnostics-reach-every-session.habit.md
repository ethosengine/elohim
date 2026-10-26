---
epr-habit-version: 1
id: edit-diagnostics-reach-every-session
invariant: >
  Sessions editing one checkout share one language server per toolchain and root. Each receives
  its own and caused-by-me diagnostics in full, with foreign errors folded by area and owner.
  Flychecks run one at a time on the device and defer while build capacity is held. An idle
  root's server exits, while the one device mux keeps its single toolchain:lsp lease.
status: unwired
active: false
checks: []
refs:
  - "planned check: a2o @concern:edit-diagnostics-reach-every-session (genesis/a2o/features/devflow/edit-diagnostics-share-one-server.feature; step definitions and live probe pending)"
retire-when: >
  when language servers are own children of the device root envelope under ark, through an
  `epr lsp serve --device` resident in the device RuntimeManifest, and arrival reads them
  from one berth.
---
DELTA 2026-10-26: declared unwired; spec + slice 1 in flight on feat/shared-toolchain-servers

DELTA 2026-10-26b (STAYS unwired; C6 follow-up): `python3 -m unittest genesis.agentic.lsp_flycheck_test` passed 9 tests (exit 0), including one live cargo lease with capacity 2, unleased cargo/rustc deferral, child marker propagation and cancellation/lease cleanup; `python3 -m unittest discover -s .claude/hooks/tests -p 'test_cargo_disk_guard*.py'` passed 14 tests (exit 0), including marked-root-only SIGTERM, non-yieldable capacity, unreadable environments and the 3 s wait bound. Story refinement for the existing busy-slot scenario: Given cargo capacity 2 with one gate holder, When a file is saved, Then no flycheck cargo starts; Given a running marked flycheck, When an admissible gate arrives, Then its wrapper yields and releases its lease. These are local regression proofs; the shared-server a2o scenario/live probe remains unwired. Existing berth regression runs: Python 100 passed/1 guest-UID preexec error (exit 1); pre-push 16 passed/2 `spawnSync bash EPERM` errors with test isolation disabled (exit 1).

DELTA 2026-10-26: re-scoped to an ark device resident (epr lsp serve --device); review fixes in flight

DELTA 2026-10-26c (STAYS unwired; resident slice committed on feat/shared-toolchain-servers 01bae0c9d..97ea46685): `just gate eprfs` EXIT=0 and `just gate rust-style` EXIT=0 at c4bc5df0d (independent re-run); `cargo test -p elohim-epr-lsp` 51 unit + 7 integration (two sessions on one server, attribution/fold/ambiguity, crash, idle reap, pin change, slow client, oversize frame, null server) EXIT=0 and clippy -D warnings EXIT=0 at be6f3b4af; `lsp_flycheck_test` 18 EXIT=0 at 97ea46685, whose new same-session test fails on the prior script (two resident flychecks both ran cargo — fixed with a device-wide run lock). All against a fake server; never yet run against real rust-analyzer or under ark. Feature READY (blind-reader rounds 6–9). Wiring to red needs: the resident declaration and `epr device supervise` (feat/gate-live), step definitions, and the live two-session probe on elohim-storage.
