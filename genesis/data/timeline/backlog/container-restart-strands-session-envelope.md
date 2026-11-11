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
  Weigh it against `security-session-successor-admitted-before-facet-release.md`: successor
  admission (`existing`, ~:234) repeats `owner_dead(/proc)`, so a hand-set `closedAt` alone does
  not admit a successor.
- **The generation question.** The 2026-11-10 unblock (steward ruling, journalled in the
  artifact-store `STATE.md` 11-10 16:37Z) moved the stranded record aside unedited, and `open`
  minted a fresh generation 1. That keeps the covenant session key, so the session's clauses stayed
  in force. A proper successor would be generation 2 and inherit no clauses. Decide which a restart
  successor should be, and whether the steward re-offers its clauses.
- **Stale PATH epr.** `~/.local/bin/epr` (bin-launch) resolves the old build bafyreibrqq…, which has
  no `session commit-trailers`, so every commit hook prints `UNMEASURED`. `epr device arrive`
  reports "epr has no verified current" for the head build bafyreiexj56….
- `just status device` gains lines for the pinned toolchain, git hooks-path trust, peer daemon
  liveness (sibling head age, not process presence) and the object store's pack count against
  the gix slot limit; each REFUSED line names its cure.
- `peer-serve` and a repack-when-over-limit step join the devfile postStart, or arrival runs them.

## Rulings and design (2026-11-10, session 785ceff4, steward human:matthew)

- **Design gate (Codex astra).** No in-container fact proves the old owner dead. A helper's flock
  can be freed while its harness lives, and a container id or postStart marker identifies a new
  container without retiring the old one (an RWO volume allows overlapping pods on one node).
  Proof needs a host/runtime termination receipt, and no such issuer exists today.
- **Death proof, ruled "Both".** (1) An automatic path: `epr session open` arms a lifetime holder
  flock on `sessions/<id>.gen<N>.owner.lock`, and a foreign-namespace observer reads a free lock
  as death. Accepted residual: a false death when the holder dies while its owner lives AND an
  overlapping pod observes it. Re-arming on every open by the live owner narrows the window.
  (2) An explicit path: `epr session retire`, steward-quoted and bound to the exact record CID and
  generation. It closes, verifies release, and may admit a successor without ever making
  `owner_dead` true.
- **Successor, ruled "Gen 2, re-offer, release first".** A restart successor is generation n+1 in
  its own covenant namespace. `epr covenant offer` cites the predecessor's sealed clauses by CID as
  history for a fresh affirmation. Admission requires a release receipt from
  `berth verify-released`; this closes `security-session-successor-admitted-before-facet-release`.
- **Option 2 (arrival lines)** shipped beside this: a serve-lock liveness line for the peer daemon,
  plus rust toolchain, git hooks, brit trust and object packs lines. Finding: gix caps index
  slots at 32 only at *reduced* trust (`gix/src/open/options.rs` `resolve_object_store_slots`), so
  the 45-pack failure came from the missing trust, and the device's `epr device trust`
  declaration never reaches brit.
- **peer-serve stays out of postStart (steward ruling).** "hooking that into the devfile.yaml the
  container will fail if anything in the postHook fails to boot. The risk isn't worth it." It stays
  a routine restart step: the arrival line's peer-daemon cure names the devfile `peer-serve`
  command. This row's "join postStart" wish for peer-serve is withdrawn; a repack step is likewise
  left to the `object packs` line's cure rather than postStart.
- **Left open.** A host-issued termination receipt (the strong death proof) is the operator's
  frontier.
