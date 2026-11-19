---
id: "backlog-ark-brit-index-spawn-skips-declared-trust"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "ark's brit index spawns brit-build-ref without the device's declared repository trust"
slug: "ark-brit-index-spawn-skips-declared-trust"
written: "2026-10-27"
author: "claude (overnight integration shift, session 3f9afd7a, steward human:matthew)"
status: "wip"
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

## Fix (branch `feat/ark-declared-trust`, 2026-11-19)

epr-cli keeps the authoritative reader in `git.rs`, hardened in place:

- The declaration and its directory are held to `device_key::check_key_dir`'s custody rule.
  Neither may be writable by everyone or by a group the process does not hold. The file is
  stat'ed through the descriptor it is read from, and ownership is not checked.
- A non-UTF-8 `GIT_CONFIG_COUNT` is malformed and adds nothing.
- A reason is kept when a declaration exists but cannot be used. `repository trusted` in
  `epr device status` names it.

The format contract is stated once, in that module's doc. ark has its own reader of the same
format, `elohim/ark/cli/src/declared_trust.rs`. It carries the trust on all three `brit_index.rs`
spawns: `build find`, `signer --json`, and the `git diff` of `unchanged_since`. When a child fails
and the declaration could not be applied, each failure message ends with "; no declared trust
applied: <why>".

The two readers cannot drift silently. epr-cli mints
`elohim/eprfs/epr-cli/tests/fixtures/declared-trust-vectors.json` from its own functions and
compares it byte for byte, so a hand edit fails that test. ark's test runs its reader against the
same file.

The done-when cross-uid test is `a_checkout_another_uid_owns_is_read_with_only_the_device_declaration`.
It is `#[ignore]`d because it needs root, and it fails if it cannot stage. The installed
`brit-build-ref` was also probed by hand on a checkout owned by uid 65534, with global and system
git config disabled:

- without the entries, `build find` and `signer --json` exit 1 with "detected dubious ownership";
- with `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0=<root>` they exit 0.

### Why not elohim-device-runtime

The first design moved the reader into `elohim-device-runtime`; it is kept on
`backup/ark-brit-index-trust-device-runtime`. That crate is a declared input of the storage image
step (`elohim/holochain/build-manifest.json` lines 102 and 427). Any change there selects
`elohim-edge`, which runs the storage build and image and rolls the alpha fleet, and also selects
`elohim-steward`. `genesis/orchestrator/graph-walker.mjs` confirms it: given that branch's paths, it
selects `elohim-edge` with the reason `source: elohim/eprfs/device-runtime/src/checkout_trust.rs`.
A developer-checkout trust reader also does not belong in a crate the node's storage runtime
links.

