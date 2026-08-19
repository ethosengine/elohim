---
id: "backlog-doorway-agent-cid-resolver-namespace-mismatch"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doorway `resolve_agent_cid_from_request` returns `human_id`, not an agent CID"
slug: "doorway-agent-cid-resolver-namespace-mismatch"
written: "2026-08-19"
author: "claude (shift 2026-08-19T03-37-operator-positive-path-green)"
status: "backlog"
priority: "high"
cites:
  - doorway/doorway-service/src/server/http.rs
relatedNodeIds:
  - backlog-security-doorway-auth-required-unenforced
  - backlog-doorway-auth-required-metadata-unenforced
  - backlog-http-reach-enforcement-gap
tags: [security, identity-coherence, doorway, auth, agent-cid, http]
shift_objective: |
  Decide and implement the doorway agent-CID resolver split: either flip
  the shared resolve_agent_cid_from_request to prefer claims.agent_pub_key
  for the identity-scoped views (ForwardCtx.agent_cid -> X-Agent-Cid ->
  /cluster, /peer-topology, /reciprocity, private-blob reach), or key those
  views by human_id server-side instead. Verify against hosted
  (doorway-auth) users where human_id (UUID) currently matches no
  humans.agent_pub_key row. Retire or update the pinned regression test
  agent_cid_resolver_semantics_are_unchanged once the decision lands.
  Reckon with dev_mode's singleton conductor key (auth_routes.rs
  hosted-register fallback) before treating agent_pub_key as per-user
  locally.
---

# Doorway `resolve_agent_cid_from_request` returns `human_id`, not an agent CID

`doorway/doorway-service/src/server/http.rs` — the function named
`resolve_agent_cid_from_request` returns `claims.human_id` (the doorway-local
user UUID), not `claims.agent_pub_key`. Its comment ("alpha-substrate:
claims.human_id IS agent_cid") is falsified on alpha: matthew's session showed
`human_id = ba3a0a01-…` (UUID) vs `agentPubKey = uhCAk…`.

Two call sites inherit the mismatch (the third, membrane keying at ~http.rs:6048,
only needs a stable key and is unaffected):

- `ForwardCtx.agent_cid` → storage `X-Agent-Cid` → identity-scoped views
  (`/cluster`, `/peer-topology`, `/reciprocity`) and reach gating on private
  blobs. For doorway-auth (hosted) users these views are keyed by a UUID that
  matches no `humans.agent_pub_key` row — likely silently empty/misgated.

The op-gate call site was CURED 2026-08-19 (shift operator-positive-path-green)
with a dedicated `resolve_op_gate_performer_from_request` that prefers
`claims.agent_pub_key`; a pinned test (`agent_cid_resolver_semantics_are_unchanged`)
deliberately freezes the shared resolver's old behavior until this item decides
the identity-scoped-view story.

**Current decision needed:** flip the shared resolver to agent_pub_key-first
(and verify /cluster, /peer-topology, /reciprocity, private-blob reach against
hosted users), or key those views by human_id server-side. Canonical join key
per elohim-storage CLAUDE.md is `agent_cid`. Note also that in dev_mode every
hosted register shares the singleton conductor key (auth_routes.rs:1080-1096),
so agent_pub_key is NOT a per-user key locally — any flip must reckon with that.
