---
id: "backlog-ark-brit-index-spawn-skips-declared-trust"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "ark's brit index spawns brit-build-ref without the device's declared repository trust"
slug: "ark-brit-index-spawn-skips-declared-trust"
written: "2026-10-27"
author: "claude (overnight integration shift, session 3f9afd7a, steward human:matthew)"
status: "open"
priority: "medium"
ci_status: none
jobs: []
tags: [ark, brit, trust, safe-directory, restart, gate-task]
---

# ark's brit index spawns brit-build-ref without the declared trust

## What is wrong

`c66b9b0f8` made epr's own children carry the trust declared by `epr device trust`. It added
`git::trust_args` and `git::trust_env` in `elohim/eprfs/epr-cli/src/git.rs:256` and `:268`, so
`brit-build-ref` and the recall fold stop hitting git's "dubious ownership" refusal after a
restart, when a different uid owns `/projects`.

`feat/gate-live` (landed `c832238fe`) added a second spawn of the same binary that skips this.
`elohim/ark/cli/src/brit_index.rs` `find()` runs `Command::new(brit-build-ref) … build find`
with no trust arguments or environment. After a restart, before global `safe.directory` is
restored, `build find` fails. Its callers at `:290` and `:381` then report the index as
unreachable or refuse, and the gate re-runs work it could have reused. Nothing is unsafe. The
cost is a gate that silently stops paying once.

Found by an independent Codex review of gate-live (rated PLAUSIBLE, not yet reproduced). The
receipt is `genesis/local-dev/overnight-20261027/leg1-gate-live.receipt.md` §2.

## Why the fix is not one line

Both helpers are `pub(crate)` in `elohim-epr-cli`. ark depends on `elohim-epr`, `eprfs-core`
and `elohim-device-runtime`, not on the CLI. The declaration reader has to move to a crate both
sides already share (`elohim-device-runtime` is the likely home, since it owns device state)
before ark can append the same `GIT_CONFIG_*` entries.

## Done when

- One shared function gives the trust env for a root.
- epr-cli's `trust_env` and `trust_args` delegate to it.
- ark's `find()` appends it to the spawn.
- A test proves `build find` succeeds in a scratch repo whose owner differs from the caller,
  with only the device declaration present. The test needs no global `safe.directory`.
