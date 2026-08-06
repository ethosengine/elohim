---
id: "backlog-prepush-cargo-target-pool"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Pre-push cargo-target-pool: originally bypassed entirely (→ ENOSPC); reopened 2026-08-05 because a dangling /tmp slot symlink makes the pooling fail SILENTLY back to an in-tree build"
slug: "prepush-cargo-target-pool"
written: "2026-06-02"
author: "cartographer"
status: "open"
resolved: "2026-06-04"
reopened: "2026-08-05"
priority: "high"
area: "cargo"
recurrence: 2
source_shifts:
  - "2026-05-18"
  - "2026-08-05"
domain: "code"
relatedNodeIds:
  - "memory:feedback_multi_agent_pvc_pacing"
  - "memory:project_devspace_disk_cleanup_procedure"
  - "memory:project_ci_storage_topology"
tags: [cargo, prepush, target-pool, enospc, code-domain]
shift_objective: |
  The pre-push hook compiles into the default target dir instead of sourcing a per-crate key
  from the cargo-target-pool, so a multi-crate push piles all build artifacts into one
  location and runs the volume out of space (ENOSPC) — observed 2026-05-18. The pool exists
  precisely to scope each crate's target dir, but the hook doesn't use it.
  Resolve it by having the pre-push hook source a per-crate CARGO_TARGET_DIR from the
  cargo-target-pool (the same keying CI uses), so each crate's artifacts land in their own
  pooled slot and a push doesn't ENOSPC. This is code-domain (.husky pre-push + the
  cargo-pool helper). Mind the shared-target hazard (concurrent agents sharing a target dir is
  unsafe — feedback_multi_agent_pvc_pacing) and the disk-pressure thresholds
  (project_devspace_disk_cleanup_procedure). Done when a multi-crate pre-push uses per-crate
  pooled target dirs and no longer ENOSPCs.
---

# Pre-push hook sources the cargo-target-pool per crate

## Why this matters

Code-domain. An ENOSPC at pre-push time blocks the push entirely and often requires a manual
target-dir clean to recover — friction on every multi-crate change. The pool already solves
this for CI; the hook just doesn't use it.

## The failure shape

- Pre-push compiles into the default target dir (no per-crate CARGO_TARGET_DIR).
- A multi-crate push accumulates all artifacts in one place.
- The volume runs out of space → ENOSPC → push blocked.

## Shape of the fix (code-domain)

