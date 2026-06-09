---
name: project_mishpat_commitment_cid_is_entry_hash
description: "Mishpat commitment CID = entry_hash (the projection `cid`, used by bounded_by/graduation/revocation/fetch); action_hash is only the dht_anchor_hash null-guard — confusing them silently breaks every bounds-gate while per-task tests still pass"
metadata: 
  node_type: memory
  type: project
  originSessionId: dae47f08-130c-420b-b02f-7a704dea71d8
---

In the Mishpat→`mishpat_commitments` storage projection, a commitment row's `cid`
column = the Holochain **`entry_hash`** (stable content address), while
`dht_anchor_hash` = the **`action_hash`** (notarization provenance, the
fail-closed null-guard). EVERYTHING that references a commitment downstream keys
on `cid` = entry_hash: the EconomicEvent's `bounded_by`, `ProjectionCommitmentFetcher::fetch`,
`graduate_to_active`, `set_revoked_at`, and `pin.commitment_cid`.

The conductor's `create_commitment` returns BOTH (`CommitmentOutput { action_hash, entry_hash }`).
Returning `action_hash` as "the CID" (the natural-looking mistake) silently breaks
every bounds-gate: `fetch(cid)` queries the `cid` column with an action_hash →
finds nothing → `CommitmentNotFound` → the emit is refused on EVERY call. This
passed all per-task tests (the round-trip test asserted `dht_anchor_hash == action_hash`
separately) and was caught only by an integration-seam review. **Always return
`entry_hash` as the commitment CID.**

Two sibling traps from the same substrate (EPR Slice 2b, `feat/native-content-graph-seam`):
1. **Projection latency:** for a JUST-authored commitment, use `ConductorCommitmentFetcher`
   (reads the conductor directly via `get_commitment` — available immediately) for the
   emit's bounds-check, NOT `ProjectionCommitmentFetcher` (the `CommitmentCommitted`
   post_commit signal projects async, so the SQL row lags the conductor).
2. **Signal subscription gap:** the Mishpat `CommitmentCommitted` signal must be
   SUBSCRIBED in storage (`subscribe_mishpat_signals` → `handle_mishpat_signal`).
   This was a pre-existing 2a gap — `handle_mishpat_signal` existed and was tested
   on direct-row fixtures, but no live subscriber was wired, so `mishpat_commitments`
   never populated from real authoring → dedup query empty → unbounded re-author.

Related: [[project_rea_compute_commitment_primitive]], [[project_principle_p1_reconciliation_controller]].
