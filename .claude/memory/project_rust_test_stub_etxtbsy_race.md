---
name: project_rust_test_stub_etxtbsy_race
title: "project_rust_test_stub_etxtbsy_race"
description: Rust tests that write an executable stub and exec it race sibling tests' forks (ETXTBSY) under load — probe the stub before use
metadata:
  type: project
---

A multi-threaded `cargo test` that writes a shell stub (`std::fs::write` + chmod 755) and immediately spawns it can fail with `Text file busy (os error 26)`: another test thread forks while the write descriptor is still open, the child holds the inherited fd until its own exec, and our exec of the stub is refused. Seen once at 2026-10-30 10:5xZ in the pre-push elohim-ark gate (`brit_index::tests::a_find_miss_is_empty_and_a_failed_git_read_is_an_error`) after passing three plates in a row.

**Why:** ETXTBSY is checked against every open-for-write descriptor on the inode at exec time, in any process; CLOEXEC closes the child's copy only when the child execs, so the window is one fork-to-exec of a sibling.

**How to apply:** in the stub helper, after chmod, loop `Command::new(&path).output()` while the error's `raw_os_error() == Some(26)` (sleep 5 ms, bounded); the probe's output is discarded. Landed in `elohim/ark/cli/src/brit_index.rs` and `elohim/eprfs/epr-cli/src/flow/walk.rs` test modules (dev 370721d7f). Any new stub-writing test helper needs the same guard. Related: [[project_elohim_app_local_build_verification_gaps]].
