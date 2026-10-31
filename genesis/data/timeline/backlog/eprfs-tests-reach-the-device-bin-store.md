---
id: "backlog-eprfs-tests-reach-the-device-bin-store"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "epr-cli integration tests spawn epr against the device's shared bin store, and a lost lock race renders as no reader"
slug: "eprfs-tests-reach-the-device-bin-store"
written: "2026-10-30"
author: "claude-opus-5-5"
status: "open"
priority: "medium"
area: "eprfs / epr-cli tests / recall lens"
domain: "D5"
tags: [eprfs, tests, hermetic, bin-store, recall, flake]
---

# epr-cli tests reach the device bin store

Push 4 of the 2026-10-27..30 overnight integration (f3544b4f3) failed the eprfs gate after 3245 s
on two `flow_memory_recall_lens` tests that had passed on push 3 with no change to their code:

- `a_declared_lens_table_that_parses_wins_over_the_builtin`: `epr actor claim` refused
  `lock-timeout: store lock remained busy for 5s`.
- `a_correction_candidate_renders_at_every_lens`: the haiku session rendered `lens: standard ·
  stated none` instead of `minimal`, so `rank2.md` appeared as a Linked choice.

Both are the same race. An `epr` spawned with no `EPR_STORE`/`BIN_STORE` resolves
`/projects/.claude-config/tools/bin-store/epr` and takes its lock (`files::EXEC_WAIT`, 5 s) on
the way to `session_envelope::executor_cid`. On a loaded gate host that lock is held longer.

| Row | What | State |
|---|---|---|
| 1 | The recall harness (`epr-cli/tests/common/mod.rs`) spawns every `epr` through `epr(root)` with a store inside the fixture's `.git` | fixed by 681f572e7 |
| 2 | 23 other `epr-cli/tests/*.rs` binaries spawn `CARGO_BIN_EXE_epr` and set neither `BIN_STORE` nor `EPR_STORE`; any that resolves the session envelope can lose the same race | open: give them the same helper, or have the test profile export a per-run `BIN_STORE` |
| 3 | `recall::lens::reader_from_session` maps `Err(_)` from `actor::current_claim` to `ReaderRef::Unknown`, so an unreadable actor log looks like "no claim" and silently changes the offered lens. A malformed lens table is "never silent"; an unreadable claim should be named the same way (`stated unresolved: <reason>`) | open: a semantic change to the lens provenance; needs the recall contract owner's reading |

## Done when

- No epr-cli integration test reads or locks the device bin store (row 2).
- A recall view whose actor claim could not be read says so in its `lens:` provenance line (row 3).
