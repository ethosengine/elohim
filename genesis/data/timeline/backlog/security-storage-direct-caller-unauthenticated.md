---
id: "backlog-security-storage-direct-caller-unauthenticated"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-storage trusts any direct caller on :8090 — X-Agent-Cid is forgeable and PATCH /db/content/{id} re-signs through the node's conductor with no caller check"
slug: "security-storage-direct-caller-unauthenticated"
written: "2026-09-28"
author: "claude-opus-5-5 (found while designing steward-authorized seeding for FCT v2)"
status: "refined"
priority: "high"
area: "storage/http"
domain: "protocol"
jobs: [elohim-holochain]
relatedNodeIds:
  - "habit:reach-enforced-everywhere"
cites:
  - genesis/data/timeline/backlog/security-doorway-blob-pantry-ungated.md
  - genesis/data/timeline/backlog/security-content-head-route-bypasses-reach-gate.md
tags: [security, storage, reach, identity, network-policy]
---

# Storage trusts any direct caller

## What is true

Only the doorway authenticates a caller before injecting `X-Agent-Cid`
(`doorway/doorway-service/src/routes/storage_proxy.rs` ~630). elohim-storage itself
accepts connections with no caller authentication (`elohim/elohim-storage/src/http.rs`
~1871) and takes the header as given:

- the reach gate resolves the raw `X-Agent-Cid` (`api/content_reach_gate.rs` 237-251);
  `resolve_requester` / `resolve_writer` (`http.rs` ~18513 / ~18543) likewise;
- `PATCH /db/content/{id}` has **no caller check** (`http.rs` ~10215-10311): any caller
  that reaches :8090 can make the node's conductor re-sign a row at any reach
  (`content_service.rs` `update_via_conductor` ~440-540) — the signature is the node
  steward's;
- the head-declare POST checks authorship only against the forgeable header
  (`http.rs` ~9198-9245); the zome remains the real gate there.

Reachability of :8090 (`genesis/orchestrator/manifests/network-policies.yaml` 58-116):
same namespace, the `jenkins` namespace, and `ipBlock 10.1.0.0/16` (the pod network).
The only prior admission is one line in `security-doorway-blob-pantry-ungated.md`
("pre-existing direct-to-storage `X-Agent-Cid` trust").

## Why it matters

Anything on the pod network can read any reach tier by naming a human in a header,
and can have the steward's own conductor sign reach changes to that steward's content.
Reach is only as strong as the network boundary around :8090.

## Direction (not decided here)

Notarized writes and non-open reads on storage require a proof the caller can't mint:
a doorway-signed assertion, or a node-local capability (the constant-time
`ELOHIM_COMPUTE_LOCAL_TOKEN` pattern in `api/compute_tasks.rs` 21-36), plus narrowing
the network policy. Steward acts (seeding, publish) go through the steward's conductor
app interface, never through storage headers.

## Current decision

Recorded; not started. The FCT v2 steward-publish work (2026-09-28) deliberately routes
identity through the conductor WS and does not build on the header.
