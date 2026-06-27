---
id: "backlog-content-edit-dht-propagation-sweettest"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "content-edit DHT propagation sweettest — upgrade content_visible_across_agents to edit→visible (the OTHER sync plane)"
slug: "content-edit-dht-propagation-sweettest"
written: "2026-06-27"
author: "plan flow Step 1c(3) complementary-capture (automerge content-sync plane sprint)"
status: "backlog"
priority: "medium"
jobs: [elohim]
---

## What

A cheap regression proof for the **Holochain DHT** content-sync plane (distinct from the
Automerge/libp2p storage plane being lit in
`genesis/docs/superpowers/plans/2026-06-27-automerge-content-sync-plane-lighting-plan.md`).

The existing `content_visible_across_agents()` sweettest
(`elohim/holochain/tests/sweettest/src/tests/lamad.rs:177`) already passes (create→visible
across two conductors). Add a sibling `content_edit_visible_across_agents()` that:
conductor A creates a content node, B observes it, A calls the `update_content` extern
(`elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:2463`, which `update_entry`s
and re-points the `IdToContent` link), then B's `get_content_by_id(...)` polls until it
returns the **mutated** title within a 30s deadline.

## Why it's its own backlog item (not in the Automerge plan)

This proves a DIFFERENT plane (DHT gossip, not Automerge CRDT). The Automerge plan is
scoped to lighting the inert storage-sync plane; folding a DHT sweettest into it would blur
the "data syncing" scope. Captured here so the quick DHT-plane regression isn't lost.

## Pointers (verified during grounding 2026-06-27)

- Compose-from: `two_agent_conductors()` (`common/conductors.rs:66`), network seed
  `elohim_lamad_alpha` (`fixtures.rs:63`), the poll-until-deadline idiom (`lamad.rs:196-228`),
  the `CreateContentInput`/`QueryByIdInput` mirror pattern.
- New mirror struct needed: `UpdateContentInput` (wire shape at
  `elohim/sdk/domains/lamad/types/src/lib.rs:68` — minimal update = `id` + `title`).
- No `Cargo.toml` change (`[[test]] name = "lamad"` exists). Run with `RUSTFLAGS=""`.
- Risk: confirm `get_content_by_id` returns the NEWEST version after the `IdToContent`
  re-point (eventual-consistency poll must assert only on the final converged value).
- Confirm `elohim/holochain/dna/elohim/workdir/lamad.dna` is current with the `update_content`
  zome code; repack via `just pack` in `dna/elohim` if stale.

Domain D5 (data plane). Effort: S (~40 lines + one mirror struct).
