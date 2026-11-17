---
id: "backlog-steward-device-arrival-open-items"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Steward-device arrival sprint: what four landings left open"
slug: "steward-device-arrival-open-items"
written: "2026-11-17"
author: "claude-fable-5-2 (shem, steward-device-arrival sprint, session 76dc84be)"
status: "open"
priority: "high"
tags: [device-arrival, artifact-store, gate, brit, berth, onboarding]
---

# Steward-device arrival sprint: what four landings left open

Matthew enrolled a physical laptop (durable `$HOME`, no `/projects`, outside Kubernetes) as a
steward peer on 2026-11-16 and had to compile `epr` for 25m34s to do it. The sprint that followed
landed four changes on `dev` across shem and ethosengine: `7c8cbb94e` (one device layout
resolver, install destination, envelope model re-open), `ef0b1db12` (eprfs gate key fix,
durable-home persistence contract), `521649ed0` (layout wiring into device key, device status and
berth; the session-open model fix). The habit `workspace-device-arrival` stays red: nothing has
been tried on the laptop yet. The rows below are what the sprint found and did not close.

## Row 1 — a device cannot yet draw `epr` from a sibling

`epr bin follow` takes bytes only from a record already in the local store, an explicit closure
directory, or a local `cargo build` (`device-runtime/src/bin_store/lifecycle.rs`). Train 2
(ethosengine, branch `feat/steward-arrival-t2`, unpushed on 2026-11-17) adds the consumer half:
`follow` and `bootstrap` look for a sibling's crossed Produce at the published head, fetch by
output CID into `<bin_store>/incoming`, ingest with `gate_run sibling:<did>` and refuse unless
the release CID equals the producer's. The producer half records the Produce where the bytes
are built, in the eprfs gate's ingest step.

Blocked on the steward: ethosengine's permission layer refused a relayed grant, so the branch
does not land until the steward states the write-set there (`epr-cli/src/bin.rs`,
`epr-cli/src/artifact.rs`, `epr-cli/src/artifact/**`, `justfile`).

Probe: on a device with no cargo pool, `epr bin bootstrap --tool epr` lands the sibling's
release CID with no compile; on shem, `EPR_FOLLOW_SIBLING=1 epr bin follow --tool epr` does the
same and the wall-clock is recorded against a local build.

## Row 2 — no eprfs-view build could be keyed on any device

`epr artifact key` refused every method over the eprfs workspace: the generated-output scan
(`epr-cli/src/artifact/generated.rs`) lexes tracked Rust sources for string literals that resolve
into ignored generated directories, and it read its own unit-test fixtures and the workspace's
integration tests. Train 2 carries a bounded cure in separate commits (skip the body of a
top-level `#[cfg(test)] mod`, then files under `tests/`, `benches/`, `examples/` under five
conditions each). Two reviews on ethosengine found and closed: a build script printing
`cargo:rustc-cfg=test`; a pre-existing bypass of the whole scan through raw C strings
(`cr"\"`); a leading shebang or byte-order mark; `RUSTFLAGS=@file` and `--config`.
Residual named there: identifiers are matched as alphanumeric, not by XID.

This loosens an integrity check. It was ruled by the session model in the steward's absence and
lands only with the steward's confirmation.

## Row 3 — two devices' build notes diverge on origin

`git push origin 'refs/notes/brit/build/*:refs/notes/brit/build/*'` fast-forwards only for the
device that pushes first. Neither device has a notes fetch refspec or a merge strategy. After
ethosengine pushed 32 refs for `7c8cbb94e`, shem's push for `ef0b1db12` was rejected on about 40
refs; shem's verdicts for `ef0b1db12` and `521649ed0` are local only. No one should guess a merge
strategy over signed attestations. Needs a ruling: how per-device verdict notes combine.

## Row 4 — a fresh worktree fails a push minutes in, for a reason preflight already printed

Both devices lost a push cycle on 2026-11-16: preflight printed the missing
`elohim/sdk/storage-client-ts/dist/index.js` (and, earlier, the uninitialised submodules), the
push carried on, and `genesis-a2o` failed typecheck minutes later. `just dev prep` cures both in
under a minute. The missing dist should refuse at preflight when a selected gate reads it.

## Row 5 — Brit gaps met while rebasing

- `brit rebase origin/dev` failed with "Directory staging is not supported; name explicit files"
  and left no active rebase.
- After a completed `git rebase`, `brit commit` refuses with "merge metadata exists without
  MERGE_HEAD" because of a stale `AUTO_MERGE` in the worktree's git directory.

## Row 6 — path guessers still outside the layout

`genesis/agentic/bin/berth` `envelope_directory()` restates the device-key path rule, and
`genesis/agentic/shift_lease.py:215` keeps its own `/projects/.claude-config` default. The gate
looks for `ark` in the pushing worktree's own pool family and falls back to the direct path when a
new family has none.

## Row 7 — eprfs still keys on whole documentation directories

`epr-cli/tests/flow_scope.rs` declares reads of `genesis/docs/superpowers/specs`, `plans`,
`held`, `genesis/docs/plans`, `genesis/docs/held` and `genesis/a2o/features`, so any spec or story
edit re-runs the eprfs workspace gate (26 to 60 minutes). The `genesis/manifests` case was closed
in `ef0b1db12`. Also seen: the narrowing line printed "0 of 13 members" for a push that changed
three `epr-cli` sources.

## Row 8 — the laptop itself

Owed before the habit can turn: on the laptop, an `epr` at `521649ed0` or later;
`epr actor device persistence --declare durable-home`; `epr session open`; the roster commit
`c6a5f57fc` published through the normal gate; a verified head exchanged with a sibling; a
start, stop and restart that keep the same device key.
