---
name: project_holochain_post_commit_signals_are_cell_local
title: "post_commit signals are cell-local — remote peers need re…"
id: project-holochain-post-commit-signals-are-cell-local
description: "post_commit/emit_signal fire only on the AUTHORING cell — a remote peer never gets a Committed signal via the DHT."
metadata: 
  node_type: memory
  title: post_commit signals are cell-local — remote peers need replay
  type: project
  originSessionId: 62953c2b-d161-4b7d-8ec3-88beb0ae56de
  modified: 2026-09-06T19:13:19.990Z
---

**The trap (caught in plan review 2026-09-06):** I planned a storage consumer for `FeedbackSignalCommitted`
on the steward's peer as "the DHT transport" for a correction authored on another peer. That never fires:
`post_commit` (and its `emit_signal`) runs on the authoring cell's own conductor and goes to that
conductor's app websocket. Gossiping the record to peer A does not invoke A's post_commit.

**Why it matters:** every "…Committed" projection arm in `elohim-storage/src/rea_projection.rs` is a
LOCAL projection of the peer's OWN writes. Any design that needs a remote peer to react to a witnessed
act must have (1) durable discovery/replay of the act from the DHT (zome query fns + a persisted cursor,
e.g. `get_feedback_signals_for_target` / `list_feedback_signals_by_signer` in
`content_store/src/feedback_signal.rs:304,331`) and (2) at most a notification (direct p2p or
`send_remote_signal`) that carries a REFERENCE (validation context + action hash) and accelerates the
same fetch path. Replay must work when every notification was missed.

**Also:** dedup identity for a witnessed act = (origin DNA hash, action hash) — see the second-round note below. Don't invent nonces or
new entry fields to get idempotency — that would move the DNA hash.

See [[project_head_reach_freshness_semantics]], [[feedback_verify_the_measure_before_the_ranking]].

**Second-round corrections (same review, 2026-09-06):** act identity is (ORIGIN DNA hash, action hash) — the
receiving cell/installation is routing context, never part of dedup (else one act gets N identities); link
enumeration (`get_*_for_target`) is NOT an ordered stream — no high-water cursor; use a persisted subscription
set + periodic enumeration + per-act consumption/retry rows, and test late arrival; a notification is a NEW
envelope carrying the reference — never replace the semantic domain type with a fetch hint; rebuild must reset
consumption state atomically with derived tables or replay reconstructs nothing; dedup after the act exists does
not cover commit-succeeded-response-lost — that needs a durable operation→act binding.
