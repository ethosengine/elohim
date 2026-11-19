---
id: "backlog-brit-declared-trust-and-reduced-trust-pack-cap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brit ignores command-scope safe.directory, so it runs at reduced trust and cannot open a repository with more than 32 pack indexes"
slug: "brit-declared-trust-and-reduced-trust-pack-cap"
written: "2026-11-19"
author: "claude-opus-5-5 (brit sprint 3, session 0e0924d5, steward of record human:matthew)"
status: "wip"
priority: "high"
relatedNodeIds: []
tags: [brit, gix, trust, safe-directory, daily-driver, gate-evidence, epr-cli]
cites:
  - genesis/data/timeline/backlog/brit-build-fingerprint-cannot-resolve-any-commit-in-monorepo.md
  - genesis/orchestrator/gate-runner.mjs
  - genesis/orchestrator/gate-attest.mjs
  - genesis/orchestrator/gate-evidence.mjs
  - .claude/hooks/pre-flight-check.py
  - .claude/scripts/_lib/epr_meta_git.py
  - elohim/eprfs/epr-cli/src/artifact.rs
  - elohim/eprfs/epr-cli/src/integrate.rs
  - elohim/eprfs/epr-cli/src/device/arrival_work.rs
  - elohim/eprfs/epr-cli/src/git.rs
  - elohim/eprfs/device-runtime/src/layout.rs
shift_objective: |
  An independent integrator merges lane sprint3/trust into sprint3/integrate once it clears
  review (decided order: brit-sprint-3-handoff-2026-11-19.md).
---

# brit at reduced trust: command-scope safe.directory and the 32-slot pack cap

## Why brit stays at reduced trust

gix accepts `safe.directory` only from the system and global files. git also honours the command
scope (`-c`, `GIT_CONFIG_COUNT`/`KEY`/`VALUE`). So the trust the elohim tools pass to children reaches
brit-build-ref's git children but not brit itself, which stays at reduced trust: 32 pack-index slots,
a 16 MiB allocation limit, hooks refused. gix does not read `GIT_CONFIG_PARAMETERS`.

## What fails

With more than 32 pack indexes, brit 0.2.6 cannot open the repository at reduced trust:

- `brit log` → "The object database has too few index slots";
- `brit build fingerprint` → "could not resolve commit 'HEAD': The rev-spec is malformed … object
  database slots exhausted".

The monorepo on ethosengine had 33 pack indexes on 2026-11-19 and 36 later the same day (177 MB of
packs); shem had 28. Every `brit fetch` adds a pack and nothing consolidates. It works only while the
global `safe.directory = *` is present; after a workspace restart every brit read fails until setup
restores it. Cure today: `git repack -a -d`.

This is the reduced-trust leftover of
`brit-build-fingerprint-cannot-resolve-any-commit-in-monorepo.md`, whose full-trust case is fixed in
brit 0.2.6.

## Lane `sprint3/trust` (tip `d96ff65b2d`, in review, not merged)

- gix honours command-scope `safe.directory`.
- At reduced trust, slots are sized from the pack indexes present, up to a ceiling of 512, with one
  named error naming both cures.
- brit-epr's git children report real failures.
- One error printer, with no backtrace unless `BRIT_BACKTRACE=1`. `RUST_BACKTRACE` is set on these
  devices, so the last stderr line of a failed brit command was a stack frame, and every caller that
  reports the last line showed that instead of the cause.

It changes upstream-owned gix files (`gix/src/open/*`, `gix/src/config/tree/sections/safe.rs`), against
the letter of brit's CLAUDE.md ("Upstream-owned gix-* code stays byte-identical to upstream"). Earlier
`fix(gix)` commits such as `fe6da99ffb` touch the same files. The steward has been told. These are
candidates to offer upstream.

rakia-brit 0.1.1 (gix 0.81) has no 32-slot wall.

## Inside a gate brit has no command-scope trust

The monorepo's gate-runner deliberately deletes `GIT_CONFIG_PARAMETERS` and
`GIT_CONFIG_COUNT`/`KEY`/`VALUE` from its children (`genesis/orchestrator/gate-runner.mjs:186-188`),
because `git -c core.hooksPath=… push` leaked into fixture repositories. So inside a gate brit runs with
no command-scope trust. Reported to the session that owns those files, together with these
silent-degrade sites:

- `genesis/orchestrator/gate-attest.mjs:75` (a refusal reads as clean);
- `genesis/orchestrator/gate-evidence.mjs:157` and `:269`;
- `genesis/orchestrator/gate-runner.mjs:122`;
- `.claude/hooks/pre-flight-check.py:77`;
- `.claude/scripts/_lib/epr_meta_git.py:35`.

## epr-cli spawn audit (dev checkout)

- `elohim/eprfs/epr-cli/src/artifact.rs:1104` spawns brit-build-ref with no trust env.
- `integrate.rs:210` gives the gate child no trust env.
- `device/arrival_work.rs:45` builds a bin-store `Store` that keeps the plain runner, and its failure
  goes only to `arrival.log`.
- `git.rs` `discover_root` reports "could not find a git worktree" for a dubious-ownership refusal.

## Open for the steward

- Declared trust applies per exact checkout root, and trusted-checkouts on ethosengine lists
  `/projects/elohim` and one worktree, so it does not apply in other linked worktrees. This is a design
  question for the steward.
- `elohim/eprfs/device-runtime/src/layout.rs` `read_saved` reads `layout.json` (bin-store, state,
  cargo-pool, project-root paths) with no custody check. It is on shem's deliberate-roll list.
- The custody predicate checks writable-by-others, not owned-by-others.
