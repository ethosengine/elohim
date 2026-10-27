---
id: "backlog-device-plane-fixture-vs-real-stage-seam"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The device and steward-space plane cannot tell a test-fixture network from a real one"
slug: "device-plane-fixture-vs-real-stage-seam"
written: "2026-10-27"
author: "claude-opus-5-5"
status: "open"
priority: "high"
area: "eprfs / berth / epr peer / a2o devflow"
domain: "D5"
relatedNodeIds:
  - "habit:devices-share-one-berth"
tags: [fixture, network-stakes, session-envelope, steward-space, isolation]
---

# The device plane needs the network stakes stage

The dataplane has a first-class seam between test and real networks: the network stakes stage
(`elohim/elohim-storage/src/trust/stage.rs`, Simulacra < Bootstrap < Coordinated < Enforced).
Simulacra is reached only by explicit declaration, never by default and never from `DEV_MODE`.
The device plane has no equivalent. That plane covers session envelopes, berth moorings and
leases, the `epr peer` steward channel, and the flow store it projects. Fixture runs write into
real device state there.

Two leaks have been observed:

- **Leaked T8 holder sessions.** Four `t8-*-holder` sessions, declared by the T8
  cross-workspace lane on 2026-10-25, sat in the real store `/nix/xdg/config/elohim/device/sessions/`
  bound to a since-deleted worktree (`.worktrees/endpoint-row`). `epr session gc` aborted on the
  first one, so every dead-owner envelope on the device stayed open. A resumed Claude session could
  not re-own its envelope, and every `epr` verb refused `envelope-owner-mismatch`.
  - The gc half is fixed on `fix/session-gc-continues` (daaae3744): gc reports `failed[]` and
    keeps sweeping.
  - The four records remain until the lane that wrote them closes them, or a stage seam excludes
    them.
- **Leaked fixture flow rows.** Fixture rows (`s-a`/`s-b`, `at=1000`, dseq 71/75/77) crossed
  the steward channel into both devices' flow stores. They collide with real dseq numbers, and
  the crossing is permanent. Challenger item M1 in the 2026-10-27 steward-space design run.

## Decision for now (interim boundary)

Isolation by declared home is acceptable as the interim boundary. A fixture lane runs under its
own device home, as the session-envelope tests do with `config-home`, and never under
`/nix/xdg/config/elohim/device`. The T8 lane does not meet this yet.

It is not acceptable as the resting state, because nothing enforces it.

## Done when

- **The stage is carried.** Every device-plane record kind (session envelope, mooring, lease,
  note, crossing event) and the steward space carry a declared network stakes stage. The default
  is `Enforced` on a device with a standing human; `Simulacra` must be declared.
- **The stage is in the identity.** The space id, and therefore the gossip topic, is derived
  with the stage (steward-space design slice A12: `space_id = CID(genesis{chain_root, salt,
  stage})`). A Simulacra record can never enter an Enforced store or topic, by construction.
- **Wrong stages are refused.** `epr session open --declare` and berth refuse a Simulacra
  identity in an Enforced store, naming the isolated-home cure.
- **The T8 lane is isolated.** It runs under an isolated home, and its four leaked records are
  closed or retired.
