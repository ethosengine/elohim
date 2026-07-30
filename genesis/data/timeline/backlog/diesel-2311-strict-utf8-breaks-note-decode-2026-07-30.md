---
id: "backlog-diesel-2311-strict-utf8-note-decode"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "diesel 2.3.5→2.3.11 (vuln-lane bump) makes SQLite TEXT decode strict — economic_events note round-trip test fails"
slug: "diesel-2311-strict-utf8-note-decode"
written: "2026-07-30"
author: "agentic-developer"
status: "open"
priority: "high"
area: "elohim-storage"
domain: "dev"
severity: "gate-blocker"
tags: [diesel, sqlite, utf8, vulnerability-remediation, pre-push-gate, elohim-storage]
relatedNodeIds:
  - "memory:feedback_concurrent_sessions_shared_worktree"
shift_objective: |
  Owner: the vulnerability-remediation lane (Codex session of 2026-07-29/30) whose UNCOMMITTED
  elohim/elohim-storage/Cargo.lock bump (diesel 2.3.5 → 2.3.11, part of the rustls/reqwest
  vuln sweep) breaks the committed test
  db::economic_events::tests::list_load_path_decodes_every_constructible_row_shape:
  "Error deserializing field 'note': invalid utf-8 sequence of 1 bytes from index 0".
  Deterministic (fails in isolation in 0.21s), NOT a flake. The test is the tree's guard that
  every constructible row decodes on the .load() path — diesel 2.3.11 made TEXT→String decode
  strict where 2.3.5 tolerated/lossy-decoded invalid UTF-8. Committed dev (diesel 2.3.5) is
  green; only the working tree with the bumped lock fails.
  Resolution options for the lane owner, in preference order: (a) sanitize/refuse non-UTF-8
  `note` bytes at the INSERT path and migrate/clean any legacy rows, keeping the test's
  guarantee honest under 2.3.11; (b) custom lossy deserializer for the note column; (c) pin
  diesel below the strictness change ONLY if the vuln advisory allows. The bump must not land
  on dev until this test is green under the new lock — the pre-push gate enforces exactly this
  (it blocked the 2026-07-30 saga push; the saga lane pushed its own payload independently).
---

Evidence: pre-push gate log 2026-07-30 ~00:20Z (2247 passed / 1 failed / 521s) + isolated
re-run. Lock diff also drops reqwest 0.11/rustls 0.21 chain and bumps rand/getrandom families —
the diesel bump is the only observed test breakage.
