---
id: "backlog-brit-ci-and-workspace-limits-found-in-sprint-3"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brit CI and workspace limits met during brit sprint 3: cross-platform blind spots, first-push style red, cargo pool eviction, and smaller items"
slug: "brit-ci-and-workspace-limits-found-in-sprint-3"
written: "2026-11-19"
author: "claude-opus-5-5 (brit sprint 3, session 0e0924d5, steward of record human:matthew)"
status: "backlog"
priority: "medium"
relatedNodeIds: []
tags: [brit, ci, cross-platform, cargo-pool, attribution, constraint]
cites:
  - genesis/agentic/pool-policy.json
  - genesis/data/timeline/backlog/brit-sprint-3-handoff-2026-11-19.md
---

# brit CI and workspace limits found in sprint 3

Each item below was met during brit sprint 3 (integration branch `sprint3/integrate`, pushed as
`run-ci/sprint3-integrate`).

## (a) Breaks the device's local lanes do not show: cross-platform and documentation

No non-Linux Rust target is installed on the device, so two compile breaks reached CI or were caught
only by a manual audit:

- a Unix-only permissions API in a new test (four Windows jobs red on run 44477093766);
- a function the lock lane called that the non-Linux stub defined but did not export (gitoxide-core
  would not have compiled on macOS or Windows).

A third class, the `small`/doc lint legs, caught dead code and private intra-doc links that the
default build never shows.

Two more classes were met later in the sprint:

- **Runtime differences on Windows that compile fine:** command-string splitting of paths (a test
  helper formatted a Windows path into a command string that `gix_testtools::git` splits, and the
  splitting ate the backslashes); a dependency's platform-dependent default (`globset` makes a
  backslash an escape on Unix and a path separator on Windows); and Git for Windows refusing to stage
  a path containing a backslash, found by reading before the push and gated. CI run 44481834878 showed
  four such test failures on both Windows runners, all from the build-inputs lane.
- **The documentation legs:** `just doc` builds with warnings denied and is part of the required
  `lint` job. Lane-level verification never ran it, and a public doc comment linking to an item in a
  private module broke it (caught at integration).

## (b) The first push of a new `run-ci/**` branch always fails brit-rust-style

`scripts/ci/rust-style-ci.sh` (brit repository) falls back to `origin/main` when the event has no
`before` id, and reports UNMEASURED when main is not an ancestor.

## (c) Journey snapshots and variants

A bash journey snapshot changed because a global option adds `[OPTIONS]` to subcommand usage lines.
The four journey variants (small, pure, async, max) are not all run by any local lane.

## (d) cargo-deny-advisories is red on main too

RUSTSEC-2026-0543 (chrono unmaintained) and a libz-sys heap overflow advisory. The job is not among
the jobs `Tests pass` requires.

## (e) The shared cargo pool evicts live lanes

The shared cargo pool is capped at 150 GB (`genesis/agentic/pool-policy.json`), and a hook allows one
cargo build at a time. Six parallel lanes pushed the pool to 157 GB, and its enforcement evicted live
lanes' build caches (twice for some), forcing cold rebuilds. A subagent pinned its slot against policy,
and the pin was removed. Practical width on this device is about four build lanes.

Build caches placed in session scratch sit on the same volume as the pool, and so sidestep its cap.
One lane did this (about 11 GB, removed).

## (f) Attribution UNMEASURED on every brit commit

After `epr setup` in the brit clone, every commit prints "Brit attribution UNMEASURED: EPR lookup failed
or timed out (requires session commit-trailers --json)". The hook never blocks. Trailers were written
by hand.

## (g) A tracked advisory file

`.claude/data/epr-meta-advice.json` was tracked in brit by accident (upstream merge `e8ec5252b8`). It is
untracked and ignored on the integration branch.

## (h) A journey test that reads the enclosing monorepo

`tests/cli-journey` `rakia.rs::fingerprint_emits_64_char_blake3_hex` reads the enclosing monorepo, and
fails on a host where that checkout is foreign-owned and over the reduced-trust pack cap (see
`brit-declared-trust-and-reduced-trust-pack-cap.md`).

## (i) An intermittent failure, seen once

`memory_object_store::explicit_limit_streams_sixty_four_mib_ofs_and_ref_deltas`: "At least one object
couldn't be looked up even though it must exist". Seen once and not reproduced in five reruns.

## (j) The main checkout's local `dev` diverged from origin/dev

On ethosengine, merge base `e239f3393`; local-only commits `a807b7ff6` and `5d53c7ac8`. Reported to the
steward, untouched.
