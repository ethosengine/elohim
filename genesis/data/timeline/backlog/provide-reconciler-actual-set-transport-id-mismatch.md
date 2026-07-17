---
id: "backlog-provide-reconciler-actual-set-transport-id-mismatch"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Provide-reconciler actual set queried under transport id — dedup/revoke never sees real provider rows"
slug: "provide-reconciler-actual-set-transport-id-mismatch"
written: "2026-07-17"
author: "track-b card-lighting session"
status: "open"
priority: "medium"
---

# Provide-reconciler "actual" set queried under transport id — dedup never sees real provider rows

discovered: 2026-07-17 (joint verification of the rekey self-heal, Job 4)
domain: D-resilience / storage provide-loop / identity coherence
relates: `elohim/elohim-storage/CLAUDE.md` (Identity & Transport-Identity Coherence) · rekey self-heal (genesis_self_heal.rs stale-for-self arms) · provider-namespace hardening (conductor_commitment_author.rs resolve_provider)

## Symptom (pre-existing; NOT a regression of the 2026-07-17 changes)

`main.rs` threads `config.self_cid` — the node's **transport** identity (libp2p
`12D3Koo…` or iroh NodeId) — into `ProvideReconciler::reconcile_provides` as
`self_provider`. The reconciler's "actual" set is loaded via
`live_commons_provides_for_provider(self_provider)`
(`provide_reconcile.rs:307` → `db/mishpat_commitments.rs:203`,
`.filter(mc::provider.eq(provider))`), but real commitment rows carry the
**agent_cid** (`uhCAk…`) provider — `author_commons` resolves the provider
independently via `resolve_provider([session, cell key])` and never writes the
transport id (post-hardening). Cross-namespace equality ⇒ the actual set is
effectively always empty regardless of what has actually been authored.

## Effect

- The already-provided suppression check (`provide_reconcile.rs:322-334`,
  keyed `(provider, head_ref)`) cannot match genuine rows → the reconciler
  believes nothing is provided every tick.
- In-practice blast radius is bounded: the per-process latch and the
  day-bucketed deterministic commitment cid make re-authoring idempotent at
  the mishpat layer, so this reads as wasted authoring attempts / incoherent
  dedup rather than data corruption. But the dedup layer is load-bearing for
  revoke-on-unpin (`desired` vs `actual` diffing) — the revoke path can never
  find the row it should revoke, for the same reason.

## Fix shape (not implemented)

Thread the RESOLVED agent_cid (same source as `resolve_provider`: active
session key, else the pod's cell key `hc.agent_key_uhcak()`) into
`reconcile_provides` as `self_provider`, instead of `config.self_cid`. The
transport id remains correct for the commitment's idempotency-id string
(`provide:{self_cid}:{head_ref}`) — only the DB-side provider filter is in the
wrong namespace. Add a regression test: author under agent_cid, run reconcile,
assert the actual-set query finds the row (suppression fires) and unpin
triggers the revoke path.
