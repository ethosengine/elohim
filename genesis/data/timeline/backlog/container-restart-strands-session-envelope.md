---
id: "backlog-container-restart-strands-session-envelope"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A workspace container restart strands every open session envelope, and the postStart arrival steps it needs do not all run"
slug: "container-restart-strands-session-envelope"
written: "2026-11-10"
author: "artifact-store storage-closure shift (session f1769d1c)"
status: "open"
priority: "high"
tags: [session-envelope, workspace-restart, device-arrival, berth, peer-daemon, harness-native]
cites:
  - elohim/eprfs/device-runtime/src/envelope.rs
  - elohim/eprfs/epr-cli/src/session_envelope.rs
  - genesis/agentic/bin/berth
  - devfile.base.yaml
---

# A workspace container restart strands every open session envelope

## Observed (2026-11-10, after the kubelet DNS fault was healed by restarting the workspace)

1. **Envelope stranded.** `/nix/xdg/config/elohim/device/sessions/<id>.json` survives on the PVC
   with `ownerPid` in the old container's pid namespace (`ownerPidNamespace` 4026533711) and the
   **same** `ownerBootId` (the kernel did not reboot). The resumed Claude session has the same
   session id and a new pid in namespace 4026532699. `Envelope::owner_dead`
   (`device-runtime/src/envelope.rs:123`) returns false for a foreign namespace (correctly:
   "unknown owners are not declared dead"), so `epr session gc` closes nothing, and
   `show`, `open`, `close` and berth's `prepare-berth` all refuse `envelope-owner-mismatch`.
   No berth lease can be claimed from the resumed session, so it cannot land.
2. **Arrival steps skipped.** `safe.directory '*'` (devfile `setup-git`) was missing, so git
   refused the repository and brit refused `core.hooksPath` as untrusted. rustup's 1.96.1
   toolchain (the gate pin) was gone with `$HOME`. `just status device` names the trust line,
   but not the toolchain or the hooks-path trust.
3. **Peer daemon down.** `peer-serve` is not a postStart event, so nothing restarted it. The
   sibling read this device as silent for 9 h, and the steward lease could not confirm.
4. **Pack count.** The shared object store had 45 packs; gix opens 32 index slots, so every brit
   read failed ("too few index slots, current=32, needed=13") until `git repack -a -d`.

## Wanted

- A proof of owner death that a container restart can satisfy without a reboot: for example, the
  envelope records a per-container identity that is provably retired (the container's own start
  marker on the RWO PVC, or a flock the live owner holds on the PVC), so that `gc` can seal it.
  Until then the steward needs a recorded way to close an envelope whose owner is unprovable.
- `just status device` gains lines for the pinned toolchain, git hooks-path trust, peer daemon
  liveness (sibling head age, not process presence) and the object store's pack count against
  the gix slot limit; each REFUSED line names its cure.
- `peer-serve` and a repack-when-over-limit step join the devfile postStart, or arrival runs them.