The `.husky` pre-push hook sources a per-crate CARGO_TARGET_DIR from the cargo-target-pool
(matching CI's keying), so artifacts land in per-crate pooled slots. Respect the shared-target
hazard — concurrent agents must not share a slot (`feedback_multi_agent_pvc_pacing`) — and the
85% PVC act-threshold / cleanup procedure (`project_devspace_disk_cleanup_procedure`).

## Acceptance

A multi-crate pre-push uses per-crate pooled target dirs and no longer ENOSPCs.

## Resolution (2026-06-04)

`.husky/pre-push` now has `gate_pool_slot <ws_rel>` (generalizing the sweettest-only
`sweettest_pool_slot`) and `run_gate` exports a per-crate pooled `CARGO_TARGET_DIR` for
elohim-storage / epr-storage / doorway / steward-node gates (subshell-scoped, fail-open,
explicit ws_rel constants — NOT dynamic `cargo-pool key`, which mis-keys storage). Beyond
the original ask, the hook is now PVC-pressure-aware: it reads
`genesis/agentic/pool-policy.json`, runs the guarded `cargo-pool enforce --yes` reclaim at
the soft watermark, and defers heavy Rust gates with a DEFERRED-BY-PVC banner at the hard
ceiling (FORCE_HEAVY_GATES=1 overrides). ENOSPC mid-push is now structurally prevented from
both directions: builds land in policy-bounded slots, and pushes that can't fit defer
instead of starving the volume.

## REOPENED 2026-08-05 — the pooling fails SILENTLY when a slot's /tmp target is gone

The 2026-06-04 resolution is correct in design and **fails open in exactly the wrong direction**.
Pool slots for the `/tmp`-backed workspaces are symlinks:

```
/projects/.cargo-target-pool/family/angular22/doorway__doorway-service/dev -> /tmp/cargo-doorway
```

`/tmp` does not survive a host reboot (or a tmp reaper), so the symlink goes **dangling**. The
guard in `run_gate` is:

```bash
if [ -n "${GATE_SLOT:-}" ] && mkdir -p "$GATE_SLOT" 2>/dev/null; then
  export CARGO_TARGET_DIR="$GATE_SLOT"
  echo "  [$PROJECT_NAME] pooled target: $GATE_SLOT"
fi
```

`mkdir -p` on a **dangling symlink** fails with `File exists` (the symlink exists, but does not
resolve to a directory) — verified directly:

```
$ mkdir -p /projects/.cargo-target-pool/family/angular22/doorway__doorway-service/dev
mkdir: cannot create directory '...': File exists   # exit 1
```

So the `if` is false, `CARGO_TARGET_DIR` is never exported, **no message is printed**, and the
gate falls back to an in-tree `target/` — the one path `project_container_cargo_environment_quirks`
records as ENOENT-prone in this container. Observed 2026-08-05 12:18Z on a `dev`-targeted push:

```
[elohim-storage] pooled target: /projects/.cargo-target-pool/family/angular22/elohim__elohim-storage/dev
elohim-storage: PASSED (258s)
...
[doorway] Running quality gate via just...        # <-- no "pooled target:" line
error: failed to write `/projects/elohim/doorway/doorway-service/target/debug/.fingerprint/lzma-sys-.../invoked.timestamp`
error: failed to write /projects/elohim/doorway/doorway-service/target/debug/deps/libclang_sys-....rmeta: No such file or directory (os error 2)
PRE-PUSH GATE: FAILED (284s)  —  Failed: doorway
```

Storage passed (its slot is a real directory); doorway failed. **Nothing in the output says
pooling was skipped** — the failure presents as a doorway code/build problem, which it is not.
Disk was fine throughout (64% used, 318G free), so the PVC-pressure path is not involved.

**Other dangling slots found in the same sweep** (any of these will silently degrade the same
way for whoever pushes next on that family):

```
/projects/.cargo-target-pool/family/dev/crates/dev                  -> /tmp/cargo-pool-devfam-crates
/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev -> /tmp/cargo-target/doorway-gate-dev
/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev   -> /tmp/cargo-target/pool-elohim__elohim-storage-dev
/projects/.cargo-target-pool/family/dev/steward__node/dev            -> /tmp/cargo-pool-devfam-steward__node
```

**Shape of the fix (two parts, both small):**

1. **Resolve through the symlink before creating.** `mkdir -p "$(readlink -f "$GATE_SLOT")"` (or
   test `[ -L "$GATE_SLOT" ] && [ ! -e "$GATE_SLOT" ]` and recreate the target) so a `/tmp`-backed
   slot self-heals after a reboot.
2. **Never fall back silently.** If the slot cannot be prepared, print a loud
   `⚠ POOL SLOT UNAVAILABLE — building in-tree` warning naming the slot. Fail-open on the *build*
   is defensible; fail-open on the *signal* is what cost this push. Same lesson as the
   `ci-rbac-jenkins-deployer` phantom-green class: a degraded mode that announces itself is fine,
   a silent one is not.

A `cargo-pool doctor` (or a check folded into `cargo-pool status`) that reports dangling slots
across all families would catch these before a push does.

**Immediate workaround** (what unblocked 2026-08-05): `mkdir -p /tmp/cargo-doorway`, then re-push.
Leaves a junk in-tree `doorway/doorway-service/target` (5.9M here) for `cargo-pool legacy-targets`
to reclaim.

## Fix implemented in-tree 2026-08-06 — pending commit/CI verification

Both parts of the fix landed in `.husky/pre-push.bash` at all four call sites that guard a pooled
`CARGO_TARGET_DIR` (the `crates/seam-contracts` standalone block, `run_gate`'s
elohim-storage/doorway/steward-node case, and both `sweettest-check` case-statement copies — one
per project-detection path per `CLAUDE.md`'s manifest-driven-vs-grep-fallback split):

1. `mkdir -p "$GATE_SLOT"` → `mkdir -p "$(readlink -f "$GATE_SLOT")"` — resolves through the slot
   symlink before creating, so a dangling `/tmp`-backed slot self-heals (recreates the vanished
   `/tmp` target, then the symlink resolves normally).
2. The `if` guard's silent no-op on failure now has an `else`: when the slot path is non-empty
   (pool present) but preparation still fails, it prints
   `  ⚠ POOL SLOT UNAVAILABLE — building in-tree: <slot path>` naming the slot. The pool-absent
   case (`GATE_SLOT` empty — no pool root at all) still fails open with no message, unchanged from
   before — that path was never the bug.

Verified locally in the scratchpad (not against a live push): `bash -n .husky/pre-push.bash`
passes; a simulated dangling symlink (target removed, mirroring a post-reboot `/tmp`) now
self-heals and exports `CARGO_TARGET_DIR` with the `pooled target:` line, where before it silently
produced no output and left `CARGO_TARGET_DIR` unset; a simulated unpreparable slot (target path
traverses a regular file, so `mkdir -p` cannot succeed even after `readlink -f`) now prints the
`⚠ POOL SLOT UNAVAILABLE` warning instead of failing silently; the pool-absent case (`GATE_SLOT=""`)
still produces no `CARGO_TARGET_DIR` and no warning, confirming fail-open on absence is untouched.

Not yet committed, not yet pushed, no CI run against it. Leaving `status: open` — closure needs a
real pre-push (ideally against one of the dangling slots listed above, or a freshly re-dangled one)
showing either the pooled-target line or the loud warning, never silence.

**2026-08-06, fifth site, same reboot class, different mechanism:** `elohim/target` is an
in-tree symlink → `/tmp/cargo-elohim-workspace` (created 2026-07-04, not a pool slot), and the
elohim-epr gate's no-just fallback runs plain cargo against it — dangling post-reboot it fails
`failed to create directory .../elohim/target: Not a directory (os error 20)` and reds the gate.
Healed by recreating the /tmp dir. Durable fix belongs with this item's scope: either migrate the
elohim workspace to a pool slot (crates family) or extend the resolve-through-symlink guard to the
fallback cargo sites that use in-tree `./target`.
