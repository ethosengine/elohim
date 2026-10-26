---
id: "backlog-gate-live-device-crossing"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Gate-live device crossing waits on path disclosure and bounded retention"
slug: "gate-live-device-crossing"
written: "2026-10-26"
author: "codex"
status: "open"
priority: "medium"
area: "gate / device channel / disclosure"
relatedNodeIds:
  - "gate-live-failure-channel-design"
  - "workspaces-cooperate-as-devices-design"
tags: [gate-live, steward-channel, disclosure, retention, constraint]
shift_objective: >-
  Obtain the disclosure ruling for failure loci and paths and declare a channel
  retention bound before adding gate-live events to device crossing. Prove the
  projection stays within the flat channel tree's per-node limit.
---

# Crossing requires a disclosure and retention ruling

chain: gate-live across a person's devices / between A: the private device journal → C: another authorized device follows the gate / missing node B: a disclosure ruling names which loci and paths may cross, and a retention bound keeps the channel projection within its node limit; probe: inspect the crossing bytes for forbidden paths and excerpts, then project the maximum retained event set and verify every node is at most 4 MiB / current state: open; both rulings are prerequisites to crossing.

Failure excerpts are device-only and never cross. Removing the excerpt is necessary but does not
decide whether the remaining file path, test name or source line is safe to disclose. An absolute
path can reveal a workspace, account name or repository layout; a test name can expose subject
matter. The crossing ruling must name the allowed locus representation and its scope explicitly.
The schema's ability to represent a path is not permission to put that path on a shared channel.

The existing channel also imposes a concrete storage constraint.
[project_channel](../../../../elohim/eprfs/eprfs-peer/src/channel.rs#L91) reads allowed events and
places their CID-named entries in one flat event node before wrapping it under `channel/events`.
Its node encoder uses `TreeLimits::default()`;
[the default node bound](../../../../elohim/eprfs/eprfs-core/src/tree.rs#L83) is `4 * 1024 * 1024`
bytes, or 4 MiB. The wrapper directories do not shard that event node. More event history can
therefore overflow the index even when each individual event is small.

Declare a retention bound against serialized node bytes, with a stated age or count policy and
its interaction with existing channel events. The local journal's last-twenty-runs policy does not
automatically prune the flow store used for crossing. Do not turn a refusal at the node limit into
silent event loss, and do not introduce an unreviewed tree layout as a side effect of enabling
gate-live. If sharding is needed, that is a separate substrate change with its own evidence.

Proposed acceptance includes allowed and forbidden loci, excerpt removal, worst-case event names,
and projection at and beyond the declared retention limit. A disconnected receiving device must
get an honest retained-history boundary when it returns. The local gate remains usable throughout.
The [workspaces design](../../../docs/superpowers/specs/2026-10-23-workspaces-cooperate-as-devices-design.md)
provides the channel context; none of these future crossing probes has run for this entry.
