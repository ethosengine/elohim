---
id: "backlog-brit-covenant-session-segment-aliasing"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brit's covenant refs let a session label nest under another session's clauses"
slug: "brit-covenant-session-segment-aliasing"
written: "2026-10-25"
author: "claude-opus-5-5 (device-runtime, pre-push head landing)"
status: "wip"
priority: "medium"
tags: [brit, covenant, session-envelope, security, refs]
cites:
  - elohim/brit/brit-epr/src/elohim/refs.rs
  - elohim/eprfs/epr-cli/src/session_envelope.rs
---

# brit's covenant refs let a session label nest under another session's clauses

## Observed

Covenant clauses live at `refs/notes/brit/covenant/<encode_segment(session)>/<cid>`, and
`list_covenant_clauses(session)` reads every ref under `refs/notes/brit/covenant/<encoded>/`.
`encode_segment` (`brit-epr/src/elohim/refs.rs:270`) escapes `:`, `@`, `?`, `*`, `[`, `\`, `^`, `~`,
space, control characters and `..`, but neither `/` nor `%`. So:

- a clause put under session `victim/x` lands at `covenant/victim/x/<cid>`, inside the prefix
  `covenant/victim/` that session `victim` lists as its own;
- `a%3Ab` and `a:b` encode to the same segment.

## Guarded in epr since 2026-10-25

`live_guard` (epr's facet guard for actor claims, covenant record/check and flow claims) refuses
any session label that is not a P1 envelope id (`envelope::is_envelope_id`: ASCII letters, digits,
`_`, `.`, `-`), on every device. Before that it refused such labels only on a device that had an
envelope directory, and not at all on a fresh device. A caller of `brit-build-ref covenant`
directly is still exposed.

## Fix shape

In brit: percent-encode `/` and `%` in `encode_segment` (and decode them), or refuse a session
segment outside the P1 shape; list clauses for exactly one segment rather than a prefix walk. A
test puts a clause under `victim/x` and asserts `list --session victim` does not return it.

## Done when

`brit-build-ref covenant list --session victim` never returns a clause put under any other label.

## Local implementation and proof — 2026-11-02

Implemented in `.worktrees/brit-session-isolation`, branch
`fix/covenant-session-isolation`, commit `10d083b55`, based on Brit `e93a0206e`. Integration and
installed-release verification remain pending; this is local implementation evidence.

The current baseline already excludes nested ref paths, so `victim/x` does not
appear under `victim`. The remaining failure is encoded-label aliasing:
`a:b` and `a%3Ab` share a segment, as do `dots..name` and `dots%2E%2Ename`.
The new regression failed on the baseline with two verified records for `a:b`.
Listing now binds the verified clause's exact signed session to the requested
label. Mismatches enter the existing `unverifiable` diagnostics; they are not
silently discarded. Ref names, signatures and CIDs remain unchanged, and CID
lookup still finds the original record. No migration or public type change.

Proof: `cargo test --locked -p brit-epr` passes 98 tests, zero ignored;
`cargo clippy --locked -p brit-epr --all-targets -- -D warnings` exits 0;
scoped rustfmt check passes (Rust 1.96.1, empty RUSTFLAGS, dedicated cargo-pool
slot). Existing dependency warnings mention an unknown Clippy lint and ts-rs
serde attributes. The regression covers nested paths, both encoded aliases,
an unwritten alias, unknown sessions, diagnostics and CID lookup.

Story harvest — regression boundary for the existing daily-driver habit:
Given signed clauses under distinct labels with identical encoded ref segments,
when a developer lists one session, only that exact signed session is verified
and the mismatched records are reported. This is executable in
`brit-epr/tests/ref_management.rs` as
`covenant_listing_never_admits_another_signed_session`.

## First-sprint dogfooding friction — priority follow-ups

Observed with installed Brit 0.1.6 at `e93a0206e`. Agent-authored repository
operations used Brit; no direct Git fallback was needed. Existing test helpers
and governance scripts retain their internal Git calls.

1. **First Brit priority — unscoped staged diff:** `brit add` staged all three
   intended files, but `brit diff --cached` refused with `diff attributes and
   text conversion are not supported`. Explicitly naming those same three
   staged files succeeded. `gitoxide-core/src/repository/diff.rs` checks every
   tracked path's attributes before eliminating unchanged entries; this repo's
   unchanged fixture archives have `diff=lfs`. Acceptance: unchanged files with
   unsupported attributes must not block staged ordinary-file patches; changed
   unsupported files must still refuse. Scoped Brit diff is the safe workaround;
   no Git fallback was used.
2. **High — command reminder coverage:** this Codex tool adapter repeatedly
   reports `codex/Bash invocation adapter unsupported`. Project hooks are
   configured according to `epr doctor`, but this is not proof that a shell Git
   invocation receives the intended Brit reminder. Owner: harness projection.
   Acceptance: an isolated Git invocation through this adapter receives the
   reminder, while a Brit invocation does not. Do not claim enforced adoption
   from instruction compliance alone.
3. **High — bounded history inspection:** `brit log` has no limit option;
   `brit -r elohim/brit log | head -45` emits a broken-pipe error and backtrace.
   Redirecting to a disposable file works but materialized 17,618 lines.
   Owner: Brit CLI. Acceptance: bounded history output and a quiet early-closing
   consumer, with an isolated CLI regression. Inspecting command help is a
   normal adaptation cost, not a Git-fallback justification.
4. **High — gate discovery:** `just gate rust-style` refuses before executing
   the checker because `genesis/local-dev/gate-task-proof/repo/app/elohim-app/`
   contributes a duplicate project manifest. Owner: orchestrator discovery.
   The existing fixture was preserved; the native checker can be invoked
   directly, but whole-tree standalone checking reports a missing style policy.
   Both changed Rust files pass the native prospective checker using the parent
   policy (`failures: []`); this does not establish the full reach gate.

Brit status, worktree creation, scoped staging, scoped cached diff and commit succeeded.
The parent habit projection check passes; the isolated branch's habit delta
travels with its implementation and will enter the parent census on integration.

## Priority follow-up 1 implemented locally — 2026-11-02

The stacked change in `.worktrees/brit-session-isolation` skips cached paths
whose HEAD and index mode/object-ID pairs are identical, before checking
unsupported patch attributes. The installed CLI reproduced the failure first;
the candidate max-pure binary passes all 18 native-authoring CLI journeys.
The regression compares ordinary patches with Git and covers unchanged LFS
attributes and gitlinks, empty staged diffs, index preservation, and refusal
without partial output for changed attributed content, mode-only changes and
gitlinks. Working-tree diff semantics are unchanged.

### Native SCM direction — operator clarification, 2026-11-02

Brit is the native SCM for Elohim developers, with first-class EPRFS support.
Developers work through a local, editable projection of content held in the
P2P datastore, backed by a local cache. Brit manages changes and version
history through EPRFS, while Git and LFS provide interoperability with
Git-based workflows. This describes the intended architecture.

The editable working projection and cache have distinct preservation
requirements, even if they share storage: cached content can be evicted and
reconstructed, while uncommitted developer edits must be preserved. Cache
eviction must not discard those edits.

Carry this distinction into backlog planning and acceptance: the native
developer workflow is Brit working with the EPRFS editable local projection;
LFS interoperability is a Git compatibility concern. The staged-diff repair
above addresses that compatibility concern and does not establish native
EPRFS support. Cache population, local edits and their return to the P2P
datastore still need their own specified and verified behavior; this note
records the operator's direction, not an implemented synchronization contract.

Planning questions: establish ownership of version history, the boundary
between local change recording and publication, offline editing and conflict
handling, and recovery after interrupted synchronization or cache eviction.
Specify which responsibilities belong to Brit, EPRFS and the P2P datastore
before choosing APIs or an implementation sprint.

A separate backlog write-up was reported from a btw session, but its exact
path has not been verified in this checkout. This section preserves the
clarified intent until that source can be connected.

The root style gate still refuses at the previously recorded duplicate fixture
manifest; prospective checks and formatting pass. This remains local delivery
proof, not upstream integration or installed-release acceptance.

Committed as `d99e80c2f` on `fix/covenant-session-isolation`, stacked after the
session-isolation fix. The candidate successfully rendered the real worktree's
unscoped staged diff for all three intended paths; no direct Git fallback.
The branch is locally committed, with upstream integration still pending.

## Native-storage ownership

The native source capture, durable restore and fixed-candidate execution progression
is owned by the [daily-driver backlog's native-storage section](brit-daily-driver-handoff-2026-10-27.md#native-storage-source-to-explicit-execution-2026-11-02).
Its scoped measurements and ordered successor stations live there, with runnable
commands in the owning sprint result. This covenant/session-segment bug remains
open: local source execution does not repair ref encoding, clause verification or
session authority, and its evidence must not be read as closure of this note.
